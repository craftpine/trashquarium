import "./hud.css";
import { listen } from "@tauri-apps/api/event";
import { api, asFailure } from "./api";
import { reason } from "./i18n";
const dock = document.getElementById("dock")!;
let expanded = false;
let remaining = 0;
let more = false;
let working = false;
let message = "";
function button(label: string, run: () => Promise<unknown>) {
  const b = document.createElement("button"); b.textContent = label; b.disabled = working;
  b.onclick = async () => { if (working) return; working = true; message = ""; render();
    try { await run(); } catch (e) { message = reason(asFailure(e).code); }
    finally { working = false; render(); } };
  return b;
}
async function toggle() { await api.resizeDock(!expanded); expanded = !expanded; more = false; }
function render() {
  dock.replaceChildren();
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
async function refresh() { const view = await api.huntStatus(); remaining = view.batch?.shells.filter((s) => !s.collected).length ?? 0; render(); }
void listen("state-changed", () => refresh().catch(() => {}));
void refresh().catch(() => render());
