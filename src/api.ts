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
  origin: "starter" | "shop";
  parent_ids: string[];
  generation: number;
  stage: Stage;
  exp: number;
  acquired_at: number;
}

export interface Settings {
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
  shells: number;
  capacity: number;
  fish: Fish[];
  daily: { shells: number; exp: number; shell_cap: number; exp_cap: number };
  dex: Record<string, { seen: boolean; owned: boolean }>;
  settings: Settings;
  species: Species[];
  disclaimer: string;
  stage_exp: { juvenile: number; adult: number };
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

export const api = {
  state: () => invoke<StateView>("get_state"),
  preview: (paths: string[]) => invoke<Inspection[]>("preview_files", { paths }),
  feed: (items: Inspection[], fishId: string) => invoke<FeedReport>("feed", { items, fishId }),
  buy: (speciesId: string, price: number) => invoke<Fish>("buy", { speciesId, price }),
  bellyList: () => invoke<BellyEntry[]>("belly_list"),
  restore: (entryId: string) => invoke<{ path: string; renamed: boolean }>("belly_restore", { entryId }),
  recover: () => invoke<StateView>("belly_recover"),
  setTank: (enabled: boolean) => invoke<StateView>("set_tank", { enabled }),
  setMeetingMode: (enabled: boolean) => invoke<StateView>("set_meeting_mode", { enabled }),
  finishOnboarding: () => invoke<StateView>("finish_onboarding"),
  createSample: () => invoke<string>("create_sample_file"),
  idleSeconds: () => invoke<number>("system_idle_seconds"),
  quit: () => invoke<void>("quit_app"),
};

export function asFailure(e: unknown): Failure {
  if (e && typeof e === "object" && "code" in e) return e as Failure;
  return { code: "unknown", detail: String(e), undetermined: false };
}
