import "./hud.css";
import { listen } from "@tauri-apps/api/event";
import { api, asFailure, type HuntView } from "./api";
import { onLangChange, reason, t } from "./i18n";
import { setSound, soundCues, soundEnabled, unlockSound } from "./hunt-sound";
const dock = document.getElementById("dock")!;
let expanded = false;
let remaining = 0;
let more = false;
let working = false;
let message = "";
let hunt: HuntView | null = null;
let seq = Date.now();
const hotkeys = () => (navigator.userAgent.includes("Windows") ? t("Phím tắt: Space (hoặc Ctrl+Alt+Space)") : t("Phím tắt: Ctrl+Alt+Space"));
function button(label: string, run: () => Promise<unknown>) {
  const b = document.createElement("button"); b.textContent = label; b.disabled = working;
  b.onclick = async () => { if (working) return; working = true; message = ""; render();
    try { await run(); } catch (e) { message = reason(asFailure(e).code); }
    finally { working = false; render(); } };
  return b;
}
async function toggle() { await api.resizeDock(!expanded); expanded = !expanded; more = false; }
const nextSeq = () => (seq = Math.max(seq + 1, Date.now()));
const huntAction = (action: "drop" | "pause" | "resume" | "leave" | "keep" | "sell") => () => {
  const id = hunt?.session?.id;
  return id ? api.huntAction(id, nextSeq(), action).then((v) => { hunt = v; }) : Promise.resolve();
};
function huntStatus(v: HuntView): string {
  const s = v.session!;
  const left = v.batch?.shells.filter((x) => !x.collected).length ?? 0;
  if (s.error) return reason(s.error);
  if (s.paused) return t("Đã tạm dừng");
  if (s.phase === "deciding") return t("🎣 Cá lên thuyền rồi! Nuôi thêm hay bán?");
  if (s.phase === "swinging") return t("{left} sò đang chờ · Hôm nay {earned}/{cap}", { left, earned: v.earned, cap: v.daily_cap });
  return s.phase === "extending" ? t("Móc đang xuống…") : t("Đang kéo về thuyền…");
}
function renderHunt(v: HuntView) {
  const s = v.session!;
  const header = document.createElement("header"); header.textContent = t("ĐANG ĐÀO SÒ");
  const sound = button(soundEnabled() ? "🔊" : "🔇", () => setSound(!soundEnabled()));
  sound.title = soundEnabled() ? t("Âm thanh đào sò: Bật (bấm để tắt)") : t("Âm thanh đào sò: Tắt (bấm để bật)");
  sound.setAttribute("aria-label", sound.title); sound.setAttribute("aria-pressed", String(soundEnabled()));
  header.append(sound); dock.append(header);
  const status = document.createElement("p"); status.textContent = huntStatus(v); status.setAttribute("role", "status"); dock.append(status);
  if (s.phase === "deciding" && !s.paused) {
    const keep = button(t("🐟 Cá gầy quá nuôi thêm chút vậyyyy"), huntAction("keep"));
    const sell = button(t("💰 Yehh nay có cơm ăn rồiiii"), huntAction("sell"));
    sell.className = "primary";
    dock.append(sell, keep, button(t("Rời thuyền"), huntAction("leave")));
    return;
  }
  const canDrop = s.phase === "swinging" && !s.paused && (v.earned < v.daily_cap || s.fish.length > 0);
  const main = s.paused ? button(t("▶  Tiếp tục"), huntAction("resume")) : button(t("⚓  Thả móc"), huntAction("drop"));
  main.className = "primary"; if (!s.paused) main.disabled = working || !canDrop; dock.append(main);
  if (!s.paused) dock.append(button(t("⏸  Tạm dừng"), huntAction("pause")));
  dock.append(button(t("Rời thuyền"), huntAction("leave")));
  const tip = document.createElement("p"); tip.textContent = hotkeys(); dock.append(tip);
}
function render() {
  dock.replaceChildren();
  document.documentElement.classList.toggle("collapsed", !expanded);
  if (hunt?.session && expanded) { renderHunt(hunt); return; }
  if (!expanded) {
    const b = button("", toggle); b.title = t("Mở nút nhanh TrashQuarium"); b.setAttribute("aria-label", b.title); b.className = "anchor";
    const logo = document.createElement("img"); logo.src = "/logo.png"; logo.alt = ""; b.append(logo);
    if (remaining) { const badge = document.createElement("span"); badge.className = "badge"; badge.textContent = String(remaining); b.append(badge); }
    dock.append(b); return;
  }
  const header = document.createElement("header"); header.textContent = "TRASHQUARIUM";
  header.append(button("−", toggle)); dock.append(header);
  if (more) {
    dock.append(button(t("Bụng cá"), () => api.openTab("belly")), button(t("Cài đặt / Mở cùng Windows"), () => api.openTab("settings")),
      button(t("Ẩn nút nhanh"), () => api.setDock(false)), button(t("← Quay lại"), async () => { more = false; }));
  } else {
    dock.append(button(t("🐟  Cho cá ăn"), () => api.openTab("feed")),
      button(t("⛵  Gọi thuyền") + (remaining ? t(" · {n} sò", { n: remaining }) : ""), () => api.startHunt().catch(() => api.openTab("hunt"))),
      button(t("🐠  Mua cá"), () => api.openTab("shop")), button(t("⋯  Thêm…"), async () => { more = true; }));
  }
  if (message) { const p = document.createElement("p"); p.textContent = message; p.setAttribute("role", "status"); dock.append(p); }
}
async function refresh() {
  const view = await api.huntStatus();
  const started = !!view.session && !hunt?.session;
  soundCues(hunt, view);
  hunt = view; remaining = view.batch?.shells.filter((s) => !s.collected).length ?? 0;
  // A new session opens the dock so the hunt controls are right there.
  if (started && !expanded) { await api.resizeDock(true); expanded = true; more = false; }
  render();
}
// Browsers only allow audio after a gesture: any click on the dock unlocks it.
document.addEventListener("pointerdown", () => { void unlockSound(); });
onLangChange(() => render());
void listen("state-changed", () => refresh().catch(() => {}));
// While hunting, follow the claw so the Drop button is only live when it can be used.
window.setInterval(() => { if (hunt?.session && !working) refresh().catch(() => {}); }, 200);
void refresh().catch(() => render());
