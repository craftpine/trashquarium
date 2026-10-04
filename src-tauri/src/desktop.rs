//! The desktop tank: a full-screen window placed below the desktop icons so
//! other apps and the icons keep working normally. If attaching fails the
//! window is closed and the error reported; it never falls back to an
//! always-on-top full-screen window.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const TANK: &str = "tank";

/// A small input window, separate from the full-screen click-through ocean.
pub fn ensure_hud(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("hud").is_some() { return Ok(()); }
    let monitor = app.primary_monitor().map_err(|e| e.to_string())?.ok_or("no monitor")?;
    let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let pos = monitor.position().to_logical::<f64>(monitor.scale_factor());
    let w = WebviewWindowBuilder::new(app, "hud", WebviewUrl::App("hud.html".into()))
        .title("TrashQuarium Quick Dock").decorations(false).resizable(false)
        .skip_taskbar(true).focused(false).visible(false).shadow(false)
        .position(pos.x + size.width - 80.0, pos.y + size.height - 120.0)
        .inner_size(56.0, 56.0).build().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let hwnd = w.hwnd().map_err(|e| e.to_string())?.0 as windows_sys::Win32::Foundation::HWND;
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW as isize | WS_EX_NOACTIVATE as isize);
    }
    let _ = w;
    Ok(())
}

pub fn resize_hud(app: &AppHandle, expanded: bool) -> Result<(), String> {
    let w = app.get_webview_window("hud").ok_or("dock closed")?;
    let monitor = w.current_monitor().map_err(|e| e.to_string())?.ok_or("no monitor")?;
    let area = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let origin = monitor.position().to_logical::<f64>(monitor.scale_factor());
    let (width, height) = if expanded { (250.0, 254.0) } else { (56.0, 56.0) };
    w.set_size(tauri::LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    w.set_position(tauri::LogicalPosition::new(origin.x + area.width - width - 24.0, origin.y + area.height - height - 64.0)).map_err(|e| e.to_string())
}

/// Show the dock only on the desktop (or while using the dock itself).
pub fn desktop_foreground(app: &AppHandle) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let foreground = GetForegroundWindow();
        if foreground.is_null() { return false; }
        if let Some(w) = app.get_webview_window("hud") {
            if w.hwnd().ok().is_some_and(|h| h.0 as windows_sys::Win32::Foundation::HWND == foreground) { return true; }
        }
        let mut name = [0u16; 128];
        let n = GetClassNameW(foreground, name.as_mut_ptr(), name.len() as i32);
        let class = String::from_utf16_lossy(&name[..n.max(0) as usize]);
        return matches!(class.as_str(), "Progman" | "WorkerW");
    }
    #[cfg(not(windows))]
    { let _ = app; false }
}

/// Opens the tank. Attachment finishes asynchronously on the main thread;
/// `on_fail` receives the reason if it does not work.
pub fn show_tank(app: &AppHandle, on_fail: impl FnOnce(String) + Send + 'static) -> Result<(), String> {
    if app.get_webview_window(TANK).is_some() {
        return Ok(()); // already attached (or attaching); see the note on show() below
    }
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("no monitor")?;
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let pos = monitor.position().to_logical::<f64>(scale);
    let window = WebviewWindowBuilder::new(app, TANK, WebviewUrl::App("tank.html".into()))
        .title("TrashQuarium Ocean")
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .focused(false)
        .shadow(false)
        .visible(false)
        .visible_on_all_workspaces(true)
        .disable_drag_drop_handler()
        .position(pos.x, pos.y)
        .inner_size(size.width, size.height)
        .build()
        .map_err(|e| e.to_string())?;
    window.set_ignore_cursor_events(true).map_err(|e| e.to_string())?;
    let w = window.clone();
    app.run_on_main_thread(move || match attach(&w) {
        Ok(()) => {
            // On Windows attach() shows the window itself: tao's show() rewrites
            // the window styles, dropping WS_CHILD and lifting the ocean over the icons.
            #[cfg(not(windows))]
            let _ = w.show();
        }
        Err(reason) => {
            let _ = w.close();
            on_fail(reason);
        }
    })
    .map_err(|e| e.to_string())
}

/// Closes the tank window entirely so nothing renders while it is off.
pub fn hide_tank(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(TANK) {
        let _ = w.close();
    }
}

#[cfg(target_os = "macos")]
fn attach(window: &WebviewWindow) -> Result<(), String> {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowLevelForKey(key: i32) -> i32;
    }
    const DESKTOP_WINDOW_LEVEL_KEY: i32 = 2; // kCGDesktopWindowLevelKey: above wallpaper, below icons
    const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
    const STATIONARY: usize = 1 << 4;
    const IGNORES_CYCLE: usize = 1 << 6;

    let ns = window.ns_window().map_err(|e| e.to_string())? as *mut AnyObject;
    if ns.is_null() {
        return Err("no native window".into());
    }
    // SAFETY: `ns` is the live NSWindow of this Tauri window and we are on the main thread.
    unsafe {
        let level = CGWindowLevelForKey(DESKTOP_WINDOW_LEVEL_KEY) as isize;
        let () = msg_send![ns, setLevel: level];
        let () = msg_send![ns, setCollectionBehavior: CAN_JOIN_ALL_SPACES | STATIONARY | IGNORES_CYCLE];
    }
    Ok(())
}

/// Port of the 0.2 WinForms host: parent the window to the WorkerW behind the
/// desktop icons, or (Windows 11 24H2 "raised desktop") into Progman below
/// SHELLDLL_DefView. Explorer's own windows are never modified.
#[cfg(windows)]
fn attach(window: &WebviewWindow) -> Result<(), String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe extern "system" fn find_worker(top: HWND, out: LPARAM) -> i32 {
        let defview = wide("SHELLDLL_DefView");
        let worker = wide("WorkerW");
        if !FindWindowExW(top, null_mut(), defview.as_ptr(), std::ptr::null()).is_null() {
            *(out as *mut HWND) = FindWindowExW(null_mut(), top, worker.as_ptr(), std::ptr::null());
        }
        1
    }

    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    // SAFETY: plain Win32 calls on window handles; `worker` outlives EnumWindows.
    unsafe {
        let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
        if progman.is_null() {
            return Err("desktop_not_found".into());
        }
        let mut result = 0usize;
        SendMessageTimeoutW(progman, 0x052C, 0, 0, SMTO_NORMAL, 1000, &mut result);
        SendMessageTimeoutW(progman, 0x052C, 0xD, 1, SMTO_NORMAL, 1000, &mut result);
        let mut worker: HWND = null_mut();
        EnumWindows(Some(find_worker), &mut worker as *mut HWND as LPARAM);
        let mut icons: HWND = null_mut();
        if worker.is_null() && !FindWindowExW(progman, null_mut(), wide("WorkerW").as_ptr(), std::ptr::null()).is_null() {
            icons = FindWindowExW(progman, null_mut(), wide("SHELLDLL_DefView").as_ptr(), std::ptr::null());
            if !icons.is_null() {
                worker = progman;
            }
        }
        if worker.is_null() {
            return Err("wallpaper_layer_unavailable".into());
        }
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        ShowWindow(hwnd, SW_HIDE);
        SetWindowLongPtrW(hwnd, GWL_STYLE, (style | WS_CHILD as isize) & !(WS_POPUP as isize));
        // Always layered: tao already marks the window layered for click-through,
        // and a layered window draws nothing until its attributes are set.
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED as isize);
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        SetParent(hwnd, worker);
        if GetParent(hwnd) != worker {
            return Err("attach_failed".into());
        }
        let mut area: RECT = std::mem::zeroed();
        GetClientRect(worker, &mut area);
        let flags = SWP_FRAMECHANGED | SWP_NOACTIVATE | if icons.is_null() { SWP_NOZORDER } else { 0 };
        SetWindowPos(hwnd, icons, 0, 0, area.right, area.bottom, flags);
        ShowWindow(hwnd, SW_SHOWNA);
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn attach(_: &WebviewWindow) -> Result<(), String> {
    Err("unsupported_platform".into())
}

/// Seconds since the last keyboard/mouse input anywhere on the system.
pub fn idle_seconds() -> f64 {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
        }
        // kCGEventSourceStateCombinedSessionState, kCGAnyInputEventType
        // SAFETY: pure query with constant arguments.
        return unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX) };
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::GetTickCount;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        // SAFETY: `info` is a properly sized out-parameter.
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            return 0.0;
        }
        return unsafe { GetTickCount() }.wrapping_sub(info.dwTime) as f64 / 1000.0;
    }
    #[allow(unreachable_code)]
    0.0
}
