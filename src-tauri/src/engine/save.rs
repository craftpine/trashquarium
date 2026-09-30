//! Atomic JSON persistence with rotating backups, for TrashQuarium's own files.
//!
//! Write: rotate `.bak2 → .bak3`, `.bak1 → .bak2`, copy main → `.bak1`, then
//! atomically rename the new file over main. Main is never missing.
//! Load: main, then each backup. A file from a newer schema is never
//! overwritten; if every copy is unreadable nothing is reset.

use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::fsops;

pub fn backup_path(path: &Path, n: usize) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bak{n}"));
    path.with_file_name(name)
}

pub fn write_with_backups<T: Serialize>(path: &Path, value: &T, backups: usize) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    if backups > 0 && path.exists() {
        for n in (1..backups).rev() {
            let from = backup_path(path, n);
            if from.exists() {
                fs::rename(&from, backup_path(path, n + 1))?;
            }
        }
        fs::copy(path, backup_path(path, 1))?;
    }
    fsops::write_atomic(path, &bytes)
}

#[derive(Debug)]
pub enum Loaded<T> {
    /// No file and no backups: a genuinely new profile.
    Missing,
    Ok { value: T, from_backup: Option<usize> },
    /// Written by a newer version. Open read-only; never overwrite.
    Future { version: u64 },
    /// Files exist but none is valid. Preserve them; never reset silently.
    Corrupt { reason: String },
}

/// `parse` receives the raw JSON and returns the validated value or a reason.
pub fn load_with_backups<T>(
    path: &Path,
    backups: usize,
    current_version: u64,
    parse: impl Fn(Value) -> Result<T, String>,
) -> Loaded<T> {
    let mut any_exists = false;
    let mut last_reason = String::new();
    for n in 0..=backups {
        let candidate = if n == 0 { path.to_path_buf() } else { backup_path(path, n) };
        let Ok(text) = fs::read_to_string(&candidate) else {
            if candidate.exists() {
                any_exists = true;
                last_reason = format!("{}: unreadable", candidate.display());
            }
            continue;
        };
        any_exists = true;
        let value: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                last_reason = format!("{}: {e}", candidate.display());
                continue;
            }
        };
        let version = value.get("schema_version").and_then(Value::as_u64).unwrap_or(0);
        if version > current_version {
            return Loaded::Future { version };
        }
        match parse(value) {
            Ok(v) => return Loaded::Ok { value: v, from_backup: (n > 0).then_some(n) },
            Err(e) => last_reason = format!("{}: {e}", candidate.display()),
        }
    }
    if any_exists {
        Loaded::Corrupt { reason: last_reason }
    } else {
        Loaded::Missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(v: Value) -> Result<i64, String> {
        v.get("n").and_then(Value::as_i64).ok_or_else(|| "missing n".into())
    }

    #[test]
    fn roundtrip_and_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.json");
        for n in 1..=5 {
            write_with_backups(&p, &json!({"schema_version": 1, "n": n}), 3).unwrap();
        }
        assert!(matches!(load_with_backups(&p, 3, 1, parse), Loaded::Ok { value: 5, from_backup: None }));
        let bak3: Value = serde_json::from_str(&fs::read_to_string(backup_path(&p, 3)).unwrap()).unwrap();
        assert_eq!(bak3["n"], 2);
    }

    #[test]
    fn corrupt_main_falls_back_to_backup() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.json");
        write_with_backups(&p, &json!({"schema_version": 1, "n": 1}), 3).unwrap();
        write_with_backups(&p, &json!({"schema_version": 1, "n": 2}), 3).unwrap();
        fs::write(&p, "{ not json").unwrap();
        assert!(matches!(load_with_backups(&p, 3, 1, parse), Loaded::Ok { value: 1, from_backup: Some(1) }));
    }

    #[test]
    fn all_corrupt_is_reported_and_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.json");
        fs::write(&p, "garbage").unwrap();
        fs::write(backup_path(&p, 1), r#"{"schema_version":1}"#).unwrap();
        assert!(matches!(load_with_backups(&p, 3, 1, parse), Loaded::Corrupt { .. }));
        assert_eq!(fs::read_to_string(&p).unwrap(), "garbage");
    }

    #[test]
    fn future_schema_is_not_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.json");
        fs::write(&p, r#"{"schema_version": 9, "n": 1}"#).unwrap();
        assert!(matches!(load_with_backups(&p, 3, 1, parse), Loaded::Future { version: 9 }));
    }

    #[test]
    fn nothing_on_disk_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(load_with_backups(&dir.path().join("s.json"), 3, 1, parse), Loaded::Missing));
    }
}
