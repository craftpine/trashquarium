//! Offline shell hunt. Rust owns the clock, collisions and reward eligibility.
use serde::{Deserialize, Serialize};
use super::Failure;

pub const PIVOT: (f64, f64) = (0.5, 0.14);
const SWING_LIMIT: f64 = 65.0;
pub const RARE_CHANCE: f64 = 0.08;
pub const PEARL_CHANCE_IF_RARE: f64 = 0.35;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ShellKind { #[default] Great, Queen, Variegated }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectionEntry { pub count: u64, pub rare_count: u64, pub first_found: String }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CatchReceipt {
    pub shell_id: String, pub kind: ShellKind, pub rare: bool,
    pub pearl: bool, pub first_of_kind: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HuntBalance {
    pub wait_min_ms: u64,
    pub wait_max_ms: u64,
    pub daily_cap: u64,
}
impl Default for HuntBalance {
    fn default() -> Self { Self { wait_min_ms: 480_000, wait_max_ms: 1_320_000, daily_cap: 60 } }
}
impl HuntBalance {
    pub fn validate(&self) -> Result<(), String> {
        if self.wait_min_ms == 0 || self.wait_max_ms < self.wait_min_ms || self.wait_max_ms > 86_400_000 || self.daily_cap == 0 {
            return Err("invalid shell hunt balance".into());
        }
        Ok(())
    }
    pub fn next_wait(&self) -> u64 {
        // Average of two independent samples: irregular waits around the middle.
        self.wait_min_ms + ((self.wait_max_ms - self.wait_min_ms) as f64 * (random() + random()) / 2.0) as u64
    }
}
/// Shell area: scattered anywhere the claw can reach (not in rows), kept apart so one
/// grab never touches two shells.
const SHELL_X: (f64, f64) = (0.06, 0.94);
const SHELL_Y: (f64, f64) = (0.55, 0.92);
const SHELL_GAP: f64 = 0.08;
const SHELL_MAX_REACH: f64 = 0.84; // claw stops at 0.88
const SHELL_MAX_ANGLE: f64 = 60.0; // claw swings ±65°

fn reachable(x: f64, y: f64) -> bool {
    let (dx, dy) = (x - PIVOT.0, y - PIVOT.1);
    dx.hypot(dy) <= SHELL_MAX_REACH && dx.atan2(dy).to_degrees().abs() <= SHELL_MAX_ANGLE
}

fn scatter_spot(placed: &[(f64, f64)]) -> (f64, f64) {
    let mut best = (0.5, 0.75);
    let mut best_gap = -1.0;
    for _ in 0..200 {
        let angle = ((random() * 2.0 - 1.0) * SHELL_MAX_ANGLE).to_radians();
        let reach = 0.42 + random() * (SHELL_MAX_REACH - 0.42);
        let (x, y) = (PIVOT.0 + angle.sin() * reach, PIVOT.1 + angle.cos() * reach);
        if !(SHELL_X.0..=SHELL_X.1).contains(&x) || !(SHELL_Y.0..=SHELL_Y.1).contains(&y) || !reachable(x, y) { continue; }
        let gap = placed.iter().map(|(px, py)| (px - x).hypot(py - y)).fold(f64::INFINITY, f64::min);
        if gap >= SHELL_GAP { return (x, y); }
        if gap > best_gap { best = (x, y); best_gap = gap; }
    }
    best // crowded batch: the most spread-out spot found
}

fn random() -> f64 {
    let b = *uuid::Uuid::new_v4().as_bytes();
    u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64 / (u32::MAX as f64 + 1.0)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shell {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub size: u8,
    pub collected: bool,
    #[serde(default)] pub kind: ShellKind,
    #[serde(default)] pub rare: bool,
    #[serde(default)] pub pearl: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Batch { pub id: String, pub shells: Vec<Shell> }
impl Batch {
    pub fn generate(count: usize) -> Self {
        let mut placed: Vec<(f64, f64)> = Vec::new();
        let shells = (0..count).map(|_| {
            let rare = random() < RARE_CHANCE;
            let (x, y) = scatter_spot(&placed);
            placed.push((x, y));
            Shell {
            id: uuid::Uuid::new_v4().to_string(),
            x,
            y,
            size: (random() * 3.0) as u8,
            collected: false,
            kind: match (random() * 3.0) as u8 { 0 => ShellKind::Great, 1 => ShellKind::Queen, _ => ShellKind::Variegated },
            rare,
            // Rolled once at spawn, not on catch/retry/restart.
            pearl: rare && random() < PEARL_CHANCE_IF_RARE,
        }}).collect();
        Self { id: uuid::Uuid::new_v4().to_string(), shells }
    }
    pub fn remaining(&self) -> usize { self.shells.iter().filter(|s| !s.collected).count() }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HuntState {
    pub remaining_wait_ms: u64,
    pub tutorial_granted: bool,
    pub date: String,
    pub earned: u64,
    pub batch: Option<Batch>,
    #[serde(default)] pub collection: std::collections::BTreeMap<ShellKind, CollectionEntry>,
    #[serde(default)] pub last_catch: Option<CatchReceipt>,
}
impl Default for HuntState {
    fn default() -> Self {
        Self { remaining_wait_ms: HuntBalance::default().next_wait(), tutorial_granted: false, date: String::new(), earned: 0, batch: None, collection: Default::default(), last_catch: None }
    }
}
impl HuntState {
    pub fn roll_day(&mut self, today: &str) {
        if today > self.date.as_str() { self.date = today.into(); self.earned = 0; }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.remaining_wait_ms > 86_400_000 { return Err("invalid hunt countdown".into()); }
        if !self.date.is_empty() && chrono::NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").is_err() { return Err("invalid hunt date".into()); }
        for entry in self.collection.values() {
            if entry.count == 0 || entry.rare_count > entry.count || chrono::NaiveDate::parse_from_str(&entry.first_found, "%Y-%m-%d").is_err() { return Err("invalid shell collection".into()); }
        }
        if let Some(receipt) = &self.last_catch {
            if receipt.shell_id.is_empty() || (receipt.pearl && !receipt.rare) || !self.collection.contains_key(&receipt.kind) { return Err("invalid catch receipt".into()); }
        }
        if let Some(b) = &self.batch {
            let mut ids = std::collections::BTreeSet::new();
            if b.id.is_empty() || b.shells.is_empty() || b.shells.len() > 10 { return Err("invalid shell batch".into()); }
            for s in &b.shells {
                if s.id.is_empty() || !ids.insert(&s.id) || !s.x.is_finite() || !s.y.is_finite() || !(SHELL_X.0..=SHELL_X.1).contains(&s.x) || !(SHELL_Y.0..=SHELL_Y.1).contains(&s.y) || s.size > 2 || (s.pearl && !s.rare) {
                    return Err("invalid shell".into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct HuntSession {
    pub id: String,
    pub batch_id: String,
    pub phase: String,
    pub angle: f64,
    pub length: f64,
    pub caught_id: Option<String>,
    pub paused: bool,
    pub error: Option<String>,
    #[serde(skip)] pub last_command: u64,
    #[serde(skip)] swing_ms: f64,
}
impl HuntSession {
    pub fn new(batch: &Batch) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), batch_id: batch.id.clone(), phase: "swinging".into(), angle: -65.0, length: 0.0, caught_id: None, paused: false, error: None, last_command: 0, swing_ms: 0.0 }
    }
    pub fn action(&mut self, id: &str, seq: u64, action: &str) -> Result<(), Failure> {
        if id != self.id { return Err(Failure::new("hunt_stale", "session changed")); }
        if seq <= self.last_command { return Ok(()); }
        match action {
            "drop" if self.phase == "swinging" && !self.paused => { self.phase = "extending".into(); self.caught_id = None; }
            "pause" => self.paused = true,
            "resume" => self.paused = false,
            _ => return Err(Failure::new("hunt_busy", "action unavailable")),
        }
        self.last_command = seq;
        Ok(())
    }
    pub fn tip(&self) -> (f64, f64) {
        let angle = self.angle.to_radians();
        (PIVOT.0 + angle.sin() * self.length, PIVOT.1 + angle.cos() * self.length)
    }
    pub fn step(&mut self, seconds: f64, batch: &Batch) {
        if self.paused { return; }
        // Fixed substeps prevent tunnelling, including when the webview renders slowly.
        let steps = (seconds * 120.0).ceil().max(1.0) as usize;
        let dt = seconds / steps as f64;
        for _ in 0..steps {
            match self.phase.as_str() {
                "swinging" => {
                    self.swing_ms += dt * 40.0;
                    let t = self.swing_ms % (4.0 * SWING_LIMIT);
                    self.angle = if t <= 2.0 * SWING_LIMIT { -SWING_LIMIT + t } else { 3.0 * SWING_LIMIT - t };
                }
                "extending" => {
                    self.length += dt * 0.55;
                    let (x, y) = self.tip();
                    if let Some(s) = batch.shells.iter().filter(|s| !s.collected).find(|s| (s.x - x).hypot(s.y - y) <= 0.026) {
                        self.caught_id = Some(s.id.clone()); self.phase = "retracting".into();
                    } else if self.length >= 0.88 || !(0.02..=0.98).contains(&x) || y >= 0.96 {
                        self.phase = "retracting".into();
                    }
                }
                "retracting" => {
                    let weight = self.caught_id.as_ref().and_then(|id| batch.shells.iter().find(|s| &s.id == id)).map(|s| [0.9, 0.75, 0.6][s.size as usize]).unwrap_or(1.0);
                    self.length = (self.length - dt * 0.7 * weight).max(0.0);
                    if self.length == 0.0 { self.phase = if self.caught_id.is_some() { "settling" } else { "swinging" }.into(); }
                }
                _ => {}
            }
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct HuntView {
    pub pearls: u64,
    pub collection: std::collections::BTreeMap<ShellKind, CollectionEntry>,
    pub last_catch: Option<CatchReceipt>,
    pub batch: Option<Batch>,
    pub earned: u64,
    pub daily_cap: u64,
    pub session: Option<HuntSession>,
    pub waiting: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn generated_shells_are_reachable_and_valid() {
        for n in 1..=10 {
            let b = Batch::generate(n);
            for s in &b.shells {
                let dx = s.x - PIVOT.0; let dy = s.y - PIVOT.1;
                assert!(dx.hypot(dy) < 0.88 && dx.atan2(dy).to_degrees().abs() < 65.0);
            }
            for (i, a) in b.shells.iter().enumerate() {
                for c in &b.shells[i + 1..] { assert!((a.x - c.x).hypot(a.y - c.y) > 0.052, "two shells inside one grab"); }
            }
            let state = HuntState { batch: Some(b), ..HuntState::default() };
            assert!(state.validate().is_ok());
        }
    }
    #[test] fn shells_are_scattered_not_in_rows() {
        let mut rows = std::collections::BTreeSet::new();
        for _ in 0..20 { for s in &Batch::generate(10).shells { rows.insert((s.y * 100.0).round() as i64); } }
        assert!(rows.len() > 15, "shell heights should vary, got {rows:?}");
    }
    #[test] fn a_catch_requires_retraction_and_replayed_drop_is_ignored() {
        let b = Batch { id: "batch".into(), shells: vec![Shell { id: "s".into(), x: 0.5, y: 0.75, size: 0, collected: false, kind: ShellKind::Great, rare: false, pearl: false }] };
        let mut s = HuntSession::new(&b); s.angle = 0.0;
        let id = s.id.clone(); s.action(&id, 1, "drop").unwrap();
        s.step(1.15, &b); assert_eq!(s.caught_id.as_deref(), Some("s")); assert_eq!(s.phase, "retracting");
        s.action(&id, 1, "drop").unwrap();
        s.step(2.0, &b); assert_eq!(s.phase, "settling");
        assert!(!b.shells[0].collected);
    }
    #[test] fn pause_and_clock_rollback_do_not_advance_progress() {
        let b = Batch::generate(1); let mut s = HuntSession::new(&b); s.paused = true;
        s.step(1.0, &b); assert_eq!(s.angle, -65.0);
        let mut state = HuntState::default(); state.roll_day("2026-10-03"); state.earned = 60;
        state.roll_day("2026-10-02"); assert_eq!(state.earned, 60);
        state.roll_day("2026-10-04"); assert_eq!(state.earned, 0);
    }
    #[test] fn spawn_metadata_roundtrips_without_reroll() {
        for _ in 0..20 {
            let batch = Batch::generate(10);
            assert!(batch.shells.iter().all(|s| !s.pearl || s.rare));
            let saved = serde_json::to_string(&batch).unwrap();
            let loaded: Batch = serde_json::from_str(&saved).unwrap();
            assert_eq!(batch.id, loaded.id);
            for (before, after) in batch.shells.iter().zip(&loaded.shells) {
                assert_eq!((&before.id, before.kind, before.rare, before.pearl, before.size, before.collected), (&after.id, after.kind, after.rare, after.pearl, after.size, after.collected));
                assert!((before.x-after.x).abs() < 1e-12 && (before.y-after.y).abs() < 1e-12);
            }
        }
    }
    #[test] fn invalid_pearl_or_collection_is_rejected() {
        let mut state = HuntState { batch: Some(Batch::generate(1)), ..HuntState::default() };
        let shell = &mut state.batch.as_mut().unwrap().shells[0]; shell.rare = false; shell.pearl = true;
        assert!(state.validate().is_err());
        state.batch = None;
        state.collection.insert(ShellKind::Great, CollectionEntry { count: 1, rare_count: 2, first_found: "2026-10-05".into() });
        assert!(state.validate().is_err());
    }
}
