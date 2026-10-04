//! Offline shell hunt. Rust owns the clock, collisions and reward eligibility.
use serde::{Deserialize, Serialize};
use super::Failure;

pub const PIVOT: (f64, f64) = (0.5, 0.14);
const SWING_LIMIT: f64 = 65.0;

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
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Batch { pub id: String, pub shells: Vec<Shell> }
impl Batch {
    pub fn generate(count: usize) -> Self {
        let shells = (0..count).map(|i| Shell {
            id: uuid::Uuid::new_v4().to_string(),
            x: 0.13 + (i % 5) as f64 * 0.185 + (random() - 0.5) * 0.035,
            y: 0.70 + (i / 5) as f64 * 0.14 + random() * 0.025,
            size: (random() * 3.0) as u8,
            collected: false,
        }).collect();
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
}
impl Default for HuntState {
    fn default() -> Self {
        Self { remaining_wait_ms: HuntBalance::default().next_wait(), tutorial_granted: false, date: String::new(), earned: 0, batch: None }
    }
}
impl HuntState {
    pub fn roll_day(&mut self, today: &str) {
        if today > self.date.as_str() { self.date = today.into(); self.earned = 0; }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.remaining_wait_ms > 86_400_000 { return Err("invalid hunt countdown".into()); }
        if !self.date.is_empty() && chrono::NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").is_err() { return Err("invalid hunt date".into()); }
        if let Some(b) = &self.batch {
            let mut ids = std::collections::BTreeSet::new();
            if b.id.is_empty() || b.shells.is_empty() || b.shells.len() > 10 { return Err("invalid shell batch".into()); }
            for s in &b.shells {
                if s.id.is_empty() || !ids.insert(&s.id) || !s.x.is_finite() || !s.y.is_finite() || !(0.10..=0.90).contains(&s.x) || !(0.65..=0.88).contains(&s.y) || s.size > 2 {
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
            let state = HuntState { batch: Some(b), ..HuntState::default() };
            assert!(state.validate().is_ok());
        }
    }
    #[test] fn a_catch_requires_retraction_and_replayed_drop_is_ignored() {
        let b = Batch { id: "batch".into(), shells: vec![Shell { id: "s".into(), x: 0.5, y: 0.75, size: 0, collected: false }] };
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
}
