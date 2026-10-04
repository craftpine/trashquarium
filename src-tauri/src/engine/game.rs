//! Game save: fish, wallet (Vỏ sò), reward ledger, Fishdex flags and settings.
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

pub const SCHEMA_VERSION: u64 = 2;
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
    pub traits: BTreeMap<String, String>,
    pub visual_recipe: VisualRecipe,
    pub rules_version: u32,
    pub acquired_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Wallet {
    pub shells: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RewardLedger {
    /// Every Belly receipt already settled, rewarded or not. Never pruned.
    pub receipts: BTreeSet<String>,
    /// Fingerprints already rewarded, oldest first. Bounded by
    /// `economy.max_fingerprints`; beyond that the oldest are forgotten, so
    /// duplicate protection is not permanent.
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
                v["schema_version"] = serde_json::Value::from(SCHEMA_VERSION);
            } else if version != SCHEMA_VERSION { return Err("unsupported save schema".into()); }
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
                fresh.wallet.shells = store.catalog.balance.economy.welcome_shells;
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
        let stage = match species.stage_on_purchase.as_str() {
            "fry" => Stage::Fry,
            "adult" => Stage::Adult,
            _ => Stage::Juvenile,
        };
        let thresholds = &self.catalog.balance.stage_exp;
        Fish {
            id: uuid::Uuid::new_v4().to_string(),
            species_id: species.id.clone(),
            name: species.name.clone(),
            origin,
            parent_ids: Vec::new(),
            generation: 0,
            stage,
            exp: match stage {
                Stage::Fry => 0,
                Stage::Juvenile => thresholds.juvenile,
                Stage::Adult => thresholds.adult,
            },
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
        if self.state.wallet.shells < species.price {
            return Err(Failure::new("not_enough_shells", species.price - self.state.wallet.shells));
        }
        let fish = self.new_fish(&species, Origin::Shop, now);
        let mut next = self.state.clone();
        next.wallet.shells -= species.price;
        next.fish.push(fish.clone());
        mark_owned(&mut next, &species.id);
        self.commit(next)?;
        Ok(fish)
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
        let eco = self.catalog.balance.economy.clone();
        let thresholds = self.catalog.balance.stage_exp.clone();
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
            while next.ledger.fingerprints.len() > eco.max_fingerprints {
                next.ledger.fingerprints.pop_front();
            }
            let diminish = (1.0 - eco.diminish_step * next.daily.rewarded as f64).max(eco.diminish_min);
            let size_factor = ((e.size as f64 / 1024.0 + 1.0).log10() / eco.size_log_divisor).clamp(eco.size_min, eco.size_max);
            let age_days = ((now - e.modified_unix) as f64 / 86400.0).max(0.0);
            let age_factor = if age_days < eco.age_recent_days {
                eco.age_recent_mult
            } else if age_days <= eco.age_old_days {
                eco.age_middle_mult
            } else {
                eco.age_old_mult
            };
            let exp_room = eco.daily_exp_cap.saturating_sub(next.daily.exp);
            let shell_room = eco.daily_shell_cap.saturating_sub(next.daily.shells);
            let exp = ((eco.base_exp * eco.category_mult(e.category) * size_factor * age_factor * diminish).round() as u64).min(exp_room);
            let shells = ((eco.base_shells * diminish).round() as u64).max(1).min(shell_room);
            if shells == 0 || exp_room == 0 {
                summary.capped = true;
            }
            next.daily.rewarded += 1;
            next.daily.exp += exp;
            next.daily.shells += shells;
            next.wallet.shells += shells;
            if let Some(fish) = next.fish.iter_mut().find(|f| f.id == e.fish_id) {
                fish.exp += exp;
                let grown = if fish.exp >= thresholds.adult {
                    Stage::Adult
                } else if fish.exp >= thresholds.juvenile {
                    Stage::Juvenile
                } else {
                    Stage::Fry
                };
                fish.stage = fish.stage.max(grown);
            }
            summary.files += 1;
            summary.shells += shells;
            summary.exp += exp;
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
        super::hunt::HuntView { batch: h.batch.clone(), earned: h.earned, daily_cap: self.catalog.balance.shell_hunt.daily_cap, session: self.hunt_session.clone(), waiting: h.batch.is_none() }
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
                            next.shell_hunt.batch.as_mut().unwrap().shells.iter_mut().find(|sh| sh.id == id).unwrap().collected = true;
                            next.shell_hunt.earned += 1;
                            next.wallet.shells = next.wallet.shells.checked_add(1).ok_or_else(|| Failure::new("wallet_overflow", ""))?;
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
        assert_eq!(s.state.wallet.shells, 40);
        assert_eq!(s.state.fish.len(), 1);
        assert_eq!(s.state.fish[0].origin, Origin::Starter);
        assert!(s.state.fish[0].parent_ids.is_empty());
        s.purchase("danio_rerio", 20, NOW).unwrap();
        let s = store(dir.path());
        assert_eq!(s.state.wallet.shells, 20, "welcome credit must not be granted again");
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
        assert_eq!(f.stage, Stage::Juvenile);
        assert_eq!(s.state.wallet.shells, 20);
        assert!(s.state.dex["danio_rerio"].owned);
    }

    #[test]
    fn full_tank_blocks_purchase_without_charging() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.state.wallet.shells = 10_000;
        while s.state.fish.len() < 12 {
            s.purchase("danio_rerio", 20, NOW).unwrap();
        }
        let before = s.state.wallet.shells;
        assert_eq!(s.purchase("danio_rerio", 20, NOW).unwrap_err().code, "tank_full");
        assert_eq!(s.state.wallet.shells, before);
    }

    #[test]
    fn failed_save_neither_charges_nor_creates_fish() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.path = dir.path().join("missing-dir/game.json");
        assert_eq!(s.purchase("danio_rerio", 20, NOW).unwrap_err().code, "save_failed");
        assert_eq!(s.state.wallet.shells, 40);
        assert_eq!(s.state.fish.len(), 1);
    }

    #[test]
    fn receipts_pay_exactly_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        let entries = vec![entry("r1", "fp1", &fish, true)];
        let first = s.apply_receipts(&entries, "2026-09-29", NOW).unwrap();
        assert_eq!(first.shells, 2);
        assert!(first.exp > 0);
        let replay = s.apply_receipts(&entries, "2026-09-29", NOW).unwrap();
        assert_eq!(replay, RewardSummary::default());
        assert_eq!(s.state.wallet.shells, 42);
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
    fn daily_cap_holds_and_clock_rollback_does_not_reset_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        let many: Vec<_> = (0..200).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        let r = s.apply_receipts(&many, "2026-09-29", NOW).unwrap();
        assert_eq!(r.shells, 100);
        assert!(r.capped);
        let more: Vec<_> = (200..210).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        let back = s.apply_receipts(&more, "2026-09-28", NOW).unwrap();
        assert_eq!(back.shells, 0, "turning the clock back must not reopen the cap");
        let next: Vec<_> = (300..301).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        assert!(s.apply_receipts(&next, "2026-09-30", NOW).unwrap().shells >= 1);
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
        assert_eq!(s.state.wallet.shells, 40);
        s.path = good;
        assert_eq!(s.apply_receipts(&entries, "2026-09-29", NOW).unwrap().shells, 2);
    }

    #[test]
    fn feeding_grows_the_fish() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let fish = s.state.fish[0].id.clone();
        let many: Vec<_> = (0..20).map(|i| entry(&format!("r{i}"), &format!("fp{i}"), &fish, true)).collect();
        s.apply_receipts(&many, "2026-09-29", NOW).unwrap();
        assert_eq!(s.state.fish[0].stage, Stage::Adult);
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
        assert!(s.read_only.is_none()); assert_eq!(s.state.schema_version, 2);
        assert_eq!(s.state.wallet.shells, 20); assert_eq!(s.state.fish.len(), 2);
        assert_eq!(s.state.fish[0].id, id); assert!(s.state.ledger.receipts.contains("paid"));
        assert!(s.state.settings.quick_dock_enabled);
    }

    #[test]
    fn hunt_reward_is_atomic_replay_safe_and_separate_from_feeding_cap() {
        let dir = tempfile::tempdir().unwrap(); let mut s = store(dir.path());
        let view = s.start_hunt("2026-10-03").unwrap();
        let b = view.batch.unwrap(); let id = b.shells[0].id.clone();
        let session = s.hunt_session.as_mut().unwrap();
        session.phase = "settling".into(); session.caught_id = Some(id.clone());
        let good = s.path.clone(); s.path = dir.path().join("missing/save.json");
        assert!(s.tick_hunt(100, "2026-10-03").is_err());
        assert_eq!(s.state.wallet.shells, 40); assert!(!s.state.shell_hunt.batch.as_ref().unwrap().shells[0].collected);
        s.path = good;
        assert!(s.tick_hunt(100, "2026-10-03").unwrap());
        assert_eq!(s.state.wallet.shells, 41); assert_eq!(s.state.shell_hunt.earned, 1); assert_eq!(s.state.daily.shells, 0);
        let session = s.hunt_session.as_mut().unwrap(); session.phase = "settling".into(); session.caught_id = Some(id);
        s.tick_hunt(100, "2026-10-03").unwrap(); assert_eq!(s.state.wallet.shells, 41);
        let loaded = store(dir.path()); assert_eq!(loaded.state.wallet.shells, 41);
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
}
