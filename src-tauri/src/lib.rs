mod engine;
mod desktop;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, Wry};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::engine::app::{AppCore, FeedReport, StateView};
use crate::engine::belly::{BellyEntry, RestoreOutcome};
use crate::engine::catalog::Catalog;
use crate::engine::game::Fish;
use crate::engine::guard::{GuardPolicy, Inspection};
use crate::engine::Failure;

type Core = Arc<Mutex<AppCore>>;

/// Refuses a second mutation while one is running (e.g. a double-click on Buy).
#[derive(Default)]
struct Busy(AtomicBool);

struct BusyGuard<'a>(&'a AtomicBool);

impl Busy {
    fn enter(&self) -> Result<BusyGuard<'_>, Failure> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| BusyGuard(&self.0))
            .map_err(|_| Failure::new("busy", "another action is running"))
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct TrayItems {
    tank: CheckMenuItem<Wry>,
    meeting: CheckMenuItem<Wry>,
}

/// Runs `f` on a blocking thread with the core locked. File moves and saves
/// never run on the UI thread.
async fn with_core<T: Send + 'static>(core: &Core, f: impl FnOnce(&mut AppCore) -> T + Send + 'static) -> T {
    let core = core.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // State only changes after a successful save, so a poisoned lock still holds consistent data.
        let mut guard = core.lock().unwrap_or_else(|p| p.into_inner());
        f(&mut guard)
    })
    .await
    .expect("core task panicked")
}

fn notify(app: &AppHandle) {
    let _ = app.emit("state-changed", ());
}

fn show_manager(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("manager") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[derive(Default)]
struct PendingRoute(Mutex<Option<String>>);

#[tauri::command]
fn take_manager_route(route: State<'_, PendingRoute>) -> Option<String> {
    route.0.lock().unwrap_or_else(|p| p.into_inner()).take()
}

#[tauri::command]
fn open_manager_tab(app: AppHandle, tab: String) -> Result<(), Failure> {
    if !["shop", "feed", "belly", "tank", "settings", "hunt"].contains(&tab.as_str()) { return Err(Failure::new("bad_tab", "")); }
    *app.state::<PendingRoute>().0.lock().unwrap_or_else(|p| p.into_inner()) = Some(tab);
    show_manager(&app);
    let _ = app.emit("manager-route", ());
    Ok(())
}

#[tauri::command]
async fn hunt_status(core: State<'_, Core>) -> Result<engine::hunt::HuntView, Failure> { Ok(with_core(&core, |c| c.game.hunt_view()).await) }

/// One key for the whole hunt: resumes when paused, drops the claw while it swings.
const HUNT_KEY: &str = "ctrl+alt+space";

/// True while the hunt hotkey is registered (it is only held during a session).
#[derive(Default)]
struct HuntKey(AtomicBool);

fn register_hunt_key(app: &AppHandle) {
    let flag = app.state::<HuntKey>();
    if flag.0.swap(true, Ordering::AcqRel) { return; }
    let result = app.global_shortcut().on_shortcut(HUNT_KEY, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed { hunt_hotkey(app); }
    });
    // Another program may own the shortcut; the dock buttons still work.
    if result.is_err() { flag.0.store(false, Ordering::Release); }
}

fn unregister_hunt_key(app: &AppHandle) {
    if app.state::<HuntKey>().0.swap(false, Ordering::AcqRel) { let _ = app.global_shortcut().unregister(HUNT_KEY); }
}

fn hunt_hotkey(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let core: Core = app.state::<Core>().inner().clone();
        let _ = with_core(&core, |c| {
            let (id, paused, phase, last) = c.game.hunt_session.as_ref().map(|s| (s.id.clone(), s.paused, s.phase.clone(), s.last_command))?;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            let action = if paused { "resume" } else if phase == "swinging" { "drop" } else { return None };
            c.game.hunt_action(&id, now.max(last + 1), action).ok()
        }).await;
    });
}

#[tauri::command]
async fn start_hunt(app: AppHandle, window: tauri::WebviewWindow, core: State<'_, Core>, busy: State<'_, Busy>) -> Result<engine::hunt::HuntView, Failure> {
    let _guard = busy.enter()?;
    let view = with_core(&core, |c| c.game.start_hunt(&engine::local_today())).await?;
    // The hunt is played on the desktop ocean itself, so that must be on.
    if !with_core(&core, |c| c.game.state.settings.tank_enabled).await {
        if let Err(e) = apply_tank(&app, core.inner().clone(), true).await {
            with_core(&core, |c| c.game.hunt_session = None).await;
            return Err(e);
        }
    }
    register_hunt_key(&app);
    // Started from the manager: get it out of the way so the desktop shows.
    if window.label() == "manager" { let _ = window.minimize(); }
    notify(&app);
    Ok(view)
}

#[tauri::command]
async fn hunt_action(app: AppHandle, core: State<'_, Core>, session_id: String, seq: u64, action: String) -> Result<engine::hunt::HuntView, Failure> {
    let leaving = action == "leave";
    let view = with_core(&core, move |c| c.game.hunt_action(&session_id, seq, &action)).await;
    if leaving { unregister_hunt_key(&app); notify(&app); }
    view
}

#[tauri::command]
async fn set_quick_dock(app: AppHandle, core: State<'_, Core>, enabled: bool) -> Result<StateView, Failure> {
    let view = with_core(&core, move |c| c.game.update_settings(|s| s.quick_dock_enabled = enabled).map(|_| c.view())).await?;
    if !enabled { if let Some(w) = app.get_webview_window("hud") { let _ = w.hide(); } }
    notify(&app); Ok(view)
}

#[tauri::command]
fn resize_quick_dock(app: AppHandle, expanded: bool) -> Result<(), Failure> { desktop::resize_hud(&app, expanded).map_err(|e| Failure::new("dock_failed", e)) }

#[tauri::command]
fn autostart_status(app: AppHandle) -> Result<bool, Failure> { app.autolaunch().is_enabled().map_err(|e| Failure::new("autostart_failed", e)) }

#[tauri::command]
fn set_autostart(app: AppHandle, window: tauri::WebviewWindow, enabled: bool) -> Result<bool, Failure> {
    if window.label() != "manager" { return Err(Failure::new("not_allowed", "")); }
    if cfg!(debug_assertions) { return Err(Failure::new("autostart_dev", "use installed release")); }
    if enabled { app.autolaunch().enable() } else { app.autolaunch().disable() }.map_err(|e| Failure::new("autostart_failed", e))?;
    autostart_status(app)
}

fn start_hunt_clock(app: AppHandle, core: Core) {
    std::thread::spawn(move || {
        let mut previous = std::time::Instant::now();
        let mut dock_visible = false;
        let mut dock_tick = 0;
        let mut hunting = false;
        let mut was_hunting = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(previous).as_millis() as u64; previous = now;
            // The hunt only runs while the desktop it is drawn on is in front.
            let away = hunting && !desktop::hunt_surface_ready(&app);
            let mut changed = false;
            let mut desired_dock = None;
            if let Ok(mut c) = core.try_lock() {
                if away || elapsed > 2000 {
                    if let Some(s) = &mut c.game.hunt_session { s.paused = true; }
                }
                changed = c.game.tick_hunt(if elapsed > 2000 { 0 } else { elapsed }, &engine::local_today()).unwrap_or(false);
                hunting = c.game.hunt_session.is_some();
                dock_tick += 1;
                if dock_tick >= 5 {
                    dock_tick = 0;
                    desired_dock = Some(c.game.state.settings.tank_enabled && (c.game.state.settings.quick_dock_enabled || hunting)
                        && !c.game.state.settings.meeting_mode && desktop::desktop_foreground(&app));
                }
            }
            // Native window operations may dispatch to the main thread. Never
            // hold the core lock while doing them (especially during app exit).
            if hunting != was_hunting {
                if hunting { register_hunt_key(&app); } else { unregister_hunt_key(&app); }
                was_hunting = hunting;
            }
            if changed { notify(&app); }
            if let Some(visible) = desired_dock {
                if visible != dock_visible {
                    if let Some(w) = app.get_webview_window("hud") { if visible { let _ = w.show(); } else { let _ = w.hide(); } }
                    dock_visible = visible;
                }
            }
        }
    });
}

#[tauri::command]
async fn get_state(core: State<'_, Core>) -> Result<StateView, Failure> {
    Ok(with_core(&core, |c| c.view()).await)
}

#[tauri::command]
async fn preview_files(core: State<'_, Core>, paths: Vec<String>) -> Result<Vec<Inspection>, Failure> {
    Ok(with_core(&core, move |c| c.preview(&paths)).await)
}

#[tauri::command]
async fn feed(
    app: AppHandle,
    core: State<'_, Core>,
    busy: State<'_, Busy>,
    items: Vec<Inspection>,
    fish_id: String,
) -> Result<FeedReport, Failure> {
    let _guard = busy.enter()?;
    let report = with_core(&core, move |c| c.feed(&items, &fish_id)).await;
    notify(&app);
    report
}

#[tauri::command]
async fn buy(
    app: AppHandle,
    core: State<'_, Core>,
    busy: State<'_, Busy>,
    species_id: String,
    price: u64,
) -> Result<Fish, Failure> {
    let _guard = busy.enter()?;
    let fish = with_core(&core, move |c| c.game.purchase(&species_id, price, engine::now_unix())).await;
    notify(&app);
    fish
}

#[tauri::command]
async fn belly_list(core: State<'_, Core>) -> Result<Vec<BellyEntry>, Failure> {
    Ok(with_core(&core, |c| c.held_entries()).await)
}

#[tauri::command]
async fn belly_restore(
    app: AppHandle,
    core: State<'_, Core>,
    busy: State<'_, Busy>,
    entry_id: String,
) -> Result<RestoreOutcome, Failure> {
    let _guard = busy.enter()?;
    let outcome = with_core(&core, move |c| c.restore(&entry_id)).await;
    notify(&app);
    outcome
}

#[tauri::command]
async fn belly_recover(app: AppHandle, core: State<'_, Core>, busy: State<'_, Busy>) -> Result<StateView, Failure> {
    let _guard = busy.enter()?;
    let view = with_core(&core, |c| {
        c.recover();
        c.view()
    })
    .await;
    notify(&app);
    Ok(view)
}

#[tauri::command]
async fn set_tank(app: AppHandle, core: State<'_, Core>, enabled: bool) -> Result<StateView, Failure> {
    apply_tank(&app, core.inner().clone(), enabled).await
}

async fn apply_tank(app: &AppHandle, core: Core, enabled: bool) -> Result<StateView, Failure> {
    if enabled {
        let (fail_app, fail_core) = (app.clone(), core.clone());
        desktop::show_tank(app, move |reason| {
            tauri::async_runtime::spawn(async move {
                let _ = with_core(&fail_core, |c| c.game.update_settings(|s| s.tank_enabled = false)).await;
                sync_tray(&fail_app, &fail_core).await;
                let _ = fail_app.emit("tank-error", reason);
                notify(&fail_app);
            });
        })
        .map_err(|e| Failure::new("tank_failed", e))?;
    } else {
        desktop::hide_tank(app);
    }
    let result = with_core(&core, move |c| c.game.update_settings(|s| s.tank_enabled = enabled).map(|_| c.view())).await;
    sync_tray(app, &core).await;
    notify(app);
    result
}

#[tauri::command]
async fn set_meeting_mode(app: AppHandle, core: State<'_, Core>, enabled: bool) -> Result<StateView, Failure> {
    let result = with_core(&core, move |c| c.game.update_settings(|s| s.meeting_mode = enabled).map(|_| c.view())).await;
    sync_tray(&app, core.inner()).await;
    notify(&app);
    result
}

#[tauri::command]
async fn finish_onboarding(app: AppHandle, core: State<'_, Core>) -> Result<StateView, Failure> {
    let result = with_core(&core, |c| c.game.update_settings(|s| s.onboarding_done = true).map(|_| c.view())).await;
    notify(&app);
    result
}

/// Writes a harmless practice file so the Belly can be tried without real files.
#[tauri::command]
async fn create_sample_file() -> Result<String, Failure> {
    let dir = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| Failure::new("no_documents_dir", ""))?
        .join("TrashQuarium Samples");
    std::fs::create_dir_all(&dir).map_err(|e| Failure::new("sample_failed", e))?;
    for n in 1..1000 {
        let path = dir.join(format!("thu-cho-ca-an-{n}.txt"));
        let file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path);
        if let Ok(mut file) = file {
            use std::io::Write;
            file.write_all("Đây là file mẫu của TrashQuarium. Cho cá ăn rồi nhả ra lại để thử Bụng cá.\n".as_bytes())
                .map_err(|e| Failure::new("sample_failed", e))?;
            return Ok(path.to_string_lossy().into_owned());
        }
    }
    Err(Failure::new("sample_failed", "no free name"))
}

#[tauri::command]
fn system_idle_seconds() -> f64 {
    desktop::idle_seconds()
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

async fn sync_tray(app: &AppHandle, core: &Core) {
    let settings = with_core(core, |c| c.game.state.settings.clone()).await;
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.tank.set_checked(settings.tank_enabled);
        let _ = items.meeting.set_checked(settings.meeting_mode);
    }
}

fn data_root() -> PathBuf {
    // A separate QA profile can be pointed at with TRASHQUARIUM_DATA_DIR.
    if let Some(dir) = std::env::var_os("TRASHQUARIUM_DATA_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    dirs::data_local_dir()
        .expect("no local data directory")
        .join("TrashQuarium")
}

fn build_tray(app: &AppHandle, core: &Core) -> tauri::Result<()> {
    let settings = core.lock().unwrap_or_else(|p| p.into_inner()).game.state.settings.clone();
    let open = MenuItem::with_id(app, "open", "Mở TrashQuarium", true, None::<&str>)?;
    let tank = CheckMenuItem::with_id(app, "tank", "Bể cá desktop", true, settings.tank_enabled, None::<&str>)?;
    let meeting = CheckMenuItem::with_id(app, "meeting", "Chế độ họp (giảm chuyển động)", true, settings.meeting_mode, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Thoát TrashQuarium", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &tank, &meeting, &PredefinedMenuItem::separator(app)?, &quit])?;
    app.manage(TrayItems { tank, meeting });
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("TrashQuarium")
        .menu(&menu)
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(|app, event| {
        let app = app.clone();
        let core: Core = app.state::<Core>().inner().clone();
        match event.id.as_ref() {
            "open" => show_manager(&app),
            "quit" => app.exit(0),
            "tank" => {
                tauri::async_runtime::spawn(async move {
                    let enabled = !with_core(&core, |c| c.game.state.settings.tank_enabled).await;
                    let _ = apply_tank(&app, core, enabled).await;
                });
            }
            "meeting" => {
                tauri::async_runtime::spawn(async move {
                    let _ = with_core(&core, |c| {
                        let on = !c.game.state.settings.meeting_mode;
                        c.game.update_settings(|s| s.meeting_mode = on)
                    })
                    .await;
                    sync_tray(&app, &core).await;
                    notify(&app);
                });
            }
            _ => {}
        }
    })
    .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| { if !args.iter().any(|a| a == "--autostart") { show_manager(app); } }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .setup(|app| {
            let catalog = Catalog::bundled().map_err(|e| format!("bundled catalog invalid: {e}"))?;
            let root = data_root();
            let policy = GuardPolicy::for_system(root.clone(), catalog.balance.max_file_bytes, catalog.balance.fingerprint_bytes);
            let core: Core = Arc::new(Mutex::new(AppCore::open(root, policy, catalog)));
            let tank_on = core.lock().unwrap_or_else(|p| p.into_inner()).game.state.settings.tank_enabled;
            app.manage(core.clone());
            app.manage(Busy::default());
            app.manage(PendingRoute::default());
            app.manage(HuntKey::default());
            build_tray(app.handle(), &core)?;
            #[cfg(windows)]
            desktop::ensure_hud(app.handle()).map_err(std::io::Error::other)?;
            start_hunt_clock(app.handle().clone(), core.clone());
            if !std::env::args().any(|a| a == "--autostart") { show_manager(app.handle()); }
            if tank_on {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = apply_tank(&handle, core, true).await;
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            preview_files,
            feed,
            buy,
            belly_list,
            belly_restore,
            belly_recover,
            set_tank,
            set_meeting_mode,
            finish_onboarding,
            create_sample_file,
            system_idle_seconds,
            quit_app,
            hunt_status, start_hunt, hunt_action,
            open_manager_tab, take_manager_route,
            set_quick_dock, resize_quick_dock,
            autostart_status, set_autostart,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build TrashQuarium");
    app.run(|app, event| {
        if let tauri::RunEvent::Exit = &event {
            let core = app.state::<Core>();
            if let Ok(mut c) = core.lock() { let _ = c.game.checkpoint_hunt(); };
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            show_manager(app);
        }
        let _ = (app, event);
    });
}
