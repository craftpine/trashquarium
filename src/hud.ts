import "./hud.css";
import { listen } from "@tauri-apps/api/event";
import { api, asFailure, type HuntView } from "./api";
import { reason } from "./i18n";
const dock = document.getElementById("dock")!;
let expanded = false;
let remaining = 0;
let more = false;
let working = false;
let message = "";
let hunt: HuntView | null = null;
let seq = Date.now();
function button(label: string, run: () => Promise<unknown>) {
  const b = document.createElement("button"); b.textContent = label; b.disabled = working;
  b.onclick = async () => { if (working) return; working = true; message = ""; render();
    try { await run(); } catch (e) { message = reason(asFailure(e).code); }
    finally { working = false; render(); } };
  return b;
}
async function toggle() { await api.resizeDock(!expanded); expanded = !expanded; more = false; }
const nextSeq = () => (seq = Math.max(seq + 1, Date.now()));
const huntAction = (action: "drop" | "pause" | "resume" | "leave") => () => {
  const id = hunt?.session?.id;
  return id ? api.huntAction(id, nextSeq(), action).then((v) => { hunt = v; }) : Promise.resolve();
};
function huntStatus(v: HuntView): string {
  const s = v.session!;
  const left = v.batch?.shells.filter((x) => !x.collected).length ?? 0;
  if (s.error) return reason(s.error);
  if (s.paused) return "Đã tạm dừng";
  if (s.phase === "swinging") return `${left} sò đang chờ · Hôm nay ${v.earned}/${v.daily_cap}`;
  return s.phase === "extending" ? "Móc đang xuống…" : "Đang kéo về thuyền…";
}
function renderHunt(v: HuntView) {
  const s = v.session!;
  const header = document.createElement("header"); header.textContent = "ĐANG ĐÀO SÒ"; dock.append(header);
  const status = document.createElement("p"); status.textContent = huntStatus(v); status.setAttribute("role", "status"); dock.append(status);
  const canDrop = s.phase === "swinging" && !s.paused && v.earned < v.daily_cap;
  const main = s.paused ? button("▶  Tiếp tục", huntAction("resume")) : button("⚓  Thả móc", huntAction("drop"));
  main.className = "primary"; if (!s.paused) main.disabled = working || !canDrop; dock.append(main);
  if (!s.paused) dock.append(button("⏸  Tạm dừng", huntAction("pause")));
  dock.append(button("Rời thuyền", huntAction("leave")));
  const tip = document.createElement("p"); tip.textContent = "Phím tắt: Ctrl+Alt+Space"; dock.append(tip);
}
function render() {
  dock.replaceChildren();
  if (hunt?.session && expanded) { renderHunt(hunt); return; }
  if (!expanded) {
    const b = button(remaining ? `⛵ ${remaining}` : "⛵", toggle); b.title = "Mở nút nhanh TrashQuarium"; b.setAttribute("aria-label", b.title); b.className = "anchor"; dock.append(b); return;
  }
  const header = document.createElement("header"); header.textContent = "TRASHQUARIUM";
  header.append(button("−", toggle)); dock.append(header);
  if (more) {
    dock.append(button("Bụng cá", () => api.openTab("belly")), button("Cài đặt / Mở cùng Windows", () => api.openTab("settings")),
      button("Ẩn nút nhanh", () => api.setDock(false)), button("← Quay lại", async () => { more = false; }));
  } else {
    dock.append(button("🐟  Cho cá ăn", () => api.openTab("feed")),
      button(`⛵  Gọi thuyền${remaining ? ` · ${remaining} sò` : ""}`, () => api.startHunt().catch(() => api.openTab("hunt"))),
      button("🐠  Mua cá", () => api.openTab("shop")), button("⋯  Thêm…", async () => { more = true; }));
  }
  if (message) { const p = document.createElement("p"); p.textContent = message; p.setAttribute("role", "status"); dock.append(p); }
}
async function refresh() {
  const view = await api.huntStatus();
  const started = !!view.session && !hunt?.session;
  hunt = view; remaining = view.batch?.shells.filter((s) => !s.collected).length ?? 0;
  // A new session opens the dock so the hunt controls are right there.
  if (started && !expanded) { await api.resizeDock(true); expanded = true; more = false; }
  render();
}
void listen("state-changed", () => refresh().catch(() => {}));
// While hunting, follow the claw so the Drop button is only live when it can be used.
window.setInterval(() => { if (hunt?.session && !working) refresh().catch(() => {}); }, 200);
void refresh().catch(() => render());
