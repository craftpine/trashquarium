import "./hunt.css";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, asFailure, type HuntView } from "./api";
import { reason } from "./i18n";

const canvas = document.getElementById("hunt-canvas") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;
const drop = document.getElementById("drop") as HTMLButtonElement;
const pause = document.getElementById("pause") as HTMLButtonElement;
const status = document.getElementById("status")!;
const wallet = document.getElementById("wallet")!;
const win = getCurrentWindow();
let view: HuntView | null = null;
let shells = 0;
let seq = Date.now();
let busy = false;
let leaving = false;
let error = "";
let last = 0;
const ocean = new Image(); ocean.src = "/art/ocean.jpg";

async function action(action: "drop" | "pause" | "resume" | "leave") {
  if (!view?.session || busy) return;
  busy = true;
  try { view = await api.huntAction(view.session.id, ++seq, action); error = ""; }
  catch (e) { error = reason(asFailure(e).code); }
  finally { busy = false; }
}
async function leave() {
  if (busy) return;
  await action("leave"); leaving = true; await win.close();
}
drop.addEventListener("click", () => action("drop"));
canvas.addEventListener("click", () => action("drop"));
pause.addEventListener("click", () => action(view?.session?.paused ? "resume" : "pause"));
document.getElementById("leave")!.addEventListener("click", leave);
window.addEventListener("keydown", (e) => {
  if (e.code === "Space" && e.target === document.body) { e.preventDefault(); if (!e.repeat) void action("drop"); }
  if (e.code === "Escape") void action("pause");
});
window.addEventListener("blur", () => action("pause"));

function drawShell(x: number, y: number, size: number, now: number) {
  const r = 10 + size * 2;
  ctx.save(); ctx.translate(x, y); ctx.rotate(Math.sin(x * 3) * .12);
  ctx.shadowColor = "#ffe8b1"; ctx.shadowBlur = 5 + Math.sin(now / 600) * 2;
  ctx.fillStyle = ["#f6ddaf", "#efbb9e", "#eee3c5"][size]; ctx.strokeStyle = "#a47c64"; ctx.lineWidth = 1.2;
  ctx.beginPath(); ctx.moveTo(0, r * .5); ctx.bezierCurveTo(-r * 1.6, 0, -r, -r * 1.3, 0, -r);
  ctx.bezierCurveTo(r, -r * 1.3, r * 1.6, 0, 0, r * .5); ctx.fill(); ctx.stroke();
  ctx.shadowBlur = 0; ctx.globalAlpha = .55;
  for (let i = -2; i <= 2; i++) { ctx.beginPath(); ctx.moveTo(0, r * .4); ctx.lineTo(i * r * .3, -r * .7); ctx.stroke(); }
  ctx.restore();
}
function draw(now: number) {
  requestAnimationFrame(draw);
  if (now - last < 1000 / 30) return; last = now;
  const rect = canvas.getBoundingClientRect(); const dpr = window.devicePixelRatio || 1;
  if (canvas.width !== Math.round(rect.width * dpr) || canvas.height !== Math.round(rect.height * dpr)) {
    canvas.width = Math.round(rect.width * dpr); canvas.height = Math.round(rect.height * dpr);
  }
  const w = rect.width; const h = rect.height;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const grad = ctx.createLinearGradient(0, 0, 0, h); grad.addColorStop(0, "#3c8890"); grad.addColorStop(1, "#143e48"); ctx.fillStyle = grad; ctx.fillRect(0, 0, w, h);
  if (ocean.complete && ocean.naturalWidth) { ctx.globalAlpha = .4; ctx.drawImage(ocean, 0, 0, w, h); ctx.globalAlpha = 1; }
  ctx.fillStyle = "#b9a983"; ctx.beginPath(); ctx.moveTo(0, h * .86); ctx.quadraticCurveTo(w / 2, h * .82, w, h * .87); ctx.lineTo(w, h); ctx.lineTo(0, h); ctx.fill();
  const side = Math.min(w, h); const ox = (w - side) / 2;
  const point = (x: number, y: number) => [ox + x * side, y * side + (h - side) / 2];
  const [px, py] = point(.5, .14);
  // Original canvas art: boat hull, sail, rig and water ripples.
  ctx.fillStyle = "#f0be83"; ctx.beginPath(); ctx.moveTo(px - 42, py - 2); ctx.lineTo(px + 42, py - 2); ctx.lineTo(px + 27, py + 15); ctx.lineTo(px - 28, py + 15); ctx.closePath(); ctx.fill();
  ctx.strokeStyle = "#dbb792"; ctx.lineWidth = 3; ctx.beginPath(); ctx.moveTo(px, py - 3); ctx.lineTo(px, py - 50); ctx.stroke();
  ctx.fillStyle = "#f4ead2"; ctx.beginPath(); ctx.moveTo(px - 4, py - 48); ctx.lineTo(px - 4, py - 10); ctx.lineTo(px - 30, py - 10); ctx.closePath(); ctx.fill();
  const s = view?.session;
  for (const sh of view?.batch?.shells ?? []) {
    if (sh.collected || s?.caught_id === sh.id) continue;
    const [x, y] = point(sh.x, sh.y); drawShell(x, y, sh.size, now);
  }
  const angle = (s?.angle ?? 0) * Math.PI / 180;
  const length = Math.max(s?.length ?? 0, .04);
  const [mx, my] = point(.5 + Math.sin(angle) * length, .14 + Math.cos(angle) * length);
  ctx.strokeStyle = "#ece5bf"; ctx.lineWidth = 2; ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(mx, my); ctx.stroke();
  ctx.strokeStyle = "#c9f2ec"; ctx.lineWidth = 4; ctx.beginPath(); ctx.moveTo(mx - 9, my - 4); ctx.lineTo(mx - 6, my + 8); ctx.lineTo(mx, my + 12); ctx.lineTo(mx + 6, my + 8); ctx.lineTo(mx + 9, my - 4); ctx.stroke();
  const caught = view?.batch?.shells.find((sh) => sh.id === s?.caught_id);
  if (caught) drawShell(mx, my + 10, caught.size, now);
  const remaining = view?.batch?.shells.filter((sh) => !sh.collected).length ?? 0;
  wallet.textContent = `${shells} Vỏ sò · Hôm nay ${view?.earned ?? 0}/${view?.daily_cap ?? 60}`;
  drop.disabled = busy || !s || s.paused || s.phase !== "swinging" || (view?.earned ?? 0) >= (view?.daily_cap ?? 60);
  pause.disabled = !s; pause.textContent = s?.paused ? "Tiếp tục" : "Tạm dừng";
  status.textContent = error || (s?.error ? reason(s.error) : s?.paused ? "Đã tạm dừng — bấm Tiếp tục khi sẵn sàng" : !s ? "Đã gắp hết! Sò mới sẽ xuất hiện sau." : s.phase === "swinging" ? `${remaining} sò đang chờ · Canh hướng rồi thả móc` : s.phase === "extending" ? "Móc đang xuống…" : "Đang kéo về thuyền…");
}
async function poll() {
  if (leaving) return;
  try { view = await api.huntStatus(); const state = await api.state(); shells = state.shells; }
  catch (e) { error = reason(asFailure(e).code); }
  window.setTimeout(poll, document.hidden ? 1000 : 100);
}
async function main() {
  await win.onCloseRequested(async (event) => {
    if (leaving) return;
    event.preventDefault(); await leave();
  });
  requestAnimationFrame(draw); await poll();
}
void main();
