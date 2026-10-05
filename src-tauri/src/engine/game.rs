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

pub const SCHEMA_VERSION: u64 = 4;
pub const RULES_VERSION: u32 = 1;
pub const RENDERER_VERSION: u32 = 1;
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
    pub traits: BTreeMap<String, String>,
    pub visual_recipe: VisualRecipe,
    pub rules_version: u32,
    pub acquired_at: i64,
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
            if self.schema_version >= 4 && (f.exp > 10000 || f.pending_exp > 10000 - f.exp || f.resting_until < 0) {
                return Err(format!("invalid growth state {}", f.id));
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
            } else if version != 2 && version != 3 && version != SCHEMA_VERSION { return Err("unsupported save schema".into()); }
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
            exp: 0, pending_exp: 0, resting_until: 0, purchase_price: species.price,
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
        if self.state.fish.len() >= self.catalog.balance.tank_capacity {
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

    pub fn can_feed(&self, fish_id: &str, now: i64) -> Result<(), Failure> {
        let f = self.state.fish.iter().find(|f| f.id == fish_id).ok_or_else(|| Failure::new("fish_not_found", fish_id))?;
        if f.exp >= 10000 { return Err(Failure::new("fish_adult", "level 100")); }
        if f.resting_until > now { return Err(Failure::new("fish_full", f.resting_until)); }
        Ok(())
    }

    /// Calling the sale boat transfers only the game fish, never its Belly files.
    pub fn sell(&mut self, fish_id: &str) -> Result<u64, Failure> {
        let mut next = self.state.clone();
        let i = next.fish.iter().position(|f| f.id == fish_id).ok_or_else(|| Failure::new("fish_not_found", fish_id))?;
        if next.fish[i].exp < 10000 { return Err(Failure::new("fish_not_adult", "level 100 required")); }
        let price = next.fish[i].purchase_price.checked_mul(100).ok_or_else(|| Failure::new("wallet_overflow", "sale"))?;
        next.wallet.cbcoins = next.wallet.cbcoins.checked_add(price).ok_or_else(|| Failure::new("wallet_overflow", "sale"))?;
        next.fish.remove(i);
        self.commit(next)?;
        Ok(price)
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
                let room = 10000u64.saturating_sub(fish.exp + fish.pending_exp);
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
        super::hunt::HuntView { pearls: self.state.wallet.pearls, collection: h.collection.clone(), last_catch: h.last_catch.clone(), batch: h.batch.clone(), earned: h.earned, daily_cap: self.catalog.balance.shell_hunt.daily_cap, session: self.hunt_session.clone(), waiting: h.batch.is_none() }
    }

    pub fn start_hunt(&mut self, today: &str) -> Result<super::hunt::HuntView, Failure> {
        if let Some(e) = &self.read_only { return Err(e.clone()); }
        if self.state.settings.meeting_mode { return Err(Failure::new("hunt_meeting", "meeting mode")); }
        let mut next = self.state.clone();
        next.shell_hunt.roll_day(today);
        let room = self.catalog.balance.shell_hunt.daily_cap.saturating_sub(next.shell_hunt.earned);
        if room == 0 { return Err(Failure::new("hunt_cap", "daily shell hunt limit")); }
        if !next.shell_hunt.tutorial_granted {
            next.shell_hunt.tutorial_granted = true;
            if next.shell_hunt.batch.is_none() { next.shell_hunt.batch = Some(super::hunt::Batch::generate(room.min(3) as usize)); }
        }
        if next.shell_hunt.batch.is_none() { return Err(Failure::new("hunt_waiting", "no shells yet")); }
        self.commit(next)?;
        if self.hunt_session.is_none() {
            self.hunt_session = Some(super::hunt::HuntSession::new(self.state.shell_hunt.batch.as_ref().unwrap()));
        }
        Ok(self.hunt_view())
    }

    pub fn hunt_action(&mut self, session_id: &str, seq: u64, action: &str) -> Result<super::hunt::HuntView, Failure> {
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
            if let Some(b) = &next.shell_hunt.batch {
                if s.batch_id == b.id && room > 0 {
                    s.step(elapsed_ms as f64 / 1000.0, b);
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
        self.hunt_checkpoint_ms += elapsed_ms;
        if changed || self.hunt_checkpoint_ms >= 60_000 {
            if let Err(e) = self.commit(next) {
                if let Some(s) = &mut self.hunt_session { s.paused = true; s.error = Some(e.code.clone()); }
                return Err(e);
            }
            self.hunt_checkpoint_ms = 0;
            if finished { self.hunt_session = None; }
            else if let Some(s) = &mut self.hunt_session {
                if s.phase == "settling" && changed { s.phase = "swinging".into(); s.caught_id = None; s.error = None; }
            }
        } else {
            // Countdown is in memory until checkpoint; wallet/collected change only via commit.
            self.state.shell_hunt.remaining_wait_ms = next.shell_hunt.remaining_wait_ms;
        }
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
    if f.resting_until > now || f.pending_exp == 0 || f.exp >= 10000 { return; }
    let boundary = ((f.exp / 500 + 1) * 500).min(10000);
    let amount = f.pending_exp.min(boundary - f.exp);
    f.exp += amount; f.pending_exp -= amount;
    f.stage = if f.exp >= 10000 { Stage::Adult } else if f.exp >= 5000 { Stage::Juvenile } else { Stage::Fry };
    if f.exp == boundary && f.exp < 10000 { f.resting_until = now.saturating_add(7200); }
    if f.exp == 10000 { f.pending_exp = 0; f.resting_until = 0; }
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
        assert_eq!(s.state.fish[0].exp,10000);assert_eq!(s.state.fish[0].stage,Stage::Adult);
        assert!(s.state.ledger.fingerprints.contains(&"p1m:legacy".into()));
        let id=s.state.fish[0].id.clone();let price=s.state.fish[0].purchase_price;s.sell(&id).unwrap();
        assert_eq!(store(dir.path()).state.wallet.cbcoins,123+price*100);
    }

    #[test]
    fn duplicate_history_is_not_pruned_and_sale_overflow_is_atomic() {
        let dir=tempfile::tempdir().unwrap();let mut s=store(dir.path());s.catalog.balance.economy.max_fingerprints=1;
        let id=s.state.fish[0].id.clone();s.apply_receipts(&[entry("a","A",&id,true),entry("b","B",&id,true)],"2026-10-05",NOW).unwrap();
        assert_eq!(s.apply_receipts(&[entry("c","A",&id,true)],"2026-10-05",NOW).unwrap().duplicates,1);
        s.state.fish[0].exp=10000;s.state.fish[0].stage=Stage::Adult;s.state.wallet.cbcoins=u64::MAX;
        assert_eq!(s.sell(&id).unwrap_err().code,"wallet_overflow");assert_eq!(s.state.fish.len(),1);assert_eq!(s.state.wallet.cbcoins,u64::MAX);
    }

    #[test]
    fn growth_rest_bank_restart_adult_and_sale() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let id = s.state.fish[0].id.clone();
        let mut e = entry("big", "big-hash", &id, true); e.size = 599_999_999;
        assert_eq!(s.apply_receipts(&[e], "2026-10-05", NOW).unwrap().exp, 600);
        assert_eq!((s.state.fish[0].exp, s.state.fish[0].pending_exp), (500,100));
        assert_eq!(s.can_feed(&id, NOW+7199).unwrap_err().code, "fish_full");
        drop(s); let mut s = store(dir.path());
        s.digest(NOW+7199).unwrap(); assert_eq!(s.state.fish[0].exp,500);
        s.digest(NOW+7200).unwrap(); assert_eq!((s.state.fish[0].exp,s.state.fish[0].pending_exp),(600,0));
        assert!(s.can_feed(&id,NOW+7200).is_ok());
        assert_eq!(s.sell(&id).unwrap_err().code,"fish_not_adult");
        s.state.fish[0].exp = 4900; s.state.fish[0].pending_exp = 200; s.state.fish[0].resting_until = 0;
        s.digest(NOW+7200).unwrap(); assert_eq!(s.state.fish[0].stage,Stage::Juvenile); assert_eq!(s.state.fish[0].exp,5000);
        s.state.fish[0].exp = 9900; s.state.fish[0].pending_exp = 100; s.state.fish[0].resting_until = 0;
        s.digest(NOW+14400).unwrap(); assert_eq!(s.state.fish[0].stage,Stage::Adult);
        assert_eq!(s.can_feed(&id,NOW+14400).unwrap_err().code,"fish_adult");
        let paid = s.state.fish[0].purchase_price;
        let good = s.path.clone(); s.path = dir.path().join("missing/game.json");
        assert!(s.sell(&id).is_err()); assert_eq!(s.state.fish.len(),1); assert_eq!(s.state.wallet.cbcoins,40);
        s.path = good; assert_eq!(s.sell(&id).unwrap(),paid*100); assert_eq!(s.state.wallet.cbcoins,40+paid*100);
        assert_eq!(s.sell(&id).unwrap_err().code,"fish_not_found");
        let loaded = store(dir.path()); assert!(loaded.state.fish.is_empty()); assert_eq!(loaded.state.wallet.cbcoins,40+paid*100);
    }

    #[test]
    fn all_shell_colors_credit_coin_values_once() {
        use super::super::hunt::ShellKind;
        for (kind,value) in [(ShellKind::Great,1),(ShellKind::Queen,10),(ShellKind::Variegated,100)] {
            let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path()); s.start_hunt("2026-10-05").unwrap();
            let shell = &mut s.state.shell_hunt.batch.as_mut().unwrap().shells[0]; shell.kind=kind; shell.pearl=false; let id=shell.id.clone();
            let session=s.hunt_session.as_mut().unwrap(); session.phase="settling".into();session.caught_id=Some(id.clone());
            s.tick_hunt(100,"2026-10-05").unwrap(); assert_eq!(s.state.wallet.cbcoins,40+value);
            let session=s.hunt_session.as_mut().unwrap();session.phase="settling".into();session.caught_id=Some(id);
            s.tick_hunt(100,"2026-10-05").unwrap();assert_eq!(s.state.wallet.cbcoins,40+value);
        }
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
        assert_eq!(s.state.wallet.cbcoins, 40);
        assert_eq!(s.state.fish.len(), 1);
        assert_eq!(s.state.fish[0].origin, Origin::Starter);
        assert!(s.state.fish[0].parent_ids.is_empty());
        s.purchase("danio_rerio", 20, NOW).unwrap();
        let s = store(dir.path());
        assert_eq!(s.state.wallet.cbcoins, 20, "welcome credit must not be granted again");
        assert_eq!(s.state.fish.len(), 2);
    }

    #[test]
    fn purchase_rules() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        assert_eq!(s.purchase("nope", 1, NOW).unwrap_err().code, "unknown_species");
        assert_eq!(s.purchase("danio_rerio", 19, NOW).unwrap_err().code, "price_changed");
        assert_eq!(s.purchase("symphysodon_aequifasciatus", 80, NOW).unwrap_err().code, "not_enough_shells");
        let f = s.purchase("danio_rerio", 20, NOW).unwrap();
        assert_eq!(f.origin, Origin::Shop);
        assert_eq!(f.name, "Cá ngựa vằn");
        assert_eq!(f.stage, Stage::Fry);
        assert_eq!(s.state.wallet.cbcoins, 20);
        assert!(s.state.dex["danio_rerio"].owned);
    }

    #[test]
    fn full_tank_blocks_purchase_without_charging() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.state.wallet.cbcoins = 10_000;
        while s.state.fish.len() < 12 {
            s.purchase("danio_rerio", 20, NOW).unwrap();
        }
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
        assert_eq!(s.state.wallet.cbcoins, 40);
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
        assert_eq!(s.state.wallet.cbcoins, 40);
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
        let many: Vec<_> = (0..200).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        let r = s.apply_receipts(&many, "2026-09-29", NOW).unwrap();
        assert_eq!(r.shells, 0);
        assert_eq!(r.exp, 4000);
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
        assert_eq!(s.state.wallet.cbcoins, 40);
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
        assert_eq!(s.state.fish[0].exp, 400);
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
        assert_eq!(s.state.wallet.cbcoins, 20); assert_eq!(s.state.fish.len(), 2);
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
        assert_eq!(s.state.wallet.cbcoins, 40); assert!(!s.state.shell_hunt.batch.as_ref().unwrap().shells[0].collected);
        s.path = good;
        assert!(s.tick_hunt(100, "2026-10-03").unwrap());
        assert_eq!(s.state.wallet.cbcoins, 41); assert_eq!(s.state.shell_hunt.earned, 1); assert_eq!(s.state.daily.shells, 0);
        let session = s.hunt_session.as_mut().unwrap(); session.phase = "settling".into(); session.caught_id = Some(id);
        s.tick_hunt(100, "2026-10-03").unwrap(); assert_eq!(s.state.wallet.cbcoins, 41);
        let loaded = store(dir.path()); assert_eq!(loaded.state.wallet.cbcoins, 41);
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
        assert_eq!(s.state.wallet.pearls, 1); assert_eq!(s.state.wallet.cbcoins, 50);
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
        assert_eq!(loaded.state.wallet.pearls, 0); assert_eq!(loaded.state.wallet.cbcoins, 40); assert!(loaded.state.shell_hunt.collection.is_empty());
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
        assert_eq!(loaded.state.wallet.cbcoins, 43); assert_eq!(loaded.state.wallet.pearls, 0);
    }
}
