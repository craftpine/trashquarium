// While the boat is out, the top of the desktop ocean becomes the surface: open sky with a
// sun and drifting clouds, and a moving waterline the boat floats on.

/** Height of the waterline at x (the boat bobs on the same wave). */
export function waveY(x: number, waterY: number, clock: number, amp: number): number {
  return waterY + Math.sin(x * 0.012 + clock * 1.4) * amp + Math.sin(x * 0.031 - clock * 2.1) * amp * 0.4;
}

export const waveAmp = (waterY: number) => Math.max(2, waterY * 0.03);

export function drawSky(ctx: CanvasRenderingContext2D, width: number, waterY: number, clock: number, alpha: number, still: boolean) {
  if (alpha <= 0 || waterY <= 0) return;
  const t = still ? 0 : clock;
  const amp = waveAmp(waterY);
  ctx.save();
  ctx.globalAlpha = alpha;

  // Sky, clipped to the wavy waterline.
  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(width, 0);
  for (let x = width; x >= 0; x -= 12) ctx.lineTo(x, waveY(x, waterY, t, amp));
  ctx.closePath();
  ctx.save();
  ctx.clip();
  const sky = ctx.createLinearGradient(0, 0, 0, waterY + amp);
  sky.addColorStop(0, "#3f9be0");
  sky.addColorStop(0.65, "#8fd0f4");
  sky.addColorStop(1, "#d9f3ff");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, width, waterY + amp * 2);
  // Sun with a soft glow.
  const sx = width * 0.82, sy = waterY * 0.38, sr = Math.max(14, waterY * 0.16);
  const glow = ctx.createRadialGradient(sx, sy, sr * 0.5, sx, sy, sr * 3.2);
  glow.addColorStop(0, "rgba(255, 246, 200, 0.9)");
  glow.addColorStop(1, "rgba(255, 246, 200, 0)");
  ctx.fillStyle = glow;
  ctx.fillRect(sx - sr * 3.2, sy - sr * 3.2, sr * 6.4, sr * 6.4);
  ctx.fillStyle = "#fff3b8";
  ctx.beginPath(); ctx.arc(sx, sy, sr, 0, Math.PI * 2); ctx.fill();
  // Clouds drift slowly to the right and wrap around.
  const cloud = (base: number, y: number, s: number, speed: number) => {
    const span = width + s * 6;
    const x = ((base * width + t * speed) % span + span) % span - s * 3;
    ctx.fillStyle = "rgba(255, 255, 255, 0.92)";
    ctx.beginPath();
    ctx.ellipse(x, y, s * 1.6, s * 0.55, 0, 0, Math.PI * 2);
    ctx.ellipse(x - s * 0.8, y + s * 0.1, s * 0.75, s * 0.45, 0, 0, Math.PI * 2);
    ctx.ellipse(x + s * 0.2, y - s * 0.35, s * 0.85, s * 0.6, 0, 0, Math.PI * 2);
    ctx.ellipse(x + s * 1.0, y - s * 0.05, s * 0.7, s * 0.45, 0, 0, Math.PI * 2);
    ctx.fill();
  };
  const cs = Math.max(12, waterY * 0.12);
  cloud(0.08, waterY * 0.3, cs, 9);
  cloud(0.42, waterY * 0.22, cs * 0.8, 6);
  cloud(0.63, waterY * 0.5, cs * 1.1, 12);
  // Far sea horizon just above the waves.
  ctx.fillStyle = "rgba(40, 120, 150, 0.55)";
  ctx.fillRect(0, waterY - amp * 2.2, width, amp * 4);
  ctx.restore();

  // Foam along the surface, and light just below it.
  ctx.strokeStyle = "rgba(255, 255, 255, 0.75)";
  ctx.lineWidth = Math.max(2, amp * 0.6);
  ctx.beginPath();
  for (let x = 0; x <= width; x += 12) {
    const y = waveY(x, waterY, t, amp);
    if (x === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  }
  ctx.stroke();
  const under = ctx.createLinearGradient(0, waterY, 0, waterY + waterY * 0.5);
  under.addColorStop(0, "rgba(180, 240, 255, 0.35)");
  under.addColorStop(1, "rgba(180, 240, 255, 0)");
  ctx.fillStyle = under;
  ctx.fillRect(0, waterY, width, waterY * 0.5);
  ctx.restore();
}
