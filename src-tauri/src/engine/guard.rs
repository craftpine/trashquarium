//! FileGuard: decides whether a user-selected file may go into the Belly.
//!
//! Fail-closed: anything that cannot be checked is refused. Inspection never
//! modifies the file; duplicate protection reads the whole file in bounded chunks.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use super::fsops;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Doc,
    Media,
    Tech,
}

const DOC: &[&str] = &["txt", "md", "rtf", "doc", "docx", "pdf", "xls", "xlsx", "csv", "ppt", "pptx", "odt"];
const MEDIA: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "heic", "mp4", "mov", "mkv", "avi", "mp3", "wav", "flac", "m4a",
];
const TECH: &[&str] = &["zip", "rar", "7z", "log", "tmp", "bak", "old", "cache", "dmp"];
/// Installers are only accepted from the real Downloads folder.
const INSTALLERS: &[&str] = &["exe", "msi", "dmg", "pkg"];
/// Files inside these packages belong to an app or library (e.g. Photos) and must stay put.
const PACKAGES: &[&str] = &[
    "app", "bundle", "framework", "plugin", "kext", "photoslibrary", "musiclibrary", "tvlibrary",
    "aplibrary", "fcpbundle", "logicx", "xcodeproj", "xcworkspace", "sparsebundle", "lrdata", "lrlibrary",
];
const REPO_MARKERS: &[&str] = &[".git", ".svn", ".hg"];

pub fn category_for(ext: &str) -> Option<Category> {
    let ext = ext.to_ascii_lowercase();
    if DOC.contains(&ext.as_str()) {
        Some(Category::Doc)
    } else if MEDIA.contains(&ext.as_str()) {
        Some(Category::Media)
    } else if TECH.contains(&ext.as_str()) || INSTALLERS.contains(&ext.as_str()) {
        Some(Category::Tech)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct GuardPolicy {
    /// System and application locations that are never touched.
    pub blocked_roots: Vec<PathBuf>,
    /// TrashQuarium's own data folder (saves, Belly).
    pub data_root: PathBuf,
    /// The OS-reported Downloads folder, if any.
    pub downloads: Option<PathBuf>,
    pub max_bytes: u64,
    pub fingerprint_bytes: u64,
}

impl GuardPolicy {
    pub fn for_system(data_root: PathBuf, max_bytes: u64, fingerprint_bytes: u64) -> Self {
        GuardPolicy {
            blocked_roots: system_blocked_roots(),
            data_root,
            downloads: dirs::download_dir(),
            max_bytes,
            fingerprint_bytes,
        }
    }
}

fn system_blocked_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    for key in ["WINDIR", "ProgramFiles", "ProgramFiles(x86)", "ProgramW6432", "ProgramData", "APPDATA", "LOCALAPPDATA"] {
        if let Some(v) = std::env::var_os(key).filter(|v| !v.is_empty()) {
            roots.push(PathBuf::from(v));
        }
    }
    #[cfg(target_os = "macos")]
    {
        for r in ["/System", "/Library", "/Applications", "/usr", "/bin", "/sbin", "/private", "/opt", "/cores", "/dev", "/etc", "/var", "/tmp", "/Volumes/Recovery"] {
            roots.push(r.into());
        }
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Library"));
            roots.push(home.join("Applications"));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for r in ["/bin", "/boot", "/dev", "/etc", "/lib", "/lib64", "/opt", "/proc", "/run", "/sbin", "/snap", "/sys", "/usr", "/var"] {
            roots.push(r.into());
        }
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join(".config"));
            roots.push(home.join(".local"));
        }
    }
    roots
}

/// Result of inspecting one file. `code` is `ok` or a refusal reason code that
/// the UI turns into a sentence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Inspection {
    pub path: String,
    pub name: String,
    pub ok: bool,
    pub code: String,
    pub size: u64,
    pub category: Option<Category>,
    pub modified_unix: i64,
    pub fingerprint: Option<String>,
    #[serde(default)] pub legacy_fingerprint: Option<String>,
}

impl Inspection {
    fn deny(path: &Path, code: &str) -> Self {
        Inspection {
            path: path.to_string_lossy().into_owned(),
            name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            ok: false,
            code: code.into(),
            size: 0,
            category: None,
            modified_unix: 0,
            fingerprint: None,
            legacy_fingerprint: None,
        }
    }
}

pub fn inspect(policy: &GuardPolicy, path: &Path) -> Inspection {
    let original = path;
    if path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Inspection::deny(path, "parent_ref");
    }
    // Rust canonicalize produces a verbatim local-drive prefix on Windows.
    // Only normalize that specific prefix; UNC and device namespaces remain denied.
    let normalized = local_disk_path(path);
    let path = normalized.as_path();
    match check(policy, path) {
        Ok(mut ok) => { ok.path = original.to_string_lossy().into_owned(); ok },
        Err(code) => {
            let mut denied = Inspection::deny(path, code);
            if let Ok(meta) = fs::symlink_metadata(path) {
                if meta.is_file() {
                    denied.size = meta.len(); // shown to the user; the file is still refused
                }
            }
            denied
        }
    }
}

fn local_disk_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    if let Some(Component::Prefix(p)) = path.components().next() {
        if matches!(p.kind(), std::path::Prefix::VerbatimDisk(_)) {
            if let Some(text) = path.to_str().and_then(|s| s.strip_prefix("\\\\?\\")) { return PathBuf::from(text); }
        }
    }
    path.to_path_buf()
}

fn check(policy: &GuardPolicy, path: &Path) -> Result<Inspection, &'static str> {
    check_textual(path)?;
    let meta = fs::symlink_metadata(path).map_err(|_| "not_found")?;
    if fsops::is_link(&meta) {
        return Err("link");
    }
    if meta.is_dir() {
        return Err("directory");
    }
    if !meta.is_file() {
        return Err("not_regular");
    }
    check_location(policy, path)?;
    if let Some(code) = fsops::attribute_block(path, &meta) {
        return Err(code);
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let category = category_for(&ext).ok_or("extension")?;
    if INSTALLERS.contains(&ext.as_str()) && !policy.downloads.as_deref().is_some_and(|d| is_inside_folder(path, d)) {
        return Err("installer_outside_downloads");
    }
    let size = meta.len();
    if size > policy.max_bytes {
        return Err("too_large");
    }
    let file_volume = fsops::volume_id(path).map_err(|_| "unreadable")?;
    let belly_volume = fsops::volume_id(&policy.data_root).map_err(|_| "unreadable")?;
    if file_volume != belly_volume {
        return Err("cross_volume");
    }
    if fsops::is_locked(path).map_err(|_| "unreadable")? {
        return Err("locked");
    }
    let fingerprint = fingerprint(path, size, policy.fingerprint_bytes).map_err(|_| "unreadable")?;
    let modified_unix = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Ok(Inspection {
        path: path.to_string_lossy().into_owned(),
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        ok: true,
        code: "ok".into(),
        size,
        category: Some(category),
        modified_unix,
        fingerprint: Some(fingerprint),
        legacy_fingerprint: Some(legacy_fingerprint(path, size, policy.fingerprint_bytes).map_err(|_| "unreadable")?),
    })
}

/// Rejects path shapes that are unsafe before touching the filesystem.
fn check_textual(path: &Path) -> Result<(), &'static str> {
    let text = path.to_string_lossy();
    if cfg!(windows) && (text.starts_with("\\\\") || text.starts_with("//")) {
        return Err("device_path"); // UNC shares and \\?\ / \\.\ device paths
    }
    if !path.is_absolute() {
        return Err("not_absolute");
    }
    if cfg!(windows) && text.char_indices().any(|(i, c)| c == ':' && i != 1) {
        return Err("alternate_stream");
    }
    if path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Err("parent_ref");
    }
    Ok(())
}

/// Location checks shared by intake and restore.
pub fn check_location(policy: &GuardPolicy, path: &Path) -> Result<(), &'static str> {
    // A file directly in a drive/volume root.
    if path.parent().is_some_and(|p| p.parent().is_none()) {
        return Err("drive_root");
    }
    if fsops::is_within(path, &policy.data_root) {
        return Err("app_data");
    }
    if policy.blocked_roots.iter().any(|r| fsops::is_within(path, r)) {
        return Err("blocked_location");
    }
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d.parent().is_none() {
            break;
        }
        let meta = fs::symlink_metadata(d).map_err(|_| "unreadable")?;
        if fsops::is_link(&meta) {
            return Err("link");
        }
        if d.extension().and_then(|e| e.to_str()).is_some_and(|e| PACKAGES.contains(&e.to_ascii_lowercase().as_str())) {
            return Err("package");
        }
        if REPO_MARKERS.iter().any(|m| fs::symlink_metadata(d.join(m)).is_ok()) {
            return Err("repository");
        }
        dir = d.parent();
    }
    Ok(())
}

fn is_inside_folder(path: &Path, dir: &Path) -> bool {
    fsops::is_within(path, dir) && fsops::path_key(path) != fsops::path_key(dir)
}

/// SHA-256 of size plus all bytes; filenames/locations do not affect duplicates.
pub fn fingerprint(path: &Path, size: u64, _limit: u64) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(size.to_le_bytes());
    let mut reader = File::open(path)?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("sha256:{hex}"))
}

/// Compatibility only: never add new prefix hashes to the paid ledger.
pub fn legacy_fingerprint(path: &Path, size: u64, limit: u64) -> std::io::Result<String> {
    let mut hasher = Sha256::new(); hasher.update(size.to_le_bytes());
    let mut reader = File::open(path)?.take(limit); let mut buf = [0u8; 64*1024];
    loop { let n = reader.read(&mut buf)?; if n == 0 { break; } hasher.update(&buf[..n]); }
    let hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("p1m:{hex}"))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs;

    /// A sandbox with its own data root, Downloads and blocked folder, all on
    /// the same volume. Paths are canonicalised so macOS `/var → /private/var`
    /// links do not trip the link check.
    pub struct Sandbox {
        _dir: tempfile::TempDir,
        pub root: PathBuf,
        pub policy: GuardPolicy,
    }

    impl Sandbox {
        pub fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = fs::canonicalize(dir.path()).unwrap();
            for d in ["data", "Downloads", "blocked", "docs"] {
                fs::create_dir_all(root.join(d)).unwrap();
            }
            let policy = GuardPolicy {
                blocked_roots: vec![root.join("blocked")],
                data_root: root.join("data"),
                downloads: Some(root.join("Downloads")),
                max_bytes: 1024,
                fingerprint_bytes: 16,
            };
            Sandbox { _dir: dir, root, policy }
        }

        pub fn file(&self, rel: &str, body: &str) -> PathBuf {
            let p = self.root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, body).unwrap();
            p
        }

        pub fn code(&self, p: &Path) -> String {
            inspect(&self.policy, p).code
        }
    }

    #[test]
    fn accepts_ordinary_document() {
        let s = Sandbox::new();
        let p = s.file("docs/old-notes.txt", "hello");
        let r = inspect(&s.policy, &p);
        assert!(r.ok, "{r:?}");
        assert_eq!(r.category, Some(Category::Doc));
        assert_eq!(r.size, 5);
        assert!(r.fingerprint.unwrap().starts_with("sha256:"));
    }

    #[test]
    fn refuses_unsafe_shapes() {
        let s = Sandbox::new();
        assert_eq!(s.code(Path::new("relative.txt")), "not_absolute");
        assert_eq!(s.code(&local_disk_path(&s.root).join("docs/../docs/x.txt")), "parent_ref");
        assert_eq!(s.code(&s.root.join("docs/missing.txt")), "not_found");
        assert_eq!(s.code(&s.root.join("docs")), "directory");
    }

    #[cfg(windows)]
    #[test]
    fn canonical_drive_paths_work_but_network_device_and_ads_stay_denied() {
        let s=Sandbox::new();let file=s.file("docs/canonical.txt","test");
        assert!(inspect(&s.policy,&fs::canonicalize(&file).unwrap()).ok);
        for path in [r"\\server\share\a.txt",r"\\?\UNC\server\share\a.txt",r"\\.\C:\docs\a.txt",r"\\?\GLOBALROOT\Device\HarddiskVolume1\a.txt"] {
            assert_eq!(s.code(Path::new(path)),"device_path");
        }
        assert_eq!(s.code(&PathBuf::from(format!("{}:stream",local_disk_path(&file).display()))),"alternate_stream");
    }

    #[test]
    fn refuses_protected_locations() {
        let s = Sandbox::new();
        assert_eq!(s.code(&s.file("blocked/a.txt", "x")), "blocked_location");
        assert_eq!(s.code(&s.file("data/save.txt", "x")), "app_data");
        assert_eq!(s.code(&s.file("Pictures/Photos Library.photoslibrary/originals/a.jpg", "x")), "package");
        fs::create_dir_all(s.root.join("project/.git")).unwrap();
        assert_eq!(s.code(&s.file("project/src/notes.txt", "x")), "repository");
    }

    #[test]
    fn refuses_hidden_bad_extension_and_large() {
        let s = Sandbox::new();
        assert_eq!(s.code(&s.file("docs/.secret.txt", "x")), "hidden");
        assert_eq!(s.code(&s.file("docs/script.sh", "x")), "extension");
        assert_eq!(s.code(&s.file("docs/big.txt", &"x".repeat(2048))), "too_large");
    }

    #[test]
    fn installers_only_in_real_downloads() {
        let s = Sandbox::new();
        assert_eq!(s.code(&s.file("docs/setup.exe", "x")), "installer_outside_downloads");
        assert_eq!(s.code(&s.file("Downloads/setup.exe", "x")), "ok");
        // A folder merely *named* Downloads elsewhere does not count.
        assert_eq!(s.code(&s.file("docs/Downloads/setup.exe", "x")), "installer_outside_downloads");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_links_and_linked_parents() {
        let s = Sandbox::new();
        let target = s.file("docs/real.txt", "x");
        let link = s.root.join("docs/link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(s.code(&link), "link");
        std::os::unix::fs::symlink(s.root.join("docs"), s.root.join("linked-dir")).unwrap();
        assert_eq!(s.code(&s.root.join("linked-dir/real.txt")), "link");
    }

    #[test]
    fn fingerprint_reads_full_content_and_ignores_copy_name() {
        let s = Sandbox::new();
        let a = s.file("docs/a.txt", "0123456789abcdefTAIL-A");
        let b = s.file("docs/b.txt", "0123456789abcdefTAIL-B");
        let c = s.file("docs/c.txt", "0123456789abcdefTAIL-CC");
        let fp = |p: &Path| fingerprint(p, fs::metadata(p).unwrap().len(), 16).unwrap();
        assert_ne!(fp(&a), fp(&b));
        assert_ne!(fp(&a), fp(&c));
        let copy = s.file("docs/renamed.txt", "0123456789abcdefTAIL-A");
        assert_eq!(fp(&a), fp(&copy));
    }
}
