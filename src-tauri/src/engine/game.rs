//! Game save: fish, CBCoin wallet, reward ledger, Fishdex flags and settings.
//!
//! Every change is made on a copy and only becomes current after the atomic
//! save succeeds, so a failed save never loses shells or creates a fish.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use super::belly::BellyEntry;
use super::catalog::{Catalog, Species};
use super::save::{self, Loaded};
use super::Failure;

pub const SCHEMA_VERSION: u64 = 5;
/// Growth (schema 5): 10 EXP = 1 level, Lv.50 juvenile, Lv.100 adult, a 2-hour rest every 5 levels.
pub const EXP_PER_LEVEL: u64 = 10;
pub const MAX_EXP: u64 = 100 * EXP_PER_LEVEL;
pub const JUVENILE_EXP: u64 = 50 * EXP_PER_LEVEL;
pub const REST_EVERY_EXP: u64 = 5 * EXP_PER_LEVEL;
pub const REST_SECONDS: i64 = 7200;
pub const RULES_VERSION: u32 = 1;
pub const RENDERER_VERSION: u32 = 1;
/// Sale value runs from price x100 down to price x1: at most 99 eggs per fish.
pub const MAX_EGGS_PER_FISH: u32 = 99;
const BACKUPS: usize = 3;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Fry,
    Juvenile,
    Adult,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Starter,
    Shop,
    Hatched,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VisualRecipe {
    pub base: String,
    pub renderer_version: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Fish {
    pub id: String,
    pub species_id: String,
    pub name: String,
    pub origin: Origin,
    /// Known parents. Starter and shop fish have none; no parents are invented.
    pub parent_ids: Vec<String>,
    pub generation: u32,
    pub stage: Stage,
    pub exp: u64,
    #[serde(default)] pub pending_exp: u64,
    #[serde(default)] pub resting_until: i64,
    #[serde(default)] pub purchase_price: u64,
    /// Eggs this fish has laid. Each costs one purchase price of sale value.
    #[serde(default)] pub eggs_used: u32,
    pub traits: BTreeMap<String, String>,
    pub visual_recipe: VisualRecipe,
    pub rules_version: u32,
    pub acquired_at: i64,
}

impl Fish {
    /// What the sale boat pays. Adults (Lv.100): purchase price x100, minus one purchase price
    /// per egg laid. Younger fish (a wrong buy, a full tank): half the purchase price.
    pub fn sale_value(&self) -> Option<u64> {
        if self.exp < MAX_EXP { return Some(self.purchase_price / 2); }
        self.purchase_price.checked_mul(100u64.saturating_sub(self.eggs_used as u64))
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct RedeemOutcome {
    /// "cbcoins" or "full_exp"
    pub kind: String,
    pub cbcoins: u64,
    pub fish: u32,
}

/// An egg in the egg den. It hatches (or not) once `hatch_at` has passed and the tank
/// has room for the young fish.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Egg {
    pub id: String,
    pub species_id: String,
    pub parent_ids: Vec<String>,
    pub generation: u32,
    pub laid_at: i64,
    pub hatch_at: i64,
}

/// What became of one egg, newest first in `GameState::hatch_log`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HatchResult {
    pub egg_id: String,
    pub species_id: String,
    pub hatched: bool,
    pub fish_id: Option<String>,
    pub at: i64,
}
const HATCH_LOG: usize = 30;

/// Result of one breeding: all eggs go into the egg den.
#[derive(Serialize, Clone, Debug)]
pub struct BreedOutcome {
    pub laid: u32,
    pub hatch_rate: f64,
    pub eggs: Vec<Egg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Wallet {
    #[serde(alias = "shells")] pub cbcoins: u64,
    #[serde(default)] pub pearls: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RewardLedger {
    /// Every Belly receipt already settled, rewarded or not. Never pruned.
    pub receipts: BTreeSet<String>,
    /// Fingerprints already rewarded, never pruned (copy/rename protection).
    pub fingerprints: VecDeque<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Daily {
    pub date: String,
    pub shells: u64,
    pub exp: u64,
    pub rewarded: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DexEntry {
    pub seen: bool,
    pub owned: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    #[serde(default = "dock_default")]
    pub quick_dock_enabled: bool,
    pub tank_enabled: bool,
    pub meeting_mode: bool,
    pub onboarding_done: bool,
}
fn dock_default() -> bool { true }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState {
    pub shell_hunt: super::hunt::HuntState,
    pub schema_version: u64,
    pub rules_version: u32,
    pub created_at: i64,
    pub fish: Vec<Fish>,
    pub wallet: Wallet,
    pub welcome_granted: bool,
    pub ledger: RewardLedger,
    pub daily: Daily,
    pub dex: BTreeMap<String, DexEntry>,
    pub settings: Settings,
    /// The egg den: eggs waiting to hatch.
    #[serde(default)] pub eggs: Vec<Egg>,
    #[serde(default)] pub hatch_log: VecDeque<HatchResult>,
}

impl GameState {
    fn blank(now: i64) -> Self {
        GameState {
            shell_hunt: super::hunt::HuntState::default(),
            schema_version: SCHEMA_VERSION,
            rules_version: RULES_VERSION,
            created_at: now,
            fish: Vec::new(),
            wallet: Wallet::default(),
            welcome_granted: false,
            ledger: RewardLedger::default(),
            daily: Daily::default(),
            dex: BTreeMap::new(),
            settings: Settings { tank_enabled: false, meeting_mode: false, onboarding_done: false, quick_dock_enabled: true },
            eggs: Vec::new(),
            hatch_log: VecDeque::new(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        self.shell_hunt.validate()?;
        let mut ids = BTreeSet::new();
        for f in &self.fish {
            if f.id.is_empty() || !ids.insert(f.id.as_str()) {
                return Err(format!("invalid or duplicate fish id {:?}", f.id));
            }
            if f.species_id.is_empty() {
                return Err(format!("fish {} has no species", f.id));
            }
            if f.eggs_used > MAX_EGGS_PER_FISH {
                return Err(format!("invalid egg count {}", f.id));
            }
            if f.exp > MAX_EXP || f.pending_exp > MAX_EXP - f.exp || f.resting_until < 0 {
                return Err(format!("invalid growth state {}", f.id));
            }
        }
        let mut egg_ids = BTreeSet::new();
        for e in &self.eggs {
            if e.id.is_empty() || !egg_ids.insert(e.id.as_str()) || e.species_id.is_empty() || e.hatch_at < e.laid_at {
                return Err(format!("invalid egg {:?}", e.id));
            }
        }
        if !self.daily.date.is_empty() && chrono::NaiveDate::parse_from_str(&self.daily.date, "%Y-%m-%d").is_err() {
            return Err(format!("invalid daily date {:?}", self.daily.date));
        }
        Ok(())
    }
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct RewardSummary {
    pub files: u32,
    pub shells: u64,
    pub exp: u64,
    pub duplicates: u32,
    pub not_rewardable: u32,
    pub capped: bool,
}

pub struct GameStore {
    pub hunt_session: Option<super::hunt::HuntSession>,
    hunt_checkpoint_ms: u64,
    path: PathBuf,
    pub state: GameState,
    /// Set when the save must not be written (newer version or unreadable).
    pub read_only: Option<Failure>,
    pub recovered_from_backup: bool,
    catalog: Catalog,
}

impl GameStore {
    pub fn open(path: &Path, catalog: Catalog, now: i64) -> Self {
        let mut store = GameStore {
            hunt_session: None,
            hunt_checkpoint_ms: 0,
            path: path.to_path_buf(),
            state: GameState::blank(now),
            read_only: None,
            recovered_from_backup: false,
            catalog,
        };
        let loaded = save::load_with_backups(path, BACKUPS, SCHEMA_VERSION, |mut v| {
            let version = v.get("schema_version").and_then(serde_json::Value::as_u64).unwrap_or(0);
            if version == 1 {
                v["shell_hunt"] = serde_json::to_value(super::hunt::HuntState::default()).map_err(|e| e.to_string())?;
                v["settings"]["quick_dock_enabled"] = serde_json::Value::Bool(true);
            } else if !(2..=SCHEMA_VERSION).contains(&version) { return Err("unsupported save schema".into()); }
            if version == 1 || version == 2 {
                // Keep legacy batches/IDs and never reroll rare rewards during migration.
                if let Some(shells) = v.get_mut("shell_hunt").and_then(|h| h.get_mut("batch")).and_then(|b| b.get_mut("shells")).and_then(serde_json::Value::as_array_mut) {
                    for shell in shells {
                        shell["kind"] = serde_json::json!(match shell["size"].as_u64().unwrap_or(0) { 1 => "queen", 2 => "variegated", _ => "great" });
                        shell["rare"] = serde_json::json!(false);
                        shell["pearl"] = serde_json::json!(false);
                    }
                }
                v["schema_version"] = serde_json::Value::from(SCHEMA_VERSION);
            }
            if version < 4 {
                if let Some(fish) = v.get_mut("fish").and_then(serde_json::Value::as_array_mut) {
                    for f in fish {
                        // Preserve achieved life stage; scale legacy progress to new thresholds.
                        let old = f["exp"].as_u64().unwrap_or(0);
                        let exp = match f["stage"].as_str() { Some("adult") => 10000, Some("juvenile") => 5000 + old.saturating_sub(20).min(39) * 100, _ => old.min(19) * 250 };
                        f["exp"] = serde_json::json!(exp);
                        let price = f["species_id"].as_str().and_then(|id| store.catalog.species(id)).map(|s| s.price).unwrap_or(0);
                        f["purchase_price"] = serde_json::json!(price);
                    }
                }
                v["schema_version"] = serde_json::json!(SCHEMA_VERSION);
            }
            if version < 5 {
                // Schema 5 uses 10 EXP per level instead of 100: levels are kept, EXP shrinks tenfold.
                if let Some(fish) = v.get_mut("fish").and_then(serde_json::Value::as_array_mut) {
                    for f in fish {
                        for key in ["exp", "pending_exp"] {
                            let old = f[key].as_u64().unwrap_or(0);
                            f[key] = serde_json::json!(old / 10);
                        }
                    }
                }
                v["schema_version"] = serde_json::json!(SCHEMA_VERSION);
            }
            let s: GameState = serde_json::from_value(v).map_err(|e| e.to_string())?;
            s.validate()?;
            Ok(s)
        });
        match loaded {
            Loaded::Missing => {
                let mut fresh = GameState::blank(now);
                let starter = store.catalog.species(&store.catalog.balance.starter_species).cloned();
                if let Some(sp) = starter {
                    fresh.fish.push(store.new_fish(&sp, Origin::Starter, now));
                    mark_owned(&mut fresh, &sp.id);
                }
                fresh.wallet.cbcoins = store.catalog.balance.economy.welcome_shells;
                fresh.welcome_granted = true;
                if let Err(e) = store.commit(fresh) {
                    store.read_only = Some(e);
                }
            }
            Loaded::Ok { value, from_backup } => {
                store.state = value;
                store.recovered_from_backup = from_backup.is_some();
            }
            Loaded::Future { version } => {
                store.read_only = Some(Failure::new("save_future", format!("save version {version}")));
            }
            Loaded::Corrupt { reason } => {
                store.read_only = Some(Failure::new("save_corrupt", reason));
            }
        }
        store
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Writes `next` atomically and makes it current only on success.
    fn commit(&mut self, next: GameState) -> Result<(), Failure> {
        if let Some(e) = &self.read_only {
            return Err(e.clone());
        }
        save::write_with_backups(&self.path, &next, BACKUPS).map_err(|e| Failure::new("save_failed", e))?;
        self.state = next;
        Ok(())
    }

    fn new_fish(&self, species: &Species, origin: Origin, now: i64) -> Fish {
        let stage = Stage::Fry;
        Fish {
            id: uuid::Uuid::new_v4().to_string(),
            species_id: species.id.clone(),
            name: species.name.clone(),
            origin,
            parent_ids: Vec::new(),
            generation: 0,
            stage,
            exp: 0, pending_exp: 0, resting_until: 0, purchase_price: species.price, eggs_used: 0,
            traits: BTreeMap::new(),
            visual_recipe: VisualRecipe { base: species.id.clone(), renderer_version: RENDERER_VERSION },
            rules_version: RULES_VERSION,
            acquired_at: now,
        }
    }

    /// Buys one fish. `expected_price` is the price the player confirmed; if the
    /// catalog price differs the purchase is refused rather than charging more.
    pub fn purchase(&mut self, species_id: &str, expected_price: u64, now: i64) -> Result<Fish, Failure> {
        let species = self
            .catalog
            .species(species_id)
            .cloned()
            .ok_or_else(|| Failure::new("unknown_species", species_id))?;
        if species.price != expected_price {
            return Err(Failure::new("price_changed", species.price));
        }
        if self.used_slots() + self.catalog.balance.slots.for_price(species.price) > self.catalog.balance.tank_capacity {
            return Err(Failure::new("tank_full", self.catalog.balance.tank_capacity));
        }
        if self.state.wallet.cbcoins < species.price {
            return Err(Failure::new("not_enough_shells", species.price - self.state.wallet.cbcoins));
        }
        let fish = self.new_fish(&species, Origin::Shop, now);
        let mut next = self.state.clone();
        next.wallet.cbcoins -= species.price;
        next.fish.push(fish.clone());
        mark_owned(&mut next, &species.id);
        self.commit(next)?;
        Ok(fish)
    }

    /// Tank slots a fish of this species takes (1–3, by catalog price).
    pub fn slots_for(&self, species_id: &str) -> usize {
        self.catalog.species(species_id).map(|s| self.catalog.balance.slots.for_price(s.price)).unwrap_or(1)
    }

    /// Slots used by every fish now in the tank. Old saves may exceed the capacity; they keep
    /// their fish but can't add more until there is room.
    pub fn used_slots(&self) -> usize {
        self.state.fish.iter().map(|f| self.slots_for(&f.species_id)).sum()
    }

    pub fn can_feed(&self, fish_id: &str, now: i64) -> Result<(), Failure> {
        let f = self.state.fish.iter().find(|f| f.id == fish_id).ok_or_else(|| Failure::new("fish_not_found", fish_id))?;
        if f.exp >= MAX_EXP { return Err(Failure::new("fish_adult", "level 100")); }
        if f.resting_until > now { return Err(Failure::new("fish_full", f.resting_until)); }
        Ok(())
    }

    /// Calling the sale boat transfers only the game fish, never its Belly files.
    pub fn sell(&mut self, fish_id: &str) -> Result<u64, Failure> {
        let mut next = self.state.clone();
        let i = next.fish.iter().position(|f| f.id == fish_id).ok_or_else(|| Failure::new("fish_not_found", fish_id))?;
        let price = next.fish[i].sale_value().ok_or_else(|| Failure::new("wallet_overflow", "sale"))?;
        next.wallet.cbcoins = next.wallet.cbcoins.checked_add(price).ok_or_else(|| Failure::new("wallet_overflow", "sale"))?;
        next.fish.remove(i);
        self.commit(next)?;
        Ok(price)
    }

    /// Breeds two adult fish of the same species. Each egg costs both parents one purchase
    /// price of sale value (never below one purchase price) and goes into the egg den, where it
    /// incubates for 2–3 hours. `roll` (0..1) picks each egg's incubation time.
    pub fn breed_with(&mut self, a_id: &str, b_id: &str, eggs: u32, now: i64, roll: &mut dyn FnMut() -> f64) -> Result<BreedOutcome, Failure> {
        if a_id == b_id { return Err(Failure::new("breed_same_fish", a_id)); }
        let find = |id: &str| self.state.fish.iter().position(|f| f.id == id).ok_or_else(|| Failure::new("fish_not_found", id.to_string()));
        let (ai, bi) = (find(a_id)?, find(b_id)?);
        let (a, b) = (&self.state.fish[ai], &self.state.fish[bi]);
        if a.exp < MAX_EXP || b.exp < MAX_EXP { return Err(Failure::new("fish_not_adult", "level 100 required")); }
        if a.species_id != b.species_id { return Err(Failure::new("breed_species_mismatch", &a.species_id)); }
        let room = (MAX_EGGS_PER_FISH - a.eggs_used.min(MAX_EGGS_PER_FISH)).min(MAX_EGGS_PER_FISH - b.eggs_used.min(MAX_EGGS_PER_FISH));
        if room == 0 { return Err(Failure::new("breed_exhausted", "sale value is back to the purchase price")); }
        if eggs == 0 || eggs > room { return Err(Failure::new("breed_eggs_invalid", room)); }
        let cfg = self.catalog.balance.breeding.clone();
        let den_room = cfg.den_capacity.saturating_sub(self.state.eggs.len());
        if den_room == 0 { return Err(Failure::new("den_full", cfg.den_capacity)); }
        if eggs as usize > den_room { return Err(Failure::new("breed_eggs_invalid", den_room)); }
        let species = self.catalog.species(&a.species_id).cloned().ok_or_else(|| Failure::new("unknown_species", a.species_id.clone()))?;
        let generation = a.generation.max(b.generation) + 1;
        let parents = vec![a.id.clone(), b.id.clone()];
        let span = (cfg.incubate_max_s - cfg.incubate_min_s) as f64;
        let laid: Vec<Egg> = (0..eggs).map(|_| Egg {
            id: uuid::Uuid::new_v4().to_string(),
            species_id: species.id.clone(),
            parent_ids: parents.clone(),
            generation,
            laid_at: now,
            hatch_at: now + cfg.incubate_min_s + (roll().clamp(0.0, 1.0) * span).round() as i64,
        }).collect();
        let mut next = self.state.clone();
        next.fish[ai].eggs_used += eggs;
        next.fish[bi].eggs_used += eggs;
        next.eggs.extend(laid.iter().cloned());
        self.commit(next)?;
        Ok(BreedOutcome { laid: eggs, hatch_rate: cfg.hatch_rate(species.price), eggs: laid })
    }

    pub fn breed(&mut self, a_id: &str, b_id: &str, eggs: u32, now: i64) -> Result<BreedOutcome, Failure> {
        self.breed_with(a_id, b_id, eggs, now, &mut random01)
    }

    /// Settles every egg whose time is up: it hatches into a Lv.0 fry with the species' hatch
    /// chance, otherwise it is gone. An egg whose fry would not fit in the tank waits in the
    /// den (unrolled) until there is room. Returns true when something changed.
    pub fn hatch_due_with(&mut self, now: i64, roll: &mut dyn FnMut() -> f64) -> Result<bool, Failure> {
        if self.read_only.is_some() || !self.state.eggs.iter().any(|e| e.hatch_at <= now) { return Ok(false); }
        let mut next = self.state.clone();
        let mut due: Vec<Egg> = next.eggs.iter().filter(|e| e.hatch_at <= now).cloned().collect();
        due.sort_by_key(|e| e.hatch_at);
        let cap = self.catalog.balance.tank_capacity;
        let mut used: usize = next.fish.iter().map(|f| self.slots_for(&f.species_id)).sum();
        let mut changed = false;
        for egg in due {
            let Some(species) = self.catalog.species(&egg.species_id).cloned() else {
                next.eggs.retain(|e| e.id != egg.id); changed = true; continue;
            };
            let slots = self.catalog.balance.slots.for_price(species.price);
            if used + slots > cap { continue; } // wait in the den until the tank has room
            next.eggs.retain(|e| e.id != egg.id);
            changed = true;
            let hatched = roll() < self.catalog.balance.breeding.hatch_rate(species.price);
            let mut fish_id = None;
            if hatched {
                let mut kid = self.new_fish(&species, Origin::Hatched, now);
                kid.parent_ids = egg.parent_ids.clone();
                kid.generation = egg.generation;
                fish_id = Some(kid.id.clone());
                next.fish.push(kid);
                mark_owned(&mut next, &species.id);
                used += slots;
            }
            next.hatch_log.push_front(HatchResult { egg_id: egg.id.clone(), species_id: species.id.clone(), hatched, fish_id, at: now });
            next.hatch_log.truncate(HATCH_LOG);
        }
        if !changed { return Ok(false); }
        self.commit(next)?;
        Ok(true)
    }

    pub fn hatch_due(&mut self, now: i64) -> Result<bool, Failure> {
        self.hatch_due_with(now, &mut random01)
    }

    /// Redeems a gift code. The two admin test codes can be used again and again.
    pub fn redeem(&mut self, code: &str) -> Result<RedeemOutcome, Failure> {
        if code.trim().is_empty() { return Err(Failure::new("code_invalid", "empty")); }
        let hash = code_hash(code);
        let mut next = self.state.clone();
        let outcome = if hash == ADMIN_COINS_SHA256 {
            next.wallet.cbcoins = next.wallet.cbcoins.checked_add(ADMIN_COINS).ok_or_else(|| Failure::new("wallet_overflow", "code"))?;
            RedeemOutcome { kind: "cbcoins".into(), cbcoins: ADMIN_COINS, fish: 0 }
        } else if hash == ADMIN_FULL_EXP_SHA256 {
            for f in &mut next.fish {
                f.exp = MAX_EXP; f.pending_exp = 0; f.resting_until = 0; f.stage = Stage::Adult;
            }
            RedeemOutcome { kind: "full_exp".into(), cbcoins: 0, fish: next.fish.len() as u32 }
        } else {
            return Err(Failure::new("code_invalid", "unknown code"));
        };
        self.commit(next)?;
        Ok(outcome)
    }

    pub fn digest(&mut self, now: i64) -> Result<(), Failure> {
        if !self.state.fish.iter().any(|f| f.pending_exp > 0 && f.resting_until <= now) { return Ok(()); }
        let mut next = self.state.clone();
        for f in &mut next.fish { digest_fish(f, now); }
        self.commit(next)
    }

    /// Settles every Belly receipt not yet in the ledger, exactly once: the
    /// wallet, EXP and ledger are written in one atomic save. If the save fails
    /// nothing changes and the receipts stay pending for the next attempt.
    pub fn apply_receipts(&mut self, entries: &[BellyEntry], today: &str, now: i64) -> Result<RewardSummary, Failure> {
        let pending: Vec<&BellyEntry> = entries
            .iter()
            .filter(|e| !self.state.ledger.receipts.contains(&e.receipt_id))
            .collect();
        let mut summary = RewardSummary::default();
        if pending.is_empty() {
            return Ok(summary);
        }
        let mut next = self.state.clone();
        roll_day(&mut next.daily, today);
        for e in pending {
            next.ledger.receipts.insert(e.receipt_id.clone());
            if !e.rewardable {
                summary.not_rewardable += 1;
                continue;
            }
            if next.ledger.fingerprints.contains(&e.fingerprint) {
                summary.duplicates += 1;
                continue;
            }
            next.ledger.fingerprints.push_back(e.fingerprint.clone());
            let exp = file_exp(e.size);
            next.daily.rewarded += 1;
            if let Some(fish) = next.fish.iter_mut().find(|f| f.id == e.fish_id) {
                let room = MAX_EXP.saturating_sub(fish.exp + fish.pending_exp);
                let credited = exp.min(room);
                fish.pending_exp += credited;
                digest_fish(fish, now);
                summary.exp += credited;
                next.daily.exp += credited;
            }
            summary.files += 1;
        }
        self.commit(next)?;
        Ok(summary)
    }

    pub fn update_settings(&mut self, change: impl FnOnce(&mut Settings)) -> Result<(), Failure> {
        let mut next = self.state.clone();
        change(&mut next.settings);
        self.commit(next)
    }

    pub fn hunt_view(&self) -> super::hunt::HuntView {
        let h = &self.state.shell_hunt;
        super::hunt::HuntView { pearls: self.state.wallet.pearls, collection: h.collection.clone(), last_catch: h.last_catch.clone(), batch: h.batch.clone(), earned: h.earned, daily_cap: self.catalog.balance.shell_hunt.daily_cap, session: self.hunt_session.clone(), waiting: h.batch.is_none(), last_sale: h.last_sale.clone() }
    }

    pub fn start_hunt(&mut self, today: &str) -> Result<super::hunt::HuntView, Failure> {
        if let Some(e) = &self.read_only { return Err(e.clone()); }
        if self.state.settings.meeting_mode { return Err(Failure::new("hunt_meeting", "meeting mode")); }
        let mut next = self.state.clone();
        next.shell_hunt.roll_day(today);
        let room = self.catalog.balance.shell_hunt.daily_cap.saturating_sub(next.shell_hunt.earned);
        let fish = self.catchable_fish();
        // Adult fish can always be caught; shells need a batch and room under the daily limit.
        if room == 0 && fish.is_empty() { return Err(Failure::new("hunt_cap", "daily shell hunt limit")); }
        if room > 0 && !next.shell_hunt.tutorial_granted {
            next.shell_hunt.tutorial_granted = true;
            if next.shell_hunt.batch.is_none() { next.shell_hunt.batch = Some(super::hunt::Batch::generate(room.min(3) as usize)); }
        }
        if next.shell_hunt.batch.is_none() && fish.is_empty() { return Err(Failure::new("hunt_waiting", "no shells yet")); }
        self.commit(next)?;
        if self.hunt_session.is_none() {
            let batch_id = self.state.shell_hunt.batch.as_ref().map(|b| b.id.clone()).unwrap_or_default();
            self.hunt_session = Some(super::hunt::HuntSession::start(batch_id, fish));
        }
        Ok(self.hunt_view())
    }

    /// Every adult (Lv.100) fish in the tank, ready to be caught from the boat.
    fn catchable_fish(&self) -> Vec<super::hunt::FishTarget> {
        self.state.fish.iter().filter(|f| f.exp >= MAX_EXP).enumerate()
            .map(|(lane, f)| super::hunt::FishTarget::new(&f.id, &f.species_id, self.slots_for(&f.species_id), lane))
            .collect()
    }

    /// "Yay, dinner's sorted": sells the fish waiting on deck at its normal sale value.
    fn sell_caught_fish(&mut self, session_id: &str, seq: u64) -> Result<(), Failure> {
        let s = self.hunt_session.as_ref().ok_or_else(|| Failure::new("hunt_stale", "no session"))?;
        if s.id != session_id { return Err(Failure::new("hunt_stale", "session changed")); }
        if seq <= s.last_command { return Ok(()); }
        if s.phase != "deciding" { return Err(Failure::new("hunt_busy", "nothing on deck")); }
        let fish_id = s.caught_fish.clone().ok_or_else(|| Failure::new("hunt_busy", "nothing on deck"))?;
        // Coins, the fish leaving the tank and the sale notice are one atomic save.
        let mut next = self.state.clone();
        let i = next.fish.iter().position(|f| f.id == fish_id).ok_or_else(|| Failure::new("fish_not_found", &fish_id))?;
        let coins = next.fish[i].sale_value().ok_or_else(|| Failure::new("wallet_overflow", "sale"))?;
        next.wallet.cbcoins = next.wallet.cbcoins.checked_add(coins).ok_or_else(|| Failure::new("wallet_overflow", "sale"))?;
        let sold = next.fish.remove(i);
        next.shell_hunt.last_sale = Some(super::hunt::FishSale { fish_id: fish_id.clone(), species_id: sold.species_id, coins });
        self.commit(next)?;
        if let Some(s) = &mut self.hunt_session {
            s.fish.retain(|f| f.id != fish_id);
            s.caught_fish = None;
            s.phase = "swinging".into();
            s.last_command = seq;
        }
        Ok(())
    }

    pub fn hunt_action(&mut self, session_id: &str, seq: u64, action: &str) -> Result<super::hunt::HuntView, Failure> {
        if action == "sell" {
            if self.state.settings.meeting_mode { return Err(Failure::new("hunt_meeting", "meeting mode")); }
            self.sell_caught_fish(session_id, seq)?;
            return Ok(self.hunt_view());
        }
        if action == "leave" {
            if self.hunt_session.as_ref().is_some_and(|s| s.id == session_id) { self.hunt_session = None; }
            return Ok(self.hunt_view());
        }
        if self.state.settings.meeting_mode && action != "pause" { return Err(Failure::new("hunt_meeting", "meeting mode")); }
        self.hunt_session.as_mut().ok_or_else(|| Failure::new("hunt_stale", "no session"))?.action(session_id, seq, action)?;
        Ok(self.hunt_view())
    }

    /// Called by a native monotonic timer, never by a client-supplied elapsed time.
    /// Returns true when a durable change should refresh the other windows.
    pub fn tick_hunt(&mut self, elapsed_ms: u64, today: &str) -> Result<bool, Failure> {
        if self.read_only.is_some() || self.state.settings.meeting_mode { return Ok(false); }
        let cfg = self.catalog.balance.shell_hunt.clone();
        let mut next = self.state.clone();
        next.shell_hunt.roll_day(today);
        let mut changed = next.shell_hunt.date != self.state.shell_hunt.date;
        let room = cfg.daily_cap.saturating_sub(next.shell_hunt.earned);
        if next.shell_hunt.batch.is_none() && room > 0 {
            next.shell_hunt.remaining_wait_ms = next.shell_hunt.remaining_wait_ms.saturating_sub(elapsed_ms);
            if next.shell_hunt.remaining_wait_ms == 0 {
                let random_count = loop {
                    let byte = uuid::Uuid::new_v4().as_bytes()[0];
                    if byte < 250 { break byte as usize % 10 + 1; }
                };
                next.shell_hunt.batch = Some(super::hunt::Batch::generate(random_count.min(room as usize)));
                changed = true;
            }
        }
        if let Some(s) = &mut self.hunt_session {
            // A fish-only trip picks up a batch that turns up while the boat is out.
            if s.batch_id.is_empty() { if let Some(b) = &next.shell_hunt.batch { s.batch_id = b.id.clone(); } }
            // Fish can be caught any time; shells only from the session's batch, under the daily limit.
            let batch = next.shell_hunt.batch.as_ref().filter(|b| s.batch_id == b.id && room > 0);
            s.step_with(elapsed_ms as f64 / 1000.0, batch.map(|b| b.shells.as_slice()).unwrap_or(&[]));
            if let Some(b) = batch {
                {
                    if s.phase == "settling" {
                        if let Some(shell) = b.shells.iter().find(|sh| Some(&sh.id) == s.caught_id.as_ref() && !sh.collected) {
                            let id = shell.id.clone();
                            let kind = shell.kind; let rare = shell.rare; let pearl = shell.pearl;
                            let first_of_kind = !next.shell_hunt.collection.contains_key(&kind);
                            let entry = next.shell_hunt.collection.entry(kind).or_insert_with(|| super::hunt::CollectionEntry { count: 0, rare_count: 0, first_found: today.into() });
                            entry.count = entry.count.checked_add(1).ok_or_else(|| Failure::new("wallet_overflow", "collection"))?;
                            if rare { entry.rare_count = entry.rare_count.checked_add(1).ok_or_else(|| Failure::new("wallet_overflow", "rare collection"))?; }
                            if pearl { next.wallet.pearls = next.wallet.pearls.checked_add(1).ok_or_else(|| Failure::new("wallet_overflow", "pearls"))?; }
                            next.shell_hunt.last_catch = Some(super::hunt::CatchReceipt { shell_id: id.clone(), kind, rare, pearl, first_of_kind });
                            next.shell_hunt.batch.as_mut().unwrap().shells.iter_mut().find(|sh| sh.id == id).unwrap().collected = true;
                            next.shell_hunt.earned += 1;
                            next.wallet.cbcoins = next.wallet.cbcoins.checked_add(shell_value(kind)).ok_or_else(|| Failure::new("wallet_overflow", ""))?;
                            changed = true;
                        }
                    }
                }
            }
        }
        let finished = next.shell_hunt.batch.as_ref().is_some_and(|b| b.remaining() == 0);
        if finished { next.shell_hunt.batch = None; next.shell_hunt.remaining_wait_ms = cfg.next_wait(); changed = true; }
        // The trip ends when nothing is left to catch: no shells and no fish (none on deck either).
        let fish_left = self.hunt_session.as_ref().is_some_and(|s| !s.fish.is_empty());
        let shells_left = next.shell_hunt.batch.as_ref().is_some_and(|b| self.hunt_session.as_ref().is_some_and(|s| s.batch_id == b.id) && room > 0);
        let trip_over = self.hunt_session.is_some() && !fish_left && !shells_left;
        self.hunt_checkpoint_ms += elapsed_ms;
        if changed || self.hunt_checkpoint_ms >= 60_000 {
            if let Err(e) = self.commit(next) {
                if let Some(s) = &mut self.hunt_session { s.paused = true; s.error = Some(e.code.clone()); }
                return Err(e);
            }
            self.hunt_checkpoint_ms = 0;
            if trip_over { self.hunt_session = None; }
            else if let Some(s) = &mut self.hunt_session {
                if finished { s.batch_id.clear(); }
                if s.phase == "settling" && changed { s.phase = "swinging".into(); s.caught_id = None; s.error = None; }
            }
        } else {
            // Countdown is in memory until checkpoint; wallet/collected change only via commit.
            self.state.shell_hunt.remaining_wait_ms = next.shell_hunt.remaining_wait_ms;
        }
        if trip_over && self.hunt_session.is_some() { self.hunt_session = None; changed = true; }
        Ok(changed)
    }

    pub fn checkpoint_hunt(&mut self) -> Result<(), Failure> { self.commit(self.state.clone()) }
}

pub fn shell_value(kind: super::hunt::ShellKind) -> u64 {
    match kind { super::hunt::ShellKind::Great => 1, super::hunt::ShellKind::Queen => 10, super::hunt::ShellKind::Variegated => 100 }
}

/// Decimal MB, strict upper bounds: exactly 20 MB belongs to the 40 EXP tier.
pub fn file_exp(bytes: u64) -> u64 { (bytes / 20_000_000 + 1).saturating_mul(20) }

fn digest_fish(f: &mut Fish, now: i64) {
    if f.resting_until > now || f.pending_exp == 0 || f.exp >= MAX_EXP { return; }
    let boundary = ((f.exp / REST_EVERY_EXP + 1) * REST_EVERY_EXP).min(MAX_EXP);
    let amount = f.pending_exp.min(boundary - f.exp);
    f.exp += amount; f.pending_exp -= amount;
    f.stage = stage_for(f.exp);
    if f.exp == boundary && f.exp < MAX_EXP { f.resting_until = now.saturating_add(REST_SECONDS); }
    if f.exp == MAX_EXP { f.pending_exp = 0; f.resting_until = 0; }
}

fn random01() -> f64 {
    let bytes = *uuid::Uuid::new_v4().as_bytes();
    u64::from_le_bytes(bytes[..8].try_into().unwrap()) as f64 / (u64::MAX as f64 + 1.0)
}

fn stage_for(exp: u64) -> Stage {
    if exp >= MAX_EXP { Stage::Adult } else if exp >= JUVENILE_EXP { Stage::Juvenile } else { Stage::Fry }
}

/// Test codes. Only the SHA-256 of the normalised code (trimmed, upper case) is stored here.
const ADMIN_COINS_SHA256: &str = "68feb83c1194a8bbc6ea7bcd6597eac49492592b01237d1ae80e5b356e45ac47";
const ADMIN_FULL_EXP_SHA256: &str = "d5a02bd32cd4af2d9e823bc9b683f4b4cbd2b8bbc972a3831cefc3e452bbf4d5";
pub const ADMIN_COINS: u64 = 1_000_000_000;

fn code_hash(code: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(code.trim().to_uppercase().as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn mark_owned(state: &mut GameState, species_id: &str) {
    let entry = state.dex.entry(species_id.to_string()).or_default();
    entry.seen = true;
    entry.owned = true;
}

/// Starts a new day only when the local date moves forward. Turning the
/// clock back never re-opens the daily caps.
fn roll_day(daily: &mut Daily, today: &str) {
    if daily.date.is_empty() || today > daily.date.as_str() {
        *daily = Daily { date: today.to_string(), ..Daily::default() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::belly::EntryState;
    use crate::engine::guard::Category;
    use std::fs;

    const NOW: i64 = 1_790_000_000;
    const OLD: i64 = NOW - 90 * 86400;

    #[test]
    fn size_tiers_are_strict_decimal_mb() {
        for (bytes, exp) in [(0,20),(19_999_999,20),(20_000_000,40),(39_999_999,40),(40_000_000,60),(59_999_999,60),(60_000_000,80),(4_000_000_000,4020)] {
            assert_eq!(file_exp(bytes), exp);
        }
    }

    #[test]
    fn v3_save_migrates_currency_stage_price_and_keeps_ledger() {
        let dir=tempfile::tempdir().unwrap();let s=store(dir.path());let mut v=serde_json::to_value(&s.state).unwrap();
        v["schema_version"]=serde_json::json!(3);v["wallet"].as_object_mut().unwrap().remove("cbcoins");v["wallet"]["shells"]=serde_json::json!(123);
        v["fish"][0]["stage"]=serde_json::json!("adult");v["fish"][0]["exp"]=serde_json::json!(60);
        for key in ["purchase_price","resting_until","pending_exp"] {v["fish"][0].as_object_mut().unwrap().remove(key);}
        v["ledger"]["fingerprints"]=serde_json::json!(["p1m:legacy"]);
        fs::write(&s.path,serde_json::to_vec(&v).unwrap()).unwrap();drop(s);
        let mut s=store(dir.path());assert!(s.read_only.is_none());assert_eq!(s.state.wallet.cbcoins,123);
        assert_eq!(s.state.fish[0].exp,MAX_EXP);assert_eq!(s.state.fish[0].stage,Stage::Adult);
        assert!(s.state.ledger.fingerprints.contains(&"p1m:legacy".into()));
        let id=s.state.fish[0].id.clone();let price=s.state.fish[0].purchase_price;s.sell(&id).unwrap();
        assert_eq!(store(dir.path()).state.wallet.cbcoins,123+price*100);
    }

    #[test]
    fn duplicate_history_is_not_pruned_and_sale_overflow_is_atomic() {
        let dir=tempfile::tempdir().unwrap();let mut s=store(dir.path());s.catalog.balance.economy.max_fingerprints=1;
        let id=s.state.fish[0].id.clone();s.apply_receipts(&[entry("a","A",&id,true),entry("b","B",&id,true)],"2026-10-05",NOW).unwrap();
        assert_eq!(s.apply_receipts(&[entry("c","A",&id,true)],"2026-10-05",NOW).unwrap().duplicates,1);
        s.state.fish[0].exp=MAX_EXP;s.state.fish[0].stage=Stage::Adult;s.state.wallet.cbcoins=u64::MAX;
        assert_eq!(s.sell(&id).unwrap_err().code,"wallet_overflow");assert_eq!(s.state.fish.len(),1);assert_eq!(s.state.wallet.cbcoins,u64::MAX);
    }

    #[test]
    fn growth_rest_bank_restart_adult_and_sale() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let id = s.state.fish[0].id.clone();
        let mut e = entry("big", "big-hash", &id, true); e.size = 59_999_999;
        assert_eq!(s.apply_receipts(&[e], "2026-10-05", NOW).unwrap().exp, 60);
        assert_eq!((s.state.fish[0].exp, s.state.fish[0].pending_exp), (50,10)); // Lv.5: rest, 10 EXP banked
        assert_eq!(s.can_feed(&id, NOW+7199).unwrap_err().code, "fish_full");
        drop(s); let mut s = store(dir.path());
        s.digest(NOW+7199).unwrap(); assert_eq!(s.state.fish[0].exp,50);
        s.digest(NOW+7200).unwrap(); assert_eq!((s.state.fish[0].exp,s.state.fish[0].pending_exp),(60,0));
        assert!(s.can_feed(&id,NOW+7200).is_ok());
        assert_eq!(s.state.fish[0].sale_value(), Some(s.state.fish[0].purchase_price / 2)); // young: half price
        s.state.fish[0].exp = 490; s.state.fish[0].pending_exp = 20; s.state.fish[0].resting_until = 0;
        s.digest(NOW+7200).unwrap(); assert_eq!(s.state.fish[0].stage,Stage::Juvenile); assert_eq!(s.state.fish[0].exp,500);
        s.state.fish[0].exp = 990; s.state.fish[0].pending_exp = 10; s.state.fish[0].resting_until = 0;
        s.digest(NOW+14400).unwrap(); assert_eq!(s.state.fish[0].stage,Stage::Adult);
        assert_eq!(s.can_feed(&id,NOW+14400).unwrap_err().code,"fish_adult");
        let paid = s.state.fish[0].purchase_price;
        let good = s.path.clone(); s.path = dir.path().join("missing/game.json");
        assert!(s.sell(&id).is_err()); assert_eq!(s.state.fish.len(),1); assert_eq!(s.state.wallet.cbcoins,300);
        s.path = good; assert_eq!(s.sell(&id).unwrap(),paid*100); assert_eq!(s.state.wallet.cbcoins,300+paid*100);
        assert_eq!(s.sell(&id).unwrap_err().code,"fish_not_found");
        let loaded = store(dir.path()); assert!(loaded.state.fish.is_empty()); assert_eq!(loaded.state.wallet.cbcoins,300+paid*100);
    }

    #[test]
    fn all_shell_colors_credit_coin_values_once() {
        use super::super::hunt::ShellKind;
        for (kind,value) in [(ShellKind::Great,1),(ShellKind::Queen,10),(ShellKind::Variegated,100)] {
            let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path()); s.start_hunt("2026-10-05").unwrap();
            let shell = &mut s.state.shell_hunt.batch.as_mut().unwrap().shells[0]; shell.kind=kind; shell.pearl=false; let id=shell.id.clone();
            let session=s.hunt_session.as_mut().unwrap(); session.phase="settling".into();session.caught_id=Some(id.clone());
            s.tick_hunt(100,"2026-10-05").unwrap(); assert_eq!(s.state.wallet.cbcoins,300+value);
            let session=s.hunt_session.as_mut().unwrap();session.phase="settling".into();session.caught_id=Some(id);
            s.tick_hunt(100,"2026-10-05").unwrap();assert_eq!(s.state.wallet.cbcoins,300+value);
        }
    }

    fn adult_pair(s: &mut GameStore) -> (String, String) {
        // The starter guppy plus one more adult guppy.
        let sp = s.catalog.species("poecilia_reticulata").cloned().unwrap();
        let second = s.new_fish(&sp, Origin::Shop, NOW);
        let mut next = s.state.clone();
        next.fish.push(second);
        for f in &mut next.fish { f.exp = MAX_EXP; f.stage = Stage::Adult; }
        s.commit(next).unwrap();
        (s.state.fish[0].id.clone(), s.state.fish[1].id.clone())
    }

    #[test]
    fn hatch_rate_falls_from_twenty_to_five_percent_with_price() {
        let b = Catalog::bundled().unwrap().balance.breeding;
        assert!((b.hatch_rate(20) - 0.20).abs() < 1e-9);
        assert!((b.hatch_rate(300) - 0.05).abs() < 1e-9);
        assert!((b.hatch_rate(160) - 0.125).abs() < 1e-9);
        assert!(b.hatch_rate(100) < b.hatch_rate(50));
        let c = Catalog::bundled().unwrap();
        for sp in &c.species { let r = b.hatch_rate(sp.price); assert!((0.05..=0.20).contains(&r), "{}", sp.id); }
    }

    #[test]
    fn breeding_lays_eggs_in_the_den_and_charges_both_parents() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        let mut rolls = [0.0, 1.0, 0.5, 0.25, 0.75].into_iter();
        let out = s.breed_with(&a, &b, 5, NOW, &mut || rolls.next().unwrap()).unwrap();
        assert_eq!(out.laid, 5);
        assert!((out.hatch_rate - 0.20).abs() < 1e-9);
        assert_eq!(s.state.fish.len(), 2, "nothing hatches straight away");
        let times: Vec<i64> = s.state.eggs.iter().map(|e| e.hatch_at - NOW).collect();
        assert_eq!(times, vec![7200, 10800, 9000, 8100, 9900]); // 2–3 hours
        for e in &s.state.eggs { assert_eq!((e.generation, e.parent_ids.clone(), e.laid_at), (1, vec![a.clone(), b.clone()], NOW)); }
        for id in [&a, &b] {
            let f = s.state.fish.iter().find(|f| &f.id == id).unwrap();
            assert_eq!(f.eggs_used, 5);
            assert_eq!(f.sale_value(), Some(20 * 95)); // 2000 - 5 x 20
        }
        let again = store(dir.path()); // persisted atomically
        assert_eq!((again.state.eggs.len(), again.state.fish[0].eggs_used), (5, 5));
    }

    #[test]
    fn eggs_hatch_only_when_their_time_is_up_with_the_species_chance() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        s.breed_with(&a, &b, 5, NOW, &mut || 0.0).unwrap(); // all due at NOW + 2h
        assert!(!s.hatch_due_with(NOW + 7199, &mut || 0.0).unwrap());
        assert_eq!(s.state.eggs.len(), 5);
        let mut rolls = [0.01, 0.99, 0.19, 0.20, 0.5].into_iter();
        assert!(s.hatch_due_with(NOW + 7200, &mut || rolls.next().unwrap()).unwrap());
        assert!(s.state.eggs.is_empty());
        let kids: Vec<&Fish> = s.state.fish.iter().filter(|f| f.origin == Origin::Hatched).collect();
        assert_eq!(kids.len(), 2); // 0.01 and 0.19 < 20 %
        for kid in kids {
            assert_eq!((kid.stage, kid.exp, kid.generation, kid.purchase_price), (Stage::Fry, 0, 1, 20));
            assert_eq!(kid.parent_ids, vec![a.clone(), b.clone()]);
        }
        assert_eq!(s.state.hatch_log.len(), 5);
        assert_eq!(s.state.hatch_log.iter().filter(|r| r.hatched).count(), 2);
        assert_eq!(store(dir.path()).state.fish.len(), 4);
    }

    #[test]
    fn sale_value_never_falls_below_the_purchase_price() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        assert_eq!(s.breed_with(&a, &b, 100, NOW, &mut || 0.99).unwrap_err().code, "breed_eggs_invalid");
        assert_eq!(s.breed_with(&a, &b, 0, NOW, &mut || 0.99).unwrap_err().code, "breed_eggs_invalid");
        assert_eq!(s.breed_with(&a, &b, 99, NOW, &mut || 0.99).unwrap().laid, 99);
        assert_eq!(s.state.fish[0].sale_value(), Some(20)); // back to the purchase price
        assert_eq!(s.breed_with(&a, &b, 1, NOW, &mut || 0.0).unwrap_err().code, "breed_exhausted");
        assert_eq!(s.sell(&a).unwrap(), 20);
    }

    #[test]
    fn eggs_accumulate_and_the_limit_is_the_more_used_parent() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        s.breed_with(&a, &b, 40, NOW, &mut || 0.99).unwrap();
        s.state.fish[1].eggs_used = 90; // b is nearly exhausted
        assert_eq!(s.breed_with(&a, &b, 10, NOW, &mut || 0.99).unwrap_err().code, "breed_eggs_invalid");
        s.breed_with(&a, &b, 9, NOW, &mut || 0.99).unwrap();
        assert_eq!((s.state.fish[0].eggs_used, s.state.fish[1].eggs_used, s.state.eggs.len()), (49, 99, 49));
    }

    #[test]
    fn breeding_rejects_bad_pairs_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        assert_eq!(s.breed_with(&a, &a, 1, NOW, &mut || 0.0).unwrap_err().code, "breed_same_fish");
        assert_eq!(s.breed_with(&a, "nope", 1, NOW, &mut || 0.0).unwrap_err().code, "fish_not_found");
        s.state.fish[1].exp = MAX_EXP - 1;
        assert_eq!(s.breed_with(&a, &b, 1, NOW, &mut || 0.0).unwrap_err().code, "fish_not_adult");
        s.state.fish[1].exp = MAX_EXP; s.state.fish[1].species_id = "danio_rerio".into();
        assert_eq!(s.breed_with(&a, &b, 1, NOW, &mut || 0.0).unwrap_err().code, "breed_species_mismatch");
        s.state.fish[1].species_id = s.state.fish[0].species_id.clone();
        let good = s.path.clone(); s.path = dir.path().join("missing/game.json");
        assert!(s.breed_with(&a, &b, 3, NOW, &mut || 0.0).is_err());
        assert_eq!((s.state.fish.len(), s.state.fish[0].eggs_used, s.state.eggs.len()), (2, 0, 0));
        s.path = good;
    }

    #[test]
    fn the_den_has_a_capacity() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        s.catalog.balance.breeding.den_capacity = 5;
        assert_eq!(s.breed_with(&a, &b, 6, NOW, &mut || 0.0).unwrap_err().code, "breed_eggs_invalid");
        s.breed_with(&a, &b, 5, NOW, &mut || 0.0).unwrap();
        assert_eq!(s.breed_with(&a, &b, 1, NOW, &mut || 0.0).unwrap_err().code, "den_full");
    }

    #[test]
    fn full_tank_keeps_due_eggs_waiting_in_the_den() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        let cap = s.catalog.balance.tank_capacity;
        let sp = s.catalog.species("danio_rerio").cloned().unwrap();
        let mut next = s.state.clone();
        while next.fish.len() < cap - 1 { next.fish.push(s.new_fish(&sp, Origin::Shop, NOW)); }
        s.commit(next).unwrap(); // one free slot
        s.breed_with(&a, &b, 3, NOW, &mut || 0.0).unwrap(); // breeding itself needs no tank room
        assert!(s.hatch_due_with(NOW + 7200, &mut || 0.0).unwrap());
        assert_eq!((s.state.fish.len(), s.state.eggs.len()), (cap, 2)); // two eggs wait, unrolled
        assert!(!s.hatch_due_with(NOW + 9000, &mut || 0.0).unwrap());
        let mut next = s.state.clone(); next.fish.pop(); s.commit(next).unwrap(); // a slot frees up
        assert!(s.hatch_due_with(NOW + 9000, &mut || 0.0).unwrap());
        assert_eq!((s.state.fish.len(), s.state.eggs.len()), (cap, 1));
    }

    #[test]
    fn real_randomness_stays_within_expected_hatch_band() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let (a, b) = adult_pair(&mut s);
        s.catalog.balance.tank_capacity = 1000;
        let (mut hatched, mut laid) = (0usize, 0u32);
        for _ in 0..3 {
            for f in &mut s.state.fish { f.eggs_used = 0; }
            laid += s.breed(&a, &b, 99, NOW).unwrap().laid;
            let before = s.state.fish.len();
            s.hatch_due(NOW + 10800).unwrap();
            assert!(s.state.eggs.is_empty());
            hatched += s.state.fish.len() - before;
        }
        let rate = hatched as f64 / laid as f64;
        assert!((0.08..=0.33).contains(&rate), "rate {rate}");
    }

    #[test]
    fn legacy_saves_load_with_zero_eggs_used() {
        let dir = tempfile::tempdir().unwrap(); let s = store(dir.path());
        let mut v = serde_json::to_value(&s.state).unwrap();
        v["fish"][0].as_object_mut().unwrap().remove("eggs_used");
        fs::write(&s.path, serde_json::to_vec(&v).unwrap()).unwrap(); drop(s);
        let s = store(dir.path()); assert!(s.read_only.is_none()); assert_eq!(s.state.fish[0].eggs_used, 0);
    }

    #[test]
    fn v4_save_keeps_levels_with_ten_exp_per_level() {
        let dir = tempfile::tempdir().unwrap(); let s = store(dir.path());
        let mut v = serde_json::to_value(&s.state).unwrap();
        v["schema_version"] = serde_json::json!(4);
        v["fish"][0]["exp"] = serde_json::json!(5020); v["fish"][0]["pending_exp"] = serde_json::json!(130); v["fish"][0]["stage"] = serde_json::json!("juvenile");
        fs::write(&s.path, serde_json::to_vec(&v).unwrap()).unwrap(); drop(s);
        let s = store(dir.path()); assert!(s.read_only.is_none());
        assert_eq!((s.state.fish[0].exp, s.state.fish[0].pending_exp), (502, 13)); // still Lv.50
        assert_eq!(s.state.schema_version, SCHEMA_VERSION);
        let c = Catalog::bundled().unwrap();
        assert_eq!((c.balance.stage_exp.juvenile, c.balance.stage_exp.adult), (JUVENILE_EXP, MAX_EXP));
    }

    #[test]
    fn admin_codes_grant_coins_and_full_exp_and_others_fail() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let start = s.state.wallet.cbcoins;
        assert_eq!(s.redeem(" tq-admin-1ty-cb-2610 ").unwrap().cbcoins, ADMIN_COINS);
        assert_eq!(s.redeem("TQ-ADMIN-1TY-CB-2610").unwrap().kind, "cbcoins"); // reusable test code
        assert_eq!(s.state.wallet.cbcoins, start + 2 * ADMIN_COINS);
        assert_eq!(s.redeem("TQ-ADMIN-FULLEXP-2610").unwrap().fish, 1);
        assert_eq!((s.state.fish[0].exp, s.state.fish[0].stage), (MAX_EXP, Stage::Adult));
        assert_eq!(s.redeem("nope").unwrap_err().code, "code_invalid");
        assert_eq!(s.redeem("   ").unwrap_err().code, "code_invalid");
        let again = store(dir.path()); assert_eq!(again.state.wallet.cbcoins, start + 2 * ADMIN_COINS);
        s.state.wallet.cbcoins = u64::MAX - 1;
        assert_eq!(s.redeem("TQ-ADMIN-1TY-CB-2610").unwrap_err().code, "wallet_overflow");
    }

    #[test]
    fn big_fish_take_more_slots() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        assert_eq!((s.slots_for("poecilia_reticulata"), s.slots_for("thunnus_alalunga"), s.slots_for("rhincodon_typus")), (1, 2, 3));
        s.state.wallet.cbcoins = 100_000;
        // 1 starter slot + 6 whale sharks × 3 = 19 slots: a 7th whale shark (3) no longer fits, a guppy (1) does.
        for _ in 0..6 { s.purchase("rhincodon_typus", 300, NOW).unwrap(); }
        assert_eq!(s.used_slots(), 19);
        assert_eq!(s.purchase("rhincodon_typus", 300, NOW).unwrap_err().code, "tank_full");
        s.purchase("poecilia_reticulata", 20, NOW).unwrap();
        assert_eq!(s.used_slots(), 20);
    }

    #[test]
    fn hatching_big_fish_needs_room_for_their_slots() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let sp = s.catalog.species("rhincodon_typus").cloned().unwrap();
        let mut next = s.state.clone();
        next.fish.clear();
        for _ in 0..2 { let mut f = s.new_fish(&sp, Origin::Shop, NOW); f.exp = MAX_EXP; f.stage = Stage::Adult; next.fish.push(f); }
        s.commit(next).unwrap(); // 6 of 20 slots used: room for 4 whale shark pups
        let (a, b) = (s.state.fish[0].id.clone(), s.state.fish[1].id.clone());
        assert_eq!(s.breed_with(&a, &b, 10, NOW, &mut || 1.0).unwrap().laid, 10);
        s.hatch_due_with(NOW + 10800, &mut || 0.0).unwrap();
        assert_eq!((s.used_slots(), s.state.eggs.len()), (18, 6)); // 2 free < 3: the rest wait
    }

    #[test]
    fn young_fish_sell_back_for_half_the_purchase_price() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        s.state.wallet.cbcoins = 1000;
        let tuna = s.purchase("thunnus_albacares", 120, NOW).unwrap();
        assert_eq!(s.state.wallet.cbcoins, 880);
        assert_eq!(s.sell(&tuna.id).unwrap(), 60);
        assert_eq!(s.state.wallet.cbcoins, 940);
        assert!(s.state.fish.iter().all(|f| f.id != tuna.id));
        let starter = s.state.fish[0].id.clone();
        s.state.fish[0].exp = MAX_EXP - 1;
        assert_eq!(s.sell(&starter).unwrap(), 10); // a Lv.99 guppy is still "young"
    }

    fn adult_starter(s: &mut GameStore) -> String {
        let mut next = s.state.clone();
        next.fish[0].exp = MAX_EXP; next.fish[0].stage = Stage::Adult;
        s.commit(next).unwrap();
        s.state.fish[0].id.clone()
    }

    #[test]
    fn adult_fish_can_be_caught_without_shells_and_sold_from_the_boat() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let id = adult_starter(&mut s);
        s.state.shell_hunt.tutorial_granted = true; s.state.shell_hunt.batch = None;
        s.state.shell_hunt.earned = s.catalog.balance.shell_hunt.daily_cap; // shell limit reached
        let view = s.start_hunt("2026-10-06").unwrap();
        let session = view.session.unwrap();
        assert_eq!((session.fish.len(), session.fish[0].id.clone(), session.batch_id.as_str()), (1, id.clone(), ""));
        // Put the fish on deck as if the claw had brought it up.
        { let s2 = s.hunt_session.as_mut().unwrap(); s2.phase = "deciding".into(); s2.caught_fish = Some(id.clone()); }
        let before = s.state.wallet.cbcoins;
        let sid = s.hunt_session.as_ref().unwrap().id.clone();
        let view = s.hunt_action(&sid, 5, "sell").unwrap();
        assert_eq!(s.state.wallet.cbcoins, before + 2000); // guppy: 20 x 100
        assert!(s.state.fish.is_empty());
        assert_eq!(view.last_sale.unwrap().coins, 2000);
        assert_eq!(s.hunt_action(&sid, 5, "sell").unwrap().last_sale.unwrap().fish_id, id, "a replayed sell is ignored");
        assert_eq!(store(dir.path()).state.shell_hunt.last_sale.unwrap().species_id, "poecilia_reticulata");
        // Nothing left to catch: the trip ends on the next tick.
        s.tick_hunt(100, "2026-10-06").unwrap();
        assert!(s.hunt_session.is_none());
    }

    #[test]
    fn keeping_a_caught_fish_lets_it_go_and_sells_nothing() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let id = adult_starter(&mut s);
        s.start_hunt("2026-10-06").unwrap();
        let sid = s.hunt_session.as_ref().unwrap().id.clone();
        { let s2 = s.hunt_session.as_mut().unwrap(); s2.phase = "deciding".into(); s2.caught_fish = Some(id.clone()); }
        assert_eq!(s.hunt_action(&sid, 1, "drop").unwrap_err().code, "hunt_busy");
        let before = s.state.wallet.cbcoins;
        s.hunt_action(&sid, 2, "keep").unwrap();
        assert_eq!((s.state.wallet.cbcoins, s.state.fish.len()), (before, 1));
        let session = s.hunt_session.as_ref().unwrap();
        assert_eq!((session.phase.as_str(), session.fish.len()), ("swinging", 0));
        assert_eq!(s.hunt_action(&sid, 3, "sell").unwrap_err().code, "hunt_busy");
    }

    #[test]
    fn young_fish_are_not_offered_to_the_claw() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        s.start_hunt("2026-10-06").unwrap(); // tutorial shells, starter guppy is Lv.0
        assert!(s.hunt_session.as_ref().unwrap().fish.is_empty());
    }

    fn store(dir: &Path) -> GameStore {
        GameStore::open(&dir.join("game.json"), Catalog::bundled().unwrap(), NOW)
    }

    fn entry(receipt: &str, fp: &str, fish_id: &str, rewardable: bool) -> BellyEntry {
        BellyEntry {
            id: format!("e-{receipt}"),
            receipt_id: receipt.into(),
            original_path: "/x/a.txt".into(),
            stored_path: "/belly/a.txt".into(),
            name: "a.txt".into(),
            size: 200_000,
            fingerprint: fp.into(),
            category: Category::Doc,
            modified_unix: OLD,
            eaten_at: NOW,
            fish_id: fish_id.into(),
            rewardable,
            state: EntryState::Held,
            restored_path: None,
            restored_at: None,
        }
    }

    #[test]
    fn new_profile_gets_starter_and_welcome_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        assert_eq!(s.state.wallet.cbcoins, 300);
        assert_eq!(s.state.fish.len(), 1);
        assert_eq!(s.state.fish[0].origin, Origin::Starter);
        assert!(s.state.fish[0].parent_ids.is_empty());
        s.purchase("danio_rerio", 20, NOW).unwrap();
        let s = store(dir.path());
        assert_eq!(s.state.wallet.cbcoins, 280, "welcome credit must not be granted again");
        assert_eq!(s.state.fish.len(), 2);
    }

    #[test]
    fn purchase_rules() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        assert_eq!(s.purchase("nope", 1, NOW).unwrap_err().code, "unknown_species");
        assert_eq!(s.purchase("danio_rerio", 19, NOW).unwrap_err().code, "price_changed");
        s.state.wallet.cbcoins = 10; assert_eq!(s.purchase("danio_rerio", 20, NOW).unwrap_err().code, "not_enough_shells"); s.state.wallet.cbcoins = 300;
        let f = s.purchase("danio_rerio", 20, NOW).unwrap();
        assert_eq!(f.origin, Origin::Shop);
        assert_eq!(f.name, "Cá ngựa vằn");
        assert_eq!(f.stage, Stage::Fry);
        assert_eq!(s.state.wallet.cbcoins, 280);
        assert!(s.state.dex["danio_rerio"].owned);
    }

    #[test]
    fn full_tank_blocks_purchase_without_charging() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.state.wallet.cbcoins = 10_000;
        while s.used_slots() < s.catalog.balance.tank_capacity {
            s.purchase("danio_rerio", 20, NOW).unwrap();
        }
        assert_eq!(s.state.fish.len(), 20); // 20 small fish, 1 slot each
        let before = s.state.wallet.cbcoins;
        assert_eq!(s.purchase("danio_rerio", 20, NOW).unwrap_err().code, "tank_full");
        assert_eq!(s.state.wallet.cbcoins, before);
    }

    #[test]
    fn failed_save_neither_charges_nor_creates_fish() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.path = dir.path().join("missing-dir/game.json");
        assert_eq!(s.purchase("danio_rerio", 20, NOW).unwrap_err().code, "save_failed");
        assert_eq!(s.state.wallet.cbcoins, 300);
        assert_eq!(s.state.fish.len(), 1);
    }

    #[test]
    fn receipts_pay_exactly_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        let entries = vec![entry("r1", "fp1", &fish, true)];
        let first = s.apply_receipts(&entries, "2026-09-29", NOW).unwrap();
        assert_eq!(first.shells, 0);
        assert!(first.exp > 0);
        let replay = s.apply_receipts(&entries, "2026-09-29", NOW).unwrap();
        assert_eq!(replay, RewardSummary::default());
        assert_eq!(s.state.wallet.cbcoins, 300);
    }

    #[test]
    fn duplicate_fingerprint_and_recovered_entries_pay_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        s.apply_receipts(&[entry("r1", "fp1", &fish, true)], "2026-09-29", NOW).unwrap();
        let r = s
            .apply_receipts(&[entry("r2", "fp1", &fish, true), entry("r3", "fp3", &fish, false)], "2026-09-29", NOW)
            .unwrap();
        assert_eq!((r.shells, r.duplicates, r.not_rewardable), (0, 1, 1));
        assert!(s.state.ledger.receipts.contains("r2") && s.state.ledger.receipts.contains("r3"));
    }

    #[test]
    fn feeding_exp_has_no_old_daily_diminish_or_currency_reward() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        let many: Vec<_> = (0..30).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        let r = s.apply_receipts(&many, "2026-09-29", NOW).unwrap();
        assert_eq!(r.shells, 0);
        assert_eq!(r.exp, 600);
        assert!(!r.capped);
        let more: Vec<_> = (200..210).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        let back = s.apply_receipts(&more, "2026-09-28", NOW).unwrap();
        assert_eq!(back.shells, 0, "turning the clock back must not reopen the cap");
        let next: Vec<_> = (300..301).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        assert_eq!(s.apply_receipts(&next, "2026-09-30", NOW).unwrap().exp, 20);
    }

    #[test]
    fn failed_reward_save_keeps_receipts_pending() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        let entries = vec![entry("r1", "fp1", &fish, true)];
        let good = s.path.clone();
        s.path = dir.path().join("missing-dir/game.json");
        assert!(s.apply_receipts(&entries, "2026-09-29", NOW).is_err());
        assert_eq!(s.state.wallet.cbcoins, 300);
        s.path = good;
        assert_eq!(s.apply_receipts(&entries, "2026-09-29", NOW).unwrap().exp, 20);
    }

    #[test]
    fn feeding_grows_the_fish() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        let many: Vec<_> = (0..20).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        s.apply_receipts(&many, "2026-09-29", NOW).unwrap();
        assert_eq!(s.state.fish[0].stage, Stage::Fry);
        // 400 EXP earned: 50 digested up to the Lv.5 rest, the rest banked.
        assert_eq!((s.state.fish[0].exp, s.state.fish[0].pending_exp), (50, 350));
    }

    #[test]
    fn future_or_corrupt_save_is_read_only_and_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.json");
        fs::write(&path, r#"{"schema_version": 7}"#).unwrap();
        let mut s = store(dir.path());
        assert_eq!(s.read_only.as_ref().unwrap().code, "save_future");
        assert!(s.purchase("danio_rerio", 20, NOW).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), r#"{"schema_version": 7}"#);

        fs::write(&path, r#"{"schema_version": 1, "fish": "wrong type"}"#).unwrap();
        let s = store(dir.path());
        assert_eq!(s.read_only.as_ref().unwrap().code, "save_corrupt");
        assert!(s.state.fish.is_empty(), "no starter fish is invented over an unreadable save");
    }

    #[test]
    fn migration_preserves_wallet_fish_and_receipts_without_welcome_regrant() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.purchase("danio_rerio", 20, NOW).unwrap();
        s.state.ledger.receipts.insert("paid".into());
        let id = s.state.fish[0].id.clone();
        let mut old = serde_json::to_value(&s.state).unwrap();
        old["schema_version"] = serde_json::json!(1);
        old.as_object_mut().unwrap().remove("shell_hunt");
        old["settings"].as_object_mut().unwrap().remove("quick_dock_enabled");
        fs::write(&s.path, serde_json::to_vec(&old).unwrap()).unwrap();
        drop(s);
        let s = store(dir.path());
        assert!(s.read_only.is_none()); assert_eq!(s.state.schema_version, SCHEMA_VERSION);
        assert_eq!(s.state.wallet.cbcoins, 280); assert_eq!(s.state.fish.len(), 2);
        assert_eq!(s.state.fish[0].id, id); assert!(s.state.ledger.receipts.contains("paid"));
        assert!(s.state.settings.quick_dock_enabled);
    }

    #[test]
    fn hunt_reward_is_atomic_replay_safe_and_separate_from_feeding_cap() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let view = s.start_hunt("2026-10-03").unwrap();
        let b = view.batch.unwrap(); let id = b.shells[0].id.clone();
        s.state.shell_hunt.batch.as_mut().unwrap().shells[0].kind = super::super::hunt::ShellKind::Great;
        let session = s.hunt_session.as_mut().unwrap();
        session.phase = "settling".into(); session.caught_id = Some(id.clone());
        let good = s.path.clone(); s.path = dir.path().join("missing/save.json");
        assert!(s.tick_hunt(100, "2026-10-03").is_err());
        assert_eq!(s.state.wallet.cbcoins, 300); assert!(!s.state.shell_hunt.batch.as_ref().unwrap().shells[0].collected);
        s.path = good;
        assert!(s.tick_hunt(100, "2026-10-03").unwrap());
        assert_eq!(s.state.wallet.cbcoins, 301); assert_eq!(s.state.shell_hunt.earned, 1); assert_eq!(s.state.daily.shells, 0);
        let session = s.hunt_session.as_mut().unwrap(); session.phase = "settling".into(); session.caught_id = Some(id);
        s.tick_hunt(100, "2026-10-03").unwrap(); assert_eq!(s.state.wallet.cbcoins, 301);
        let loaded = store(dir.path()); assert_eq!(loaded.state.wallet.cbcoins, 301);
        assert_eq!(loaded.state.shell_hunt.batch.as_ref().unwrap().remaining(), 2);
        assert!(loaded.hunt_session.is_none());
    }

    #[test]
    fn random_batches_persist_and_tutorial_is_not_repeated() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        s.state.shell_hunt.remaining_wait_ms = 1;
        s.tick_hunt(100, "2026-10-03").unwrap();
        let batch = s.state.shell_hunt.batch.as_ref().unwrap().id.clone();
        assert!((1..=10).contains(&s.state.shell_hunt.batch.as_ref().unwrap().shells.len()));
        drop(s); let mut s = store(dir.path());
        assert_eq!(s.state.shell_hunt.batch.as_ref().unwrap().id, batch);
        s.start_hunt("2026-10-03").unwrap();
        assert_eq!(s.state.shell_hunt.batch.as_ref().unwrap().id, batch);
        s.state.shell_hunt.earned = 60;
        assert_eq!(s.start_hunt("2026-10-02").unwrap_err().code, "hunt_cap");
    }

    #[test]
    fn rare_pearl_collection_reward_is_atomic_and_replay_safe() {
        use super::super::hunt::ShellKind;
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        s.start_hunt("2026-10-05").unwrap();
        let shell = &mut s.state.shell_hunt.batch.as_mut().unwrap().shells[0];
        shell.kind = ShellKind::Queen; shell.rare = true; shell.pearl = true;
        let id = shell.id.clone();
        let session = s.hunt_session.as_mut().unwrap(); session.phase = "settling".into(); session.caught_id = Some(id.clone());
        let good = s.path.clone(); s.path = dir.path().join("missing/save.json");
        assert!(s.tick_hunt(100, "2026-10-05").is_err());
        assert_eq!(s.state.wallet.pearls, 0); assert!(s.state.shell_hunt.collection.is_empty()); assert!(s.state.shell_hunt.last_catch.is_none());
        s.path = good; s.tick_hunt(100, "2026-10-05").unwrap();
        assert_eq!(s.state.wallet.pearls, 1); assert_eq!(s.state.wallet.cbcoins, 310);
        let entry = &s.state.shell_hunt.collection[&ShellKind::Queen]; assert_eq!((entry.count, entry.rare_count), (1,1));
        assert!(s.state.shell_hunt.last_catch.as_ref().unwrap().first_of_kind);
        let session = s.hunt_session.as_mut().unwrap(); session.phase = "settling".into(); session.caught_id = Some(id);
        s.tick_hunt(100, "2026-10-05").unwrap();
        assert_eq!(s.state.wallet.pearls, 1); assert_eq!(s.state.shell_hunt.collection[&ShellKind::Queen].count, 1);
        drop(s); let mut loaded = store(dir.path());
        assert_eq!(loaded.state.wallet.pearls, 1); assert_eq!(loaded.state.shell_hunt.collection[&ShellKind::Queen].rare_count, 1);
        loaded.tick_hunt(100, "2026-10-06").unwrap();
        assert_eq!(loaded.state.shell_hunt.earned, 0); assert_eq!(loaded.state.shell_hunt.collection[&ShellKind::Queen].count, 1);
    }

    #[test]
    fn v2_migration_keeps_batch_and_does_not_reroll_rewards() {
        use super::super::hunt::ShellKind;
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        s.start_hunt("2026-10-05").unwrap();
        let batch_id = s.state.shell_hunt.batch.as_ref().unwrap().id.clone();
        let mut legacy = serde_json::to_value(&s.state).unwrap();
        legacy["schema_version"] = serde_json::json!(2);
        legacy["wallet"].as_object_mut().unwrap().remove("pearls");
        legacy["shell_hunt"].as_object_mut().unwrap().remove("collection");
        legacy["shell_hunt"].as_object_mut().unwrap().remove("last_catch");
        for shell in legacy["shell_hunt"]["batch"]["shells"].as_array_mut().unwrap() {
            for key in ["kind", "rare", "pearl"] { shell.as_object_mut().unwrap().remove(key); }
        }
        fs::write(&s.path, serde_json::to_vec(&legacy).unwrap()).unwrap(); drop(s);
        let loaded = store(dir.path()); assert!(loaded.read_only.is_none());
        let batch = loaded.state.shell_hunt.batch.as_ref().unwrap(); assert_eq!(batch.id, batch_id);
        for shell in &batch.shells {
            assert!(!shell.rare && !shell.pearl);
            assert_eq!(shell.kind, match shell.size {1=>ShellKind::Queen,2=>ShellKind::Variegated,_=>ShellKind::Great});
        }
        assert_eq!(loaded.state.wallet.pearls, 0); assert_eq!(loaded.state.wallet.cbcoins, 300); assert!(loaded.state.shell_hunt.collection.is_empty());
    }

    #[test]
    fn pearl_overflow_and_daily_cap_do_not_grant_collection() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path()); s.start_hunt("2026-10-05").unwrap();
        let shell = &mut s.state.shell_hunt.batch.as_mut().unwrap().shells[0]; shell.rare = true; shell.pearl = true; let id = shell.id.clone();
        let session = s.hunt_session.as_mut().unwrap(); session.phase = "settling".into(); session.caught_id = Some(id);
        s.state.wallet.pearls = u64::MAX;
        assert_eq!(s.tick_hunt(100, "2026-10-05").unwrap_err().code, "wallet_overflow");
        assert!(s.state.shell_hunt.collection.is_empty()); assert!(!s.state.shell_hunt.batch.as_ref().unwrap().shells[0].collected);
        s.state.wallet.pearls = 0; s.state.shell_hunt.earned = s.catalog.balance.shell_hunt.daily_cap;
        s.tick_hunt(100, "2026-10-05").unwrap(); assert_eq!(s.state.wallet.pearls, 0); assert!(s.state.shell_hunt.collection.is_empty());
    }

    #[test]
    fn collection_survives_finished_batch_and_repeat_discovery() {
        use super::super::hunt::ShellKind;
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path()); s.start_hunt("2026-10-05").unwrap();
        for shell in &mut s.state.shell_hunt.batch.as_mut().unwrap().shells { shell.kind = ShellKind::Great; shell.rare = false; shell.pearl = false; }
        let ids: Vec<_> = s.state.shell_hunt.batch.as_ref().unwrap().shells.iter().map(|s| s.id.clone()).collect();
        for (index, id) in ids.iter().enumerate() {
            let session = s.hunt_session.as_mut().unwrap(); session.phase = "settling".into(); session.caught_id = Some(id.clone());
            s.tick_hunt(100, "2026-10-05").unwrap();
            assert_eq!(s.state.shell_hunt.last_catch.as_ref().unwrap().first_of_kind, index == 0);
        }
        assert!(s.state.shell_hunt.batch.is_none()); assert!(s.hunt_session.is_none());
        assert_eq!(s.hunt_view().collection[&ShellKind::Great].count, 3);
        drop(s); let loaded = store(dir.path()); assert_eq!(loaded.hunt_view().collection[&ShellKind::Great].count, 3);
        assert_eq!(loaded.state.wallet.cbcoins, 303); assert_eq!(loaded.state.wallet.pearls, 0);
    }
}
