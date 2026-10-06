// Typed wrappers around the Rust commands. Shapes mirror src-tauri/src/engine.
import { invoke } from "@tauri-apps/api/core";

export type Stage = "fry" | "juvenile" | "adult";
export type Category = "doc" | "media" | "tech";

export interface Failure {
  code: string;
  detail: string;
  undetermined: boolean;
}

export interface Species {
  id: string;
  name: string;
  scientific_name: string;
  class: string;
  breeding_group: string;
  price: number;
  sprite: string;
  reproduction: "egg" | "live_birth";
  stage_on_purchase: Stage;
  fact: string;
  source: string;
  editorial_status: string;
}

export interface Fish {
  id: string;
  species_id: string;
  name: string;
  origin: "starter" | "shop" | "hatched";
  parent_ids: string[];
  generation: number;
  stage: Stage;
  exp: number;
  pending_exp: number;
  resting_until: number;
  purchase_price: number;
  eggs_used: number;
  acquired_at: number;
}

export interface Breeding {
  hatch_max: number;
  hatch_min: number;
  price_low: number;
  price_high: number;
  incubate_min_s: number;
  incubate_max_s: number;
  den_capacity: number;
}

export interface RedeemOutcome {
  kind: "cbcoins" | "full_exp";
  cbcoins: number;
  fish: number;
}

export interface Egg {
  id: string;
  species_id: string;
  parent_ids: string[];
  generation: number;
  laid_at: number;
  hatch_at: number;
}

export interface HatchResult {
  egg_id: string;
  species_id: string;
  hatched: boolean;
  fish_id: string | null;
  at: number;
}

export interface BreedOutcome {
  laid: number;
  hatch_rate: number;
  eggs: Egg[];
}

export interface Settings {
  quick_dock_enabled: boolean;
  tank_enabled: boolean;
  meeting_mode: boolean;
  onboarding_done: boolean;
}

export interface Attention {
  transaction_id: string;
  op: "swallow" | "restore" | null;
  name: string;
  from: string;
  to: string;
  note: string;
}

export interface StateView {
  hunt: HuntView;
  cbcoins: number;
  capacity: number;
  fish: Fish[];
  daily: { shells: number; exp: number; shell_cap: number; exp_cap: number };
  dex: Record<string, { seen: boolean; owned: boolean }>;
  settings: Settings;
  species: Species[];
  disclaimer: string;
  stage_exp: { juvenile: number; adult: number };
  breeding: Breeding;
  slots: { medium_price: number; large_price: number };
  used_slots: number;
  eggs: Egg[];
  hatch_log: HatchResult[];
  max_preview_files: number;
  read_only: Failure | null;
  recovered_from_backup: boolean;
  belly_error: Failure | null;
  attention: Attention[];
  held_count: number;
  data_dir: string;
}

export interface Inspection {
  path: string;
  name: string;
  ok: boolean;
  code: string;
  size: number;
  category: Category | null;
  modified_unix: number;
  fingerprint: string | null;
}

export interface FeedReport {
  files: { path: string; name: string; ok: boolean; code: string; undetermined: boolean }[];
  reward: { files: number; shells: number; exp: number; duplicates: number; not_rewardable: number; capped: boolean } | null;
  reward_error: Failure | null;
}

export interface BellyEntry {
  id: string;
  original_path: string;
  name: string;
  size: number;
  category: Category;
  eaten_at: number;
  fish_id: string;
}

export type ShellKind = "great" | "queen" | "variegated";
export interface HuntShell { id: string; x: number; y: number; size: number; collected: boolean; kind: ShellKind; rare: boolean; pearl: boolean }
export interface ShellEntry { count: number; rare_count: number; first_found: string }
export interface CatchReceipt { shell_id: string; kind: ShellKind; rare: boolean; pearl: boolean; first_of_kind: boolean }
export interface FishTarget { id: string; species_id: string; x: number; y: number; dir: number; radius: number }
export interface FishSale { fish_id: string; species_id: string; coins: number }
export interface HuntSession {
  id: string; batch_id: string; phase: string; angle: number; length: number;
  caught_id: string | null; paused: boolean; error: string | null;
  /** Adult fish swimming in the hunt frame; `caught_fish` is on the claw or on deck ("deciding"). */
  fish: FishTarget[]; caught_fish: string | null;
}
export interface HuntView {
  pearls: number;
  collection: Partial<Record<ShellKind, ShellEntry>>;
  last_catch: CatchReceipt | null;
  batch: { id: string; shells: HuntShell[] } | null;
  earned: number; daily_cap: number; session: HuntSession | null; waiting: boolean;
  last_sale: FishSale | null;
}

export const api = {
  state: () => invoke<StateView>("get_state"),
  preview: (paths: string[]) => invoke<Inspection[]>("preview_files", { paths }),
  feed: (items: Inspection[], fishId: string) => invoke<FeedReport>("feed", { items, fishId }),
  buy: (speciesId: string, price: number) => invoke<Fish>("buy", { speciesId, price }),
  redeem: (code: string) => invoke<RedeemOutcome>("redeem_code", { code }),
  sell: (fishId: string) => invoke<number>("sell_fish", { fishId }),
  breed: (firstId: string, secondId: string, eggs: number) => invoke<BreedOutcome>("breed_fish", { firstId, secondId, eggs }),
  bellyList: () => invoke<BellyEntry[]>("belly_list"),
  restore: (entryId: string) => invoke<{ path: string; renamed: boolean }>("belly_restore", { entryId }),
  recover: () => invoke<StateView>("belly_recover"),
  setTank: (enabled: boolean) => invoke<StateView>("set_tank", { enabled }),
  setMeetingMode: (enabled: boolean) => invoke<StateView>("set_meeting_mode", { enabled }),
  finishOnboarding: () => invoke<StateView>("finish_onboarding"),
  createSample: () => invoke<string>("create_sample_file"),
  idleSeconds: () => invoke<number>("system_idle_seconds"),
  setTrayLanguage: (lang: string) => invoke<void>("set_tray_language", { lang }),
  quit: () => invoke<void>("quit_app"),
  huntStatus: () => invoke<HuntView>("hunt_status"),
  startHunt: () => invoke<HuntView>("start_hunt"),
  huntAction: (sessionId: string, seq: number, action: "drop" | "pause" | "resume" | "leave" | "keep" | "sell") => invoke<HuntView>("hunt_action", { sessionId, seq, action }),
  openTab: (tab: string) => invoke<void>("open_manager_tab", { tab }),
  takeRoute: () => invoke<string | null>("take_manager_route"),
  setDock: (enabled: boolean) => invoke<StateView>("set_quick_dock", { enabled }),
  resizeDock: (expanded: boolean) => invoke<void>("resize_quick_dock", { expanded }),
  autostartStatus: () => invoke<boolean>("autostart_status"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),
};

export function asFailure(e: unknown): Failure {
  if (e && typeof e === "object" && "code" in e) return e as Failure;
  return { code: "unknown", detail: String(e), undetermined: false };
}
