// The egg clutch on the desktop seabed: every egg from breeding lies on the sand with its own
// countdown until it hatches (2–3 hours).

/** "2:05:09" for times of an hour or more, "5:09" below that, "0:00" once due. */
export function formatCountdown(seconds: number): string {
  const s = Math.max(0, Math.ceil(seconds));
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), r = s % 60;
  const two = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${two(m)}:${two(r)}` : `${m}:${two(r)}`;
}

export interface DenEgg { id: string; hatch_at: number }

const SHOWN = 5;

/** Draws the clutch: the eggs from breeding lying straight on the sand, each with a soft
 * shadow and its own countdown, centred at (x, y). `size` is the width of the sandy patch they
 * share. Nothing is drawn while there are no eggs. `moreLabel(n)` names the eggs not shown. */
export function drawDen(
  ctx: CanvasRenderingContext2D, x: number, y: number, size: number,
  eggs: DenEgg[], nowSec: number, clock: number, moreLabel: (n: number) => string, waitingLabel: string, still: boolean,
) {
  if (eggs.length === 0) return;
  const w = size;
  const sorted = [...eggs].sort((a, b) => a.hatch_at - b.hatch_at);
  const shown = sorted.slice(0, SHOWN);
  const ew = w * 0.05, eh = ew * 1.3;
  const gap = Math.min(w * 0.15, (w * 0.7) / Math.max(1, shown.length));
  ctx.save();
  ctx.translate(x, y);

  shown.forEach((egg, i) => {
    // A loose little clutch: alternate eggs sit a touch further back.
    const ex = (i - (shown.length - 1) / 2) * gap;
    const ey = (i % 2 ? -0.35 : 0.15) * eh;
    const left = egg.hatch_at - nowSec;

    // Each egg is nestled into the sand: a shallow dent, then its shadow.
    const dent = ctx.createRadialGradient(ex, ey, ew * 0.2, ex, ey, ew * 1.7);
    dent.addColorStop(0, "rgba(120, 90, 45, 0.35)"); dent.addColorStop(1, "rgba(120, 90, 45, 0)");
    ctx.fillStyle = dent;
    ctx.beginPath(); ctx.ellipse(ex, ey, ew * 1.7, eh * 0.45, 0, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = "rgba(60, 40, 15, 0.35)";
    ctx.beginPath(); ctx.ellipse(ex + ew * 0.15, ey, ew * 0.95, eh * 0.18, 0, 0, Math.PI * 2); ctx.fill();

    ctx.save();
    ctx.translate(ex, ey);
    if (!still && left < 600) ctx.rotate(Math.sin(clock * 9 + i) * (left <= 0 ? 0.16 : 0.08)); // about to hatch
    const shell = ctx.createRadialGradient(-ew * 0.35, -eh * 1.45, ew * 0.1, 0, -eh, eh * 1.1);
    shell.addColorStop(0, "#fffdf6"); shell.addColorStop(1, "#e8cf9f");
    ctx.fillStyle = shell;
    ctx.beginPath(); ctx.ellipse(0, -eh * 0.92, ew, eh, 0, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = "rgba(150, 110, 70, 0.45)";
    for (const [sx, sy, sr] of [[-0.3, -1.25, 0.13], [0.35, -0.85, 0.1], [0.05, -0.5, 0.12], [-0.25, -0.75, 0.08]]) {
      ctx.beginPath(); ctx.arc(sx * ew, sy * eh, sr * ew, 0, Math.PI * 2); ctx.fill();
    }
    ctx.restore();

    // Timer pill above the egg, staggered so neighbours don't overlap.
    const text = left > 0 ? formatCountdown(left) : waitingLabel;
    ctx.font = `700 ${Math.max(12, Math.round(w * 0.04))}px system-ui, sans-serif`;
    const tw = ctx.measureText(text).width + 12, th = Math.max(18, w * 0.06);
    const ty = ey - eh * 2.25 - th - (i % 2) * (th + 3);
    ctx.fillStyle = left > 0 ? "rgba(8, 30, 34, 0.82)" : "rgba(150, 90, 20, 0.9)";
    ctx.beginPath(); ctx.roundRect(ex - tw / 2, ty, tw, th, th / 2); ctx.fill();
    ctx.fillStyle = "#fff4dc"; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(text, ex, ty + th / 2 + 0.5);
  });

  // Eggs that don't fit in the clutch are counted beside it.
  const more = sorted.length - shown.length;
  if (more > 0) {
    const text = moreLabel(more);
    ctx.font = `700 ${Math.max(12, Math.round(w * 0.04))}px system-ui, sans-serif`;
    const pw = ctx.measureText(text).width + 14, ph = Math.max(18, w * 0.06);
    const px = (shown.length / 2) * gap + ew * 0.6;
    ctx.fillStyle = "rgba(255, 244, 220, 0.92)";
    ctx.beginPath(); ctx.roundRect(px, -eh - ph / 2, pw, ph, ph / 2); ctx.fill();
    ctx.fillStyle = "#4a3a1a"; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(text, px + pw / 2, -eh + 0.5);
  }
  ctx.restore();
}
