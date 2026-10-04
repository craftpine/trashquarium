//! Platform-neutral game core. Nothing in here depends on Tauri, so the whole
//! file-safety and economy contract is covered by `cargo test`.

pub mod app;
pub mod belly;
pub mod catalog;
pub mod fsops;
pub mod game;
pub mod guard;
pub mod hunt;
pub mod save;

/// Error returned to the UI. `code` is stable and localised by the front end.
#[derive(serde::Serialize, Clone, Debug)]
pub struct Failure {
    pub code: String,
    pub detail: String,
    /// The outcome is unknown (e.g. a file may be in either place); the UI must
    /// say "not yet determined, checking the Belly" instead of guessing.
    pub undetermined: bool,
}

impl Failure {
    pub fn new(code: &str, detail: impl ToString) -> Self {
        Failure { code: code.into(), detail: detail.to_string(), undetermined: false }
    }
    pub fn undetermined(code: &str, detail: impl ToString) -> Self {
        Failure { code: code.into(), detail: detail.to_string(), undetermined: true }
    }
}

pub fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Local calendar day, used for daily caps.
pub fn local_today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
