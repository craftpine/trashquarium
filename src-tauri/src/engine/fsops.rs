//! Low-level file operations.
//!
//! Nothing here deletes user data. A move never replaces an existing destination
//! and never falls back to copy-then-delete: a move across volumes simply fails.

use std::fs::{self, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Atomically renames `from` to `to`, failing if `to` already exists or the two
/// paths are on different volumes.
pub fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    imp::move_no_replace(from, to)
}

/// Identifier of the volume holding `path` (file or directory).
pub fn volume_id(path: &Path) -> io::Result<u64> {
    imp::volume_id(path)
}

/// True for symlinks and, on Windows, any reparse point (junctions, mount points…).
pub fn is_link(meta: &Metadata) -> bool {
    meta.file_type().is_symlink() || imp::is_reparse_point(meta)
}

/// Why a file must not be taken because of its attributes, if at all.
/// Returns one of `hidden`, `system`, `offline`.
pub fn attribute_block(path: &Path, meta: &Metadata) -> Option<&'static str> {
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
    {
        return Some("hidden");
    }
    imp::attribute_block(meta)
}

/// Whether another program holds the file open in a way that forbids moving it.
pub fn is_locked(path: &Path) -> io::Result<bool> {
    imp::is_locked(path)
}

/// Writes app-owned metadata atomically: temp file, flush to disk, rename over.
/// Only ever used for TrashQuarium's own JSON files, never for user files.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| io::Error::other("no parent directory"))?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("data");
    let temp = dir.join(format!(".{name}.tmp-{}", uuid::Uuid::new_v4().simple()));
    {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    if let Err(e) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp); // our own temp file, never user data
        return Err(e);
    }
    sync_dir(dir);
    Ok(())
}

/// Best-effort flush of a directory entry change (Unix only; no-op elsewhere).
pub fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// Case-folded, separator-normalised key for prefix comparisons. Windows and
/// default macOS volumes are case-insensitive, so compare case-insensitively there.
pub fn path_key(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let s = s.trim_end_matches('/').to_string();
    if cfg!(any(windows, target_os = "macos")) {
        s.to_lowercase()
    } else {
        s
    }
}

/// `path` equals `root` or lies inside it.
pub fn is_within(path: &Path, root: &Path) -> bool {
    let (p, r) = (path_key(path), path_key(root));
    !r.is_empty() && (p == r || p.starts_with(&(r + "/")))
}

/// `name.ext` → `name (restored N).ext`
pub fn restored_name(path: &Path, n: u32) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let name = match path.extension() {
        Some(ext) => format!("{stem} (restored {n}).{}", ext.to_string_lossy()),
        None => format!("{stem} (restored {n})"),
    };
    path.with_file_name(name)
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::ffi::CString;
    use std::os::macos::fs::MetadataExt;
    use std::os::unix::ffi::OsStrExt;

    const UF_HIDDEN: u32 = 0x0000_8000;
    const SF_DATALESS: u32 = 0x4000_0000;

    fn c(path: &Path) -> io::Result<CString> {
        CString::new(path.as_os_str().as_bytes()).map_err(|_| io::Error::other("path contains NUL"))
    }

    pub fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
        let (f, t) = (c(from)?, c(to)?);
        // SAFETY: both arguments are valid NUL-terminated C strings.
        if unsafe { libc::renamex_np(f.as_ptr(), t.as_ptr(), libc::RENAME_EXCL) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn volume_id(path: &Path) -> io::Result<u64> {
        Ok(fs::metadata(path)?.st_dev())
    }

    pub fn is_reparse_point(_: &Metadata) -> bool {
        false
    }

    pub fn attribute_block(meta: &Metadata) -> Option<&'static str> {
        let flags = meta.st_flags();
        if flags & SF_DATALESS != 0 {
            Some("offline") // iCloud placeholder: content is not on this Mac
        } else if flags & UF_HIDDEN != 0 {
            Some("hidden")
        } else {
            None
        }
    }

    pub fn is_locked(path: &Path) -> io::Result<bool> {
        // macOS has no mandatory locks; being able to open it is the check we can make.
        fs::File::open(path).map(|_| false)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;

    pub fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
        let f = CString::new(from.as_os_str().as_bytes()).map_err(|_| io::Error::other("NUL"))?;
        let t = CString::new(to.as_os_str().as_bytes()).map_err(|_| io::Error::other("NUL"))?;
        // SAFETY: valid C strings; AT_FDCWD resolves relative to cwd (paths are absolute).
        let r = unsafe {
            libc::renameat2(libc::AT_FDCWD, f.as_ptr(), libc::AT_FDCWD, t.as_ptr(), libc::RENAME_NOREPLACE)
        };
        if r == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }

    pub fn volume_id(path: &Path) -> io::Result<u64> {
        Ok(fs::metadata(path)?.dev())
    }

    pub fn is_reparse_point(_: &Metadata) -> bool {
        false
    }

    pub fn attribute_block(_: &Metadata) -> Option<&'static str> {
        None
    }

    pub fn is_locked(path: &Path) -> io::Result<bool> {
        fs::File::open(path).map(|_| false)
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, MoveFileExW, BY_HANDLE_FILE_INFORMATION, DELETE, FILE_ATTRIBUTE_HIDDEN,
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SYSTEM, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    const ERROR_SHARING_VIOLATION: i32 = 32;
    const ERROR_LOCK_VIOLATION: i32 = 33;

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
        // Flags 0: no MOVEFILE_REPLACE_EXISTING, no MOVEFILE_COPY_ALLOWED.
        // SAFETY: both buffers are NUL-terminated UTF-16 strings that outlive the call.
        if unsafe { MoveFileExW(wide(from).as_ptr(), wide(to).as_ptr(), 0) } != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn volume_id(path: &Path) -> io::Result<u64> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        // SAFETY: zeroed POD out-parameter; handle is valid for the call.
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(info.dwVolumeSerialNumber as u64)
    }

    pub fn is_reparse_point(meta: &Metadata) -> bool {
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    pub fn attribute_block(meta: &Metadata) -> Option<&'static str> {
        let a = meta.file_attributes();
        if a & (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS) != 0 {
            Some("offline")
        } else if a & FILE_ATTRIBUTE_SYSTEM != 0 {
            Some("system")
        } else if a & FILE_ATTRIBUTE_HIDDEN != 0 {
            Some("hidden")
        } else {
            None
        }
    }

    pub fn is_locked(path: &Path) -> io::Result<bool> {
        // Probe what the rename needs: DELETE access while sharing everything, so
        // only handles that would actually block the move count. Explorer and
        // media handlers hold videos open briefly (thumbnail, duration), so a
        // sharing violation is retried for a moment before calling it locked.
        for attempt in 0..5 {
            let probe = OpenOptions::new()
                .access_mode(DELETE)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
                .open(path);
            match probe {
                Ok(_) => return Ok(false),
                Err(e) if matches!(e.raw_os_error(), Some(ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION)) => {
                    if attempt < 4 {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_never_replaces_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        fs::write(&a, "A").unwrap();
        fs::write(&b, "B").unwrap();
        assert!(move_no_replace(&a, &b).is_err());
        assert_eq!(fs::read_to_string(&a).unwrap(), "A");
        assert_eq!(fs::read_to_string(&b).unwrap(), "B");
    }

    #[test]
    fn move_to_free_name_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        fs::write(&a, "A").unwrap();
        move_no_replace(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(fs::read_to_string(&b).unwrap(), "A");
    }

    #[test]
    fn restored_name_keeps_extension() {
        assert_eq!(restored_name(Path::new("/x/report.pdf"), 2), PathBuf::from("/x/report (restored 2).pdf"));
        assert_eq!(restored_name(Path::new("/x/README"), 1), PathBuf::from("/x/README (restored 1)"));
    }

    #[test]
    fn within_is_component_wise() {
        assert!(is_within(Path::new("/a/b/c"), Path::new("/a/b")));
        assert!(is_within(Path::new("/a/b"), Path::new("/a/b")));
        assert!(!is_within(Path::new("/a/bc"), Path::new("/a/b")));
    }
}
