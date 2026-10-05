export function closureStep(current: number, phase: string | undefined, dt: number, paused: boolean, reduced: boolean): number {
  if (paused) return current;
  const target = phase === "retracting" || phase === "settling" ? 1 : 0;
  return reduced ? target : current + (target - current) * Math.min(1, Math.max(0, dt) * 12);
}

// Animate the existing PNG as three prong regions; no generated art is altered.
export function drawClaw(ctx: CanvasRenderingContext2D, art: HTMLImageElement, rect: { x: number; y: number; width: number; height: number }, closure: number) {
  ctx.save(); ctx.translate(rect.x, rect.y);
  const w = rect.width, h = rect.height, turn = Math.max(0, Math.min(1, closure)) * .42;
  const region = (left: number, right: number, pivot: number, angle: number) => {
    ctx.save(); ctx.translate(pivot * w, h * .56); ctx.rotate(angle); ctx.translate(-pivot * w, -h * .56);
    ctx.beginPath(); ctx.rect(left * w, h * .56, (right - left) * w, h * .44); ctx.clip();
    ctx.drawImage(art, 0, 0, w, h); ctx.restore();
  };
  region(0, .43, .35, -turn); region(.43, .57, .5, 0); region(.57, 1, .65, turn);
  ctx.drawImage(art, 0, 0, art.width, art.height * .56, 0, 0, w, h * .56);
  ctx.restore();
}
