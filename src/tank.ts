// Desktop ocean. Renders only while visible: at the display rate (up to 60 FPS)
// normally, 20 FPS when the computer is idle or Meeting Mode is on, nothing when
// the window is hidden.
import { listen } from "@tauri-apps/api/event";
import { api, type Fish, type HuntShell, type HuntView, type StateView } from "./api";
import { reason, t } from "./i18n";
import { closureStep, drawClaw } from "./hunt-motion";
import { bodyWave, finStretch, nextBurst, swimStyle } from "./fish-motion";
import { drawDen } from "./egg-den";
import { drawSky, waveAmp, waveY } from "./hunt-sky";
import { COIN_SRC } from "./coin";

const VISIBLE_FPS = 60;
const QUIET_FPS = 20;
const IDLE_AFTER_SECONDS = 60;

// On-screen length follows the purchase price (renderer detail, not biology data):
// 20 CBCoin, the cheapest fish, is the baseline and size grows with sqrt(price),
// so a 300 CBCoin whale shark is ~3.9x the length of a 20 CBCoin guppy.
const BASE_PRICE = 20;
const BASE_SCALE = 0.5; // fraction of 22% of the screen's short side for a 20 CBCoin adult
const MAX_LENGTH_RATIO = 0.32; // never wider than this fraction of the screen
const prices = new Map<string, number>();
const priceScale = (price: number) => BASE_SCALE * Math.sqrt(Math.max(BASE_PRICE, price) / BASE_PRICE);
const STAGE_SCALE = { fry: 0.55, juvenile: 0.78, adult: 1 } as const;

interface Swimmer {
  fish: Fish;
  x: number;
  y: number;
  vx: number;
  vy: number;
  tx: number;
  ty: number;
  facing: number; // -1 left … 1 right, eased for smooth turns
  phase: number;
  tail: number; // tail-beat phase, faster when the fish swims faster
  fin: number; // pectoral/dorsal fin flutter phase
  burst: number; // 0 = gliding, 1 = fast tail beats
  gulp: number; // 1 right after snapping at food, fades to 0
  chasing: boolean; // heading for a food pellet
  speed: number;
}

/** Food pellets that sink slowly; nearby fish that aren't full dart over and snap them up. */
interface Pellet {
  x: number;
  y: number;
  vy: number;
  wobble: number;
  bottomFor: number; // seconds spent resting on the sand
}
let pellets: Pellet[] = [];
let nextAmbientFood = 25 + Math.random() * 40;
const lastExp = new Map<string, number>();

interface Bubble {
  x: number;
  y: number;
  r: number;
  v: number;
}

const canvas = document.getElementById("ocean") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;
const background = document.createElement("canvas");
const sprites = new Map<string, HTMLImageElement>();
let oceanImage: HTMLImageElement | null = null;
let swimmers: Swimmer[] = [];
let bubbles: Bubble[] = [];
let state: StateView | null = null;
let quiet = false;
let lastFrame = 0;
let width = 0;
let height = 0;
let clock = 0;
// Shell hunt, played on this desktop ocean: live view while a session exists.
let hunt: HuntView | null = null;
let huntBlend = 0; // 0 = idle ocean, 1 = hunt scene fully shown
const huntArt: Record<string, HTMLImageElement | null> = {};
let coinArt: HTMLImageElement | null = null;
const HUNT_KEY = navigator.userAgent.includes("Windows") ? "Space" : "Ctrl+Alt+Space"; // global hotkey shown to the player
const HUNT_PIVOT = { x: 0.5, y: 0.14 }; // keep in sync with PIVOT in engine/hunt.rs
const BOAT_HATCH = { x: 0.5, y: 299 / 360 }; // where the rope leaves the boat sprite
const BOAT_WATERLINE = 0.72; // fraction of the boat sprite height that sits at the water surface
const CLAW_GRAB = 125 / 176; // grab centre of the claw sprite, measured from its top
const SHELL_SPRITE = { great: 0, queen: 1, variegated: 2 } as const; // white 1, red 10, purple 100 CBCoin
const SHELL_COINS = { great: 1, queen: 10, variegated: 100 } as const;
const SHELL_NAME = { great: "Sò điệp lớn", queen: "Sò điệp queen", variegated: "Sò điệp đa sắc" } as const; // keys for t()
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
let closure = 0; // 0 = claw open, 1 = closed on a catch
let prevLength = 0;
let ripples: { x: number; y: number; age: number }[] = [];
let notice = { text: "", until: 0 };
let seenReceipt: string | null | undefined;

function loadImage(src: string): Promise<HTMLImageElement | null> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null); // missing art falls back to a drawn fish
    img.src = src;
  });
}

function resize() {
  const dpr = window.devicePixelRatio || 1;
  width = window.innerWidth;
  height = window.innerHeight;
  canvas.width = Math.round(width * dpr);
  canvas.height = Math.round(height * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  baked.clear();
  background.width = canvas.width;
  background.height = canvas.height;
  const bg = background.getContext("2d")!;
  bg.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (oceanImage) {
    const scale = Math.max(width / oceanImage.width, height / oceanImage.height);
    const w = oceanImage.width * scale;
    const h = oceanImage.height * scale;
    bg.drawImage(oceanImage, (width - w) / 2, (height - h) / 2, w, h);
  } else {
    const grad = bg.createLinearGradient(0, 0, 0, height);
    grad.addColorStop(0, "#1b6a73");
    grad.addColorStop(1, "#08262c");
    bg.fillStyle = grad;
    bg.fillRect(0, 0, width, height);
  }
}

const swimTop = () => height * 0.14;
const swimBottom = () => height * 0.8;

function pickTarget(s: Swimmer) {
  s.tx = width * (0.06 + Math.random() * 0.88);
  s.ty = swimTop() + Math.random() * (swimBottom() - swimTop());
}

function syncSwimmers(fish: Fish[]) {
  const existing = new Map(swimmers.map((s) => [s.fish.id, s]));
  swimmers = fish.map((f) => {
    const old = existing.get(f.id);
    if (old) {
      old.fish = f;
      return old;
    }
    const s: Swimmer = {
      fish: f,
      x: width * (0.1 + Math.random() * 0.8),
      y: swimTop() + Math.random() * (swimBottom() - swimTop()),
      vx: 0,
      vy: 0,
      tx: 0,
      ty: 0,
      facing: 1,
      phase: Math.random() * Math.PI * 2,
      tail: Math.random() * Math.PI * 2,
      fin: Math.random() * Math.PI * 2,
      burst: 0,
      gulp: 0,
      chasing: false,
      speed: 0.9 + Math.random() * 0.3,
    };
    pickTarget(s);
    return s;
  });
}

function fishLength(f: Fish) {
  const len = Math.min(width, height) * 0.22 * priceScale(prices.get(f.species_id) ?? BASE_PRICE) * STAGE_SCALE[f.stage];
  return Math.min(len, width * MAX_LENGTH_RATIO);
}

const huntActive = () => !!hunt?.session && !state?.settings.meeting_mode;
const ease = (t: number) => t * t * (3 - 2 * t);
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

/** The hunt plays in a square (engine units 0..1) centred on the screen. */
function huntFrame() {
  const side = Math.min(width, height * 0.96);
  return { side, ox: (width - side) / 2, oy: (height - side) / 2 };
}

async function pollHunt() {
  if (!hunt?.session && !state?.hunt.session) return;
  try { hunt = await api.huntStatus(); } catch { /* keep the last view */ }
}

function dropFood(x: number, count: number) {
  for (let i = 0; i < count && pellets.length < 40; i++) {
    pellets.push({ x: Math.min(width * 0.97, Math.max(width * 0.03, x + (Math.random() - 0.5) * 120)), y: height * (0.03 + Math.random() * 0.05), vy: 16 + Math.random() * 14, wobble: Math.random() * 6, bottomFor: 0 });
  }
}

const restingNow = (s: Swimmer) => s.fish.resting_until > Date.now() / 1000;
const mouthX = (s: Swimmer, len: number) => s.x + (Math.sign(s.facing) || 1) * len * 0.45;

function step(dt: number) {
  clock += dt;
  const quietTank = !!state?.settings.meeting_mode;
  // Food now and then, as if someone sprinkled flakes on the surface.
  if (!quietTank && !huntActive() && clock > nextAmbientFood) {
    nextAmbientFood = clock + 45 + Math.random() * 60;
    dropFood(width * (0.25 + Math.random() * 0.65), 3 + Math.floor(Math.random() * 4));
  }
  const floor = height * 0.9;
  for (const p of pellets) {
    if (p.y < floor) { p.y = Math.min(floor, p.y + p.vy * dt); p.x += Math.sin(clock * 2 + p.wobble) * 8 * dt; }
    else p.bottomFor += dt;
  }
  pellets = pellets.filter((p) => p.bottomFor < 20);
  huntBlend = Math.min(1, Math.max(0, huntBlend + (huntActive() ? dt : -dt) / 0.6));
  const calm = state?.settings.meeting_mode ? 0.4 : 1;
  for (const s of swimmers) {
    const len = fishLength(s.fish);
    const cruise = (len * 0.35 + 18) * s.speed * calm * (0.75 + 0.7 * s.burst);
    // Hungry fish go after the nearest pellet; full (resting) fish ignore food.
    let food: Pellet | null = null;
    if (!quietTank && !restingNow(s) && pellets.length) {
      let best = width * 0.4;
      for (const p of pellets) { const d = Math.hypot(p.x - s.x, p.y - s.y); if (d < best) { best = d; food = p; } }
    }
    if (food) {
      const dir = food.x >= s.x ? 1 : -1;
      s.tx = food.x - dir * len * 0.42; s.ty = food.y; s.chasing = true;
      s.burst = Math.max(s.burst, 0.45);
      if (Math.hypot(food.x - mouthX(s, len), food.y - s.y) < len * 0.16 + 8) {
        const eaten = food;
        pellets = pellets.filter((p) => p !== eaten); // snap!
        s.gulp = 1;
        if (bubbles.length < 24) bubbles.push({ x: mouthX(s, len), y: s.y - len * 0.05, r: 2 + Math.random() * 2, v: 30 + Math.random() * 20 });
      }
    } else if (s.chasing) { s.chasing = false; pickTarget(s); }
    const dx = s.tx - s.x;
    const dy = s.ty - s.y;
    const dist = Math.hypot(dx, dy);
    if (!s.chasing && dist < len * 0.4) pickTarget(s);
    // Steer toward the target (harder while chasing food); fish mostly swim horizontally.
    const steer = Math.min(1, dt * (s.chasing ? 2 : 0.8));
    s.vx += ((dx / (dist || 1)) * cruise - s.vx) * steer;
    s.vy += ((dy / (dist || 1)) * cruise * (s.chasing ? 0.9 : 0.45) - s.vy) * steer;
    s.x += s.vx * dt;
    s.y += s.vy * dt;
    s.phase += dt * (2 + Math.abs(s.vx) / 40);
    // Tail beats speed up with swimming speed and during a burst; fins flutter all the time.
    s.tail += dt * (3 + Math.hypot(s.vx, s.vy) / 26 + 5 * s.burst) * calm;
    s.fin += dt * (7 + 3 * s.burst) * calm;
    // Dead zone: a fish that is nearly still keeps its heading instead of flip-flopping.
    const want = s.vx > 8 ? 1 : s.vx < -8 ? -1 : Math.sign(s.facing) || 1;
    const turned = want !== (Math.sign(s.facing) || 1);
    // A quick turn (about a fifth of a second) so the fish never looks like a flat card for long.
    s.facing += (want - s.facing) * Math.min(1, dt * 11);
    s.gulp = Math.max(0, s.gulp - dt * 2.8);
    if (!quietTank && s.gulp === 0 && Math.random() < dt * 0.02) s.gulp = 0.7; // idle mouthing
    s.burst = calm < 1 ? 0 : nextBurst(s.burst, dt, Math.random(), turned);
  }
  if (!state?.settings.meeting_mode) {
    if (bubbles.length < 18 && Math.random() < dt * 1.5) {
      bubbles.push({ x: width * (0.05 + Math.random() * 0.9), y: height * 0.9, r: 2 + Math.random() * 4, v: 25 + Math.random() * 35 });
    }
    for (const b of bubbles) {
      b.y -= b.v * dt;
      b.x += Math.sin((b.y + b.r * 50) / 30) * 0.3;
    }
    bubbles = bubbles.filter((b) => b.y > height * 0.05);
  } else {
    bubbles = [];
  }
}

// Fish art is a still PNG, so the body is drawn as thin vertical slices whose
// vertical offset follows a travelling wave: calm at the head, wide at the tail.
// The shadow is baked once per size (shadowBlur every frame is slow on big screens).
const SLICES = 28;
const baked = new Map<string, { canvas: HTMLCanvasElement; shadow: HTMLCanvasElement; pad: number; w: number; h: number }>();

// Sprites have uneven transparent margins, so size each fish by its visible body.
const bounds = new Map<string, { x: number; y: number; w: number; h: number }>();
function visibleBounds(id: string, img: HTMLImageElement) {
  let b = bounds.get(id);
  if (b) return b;
  b = { x: 0, y: 0, w: img.width, h: img.height };
  try {
    const c = document.createElement("canvas");
    c.width = img.width; c.height = img.height;
    const g = c.getContext("2d")!;
    g.drawImage(img, 0, 0);
    const d = g.getImageData(0, 0, c.width, c.height).data;
    let x0 = c.width, y0 = c.height, x1 = -1, y1 = -1;
    for (let y = 0; y < c.height; y++) for (let x = 0; x < c.width; x++) {
      if (d[(y * c.width + x) * 4 + 3] > 24) { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
    }
    if (x1 >= x0 && y1 >= y0) b = { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
  } catch { /* keep the full image */ }
  bounds.set(id, b);
  return b;
}

/** Drawn size of the visible body: `len` wide, but never taller than 0.8 * len. */
function bodySize(id: string, img: HTMLImageElement, len: number) {
  const b = visibleBounds(id, img);
  let w = len, h = len * (b.h / b.w);
  if (h > len * 0.8) { w *= (len * 0.8) / h; h = len * 0.8; }
  return { w, h, b };
}

function bakeFish(id: string, img: HTMLImageElement, len: number) {
  const dpr = window.devicePixelRatio || 1;
  const key = `${id}|${Math.round(len * dpr)}`;
  let hit = baked.get(key);
  if (hit) return hit;
  if (baked.size > 160) baked.clear();
  const { w, h, b } = bodySize(id, img, len);
  const pad = Math.ceil(len * 0.12);
  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil((w + pad * 2) * dpr);
  canvas.height = Math.ceil((h + pad * 2) * dpr);
  const g = canvas.getContext("2d")!;
  g.drawImage(img, b.x, b.y, b.w, b.h, pad * dpr, pad * dpr, w * dpr, h * dpr);
  // The shadow is a separate, already blurred silhouette drawn once behind the slices. Baking it
  // into the sliced sprite made the overlapping slices stack its semi-transparent pixels, which
  // showed up as thin dark stripes under the belly.
  const shadow = document.createElement("canvas");
  shadow.width = canvas.width;
  shadow.height = canvas.height;
  const sg = shadow.getContext("2d")!;
  const far = canvas.width + 50; // draw the fish off-canvas so only its shadow lands inside
  sg.shadowColor = "rgba(0, 20, 25, 0.35)";
  sg.shadowBlur = len * 0.08 * dpr;
  sg.shadowOffsetX = far;
  sg.shadowOffsetY = len * 0.05 * dpr;
  sg.drawImage(img, b.x, b.y, b.w, b.h, pad * dpr - far, pad * dpr, w * dpr, h * dpr);
  hit = { canvas, shadow, pad, w, h };
  baked.set(key, hit);
  return hit;
}

function drawUndulating(s: Swimmer, img: HTMLImageElement, len: number) {
  const { canvas: src, shadow, pad, w, h } = bakeFish(s.fish.species_id, img, len);
  const totalW = w + pad * 2, totalH = h + pad * 2;
  ctx.drawImage(shadow, -totalW / 2, -totalH / 2, totalW, totalH);
  const sliceW = src.width / SLICES, destW = totalW / SLICES;
  if (reducedMotion.matches) { ctx.drawImage(src, -totalW / 2, -totalH / 2, totalW, totalH); return; }
  const style = swimStyle(s.fish.species_id);
  const power = 1 + 0.7 * s.burst; // a burst throws the tail harder
  // The whole body rocks a little with each tail beat.
  ctx.rotate(Math.sin(s.tail + 1.2) * (style === "ray" ? 0.03 : 0.025) * power);
  const uAt = (x: number) => Math.min(1, Math.max(0, (x + w / 2) / w)); // 0 = tail, 1 = head (sprites face right)
  const waveAt = (x: number) => bodyWave(style, uAt(x), s.tail) * w * power;
  let left = waveAt(-totalW / 2);
  for (let i = 0; i < SLICES; i++) {
    const x0 = -totalW / 2 + i * destW;
    const right = waveAt(x0 + destW);
    const u = uAt(x0 + destW / 2);
    // Snapping at food: the head end opens and closes once.
    const head = u > 0.78 ? (u - 0.78) / 0.22 : 0;
    const sh = totalH * finStretch(style, u, s.tail, s.fin) * (1 + 0.16 * Math.sin(s.gulp * Math.PI) * head);
    // Shear each slice so its edges meet the neighbours: a smooth bend, no stair steps.
    ctx.save();
    ctx.transform(1, (right - left) / destW, 0, 1, x0, left);
    ctx.drawImage(src, i * sliceW, 0, sliceW, src.height, 0, -sh / 2, destW + 0.6, sh);
    ctx.restore();
    left = right;
  }
}

function drawFallbackFish(len: number) {
  ctx.fillStyle = "#f2b36b";
  ctx.beginPath();
  ctx.ellipse(0, 0, len * 0.38, len * 0.18, 0, 0, Math.PI * 2);
  ctx.moveTo(-len * 0.32, 0);
  ctx.lineTo(-len * 0.5, -len * 0.14);
  ctx.lineTo(-len * 0.5, len * 0.14);
  ctx.fill();
}

function drawShell(shell: HuntShell, x: number, y: number, size: number) {
  const img = huntArt[`shell_${SHELL_SPRITE[shell.kind] ?? shell.size}`];
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(Math.sin(shell.x * 3) * 0.12 + Math.sin(clock * 0.9 + shell.x * 9) * 0.05);
  if (img) {
    ctx.shadowColor = shell.rare ? "#ffe5a4" : "rgba(0, 20, 25, 0.4)";
    ctx.shadowBlur = shell.rare ? size * 0.22 + (reducedMotion.matches ? 0 : Math.sin(clock * 2.4) * size * 0.04) : size * 0.1;
    ctx.drawImage(img, -size / 2, -size / 2, size, size);
  } else {
    const r = size * 0.4;
    ctx.fillStyle = "#f5d8a2"; ctx.strokeStyle = "#b18c6d"; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(0, r * .5); ctx.bezierCurveTo(-r * 1.4, 0, -r, -r, 0, -r); ctx.bezierCurveTo(r, -r, r * 1.4, 0, 0, r * .5); ctx.fill(); ctx.stroke();
  }
  ctx.restore();
}

function huntMessage(): string {
  const s = hunt?.session;
  if (!s || !hunt) return "";
  if (s.error) return reason(s.error);
  if (s.paused) return t("Đã tạm dừng — nhấn {key} để tiếp tục", { key: HUNT_KEY });
  const left = hunt.batch?.shells.filter((sh) => !sh.collected).length ?? 0;
  if (s.phase === "swinging") return t("{left} sò đang chờ · {key} để thả móc", { left, key: HUNT_KEY });
  return s.phase === "extending" ? t("Móc đang xuống…") : t("Đang kéo về thuyền…");
}

/** Boat, rope, claw and status pill. Drawn above the fish, below nothing. */
function drawHunt(e: number) {
  const s = hunt?.session;
  if (!s || !hunt || e <= 0) return;
  const { side, ox, oy } = huntFrame();
  const at = (x: number, y: number): [number, number] => [ox + x * side, oy + y * side];
  const [px, py0] = at(HUNT_PIVOT.x, HUNT_PIVOT.y);
  // The boat floats on the surface: sky above the waterline, the hull dips into the water.
  const bw = side * 0.27;
  const bh = bw * 0.75;
  const waterY0 = py0 - bh * (BOAT_HATCH.y - BOAT_WATERLINE);
  // Ride the same wave the sky's waterline draws.
  const bob = reducedMotion.matches ? 0 : waveY(px, 0, clock, waveAmp(waterY0));
  const dive = (1 - e) * side * 0.3; // the boat sails in from above
  const py = py0 + bob - dive;
  drawSky(ctx, width, waterY0, clock, e, reducedMotion.matches);
  ctx.save();
  ctx.globalAlpha = e;

  // Rope and claw first: the hull hides them while they are pulled in.
  const angle = s.angle * Math.PI / 180;
  const len = Math.max(s.length, 0.0);
  const [tx, ty0] = at(HUNT_PIVOT.x + Math.sin(angle) * len, HUNT_PIVOT.y + Math.cos(angle) * len);
  const ty = ty0 + bob - dive;
  ctx.lineCap = "round";
  ctx.strokeStyle = "rgba(60, 40, 20, 0.55)"; ctx.lineWidth = Math.max(4, side * 0.005);
  ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(tx, ty); ctx.stroke();
  ctx.strokeStyle = "#ecdfb8"; ctx.lineWidth = Math.max(2, side * 0.003);
  ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(tx, ty); ctx.stroke();

  const clawImg = huntArt.claw;
  const ch = side * 0.14;
  const cw = ch * 128 / 176;
  ctx.save();
  ctx.translate(tx, ty); ctx.rotate(-angle);
  if (clawImg) drawClaw(ctx, clawImg, { x: -cw / 2, y: -ch * CLAW_GRAB, width: cw, height: ch }, closure);
  else { ctx.strokeStyle = "#e3aa45"; ctx.lineWidth = 5; ctx.beginPath(); ctx.moveTo(-cw * .4, -ch * .3); ctx.lineTo(-cw * .25, ch * .2); ctx.lineTo(0, ch * .3); ctx.lineTo(cw * .25, ch * .2); ctx.lineTo(cw * .4, -ch * .3); ctx.stroke(); }
  ctx.restore();
  const caught = hunt.batch?.shells.find((sh) => sh.id === s.caught_id);
  if (caught) drawShell(caught, tx, ty + ch * 0.02, side * (0.06 + caught.size * 0.005));

  // Water-entry ripple when the claw dives below the surface.
  if (!s.paused && !reducedMotion.matches) {
    if (prevLength === 0 && s.length > 0 && s.phase === "extending") ripples.push({ x: px, y: waterY0, age: 0 });
    for (const r of ripples) {
      r.age += 1 / 30;
      ctx.save(); ctx.globalAlpha = Math.max(0, 1 - r.age / 0.6) * 0.8;
      ctx.strokeStyle = "#c6fff4"; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.ellipse(r.x, r.y, 8 + r.age * 60, 3 + r.age * 18, 0, 0, Math.PI * 2); ctx.stroke(); ctx.restore();
    }
    ripples = ripples.filter((r) => r.age < 0.6);
  } else ripples = [];
  prevLength = s.length;

  // Boat on top.
  const boat = huntArt.boat;
  ctx.save();
  ctx.translate(px, py); ctx.rotate(Math.sin(clock * 1.1) * 0.02);
  ctx.shadowColor = "rgba(0, 20, 25, 0.35)"; ctx.shadowBlur = bw * 0.05;
  if (boat) ctx.drawImage(boat, -bw * BOAT_HATCH.x, -bh * BOAT_HATCH.y, bw, bh);
  else { ctx.fillStyle = "#f0be83"; ctx.fillRect(-bw * .4, -bh * .25, bw * .8, bh * .25); }
  ctx.restore();

  // Status pill, top centre.
  const text = huntMessage() + (hunt.session ? "  ·  " + t("Hôm nay {earned}/{cap}", { earned: hunt.earned, cap: hunt.daily_cap }) : "");
  ctx.font = "600 15px system-ui, sans-serif";
  const w = ctx.measureText(text).width + 36;
  const x = width / 2 - w / 2;
  ctx.fillStyle = "rgba(6, 36, 46, 0.82)";
  ctx.beginPath(); ctx.roundRect(x, 18, w, 34, 17); ctx.fill();
  ctx.fillStyle = "#f4ead2"; ctx.textBaseline = "middle"; ctx.textAlign = "center";
  ctx.fillText(text, width / 2, 36);
  if (performance.now() < notice.until) {
    const nw = richWidth(notice.text) + 36;
    ctx.fillStyle = "rgba(22, 70, 76, 0.9)";
    ctx.beginPath(); ctx.roundRect(width / 2 - nw / 2, 60, nw, 34, 17); ctx.fill();
    ctx.fillStyle = "#ffe5a4"; drawRich(notice.text, width / 2, 78);
  }
  ctx.restore();
}

const COIN_PX = 22;
/** Width of text where the word "CBCoin" is drawn as the CB logo. */
function richWidth(text: string): number {
  const parts = text.split("CBCoin");
  return parts.reduce((w, part) => w + ctx.measureText(part).width, 0) + (parts.length - 1) * (COIN_PX + 4);
}
function drawRich(text: string, cx: number, cy: number) {
  const parts = text.split("CBCoin");
  let x = cx - richWidth(text) / 2;
  const align = ctx.textAlign; ctx.textAlign = "left";
  parts.forEach((part, i) => {
    if (i > 0) {
      if (coinArt) ctx.drawImage(coinArt, x + 2, cy - COIN_PX / 2, COIN_PX, COIN_PX);
      else ctx.fillText("CBCoin", x, cy);
      x += COIN_PX + 4;
    }
    ctx.fillText(part, x, cy); x += ctx.measureText(part).width;
  });
  ctx.textAlign = align;
}

/** Closing animation, catch notice and receipt tracking; runs once per frame. */
function huntTick(dt: number) {
  const s = hunt?.session;
  closure = closureStep(closure, s?.phase, dt, !!s?.paused, reducedMotion.matches);
  const receipt = hunt?.last_catch ?? null;
  if (seenReceipt === undefined) { if (hunt) seenReceipt = receipt?.shell_id ?? null; return; }
  if (receipt && receipt.shell_id !== seenReceipt) {
    seenReceipt = receipt.shell_id;
    notice = {
      text: `+${SHELL_COINS[receipt.kind]} CBCoin · ${t(SHELL_NAME[receipt.kind])}${receipt.rare ? t(" · Sò hiếm!") : ""}${receipt.pearl ? t(" · +1 Ngọc trai!") : ""}${receipt.first_of_kind ? t(" · Thẻ mới trong Bộ sưu tập") : ""}`,
      until: performance.now() + 7000,
    };
  }
}

function draw() {
  ctx.drawImage(background, 0, 0, width, height);
  // Persistent batch, not a new random field on every render/restart.
  const e = ease(huntBlend);
  const f = huntFrame();
  const batch = hunt?.session ? hunt.batch : state?.hunt.batch;
  const carried = hunt?.session?.caught_id ?? null;
  for (const shell of batch?.shells ?? []) {
    if (shell.collected || (e > 0 && shell.id === carried)) continue;
    const idle = { x: shell.x * width, y: height * (0.82 + (shell.y - 0.65) * 0.38), size: 30 + shell.size * 5 };
    const live = { x: f.ox + shell.x * f.side, y: f.oy + shell.y * f.side, size: f.side * (0.06 + shell.size * 0.006) };
    drawShell(shell, lerp(idle.x, live.x, e), lerp(idle.y, live.y, e), lerp(idle.size, live.size, e));
  }
  // The egg den: a pebble-ringed spring pool on open sand in the middle of the seabed
  // (the left side is where desktop icons usually sit).
  if (state) {
    const eggs = state.eggs ?? [];
    const label = eggs.length ? t("Hang trứng · {n} trứng", { n: eggs.length }) : t("Hang trứng");
    drawDen(ctx, width * 0.5, height * 0.915, Math.min(340, Math.min(width, height) * 0.3), eggs, Date.now() / 1000, clock, label, state.used_slots >= state.capacity ? t("chờ chỗ") : t("sắp nở!"), reducedMotion.matches || !!state.settings.meeting_mode);
  }
  ctx.fillStyle = "rgba(220, 245, 255, 0.35)";
  for (const b of bubbles) {
    ctx.beginPath();
    ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2);
    ctx.fill();
  }
  // Food pellets.
  for (const p of pellets) {
    const fade = p.bottomFor > 15 ? 1 - (p.bottomFor - 15) / 5 : 1;
    ctx.globalAlpha = Math.max(0, fade);
    ctx.fillStyle = "#c9782f"; ctx.beginPath(); ctx.arc(p.x, p.y, 3.2, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = "rgba(255, 220, 160, 0.9)"; ctx.beginPath(); ctx.arc(p.x - 1, p.y - 1, 1.2, 0, Math.PI * 2); ctx.fill();
    ctx.globalAlpha = 1;
  }
  // Farther (smaller y) fish first.
  for (const s of [...swimmers].sort((a, b) => a.y - b.y)) {
    const len = fishLength(s.fish);
    const img = sprites.get(s.fish.species_id);
    const bob = Math.sin(s.phase * 0.7) * len * 0.02;
    ctx.save();
    ctx.translate(s.x, s.y + bob);
    ctx.rotate(Math.atan2(s.vy, Math.abs(s.vx) + 1) * 0.35 * Math.sign(s.facing || 1));
    // Mid-turn the body is seen end-on: narrower but never paper thin, and a little taller.
    const turn = Math.abs(s.facing);
    ctx.scale((Math.sign(s.facing) || 1) * Math.max(0.22, turn), 1 + 0.08 * (1 - turn));
    if (img) {
      drawUndulating(s, img, len);
    } else {
      drawFallbackFish(len);
    }
    ctx.restore();
    if (s.fish.resting_until > Date.now() / 1000 && !state?.settings.meeting_mode) {
      // Fed to the next 5-level mark: the fish naps and says so.
      const jokes = [t("No căng vảy! Cho em ngủ tí"), t("Bụng em thành bóng rồi!"), t("Đừng thêm buffet… em xin thua!"), t("Đang tiêu hóa, đừng gọi em đi gym!")];
      const text = jokes[Math.floor((s.fish.exp * 20) / (state?.stage_exp.adult ?? 1000)) % jokes.length]; // a new joke every 5 levels
      ctx.save(); ctx.font = "13px system-ui, sans-serif"; ctx.textBaseline = "alphabetic"; ctx.textAlign = "left";
      const bubbleWidth = ctx.measureText(text).width + 20;
      const bx = Math.max(4, Math.min(width - bubbleWidth - 4, s.x - bubbleWidth / 2));
      const by = Math.max(4, s.y - len * 0.55 - 28);
      ctx.fillStyle = "rgba(255,255,255,.94)"; ctx.beginPath(); ctx.roundRect(bx, by, bubbleWidth, 25, 10); ctx.fill();
      ctx.fillStyle = "#17434b"; ctx.fillText(text, bx + 10, by + 17); ctx.restore();
    }
  }
  drawHunt(e);
}

function frame(now: number) {
  requestAnimationFrame(frame); // browsers stop rAF while the window is hidden
  const fps = !huntActive() && (quiet || state?.settings.meeting_mode) ? QUIET_FPS : VISIBLE_FPS;
  const elapsed = now - lastFrame;
  const interval = 1000 / fps;
  // At 60 FPS follow the display's own refresh; lower caps keep an even cadence.
  if (fps < 60 && elapsed < interval - 2) return;
  lastFrame = now;
  const dt = Math.min(elapsed, 250) / 1000;
  step(dt);
  huntTick(dt);
  draw();
}

async function refresh() {
  try {
    state = await api.state();
  } catch {
    return;
  }
  await Promise.all(
    state.species
      .filter((sp) => !sprites.has(sp.id))
      .map(async (sp) => {
        const img = await loadImage(`/${sp.sprite}`);
        if (img) sprites.set(sp.id, img);
      }),
  );
  for (const sp of state.species) prices.set(sp.id, sp.price);
  syncSwimmers(state.fish);
  // A fish that just ate files gets real food on the desktop too.
  for (const f of state.fish) {
    const total = f.exp + f.pending_exp;
    const before = lastExp.get(f.id);
    if (before !== undefined && total > before && !state.settings.meeting_mode) {
      const s = swimmers.find((w) => w.fish.id === f.id);
      if (s) dropFood(s.x, 4 + Math.floor(Math.random() * 3));
    }
    lastExp.set(f.id, total);
  }
}

async function pollIdle() {
  try {
    quiet = (await api.idleSeconds()) > IDLE_AFTER_SECONDS;
  } catch {
    quiet = false;
  }
}

async function main() {
  oceanImage = await loadImage("/art/ocean.jpg");
  for (const name of ["boat", "claw", "shell_0", "shell_1", "shell_2"]) huntArt[name] = await loadImage(`/art/hunt/${name}.png`);
  coinArt = await loadImage(COIN_SRC);
  resize();
  window.addEventListener("resize", resize);
  await refresh();
  await listen("state-changed", refresh);
  await pollIdle();
  setInterval(pollIdle, 5000);
  setInterval(pollHunt, 50);
  setInterval(refresh, 15000); // rest timers expire without a state-changed event
  requestAnimationFrame(frame);
}

main();
