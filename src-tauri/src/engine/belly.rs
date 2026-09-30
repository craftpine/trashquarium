//! The Fish Belly: a journaled holding area for files the player chose to feed.
//!
//! Every mutation is a transaction `prepared → file_moved → indexed → committed`
//! with a durable journal written before the file moves. Files are only ever
//! renamed (same volume, never replacing anything). Nothing is deleted: the
//! Belly keeps files until the player restores them.
//!
//! Recovery runs on open, before any new transaction. When the outcome cannot
//! be decided safely, the transaction is parked as `needs_attention` and both
//! copies are left untouched.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::fsops;
use super::guard::{self, Category, GuardPolicy, Inspection};
use super::{now_unix, Failure};
use super::save::{self, Loaded};

const INDEX_SCHEMA: u64 = 1;
const INDEX_BACKUPS: usize = 2;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntryState {
    Held,
    Restored,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BellyEntry {
    pub id: String,
    /// Durable receipt for the reward; equals the swallow transaction ID.
    pub receipt_id: String,
    pub original_path: String,
    pub stored_path: String,
    pub name: String,
    pub size: u64,
    pub fingerprint: String,
    pub category: Category,
    pub modified_unix: i64,
    pub eaten_at: i64,
    pub fish_id: String,
    /// False when crash recovery finished the move: recovery never pays out.
    pub rewardable: bool,
    pub state: EntryState,
    pub restored_path: Option<String>,
    pub restored_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct Index {
    schema_version: u64,
    entries: Vec<BellyEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Swallow,
    Restore,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TxState {
    Prepared,
    FileMoved,
    Indexed,
    Committed,
    RolledBack,
    Aborted,
    NeedsAttention,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub transaction_id: String,
    pub op: Op,
    pub state: TxState,
    pub entry: BellyEntry,
    pub from: String,
    pub to: String,
    pub note: Option<String>,
    pub updated_at: i64,
}

/// A transaction recovery could not settle. Shown to the player with both paths.
#[derive(Serialize, Clone, Debug)]
pub struct Attention {
    pub transaction_id: String,
    pub op: Option<Op>,
    pub name: String,
    pub from: String,
    pub to: String,
    pub note: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct RestoreOutcome {
    pub path: String,
    pub renamed: bool,
}

pub struct BellyVault {
    belly_dir: PathBuf,
    tx_dir: PathBuf,
    done_dir: PathBuf,
    index_path: PathBuf,
    entries: Vec<BellyEntry>,
    attention: Vec<Attention>,
    _lock: File,
    /// Test hook: pretend the process died at this checkpoint.
    pub(crate) crash_at: Option<&'static str>,
}

impl BellyVault {
    /// Opens the vault under `root`, taking an exclusive lock (one running game
    /// per profile) and running recovery. Fails closed: if the index is
    /// unreadable no file is moved.
    pub fn open(root: &Path) -> Result<Self, Failure> {
        fs::create_dir_all(root).map_err(|e| Failure::new("data_dir", e))?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(root.join("belly.lock"))
            .map_err(|e| Failure::new("data_dir", e))?;
        lock.try_lock().map_err(|_| Failure::new("already_running", "another TrashQuarium holds this profile"))?;
        let belly_dir = root.join("Belly");
        let tx_dir = root.join("Transactions");
        let done_dir = tx_dir.join("done");
        for d in [root, belly_dir.as_path(), tx_dir.as_path(), done_dir.as_path()] {
            fs::create_dir_all(d).map_err(|e| Failure::new("data_dir", e))?;
            let meta = fs::symlink_metadata(d).map_err(|e| Failure::new("data_dir", e))?;
            if fsops::is_link(&meta) {
                return Err(Failure::new("data_dir", format!("{} is a link", d.display())));
            }
        }
        let index_path = root.join("belly_index.json");
        let belly_for_parse = belly_dir.clone();
        let entries = match save::load_with_backups(&index_path, INDEX_BACKUPS, INDEX_SCHEMA, |v| {
            let index: Index = serde_json::from_value(v).map_err(|e| e.to_string())?;
            validate_entries(&index.entries, &belly_for_parse)?;
            Ok(index.entries)
        }) {
            Loaded::Missing => Vec::new(),
            Loaded::Ok { value, .. } => value,
            Loaded::Future { version } => {
                return Err(Failure::new("index_future", format!("Belly index version {version} is newer than this game")))
            }
            Loaded::Corrupt { reason } => return Err(Failure::new("index_corrupt", reason)),
        };
        let mut vault = BellyVault {
            belly_dir,
            tx_dir,
            done_dir,
            index_path,
            entries,
            attention: Vec::new(),
            _lock: lock,
            crash_at: None,
        };
        vault.recover()?;
        Ok(vault)
    }

    pub fn entries(&self) -> &[BellyEntry] {
        &self.entries
    }

    pub fn attention(&self) -> &[Attention] {
        &self.attention
    }

    /// Moves a previewed file into the Belly. The file is inspected again right
    /// before the move; any change since the preview refuses the file.
    pub fn swallow(&mut self, policy: &GuardPolicy, preview: &Inspection, fish_id: &str) -> Result<BellyEntry, Failure> {
        let original = PathBuf::from(&preview.path);
        let fresh = guard::inspect(policy, &original);
        if !fresh.ok {
            return Err(Failure::new(&fresh.code, "file refused on re-check"));
        }
        if fresh.fingerprint != preview.fingerprint || fresh.size != preview.size || fresh.modified_unix != preview.modified_unix {
            return Err(Failure::new("changed_since_preview", "file changed after preview"));
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        let tx_id = uuid::Uuid::new_v4().simple().to_string();
        let slot = self.belly_dir.join(&id);
        fs::create_dir(&slot).map_err(|e| Failure::new("belly_write", e))?;
        let stored = slot.join(&fresh.name);
        let entry = BellyEntry {
            id,
            receipt_id: tx_id.clone(),
            original_path: fresh.path.clone(),
            stored_path: stored.to_string_lossy().into_owned(),
            name: fresh.name.clone(),
            size: fresh.size,
            fingerprint: fresh.fingerprint.clone().unwrap_or_default(),
            category: fresh.category.unwrap_or(Category::Tech),
            modified_unix: fresh.modified_unix,
            eaten_at: now_unix(),
            fish_id: fish_id.into(),
            rewardable: true,
            state: EntryState::Held,
            restored_path: None,
            restored_at: None,
        };
        let mut tx = Transaction {
            transaction_id: tx_id,
            op: Op::Swallow,
            state: TxState::Prepared,
            entry: entry.clone(),
            from: entry.original_path.clone(),
            to: entry.stored_path.clone(),
            note: None,
            updated_at: now_unix(),
        };
        if let Err(e) = self.journal(&tx) {
            let _ = fs::remove_dir(&slot); // empty folder we just made
            return Err(Failure::new("journal_write", e));
        }
        self.checkpoint("prepared")?;
        if let Err(e) = fsops::move_no_replace(&original, &stored) {
            self.finish(&mut tx, TxState::Aborted);
            let _ = fs::remove_dir(&slot);
            return Err(Failure::new("move_failed", e));
        }
        self.checkpoint("moved")?;
        tx.state = TxState::FileMoved;
        let _ = self.journal(&tx); // if this write is lost, recovery still sees the file in the Belly
        self.checkpoint("file_moved")?;
        self.entries.push(entry.clone());
        if let Err(e) = self.save_index() {
            self.entries.pop();
            return match fsops::move_no_replace(&stored, &original) {
                Ok(()) => {
                    self.finish(&mut tx, TxState::RolledBack);
                    let _ = fs::remove_dir(&slot);
                    Err(Failure::new("index_write", e))
                }
                Err(_) => Err(Failure::undetermined("index_write", e)),
            };
        }
        self.checkpoint("indexed")?;
        tx.state = TxState::Indexed;
        let _ = self.journal(&tx);
        self.finish(&mut tx, TxState::Committed);
        Ok(entry)
    }

    /// Returns a held file to where it came from. If that name is taken, uses
    /// `name (restored N).ext`. Never overwrites. If the original folder is
    /// gone or unsafe, the file stays in the Belly.
    pub fn restore(&mut self, policy: &GuardPolicy, entry_id: &str) -> Result<RestoreOutcome, Failure> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.id == entry_id && e.state == EntryState::Held)
            .ok_or_else(|| Failure::new("not_found", entry_id))?;
        let entry = self.entries[idx].clone();
        let stored = PathBuf::from(&entry.stored_path);
        if !fs::symlink_metadata(&stored).is_ok_and(|m| m.is_file()) {
            return Err(Failure::new("stored_missing", &entry.stored_path));
        }
        let original = PathBuf::from(&entry.original_path);
        let parent = original.parent().ok_or_else(|| Failure::new("original_unsafe", "no parent"))?;
        match fs::symlink_metadata(parent) {
            Ok(m) if m.is_dir() && !fsops::is_link(&m) => {}
            _ => return Err(Failure::new("original_folder_missing", parent.display())),
        }
        guard::check_location(policy, &original).map_err(|code| Failure::new("original_unsafe", code))?;
        let same_volume = matches!((fsops::volume_id(parent), fsops::volume_id(&stored)), (Ok(a), Ok(b)) if a == b);
        if !same_volume {
            return Err(Failure::new("original_unsafe", "cross_volume"));
        }
        let mut tx = Transaction {
            transaction_id: uuid::Uuid::new_v4().simple().to_string(),
            op: Op::Restore,
            state: TxState::Prepared,
            entry: entry.clone(),
            from: entry.stored_path.clone(),
            to: String::new(),
            note: None,
            updated_at: now_unix(),
        };
        let mut placed = None;
        for n in 0..1000u32 {
            let target = if n == 0 { original.clone() } else { fsops::restored_name(&original, n) };
            if fs::symlink_metadata(&target).is_ok() {
                continue;
            }
            tx.to = target.to_string_lossy().into_owned();
            self.journal(&tx).map_err(|e| Failure::new("journal_write", e))?;
            self.checkpoint("restore_prepared")?;
            match fsops::move_no_replace(&stored, &target) {
                Ok(()) => {
                    placed = Some((target, n > 0));
                    break;
                }
                // Someone created that name in between: try the next one.
                Err(_) if fs::symlink_metadata(&target).is_ok() => continue,
                Err(e) => {
                    self.finish(&mut tx, TxState::Aborted);
                    return Err(Failure::new("move_failed", e));
                }
            }
        }
        let Some((target, renamed)) = placed else {
            self.finish(&mut tx, TxState::Aborted);
            return Err(Failure::new("name_exhausted", original.display()));
        };
        self.checkpoint("restore_moved")?;
        tx.state = TxState::FileMoved;
        let _ = self.journal(&tx);
        mark_restored(&mut self.entries[idx], &target);
        if let Err(e) = self.save_index() {
            self.entries[idx] = entry;
            return match fsops::move_no_replace(&target, &stored) {
                Ok(()) => {
                    self.finish(&mut tx, TxState::RolledBack);
                    Err(Failure::new("index_write", e))
                }
                Err(_) => Err(Failure::undetermined("index_write", e)),
            };
        }
        self.finish(&mut tx, TxState::Committed);
        if let Some(slot) = stored.parent() {
            let _ = fs::remove_dir(slot); // only succeeds when empty
        }
        Ok(RestoreOutcome { path: target.to_string_lossy().into_owned(), renamed })
    }

    /// Settles every unfinished transaction. Safe to call at any time.
    pub fn recover(&mut self) -> Result<(), Failure> {
        self.attention.clear();
        let mut journals: Vec<PathBuf> = fs::read_dir(&self.tx_dir)
            .map_err(|e| Failure::new("journal_read", e))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        journals.sort();
        let mut index_changed = false;
        let mut settled: Vec<(Transaction, TxState, String)> = Vec::new();
        for path in journals {
            let parsed = fs::read_to_string(&path)
                .ok()
                .and_then(|t| serde_json::from_str::<Transaction>(&t).ok());
            let Some(tx) = parsed else {
                self.attention.push(Attention {
                    transaction_id: path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
                    op: None,
                    name: String::new(),
                    from: String::new(),
                    to: path.to_string_lossy().into_owned(),
                    note: "journal_unreadable".into(),
                });
                continue;
            };
            match tx.state {
                TxState::NeedsAttention => self.attention.push(attention_of(&tx, tx.note.as_deref().unwrap_or(""))),
                TxState::Committed | TxState::RolledBack | TxState::Aborted => {
                    let _ = fs::rename(&path, self.done_dir.join(path.file_name().unwrap()));
                }
                TxState::Prepared | TxState::FileMoved | TxState::Indexed => {
                    let (state, note, changed) = self.resolve(&tx);
                    index_changed |= changed;
                    settled.push((tx, state, note));
                }
            }
        }
        if index_changed {
            self.save_index().map_err(|e| Failure::new("index_write", e))?;
        }
        for (mut tx, state, note) in settled {
            if state == TxState::NeedsAttention {
                self.attention.push(attention_of(&tx, &note));
            }
            if !note.is_empty() {
                tx.note = Some(note);
            }
            self.finish(&mut tx, state);
        }
        Ok(())
    }

    /// Decides one unfinished transaction from what is actually on disk.
    /// Returns (final state, note, whether the in-memory index changed).
    fn resolve(&mut self, tx: &Transaction) -> (TxState, String, bool) {
        let from = Path::new(&tx.from);
        let to = Path::new(&tx.to);
        let from_exists = fs::symlink_metadata(from).is_ok();
        let to_exists = !tx.to.is_empty() && fs::symlink_metadata(to).is_ok();
        let pos = self.entries.iter().position(|e| e.id == tx.entry.id);
        match (tx.op.clone(), from_exists, to_exists) {
            (Op::Swallow, false, true) => {
                if pos.is_some() {
                    (TxState::Committed, String::new(), false)
                } else {
                    let mut entry = tx.entry.clone();
                    entry.rewardable = false;
                    self.entries.push(entry);
                    (TxState::Committed, "recovered_into_belly".into(), true)
                }
            }
            (Op::Swallow, true, false) if pos.is_none() => {
                if let Some(slot) = to.parent() {
                    let _ = fs::remove_dir(slot);
                }
                (TxState::RolledBack, "file_never_moved".into(), false)
            }
            (Op::Restore, false, true) => {
                match pos {
                    Some(i) if self.entries[i].state == EntryState::Held => {
                        mark_restored(&mut self.entries[i], to);
                        if let Some(slot) = from.parent() {
                            let _ = fs::remove_dir(slot);
                        }
                        (TxState::Committed, "restore_completed".into(), true)
                    }
                    Some(_) => (TxState::Committed, String::new(), false),
                    None => (TxState::NeedsAttention, "index_mismatch".into(), false),
                }
            }
            (Op::Restore, true, false) => (TxState::RolledBack, "file_never_moved".into(), false),
            (_, true, true) => (TxState::NeedsAttention, "both_exist".into(), false),
            (_, false, false) => (TxState::NeedsAttention, "missing".into(), false),
            _ => (TxState::NeedsAttention, "index_mismatch".into(), false),
        }
    }

    fn save_index(&self) -> std::io::Result<()> {
        let index = Index { schema_version: INDEX_SCHEMA, entries: self.entries.clone() };
        save::write_with_backups(&self.index_path, &index, INDEX_BACKUPS)
    }

    fn journal(&self, tx: &Transaction) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(tx).map_err(std::io::Error::other)?;
        fsops::write_atomic(&self.tx_dir.join(format!("{}.json", tx.transaction_id)), &bytes)
    }

    /// Records a final state. Settled journals move to `done/`; parked ones stay.
    fn finish(&self, tx: &mut Transaction, state: TxState) {
        tx.state = state;
        tx.updated_at = now_unix();
        if self.journal(tx).is_ok() && tx.state != TxState::NeedsAttention {
            let name = format!("{}.json", tx.transaction_id);
            let _ = fs::rename(self.tx_dir.join(&name), self.done_dir.join(&name));
        }
    }

    fn checkpoint(&self, name: &'static str) -> Result<(), Failure> {
        if self.crash_at == Some(name) {
            return Err(Failure::undetermined("simulated_crash", name));
        }
        Ok(())
    }
}

fn mark_restored(entry: &mut BellyEntry, target: &Path) {
    entry.state = EntryState::Restored;
    entry.restored_path = Some(target.to_string_lossy().into_owned());
    entry.restored_at = Some(now_unix());
}

fn attention_of(tx: &Transaction, note: &str) -> Attention {
    Attention {
        transaction_id: tx.transaction_id.clone(),
        op: Some(tx.op.clone()),
        name: tx.entry.name.clone(),
        from: tx.from.clone(),
        to: tx.to.clone(),
        note: note.into(),
    }
}

fn validate_entries(entries: &[BellyEntry], belly_dir: &Path) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for e in entries {
        if e.id.is_empty() || e.receipt_id.is_empty() || !ids.insert(e.id.as_str()) {
            return Err(format!("invalid or duplicate entry id {:?}", e.id));
        }
        if !fsops::is_within(Path::new(&e.stored_path), belly_dir) {
            return Err(format!("entry {} points outside the Belly", e.id));
        }
        if !Path::new(&e.original_path).is_absolute() {
            return Err(format!("entry {} has a relative original path", e.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::guard::tests::Sandbox;

    fn vault(s: &Sandbox) -> BellyVault {
        BellyVault::open(&s.policy.data_root).unwrap()
    }

    fn preview(s: &Sandbox, p: &Path) -> Inspection {
        let i = guard::inspect(&s.policy, p);
        assert!(i.ok, "{i:?}");
        i
    }

    fn pending_journals(s: &Sandbox) -> usize {
        fs::read_dir(s.policy.data_root.join("Transactions"))
            .unwrap()
            .filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "json"))
            .count()
    }

    #[test]
    fn swallow_then_restore_roundtrip() {
        let s = Sandbox::new();
        let p = s.file("docs/old.txt", "precious bytes");
        let mut v = vault(&s);
        let e = v.swallow(&s.policy, &preview(&s, &p), "fish-1").unwrap();
        assert!(!p.exists());
        assert_eq!(fs::read_to_string(&e.stored_path).unwrap(), "precious bytes");
        assert_eq!(pending_journals(&s), 0);
        let out = v.restore(&s.policy, &e.id).unwrap();
        assert_eq!(PathBuf::from(&out.path), p);
        assert!(!out.renamed);
        assert_eq!(fs::read_to_string(&p).unwrap(), "precious bytes");
        assert_eq!(v.entries()[0].state, EntryState::Restored);
        // The state survives reopening.
        drop(v);
        assert_eq!(vault(&s).entries()[0].state, EntryState::Restored);
    }

    #[test]
    fn file_changed_after_preview_is_refused() {
        let s = Sandbox::new();
        let p = s.file("docs/a.txt", "v1");
        let pre = preview(&s, &p);
        fs::write(&p, "v2 is longer").unwrap();
        let mut v = vault(&s);
        assert_eq!(v.swallow(&s.policy, &pre, "f").unwrap_err().code, "changed_since_preview");
        assert!(p.exists());
        assert!(v.entries().is_empty());
    }

    #[test]
    fn restore_never_overwrites() {
        let s = Sandbox::new();
        let p = s.file("docs/report.txt", "old");
        let mut v = vault(&s);
        let e = v.swallow(&s.policy, &preview(&s, &p), "f").unwrap();
        fs::write(&p, "new file with same name").unwrap();
        let out = v.restore(&s.policy, &e.id).unwrap();
        assert!(out.renamed);
        assert!(out.path.ends_with("report (restored 1).txt"));
        assert_eq!(fs::read_to_string(&p).unwrap(), "new file with same name");
        assert_eq!(fs::read_to_string(&out.path).unwrap(), "old");
    }

    #[test]
    fn restore_keeps_file_when_original_folder_is_gone() {
        let s = Sandbox::new();
        let p = s.file("docs/gone/a.txt", "x");
        let mut v = vault(&s);
        let e = v.swallow(&s.policy, &preview(&s, &p), "f").unwrap();
        fs::remove_dir(s.root.join("docs/gone")).unwrap(); // empty test folder
        assert_eq!(v.restore(&s.policy, &e.id).unwrap_err().code, "original_folder_missing");
        assert!(Path::new(&e.stored_path).exists());
        assert_eq!(v.entries()[0].state, EntryState::Held);
    }

    /// Simulate the process dying at each step; after reopening, the file must
    /// exist in exactly one place and the index must agree with the disk.
    #[test]
    fn swallow_crash_matrix() {
        for (point, expect_in_belly, expect_rewardable) in [
            ("prepared", false, false),
            ("moved", true, false),
            ("file_moved", true, false),
            ("indexed", true, true),
        ] {
            let s = Sandbox::new();
            let p = s.file("docs/a.txt", "data");
            let pre = preview(&s, &p);
            let mut v = vault(&s);
            v.crash_at = Some(point);
            let err = v.swallow(&s.policy, &pre, "f").unwrap_err();
            assert!(err.undetermined, "{point}");
            drop(v);
            let v = vault(&s);
            assert!(v.attention().is_empty(), "{point}: {:?}", v.attention());
            assert_eq!(pending_journals(&s), 0, "{point}");
            let held: Vec<_> = v.entries().iter().filter(|e| e.state == EntryState::Held).collect();
            if expect_in_belly {
                assert!(!p.exists(), "{point}");
                assert_eq!(held.len(), 1, "{point}");
                assert_eq!(fs::read_to_string(&held[0].stored_path).unwrap(), "data");
                assert_eq!(held[0].rewardable, expect_rewardable, "{point}");
            } else {
                assert_eq!(fs::read_to_string(&p).unwrap(), "data", "{point}");
                assert!(held.is_empty(), "{point}");
            }
        }
    }

    #[test]
    fn restore_crash_matrix() {
        for (point, expect_restored) in [("restore_prepared", false), ("restore_moved", true)] {
            let s = Sandbox::new();
            let p = s.file("docs/a.txt", "data");
            let mut v = vault(&s);
            let e = v.swallow(&s.policy, &preview(&s, &p), "f").unwrap();
            v.crash_at = Some(point);
            assert!(v.restore(&s.policy, &e.id).unwrap_err().undetermined);
            drop(v);
            let v = vault(&s);
            assert!(v.attention().is_empty(), "{point}");
            assert_eq!(p.exists(), expect_restored, "{point}");
            assert_eq!(Path::new(&e.stored_path).exists(), !expect_restored, "{point}");
            let state = &v.entries()[0].state;
            assert_eq!(*state == EntryState::Restored, expect_restored, "{point}");
        }
    }

    #[test]
    fn ambiguous_crash_keeps_both_copies_and_asks_for_attention() {
        let s = Sandbox::new();
        let p = s.file("docs/a.txt", "data");
        let mut v = vault(&s);
        v.crash_at = Some("moved");
        let e = v.swallow(&s.policy, &preview(&s, &p), "f").unwrap_err();
        assert!(e.undetermined);
        drop(v);
        fs::write(&p, "a new file appeared").unwrap();
        let v = vault(&s);
        assert_eq!(v.attention().len(), 1);
        assert_eq!(v.attention()[0].note, "both_exist");
        assert_eq!(fs::read_to_string(&p).unwrap(), "a new file appeared");
        let stored = &v.attention()[0].to;
        assert_eq!(fs::read_to_string(stored).unwrap(), "data");
        // Parked transactions stay parked across restarts.
        drop(v);
        assert_eq!(vault(&s).attention().len(), 1);
    }

    #[test]
    fn corrupt_index_fails_closed() {
        let s = Sandbox::new();
        let p = s.file("docs/a.txt", "data");
        let mut v = vault(&s);
        let e = v.swallow(&s.policy, &preview(&s, &p), "f").unwrap();
        drop(v);
        let index = s.policy.data_root.join("belly_index.json");
        for n in 0..=INDEX_BACKUPS {
            let path = if n == 0 { index.clone() } else { save::backup_path(&index, n) };
            if path.exists() {
                fs::write(&path, "{ broken").unwrap();
            }
        }
        let err = BellyVault::open(&s.policy.data_root).err().unwrap();
        assert_eq!(err.code, "index_corrupt");
        assert!(Path::new(&e.stored_path).exists());
    }

    #[test]
    fn second_instance_is_refused() {
        let s = Sandbox::new();
        let _v = vault(&s);
        assert_eq!(BellyVault::open(&s.policy.data_root).err().unwrap().code, "already_running");
    }
}
