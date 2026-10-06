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

/** Stable pseudo-random numbers so the pebbles don't jump between frames. */
function seeded(n: number) {
  const v = Math.sin(n * 127.1 + 311.7) * 43758.5453;
  return v - Math.floor(v);
}

/** Draws the den — a little spring-fed pool on the sand ringed with pebbles and sea grass —
 * centred at (x, y). `size` is the pool width in pixels. */
export function drawDen(
  ctx: CanvasRenderingContext2D, x: number, y: number, size: number,
  eggs: DenEgg[], nowSec: number, clock: number, label: string, waitingLabel: string, still: boolean,
) {
  const w = size, h = size * 0.3; // seen at a low angle: a flat oval
  const t = still ? 0 : clock;
  ctx.save();
  ctx.translate(x, y);

  // Sea grass behind the pool, swaying.
  const blade = (bx: number, height: number, lean: number, hue: string) => {
    const sway = Math.sin(t * 1.3 + bx * 9) * w * 0.02;
    ctx.strokeStyle = hue; ctx.lineWidth = Math.max(2, w * 0.012); ctx.lineCap = "round";
    ctx.beginPath(); ctx.moveTo(bx * w, -h * 0.35);
    ctx.quadraticCurveTo(bx * w + lean * w * 0.04 + sway, -h * 0.35 - height * w * 0.5, bx * w + lean * w * 0.07 + sway * 2, -h * 0.35 - height * w);
    ctx.stroke();
  };
  for (let i = 0; i < 9; i++) {
    const bx = -0.46 + seeded(i) * 0.25 + (i > 4 ? 0.68 : 0);
    blade(bx, 0.16 + seeded(i + 20) * 0.18, seeded(i + 40) - 0.5, i % 2 ? "#4f8a3a" : "#6aa44a");
  }

  // Sand bank around the pool.
  const bank = ctx.createRadialGradient(0, 0, w * 0.2, 0, 0, w * 0.62);
  bank.addColorStop(0, "rgba(214, 186, 120, 0.95)"); bank.addColorStop(1, "rgba(190, 160, 100, 0)");
  ctx.fillStyle = bank;
  ctx.beginPath(); ctx.ellipse(0, 0, w * 0.62, h * 0.85, 0, 0, Math.PI * 2); ctx.fill();

  // The pool: deep blue-green water with moving light.
  const water = ctx.createRadialGradient(0, h * 0.05, w * 0.03, 0, 0, w * 0.5);
  water.addColorStop(0, "#1f6f78"); water.addColorStop(0.7, "#0f4650"); water.addColorStop(1, "#0a2f37");
  ctx.fillStyle = water;
  ctx.beginPath(); ctx.ellipse(0, 0, w * 0.44, h * 0.5, 0, 0, Math.PI * 2); ctx.fill();
  ctx.save();
  ctx.beginPath(); ctx.ellipse(0, 0, w * 0.44, h * 0.5, 0, 0, Math.PI * 2); ctx.clip();
  ctx.strokeStyle = "rgba(170, 240, 235, 0.28)"; ctx.lineWidth = Math.max(1.5, w * 0.006);
  for (let i = 0; i < 6; i++) {
    const cy = (-0.35 + i * 0.14) * h;
    ctx.beginPath();
    for (let k = 0; k <= 20; k++) {
      const cx = -w * 0.45 + (k / 20) * w * 0.9;
      const yy = cy + Math.sin(k * 0.9 + t * 1.6 + i * 1.7) * h * 0.06;
      if (k === 0) ctx.moveTo(cx, yy); else ctx.lineTo(cx, yy);
    }
    ctx.stroke();
  }
  // Rising spring bubbles.
  ctx.fillStyle = "rgba(220, 250, 255, 0.55)";
  for (let i = 0; i < 4; i++) {
    const phase = ((t * 0.35 + i * 0.25) % 1);
    ctx.beginPath(); ctx.arc((seeded(i + 60) - 0.5) * w * 0.6, h * 0.25 - phase * h * 0.6, Math.max(1.5, w * 0.008), 0, Math.PI * 2); ctx.fill();
  }
  ctx.restore();

  // Pebble ring: back half first, eggs, then the front half so the eggs sit inside.
  const pebbles = Array.from({ length: 26 }, (_, i) => {
    const a = (i / 26) * Math.PI * 2 + seeded(i) * 0.15;
    const r = 0.95 + seeded(i + 7) * 0.12;
    const tone = ["#8f8a78", "#a49c84", "#7b7a6a", "#6f7f5c", "#b5aa8c"][i % 5];
    return { x: Math.cos(a) * w * 0.46 * r, y: Math.sin(a) * h * 0.52 * r, s: w * (0.028 + seeded(i + 13) * 0.022), tone };
  });
  const pebble = (p: (typeof pebbles)[number]) => {
    const g = ctx.createRadialGradient(p.x - p.s * 0.35, p.y - p.s * 0.45, p.s * 0.1, p.x, p.y, p.s * 1.2);
    g.addColorStop(0, "#efe9d8"); g.addColorStop(0.35, p.tone); g.addColorStop(1, "#3e3c32");
    ctx.fillStyle = g;
    ctx.beginPath(); ctx.ellipse(p.x, p.y, p.s * 1.25, p.s * 0.8, 0, 0, Math.PI * 2); ctx.fill();
  };
  pebbles.filter((p) => p.y < 0).forEach(pebble);

  // Eggs: soonest first, each with its own timer.
  const sorted = [...eggs].sort((a, b) => a.hatch_at - b.hatch_at);
  const shown = sorted.slice(0, SHOWN);
  const ew = w * 0.045, eh = ew * 1.3;
  const gap = Math.min(w * 0.13, (w * 0.62) / Math.max(1, shown.length));
  shown.forEach((egg, i) => {
    const ex = (i - (shown.length - 1) / 2) * gap;
    const ey = h * 0.08;
    const left = egg.hatch_at - nowSec;
    ctx.save();
    ctx.translate(ex, ey);
    if (!still && left < 600) ctx.rotate(Math.sin(clock * 9 + i) * (left <= 0 ? 0.16 : 0.08)); // about to hatch
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
    ctx.font = `700 ${Math.max(12, Math.round(w * 0.04))}px system-ui, sans-serif`;
    const tw = ctx.measureText(text).width + 12, th = Math.max(18, w * 0.06);
    const ty = ey - eh * 2.3 - th - (i % 2) * (th + 3);
    ctx.fillStyle = left > 0 ? "rgba(8, 30, 34, 0.82)" : "rgba(150, 90, 20, 0.9)";
    ctx.beginPath(); ctx.roundRect(ex - tw / 2, ty, tw, th, th / 2); ctx.fill();
    ctx.fillStyle = "#fff4dc"; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(text, ex, ty + th / 2 + 0.5);
  });
  pebbles.filter((p) => p.y >= 0).forEach(pebble);

  // Name plate.
  const more = sorted.length - shown.length;
  const plate = more > 0 ? `${label} · +${more}` : label;
  ctx.font = `700 ${Math.max(12, Math.round(w * 0.045))}px system-ui, sans-serif`;
  const pw = ctx.measureText(plate).width + 18, ph = Math.max(20, w * 0.068);
  const py = -h * 0.35 - w * 0.36 - ph;
  ctx.fillStyle = "rgba(255, 244, 220, 0.92)";
  ctx.beginPath(); ctx.roundRect(-pw / 2, py, pw, ph, ph / 2); ctx.fill();
  ctx.fillStyle = "#4a3a1a"; ctx.textAlign = "center"; ctx.textBaseline = "middle";
  ctx.fillText(plate, 0, py + ph / 2 + 0.5);
  ctx.restore();
}
