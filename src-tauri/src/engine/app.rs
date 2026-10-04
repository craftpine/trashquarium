//! Application facade used by the Tauri commands.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::belly::{Attention, BellyEntry, BellyVault, EntryState, RestoreOutcome};
use super::catalog::{Catalog, Species, StageExp};
use super::game::{DexEntry, Fish, GameStore, RewardSummary, Settings};
use super::guard::{self, GuardPolicy, Inspection};
use super::{local_today, now_unix, Failure};

pub struct AppCore {
    pub data_root: PathBuf,
    pub policy: GuardPolicy,
    pub belly: Result<BellyVault, Failure>,
    pub game: GameStore,
}

#[derive(Serialize, Clone, Debug)]
pub struct FeedOutcome {
    pub path: String,
    pub name: String,
    pub ok: bool,
    pub code: String,
    pub undetermined: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct FeedReport {
    pub files: Vec<FeedOutcome>,
    pub reward: Option<RewardSummary>,
    /// Set when files are safely in the Belly but the reward could not be
    /// saved yet; it is paid on the next successful reconciliation.
    pub reward_error: Option<Failure>,
}

#[derive(Serialize, Clone, Debug)]
pub struct DailyView {
    pub shells: u64,
    pub exp: u64,
    pub shell_cap: u64,
    pub exp_cap: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct StateView {
    pub hunt: super::hunt::HuntView,
    pub shells: u64,
    pub capacity: usize,
    pub fish: Vec<Fish>,
    pub daily: DailyView,
    pub dex: BTreeMap<String, DexEntry>,
    pub settings: Settings,
    pub species: Vec<Species>,
    pub disclaimer: String,
    pub stage_exp: StageExp,
    pub max_preview_files: usize,
    pub read_only: Option<Failure>,
    pub recovered_from_backup: bool,
    pub belly_error: Option<Failure>,
    pub attention: Vec<Attention>,
    pub held_count: usize,
    pub data_dir: String,
}

impl AppCore {
    pub fn open(data_root: PathBuf, policy: GuardPolicy, catalog: Catalog) -> Self {
        let belly = BellyVault::open(&data_root);
        let game = GameStore::open(&data_root.join("game_save.json"), catalog, now_unix());
        let mut core = AppCore { data_root, policy, belly, game };
        // Pay out receipts committed before a crash or a failed save, once.
        let _ = core.reconcile();
        core
    }

    fn reconcile(&mut self) -> Result<RewardSummary, Failure> {
        let vault = self.belly.as_ref().map_err(Clone::clone)?;
        self.game.apply_receipts(vault.entries(), &local_today(), now_unix())
    }

    pub fn preview(&self, paths: &[String]) -> Vec<Inspection> {
        let mut seen = std::collections::HashSet::new();
        paths
            .iter()
            .filter(|p| seen.insert(p.as_str()))
            .take(self.game.catalog().balance.max_preview_files)
            .map(|p| guard::inspect(&self.policy, Path::new(p)))
            .collect()
    }

    pub fn feed(&mut self, items: &[Inspection], fish_id: &str) -> Result<FeedReport, Failure> {
        if let Some(e) = &self.game.read_only {
            return Err(e.clone());
        }
        if !self.game.state.fish.iter().any(|f| f.id == fish_id) {
            return Err(Failure::new("fish_not_found", fish_id));
        }
        let limit = self.game.catalog().balance.max_preview_files;
        if items.len() > limit {
            return Err(Failure::new("too_many_files", limit));
        }
        let vault = self.belly.as_mut().map_err(|e| e.clone())?;
        let mut files = Vec::new();
        for item in items {
            let outcome = if !item.ok {
                FeedOutcome { path: item.path.clone(), name: item.name.clone(), ok: false, code: item.code.clone(), undetermined: false }
            } else {
                match vault.swallow(&self.policy, item, fish_id) {
                    Ok(_) => FeedOutcome { path: item.path.clone(), name: item.name.clone(), ok: true, code: "ok".into(), undetermined: false },
                    Err(e) => FeedOutcome { path: item.path.clone(), name: item.name.clone(), ok: false, code: e.code, undetermined: e.undetermined },
                }
            };
            files.push(outcome);
        }
        let (reward, reward_error) = match self.reconcile() {
            Ok(r) => (Some(r), None),
            Err(e) => (None, Some(e)),
        };
        Ok(FeedReport { files, reward, reward_error })
    }

    pub fn restore(&mut self, entry_id: &str) -> Result<RestoreOutcome, Failure> {
        let vault = self.belly.as_mut().map_err(|e| e.clone())?;
        vault.restore(&self.policy, entry_id)
    }

    /// Re-runs Belly recovery (or retries opening it) and pending rewards.
    pub fn recover(&mut self) {
        match &mut self.belly {
            Ok(vault) => {
                if let Err(e) = vault.recover() {
                    self.belly = Err(e);
                }
            }
            Err(_) => self.belly = BellyVault::open(&self.data_root),
        }
        let _ = self.reconcile();
    }

    pub fn held_entries(&self) -> Vec<BellyEntry> {
        let mut held: Vec<BellyEntry> = match &self.belly {
            Ok(v) => v.entries().iter().filter(|e| e.state == EntryState::Held).cloned().collect(),
            Err(_) => Vec::new(),
        };
        held.sort_by(|a, b| b.eaten_at.cmp(&a.eaten_at));
        held
    }

    pub fn view(&self) -> StateView {
        let c = self.game.catalog();
        let s = &self.game.state;
        StateView {
            hunt: self.game.hunt_view(),
            shells: s.wallet.shells,
            capacity: c.balance.tank_capacity,
            fish: s.fish.clone(),
            daily: DailyView {
                shells: if s.daily.date == local_today() { s.daily.shells } else { 0 },
                exp: if s.daily.date == local_today() { s.daily.exp } else { 0 },
                shell_cap: c.balance.economy.daily_shell_cap,
                exp_cap: c.balance.economy.daily_exp_cap,
            },
            dex: s.dex.clone(),
            settings: s.settings.clone(),
            species: c.species.clone(),
            disclaimer: c.disclaimer.clone(),
            stage_exp: c.balance.stage_exp.clone(),
            max_preview_files: c.balance.max_preview_files,
            read_only: self.game.read_only.clone(),
            recovered_from_backup: self.game.recovered_from_backup,
            belly_error: self.belly.as_ref().err().cloned(),
            attention: self.belly.as_ref().map(|v| v.attention().to_vec()).unwrap_or_default(),
            held_count: self.belly.as_ref().map(|v| v.entries().iter().filter(|e| e.state == EntryState::Held).count()).unwrap_or(0),
            data_dir: self.data_root.to_string_lossy().into_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::guard::tests::Sandbox;
    use std::fs;

    fn core(s: &Sandbox) -> AppCore {
        AppCore::open(s.policy.data_root.clone(), s.policy.clone(), Catalog::bundled().unwrap())
    }

    #[test]
    fn feed_moves_files_and_pays_once() {
        let s = Sandbox::new();
        let a = s.file("docs/a.txt", "aaa");
        let b = s.file("docs/b.sh", "bbb");
        let mut c = core(&s);
        let fish = c.game.state.fish[0].id.clone();
        let preview = c.preview(&[a.to_string_lossy().into(), b.to_string_lossy().into()]);
        assert!(preview[0].ok && !preview[1].ok);
        let report = c.feed(&preview, &fish).unwrap();
        assert!(report.files[0].ok);
        assert_eq!(report.files[1].code, "extension");
        assert_eq!(report.reward.unwrap().shells, 2);
        assert_eq!(c.view().shells, 42);
        assert!(!a.exists() && b.exists());
        // Restore does not claw back the reward; feeding the same bytes again pays nothing.
        let id = c.held_entries()[0].id.clone();
        c.restore(&id).unwrap();
        assert_eq!(fs::read_to_string(&a).unwrap(), "aaa");
        let again = c.preview(&[a.to_string_lossy().into()]);
        let r = c.feed(&again, &fish).unwrap().reward.unwrap();
        assert_eq!((r.shells, r.duplicates), (0, 1));
        assert_eq!(c.view().shells, 42);
    }

    #[test]
    fn pending_receipt_is_paid_on_next_start() {
        let s = Sandbox::new();
        let a = s.file("docs/a.txt", "aaa");
        let mut c = core(&s);
        let fish = c.game.state.fish[0].id.clone();
        let preview = c.preview(&[a.to_string_lossy().into()]);
        // The Belly commits but the game save cannot be written.
        fs::rename(s.policy.data_root.join("game_save.json"), s.root.join("moved-save.json")).unwrap();
        fs::create_dir(s.policy.data_root.join("game_save.json")).unwrap();
        let report = c.feed(&preview, &fish).unwrap();
        assert!(report.files[0].ok);
        assert!(report.reward_error.is_some());
        drop(c);
        fs::remove_dir(s.policy.data_root.join("game_save.json")).unwrap();
        fs::rename(s.root.join("moved-save.json"), s.policy.data_root.join("game_save.json")).unwrap();
        let c = core(&s);
        assert_eq!(c.view().shells, 42);
        drop(c);
        assert_eq!(core(&s).view().shells, 42, "paid exactly once");
    }
}
