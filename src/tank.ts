// Desktop ocean. Renders only while visible: 30 FPS normally, 10 FPS when the
// computer is idle or Meeting Mode is on, nothing when the window is hidden.
import { listen } from "@tauri-apps/api/event";
import { api, type Fish, type StateView } from "./api";

const VISIBLE_FPS = 30;
const QUIET_FPS = 10;
const IDLE_AFTER_SECONDS = 60;

// Relative on-screen length per species (renderer detail, not biology data).
const SPECIES_SCALE: Record<string, number> = {
  cyprinus_carpio: 1.15,
  carassius_auratus: 0.85,
  poecilia_reticulata: 0.55,
  betta_splendens: 0.75,
  paracheirodon_innesi: 0.5,
  pterophyllum_scalare: 0.85,
  danio_rerio: 0.55,
  xiphophorus_hellerii: 0.7,
  trichopodus_leerii: 0.8,
  symphysodon_aequifasciatus: 0.9,
};
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
  speed: number;
}

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
      speed: 0.9 + Math.random() * 0.3,
    };
    pickTarget(s);
    return s;
  });
}

function fishLength(f: Fish) {
  return Math.min(width, height) * 0.22 * (SPECIES_SCALE[f.species_id] ?? 0.8) * STAGE_SCALE[f.stage];
}

function step(dt: number) {
  const calm = state?.settings.meeting_mode ? 0.4 : 1;
  for (const s of swimmers) {
    const len = fishLength(s.fish);
    const cruise = (len * 0.35 + 18) * s.speed * calm;
    const dx = s.tx - s.x;
    const dy = s.ty - s.y;
    const dist = Math.hypot(dx, dy);
    if (dist < len * 0.4) pickTarget(s);
    // Steer gently toward the target; fish mostly swim horizontally.
    s.vx += ((dx / (dist || 1)) * cruise - s.vx) * Math.min(1, dt * 0.8);
    s.vy += ((dy / (dist || 1)) * cruise * 0.45 - s.vy) * Math.min(1, dt * 0.8);
    s.x += s.vx * dt;
    s.y += s.vy * dt;
    s.phase += dt * (2 + Math.abs(s.vx) / 40);
    const want = s.vx >= 0 ? 1 : -1;
    s.facing += (want - s.facing) * Math.min(1, dt * 3);
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

function drawFallbackFish(len: number) {
  ctx.fillStyle = "#f2b36b";
  ctx.beginPath();
  ctx.ellipse(0, 0, len * 0.38, len * 0.18, 0, 0, Math.PI * 2);
  ctx.moveTo(-len * 0.32, 0);
  ctx.lineTo(-len * 0.5, -len * 0.14);
  ctx.lineTo(-len * 0.5, len * 0.14);
  ctx.fill();
}

function draw() {
  ctx.drawImage(background, 0, 0, width, height);
  ctx.fillStyle = "rgba(220, 245, 255, 0.35)";
  for (const b of bubbles) {
    ctx.beginPath();
    ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2);
    ctx.fill();
  }
  // Farther (smaller y) fish first.
  for (const s of [...swimmers].sort((a, b) => a.y - b.y)) {
    const len = fishLength(s.fish);
    const img = sprites.get(s.fish.species_id);
    const bob = Math.sin(s.phase * 0.7) * len * 0.02;
    const wiggle = 1 + Math.sin(s.phase * 2.2) * 0.025;
    ctx.save();
    ctx.translate(s.x, s.y + bob);
    ctx.rotate(Math.atan2(s.vy, Math.abs(s.vx) + 1) * 0.35 * Math.sign(s.facing || 1));
    ctx.scale(s.facing * wiggle, 1);
    ctx.shadowColor = "rgba(0, 20, 25, 0.35)";
    ctx.shadowBlur = len * 0.08;
    ctx.shadowOffsetY = len * 0.05;
    if (img) {
      const h = len * (img.height / img.width);
      ctx.drawImage(img, -len / 2, -h / 2, len, h);
    } else {
      drawFallbackFish(len);
    }
    ctx.restore();
  }
}

function frame(now: number) {
  requestAnimationFrame(frame); // browsers stop rAF while the window is hidden
  const fps = quiet || state?.settings.meeting_mode ? QUIET_FPS : VISIBLE_FPS;
  const elapsed = now - lastFrame;
  if (elapsed < 1000 / fps - 1) return;
  lastFrame = now;
  step(Math.min(elapsed, 250) / 1000);
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
  syncSwimmers(state.fish);
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
  resize();
  window.addEventListener("resize", resize);
  await refresh();
  await listen("state-changed", refresh);
  await pollIdle();
  setInterval(pollIdle, 5000);
  requestAnimationFrame(frame);
}

main();
