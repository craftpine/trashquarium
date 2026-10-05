// The egg den: a small rock cave on the seabed of the desktop ocean. Every egg from breeding
// waits here with its own countdown until it hatches (2–3 hours).

/** "2:05:09" for times of an hour or more, "5:09" below that, "0:00" once due. */
export function formatCountdown(seconds: number): string {
  const s = Math.max(0, Math.ceil(seconds));
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), r = s % 60;
  const two = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${two(m)}:${two(r)}` : `${m}:${two(r)}`;
}

export interface DenEgg { id: string; hatch_at: number }

const SHOWN = 5;

/** Draws the den with its bottom centre at (x, y). `size` is the den width in pixels. */
export function drawDen(
  ctx: CanvasRenderingContext2D, x: number, y: number, size: number,
  eggs: DenEgg[], nowSec: number, clock: number, label: string, waitingLabel: string, still: boolean,
) {
  const w = size, h = size * 0.56;
  ctx.save();
  ctx.translate(x, y);
  // Rock mound: a few overlapping boulders so it reads as stone, not a dome.
  const boulder = (cx: number, cy: number, rx: number, ry: number, light: string, dark: string) => {
    const g = ctx.createRadialGradient(cx * w - rx * w * 0.35, cy * h - ry * h * 0.45, rx * w * 0.1, cx * w, cy * h, rx * w * 1.05);
    g.addColorStop(0, light); g.addColorStop(0.6, dark); g.addColorStop(1, "#22231a");
    ctx.fillStyle = g;
    ctx.beginPath(); ctx.ellipse(cx * w, cy * h, rx * w, ry * h, 0, 0, Math.PI * 2); ctx.fill();
  };
  boulder(-0.3, -0.38, 0.24, 0.42, "#8f8b70", "#57553f");
  boulder(0.3, -0.36, 0.23, 0.4, "#8a856a", "#525039");
  boulder(0.02, -0.62, 0.36, 0.5, "#9b9678", "#5f5c45");
  boulder(-0.12, -0.92, 0.17, 0.2, "#a7a283", "#6a6650");
  // Cracks.
  ctx.strokeStyle = "rgba(25, 25, 15, 0.35)"; ctx.lineWidth = Math.max(1, w * 0.006);
  ctx.beginPath(); ctx.moveTo(-w * 0.3, -h * 0.9); ctx.quadraticCurveTo(-w * 0.22, -h * 0.75, -w * 0.27, -h * 0.6); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(w * 0.28, -h * 0.82); ctx.quadraticCurveTo(w * 0.33, -h * 0.66, w * 0.25, -h * 0.55); ctx.stroke();
  // Moss on top.
  ctx.fillStyle = "rgba(118, 150, 62, 0.42)";
  for (const [mx, my, mr] of [[-0.18, -1.05, 0.1], [0.08, -1.08, 0.08], [0.26, -0.86, 0.07], [-0.38, -0.66, 0.06], [0.4, -0.55, 0.05]]) {
    ctx.beginPath(); ctx.ellipse(mx * w, my * h, mr * w, mr * w * 0.42, 0, 0, Math.PI * 2); ctx.fill();
  }
  // Cave mouth.
  const mouth = ctx.createRadialGradient(0, -h * 0.15, w * 0.02, 0, -h * 0.25, w * 0.36);
  mouth.addColorStop(0, "#1e3a3a"); mouth.addColorStop(1, "#071315");
  ctx.fillStyle = mouth;
  ctx.beginPath();
  ctx.moveTo(-w * 0.36, 0);
  ctx.bezierCurveTo(-w * 0.36, -h * 0.62, -w * 0.14, -h * 0.8, 0, -h * 0.8);
  ctx.bezierCurveTo(w * 0.14, -h * 0.8, w * 0.36, -h * 0.62, w * 0.36, 0);
  ctx.closePath();
  ctx.fill();
  // Sand floor of the cave.
  ctx.fillStyle = "#b99b62";
  ctx.beginPath(); ctx.ellipse(0, -h * 0.02, w * 0.35, h * 0.1, 0, Math.PI, Math.PI * 2); ctx.fill();

  // Eggs: soonest first, each with its own timer.
  const sorted = [...eggs].sort((a, b) => a.hatch_at - b.hatch_at);
  const shown = sorted.slice(0, SHOWN);
  const ew = w * 0.055, eh = ew * 1.3;
  const gap = Math.min(w * 0.13, (w * 0.6) / Math.max(1, shown.length));
  shown.forEach((egg, i) => {
    const ex = (i - (shown.length - 1) / 2) * gap;
    const ey = -h * 0.08 - eh;
    const left = egg.hatch_at - nowSec;
    ctx.save();
    ctx.translate(ex, ey + eh);
    // Eggs about to hatch wobble.
    if (!still && left < 600) ctx.rotate(Math.sin(clock * 9 + i) * (left <= 0 ? 0.16 : 0.08));
    const shell = ctx.createRadialGradient(-ew * 0.35, -eh * 1.45, ew * 0.1, 0, -eh, eh * 1.1);
    shell.addColorStop(0, "#fffdf6"); shell.addColorStop(1, "#e8cf9f");
    ctx.fillStyle = shell;
    ctx.beginPath(); ctx.ellipse(0, -eh, ew, eh, 0, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = "rgba(150, 110, 70, 0.45)";
    for (const [sx, sy, sr] of [[-0.3, -1.3, 0.13], [0.35, -0.9, 0.1], [0.05, -0.55, 0.12], [-0.25, -0.8, 0.08]]) {
      ctx.beginPath(); ctx.arc(sx * ew, sy * eh, sr * ew, 0, Math.PI * 2); ctx.fill();
    }
    ctx.restore();
    // Timer pill above the egg, staggered so neighbours don't overlap.
    const text = left > 0 ? formatCountdown(left) : waitingLabel;
    ctx.font = `700 ${Math.max(12, Math.round(w * 0.045))}px system-ui, sans-serif`;
    const tw = ctx.measureText(text).width + 12, th = Math.max(18, w * 0.068);
    const ty = ey - eh * 0.4 - th - (i % 2) * (th + 3);
    ctx.fillStyle = left > 0 ? "rgba(8, 30, 34, 0.78)" : "rgba(150, 90, 20, 0.85)";
    ctx.beginPath(); ctx.roundRect(ex - tw / 2, ty, tw, th, th / 2); ctx.fill();
    ctx.fillStyle = "#fff4dc"; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(text, ex, ty + th / 2 + 0.5);
  });

  // Name plate on the rock.
  const more = sorted.length - shown.length;
  const plate = more > 0 ? `${label} · +${more}` : label;
  ctx.font = `700 ${Math.max(12, Math.round(w * 0.05))}px system-ui, sans-serif`;
  const pw = ctx.measureText(plate).width + 18, ph = Math.max(20, w * 0.075);
  ctx.fillStyle = "rgba(255, 244, 220, 0.9)";
  ctx.beginPath(); ctx.roundRect(-pw / 2, -h * 1.12 - ph - 4, pw, ph, ph / 2); ctx.fill();
  ctx.fillStyle = "#4a3a1a"; ctx.textAlign = "center"; ctx.textBaseline = "middle";
  ctx.fillText(plate, 0, -h * 1.12 - ph / 2 - 3.5);
  ctx.restore();
}
