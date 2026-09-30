#!/usr/bin/env node
// Writes a fake game save full of fish into a QA profile, for testing the
// tank renderer and UI. Never touches the real profile.
//
//   node scripts/seed-fish.mjs --dir /tmp/tq-qa [--count 30] [--shells 5000] [--seed 1]
//   TRASHQUARIUM_DATA_DIR=/tmp/tq-qa npm run tauri dev
//
// Shapes mirror src-tauri/src/engine/game.rs (schema_version 1).

import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { homedir, platform } from "node:os";
import { join, resolve } from "node:path";

const args = Object.fromEntries(
  process.argv.slice(2).reduce((pairs, arg, i, all) => {
    if (arg.startsWith("--")) pairs.push([arg.slice(2), all[i + 1]]);
    return pairs;
  }, []),
);
if (!args.dir) {
  console.error("usage: node scripts/seed-fish.mjs --dir <qa-profile-dir> [--count 30] [--shells 5000] [--seed 1]");
  process.exit(1);
}

const dir = resolve(args.dir);
const realProfile =
  platform() === "win32"
    ? join(process.env.LOCALAPPDATA ?? "", "TrashQuarium")
    : platform() === "darwin"
      ? join(homedir(), "Library/Application Support/TrashQuarium")
      : join(homedir(), ".local/share/TrashQuarium");
if (dir.toLowerCase() === resolve(realProfile).toLowerCase()) {
  console.error(`Refusing to seed the real profile (${realProfile}). Use a QA folder.`);
  process.exit(1);
}

const count = Number(args.count ?? 30);
const shells = Number(args.shells ?? 5000);
let seed = Number(args.seed ?? 1) >>> 0 || 1;
const random = () => {
  // xorshift32: the same --seed always gives the same tank
  seed ^= seed << 13;
  seed ^= seed >>> 17;
  seed ^= seed << 5;
  return (seed >>> 0) / 2 ** 32;
};
const pick = (list) => list[Math.floor(random() * list.length)];
const uuid = () =>
  "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = Math.floor(random() * 16);
    return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
  });

const here = new URL(".", import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(here, "../src-tauri/config/species.json"), "utf8"));
const balance = JSON.parse(readFileSync(join(here, "../src-tauri/config/balance.json"), "utf8"));
const { juvenile, adult } = balance.stage_exp;
const petNames = ["Mít", "Bơ", "Su Su", "Kem", "Bắp", "Tiêu", "Gừng", "Mochi", "Bông", "Cốm", "Na", "Sò", "Ốc", "Rong", "Mây", "Sao", "Bọt", "Nemo", "Bi", "Cam"];

const now = Math.floor(Date.now() / 1000);
const fish = [];
for (let i = 0; i < count; i++) {
  // Cycle through all species first so every sprite shows up, then random.
  const species = i < catalog.species.length ? catalog.species[i] : pick(catalog.species);
  const stage = pick(["fry", "juvenile", "juvenile", "adult", "adult"]);
  const exp =
    stage === "fry"
      ? Math.floor(random() * juvenile)
      : stage === "juvenile"
        ? juvenile + Math.floor(random() * (adult - juvenile))
        : adult + Math.floor(random() * 400);
  fish.push({
    id: uuid(),
    species_id: species.id,
    name: random() < 0.6 ? `${pick(petNames)} ${i + 1}` : species.name,
    origin: i === 0 ? "starter" : "shop",
    parent_ids: [],
    generation: 0,
    stage,
    exp,
    traits: {},
    visual_recipe: { base: species.id, renderer_version: 1 },
    rules_version: 1,
    acquired_at: now - Math.floor(random() * 60 * 86400),
  });
}

const dex = {};
for (const f of fish) dex[f.species_id] = { seen: true, owned: true };

const save = {
  schema_version: 1,
  rules_version: 1,
  created_at: now - 60 * 86400,
  fish,
  wallet: { shells },
  welcome_granted: true,
  ledger: { receipts: [], fingerprints: [] },
  daily: { date: "", shells: 0, exp: 0, rewarded: 0 },
  dex,
  settings: { tank_enabled: false, meeting_mode: false, onboarding_done: true },
};

mkdirSync(dir, { recursive: true });
const path = join(dir, "game_save.json");
if (existsSync(path)) {
  const kept = `${path}.pre-seed-${now}`;
  renameSync(path, kept);
  console.log(`Existing save kept as ${kept}`);
}
writeFileSync(path, JSON.stringify(save, null, 2));
console.log(`Wrote ${count} fish and ${shells} Vỏ sò to ${path}`);
if (count > balance.tank_capacity) {
  console.log(`Note: ${count} fish is above the tank capacity of ${balance.tank_capacity}; the shop will show "Bể đầy".`);
}
