//! Bundled, validated game data: the species roster and balance numbers.
//! Prices and roster live here, not in UI code.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};


const SPECIES_JSON: &str = include_str!("../../config/species.json");
const BALANCE_JSON: &str = include_str!("../../config/balance.json");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Species {
    pub id: String,
    pub name: String,
    pub scientific_name: String,
    pub class: String,
    pub breeding_group: String,
    pub price: u64,
    pub sprite: String,
    pub reproduction: String,
    pub stage_on_purchase: String,
    pub fact: String,
    pub source: String,
    pub editorial_status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpeciesFile {
    pub schema_version: u32,
    pub disclaimer: String,
    pub species: Vec<Species>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StageExp {
    pub juvenile: u64,
    pub adult: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Economy {
    pub welcome_shells: u64,
    pub base_shells: f64,
    pub base_exp: f64,
    pub daily_shell_cap: u64,
    pub daily_exp_cap: u64,
    pub diminish_step: f64,
    pub diminish_min: f64,
    pub category_multiplier: BTreeMap<String, f64>,
    pub size_log_divisor: f64,
    pub size_min: f64,
    pub size_max: f64,
    pub age_recent_days: f64,
    pub age_old_days: f64,
    pub age_recent_mult: f64,
    pub age_middle_mult: f64,
    pub age_old_mult: f64,
    pub max_fingerprints: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Balance {
    #[serde(default)]
    pub shell_hunt: super::hunt::HuntBalance,
    pub tank_capacity: usize,
    pub starter_species: String,
    pub max_preview_files: usize,
    pub max_file_bytes: u64,
    pub fingerprint_bytes: u64,
    pub stage_exp: StageExp,
    pub economy: Economy,
    #[serde(default)]
    pub breeding: Breeding,
    #[serde(default)]
    pub slots: Slots,
    #[serde(default)]
    pub savings: Savings,
}

/// Savings books (Chị Cua's bank). In-game CBCoin only. A book earns `daily_rate` per day
/// of its term (simple interest, fixed when the book is opened, at most `max_interest`).
/// Withdrawing before the term ends returns only the deposit.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Savings {
    pub daily_rate: f64,
    pub terms_days: Vec<u32>,
    pub min_deposit: u64,
    pub max_interest: u64,
    pub max_books: usize,
}

impl Default for Savings {
    fn default() -> Self {
        Savings { daily_rate: 0.09, terms_days: vec![1, 3, 7, 14, 30], min_deposit: 10, max_interest: 50_000, max_books: 10 }
    }
}

impl Savings {
    /// Interest a book of `principal` CBCoin earns over `days`, rounded down and capped.
    pub fn interest(&self, principal: u64, days: u32) -> u64 {
        let raw = (principal as f64 * self.daily_rate * days as f64).floor();
        if raw >= self.max_interest as f64 { self.max_interest } else { raw.max(0.0) as u64 }
    }
}

/// Tank space: `tank_capacity` counts slots, not fish. Bigger (pricier) fish take more:
/// below `medium_price` 1 slot, below `large_price` 2 slots, otherwise 3 slots.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Slots {
    pub medium_price: u64,
    pub large_price: u64,
}

impl Default for Slots {
    fn default() -> Self {
        Slots { medium_price: 100, large_price: 200 }
    }
}

impl Slots {
    pub fn for_price(&self, price: u64) -> usize {
        if price >= self.large_price { 3 } else if price >= self.medium_price { 2 } else { 1 }
    }
}

/// Breeding: each egg costs both parents one purchase price of sale value, and
/// hatches with a chance that falls linearly from `hatch_max` (cheapest fish)
/// to `hatch_min` (most expensive fish).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Breeding {
    pub hatch_max: f64,
    pub hatch_min: f64,
    pub price_low: u64,
    pub price_high: u64,
    /// Eggs incubate in the egg den for a random time in this range (seconds).
    pub incubate_min_s: i64,
    pub incubate_max_s: i64,
    /// Most eggs the den holds at once.
    pub den_capacity: usize,
}

impl Default for Breeding {
    fn default() -> Self {
        Breeding { hatch_max: 0.20, hatch_min: 0.05, price_low: 20, price_high: 300, incubate_min_s: 7200, incubate_max_s: 10800, den_capacity: 100 }
    }
}

impl Breeding {
    /// Hatch chance for one egg of a species priced `price` CBCoin.
    pub fn hatch_rate(&self, price: u64) -> f64 {
        if price <= self.price_low { return self.hatch_max; }
        if price >= self.price_high { return self.hatch_min; }
        let t = (price - self.price_low) as f64 / (self.price_high - self.price_low) as f64;
        self.hatch_max + (self.hatch_min - self.hatch_max) * t
    }
}

#[derive(Clone, Debug)]
pub struct Catalog {
    pub disclaimer: String,
    pub species: Vec<Species>,
    pub balance: Balance,
}

impl Catalog {
    pub fn bundled() -> Result<Self, String> {
        let species: SpeciesFile = serde_json::from_str(SPECIES_JSON).map_err(|e| format!("species.json: {e}"))?;
        let balance: Balance = serde_json::from_str(BALANCE_JSON).map_err(|e| format!("balance.json: {e}"))?;
        let catalog = Catalog { disclaimer: species.disclaimer, species: species.species, balance };
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn species(&self, id: &str) -> Option<&Species> {
        self.species.iter().find(|s| s.id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        let sv = &self.balance.savings;
        if !sv.daily_rate.is_finite() || sv.daily_rate < 0.0 || sv.terms_days.is_empty() || sv.terms_days.iter().any(|d| *d == 0) || sv.max_books == 0 {
            return Err("balance.json: invalid savings".into());
        }
        let mut ids = HashSet::new();
        for s in &self.species {
            if s.id.is_empty() || !ids.insert(&s.id) {
                return Err(format!("duplicate or empty species id {:?}", s.id));
            }
            if s.name.trim().is_empty() || s.scientific_name.trim().is_empty() {
                return Err(format!("{}: missing display or scientific name", s.id));
            }
            if s.price == 0 {
                return Err(format!("{}: price must be positive", s.id));
            }
            if !matches!(s.reproduction.as_str(), "egg" | "live_birth") {
                return Err(format!("{}: unknown reproduction {:?}", s.id, s.reproduction));
            }
            if !matches!(s.stage_on_purchase.as_str(), "fry" | "juvenile" | "adult") {
                return Err(format!("{}: unknown stage {:?}", s.id, s.stage_on_purchase));
            }
        }
        let b = &self.balance;
        b.shell_hunt.validate()?;
        if self.species(&b.starter_species).is_none() {
            return Err(format!("starter species {:?} not in roster", b.starter_species));
        }
        if b.tank_capacity == 0 || b.stage_exp.juvenile >= b.stage_exp.adult {
            return Err("tank capacity / stage thresholds invalid".into());
        }
        if b.slots.medium_price == 0 || b.slots.medium_price >= b.slots.large_price || b.tank_capacity < 3 {
            return Err("tank slots invalid".into());
        }
        let br = &b.breeding;
        if !(br.hatch_min.is_finite() && br.hatch_max.is_finite()) || br.hatch_min < 0.0 || br.hatch_min > br.hatch_max || br.hatch_max > 1.0 || br.price_low >= br.price_high || br.incubate_min_s < 0 || br.incubate_min_s > br.incubate_max_s || br.den_capacity == 0 {
            return Err("breeding balance invalid".into());
        }
        let e = &b.economy;
        let non_negative = [
            e.base_shells, e.base_exp, e.diminish_step, e.diminish_min, e.size_log_divisor, e.size_min, e.size_max,
            e.age_recent_days, e.age_old_days, e.age_recent_mult, e.age_middle_mult, e.age_old_mult,
        ];
        if non_negative.iter().any(|v| !v.is_finite() || *v < 0.0) || e.size_log_divisor == 0.0 {
            return Err("economy values must be finite and non-negative".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_valid_with_114_species() {
        let c = Catalog::bundled().unwrap();
        assert_eq!(c.species.len(), 114);
    }

    #[test]
    fn every_sprite_exists() {
        let c = Catalog::bundled().unwrap();
        let public = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
        for s in &c.species {
            assert!(public.join(&s.sprite).is_file(), "missing sprite for {}", s.id);
        }
    }

    #[test]
    fn prices_match_v4_baseline() {
        let c = Catalog::bundled().unwrap();
        let price = |id: &str| c.species(id).unwrap().price;
        assert_eq!(price("cyprinus_carpio"), 25);
        assert_eq!(price("symphysodon_aequifasciatus"), 80);
        assert_eq!(price("poecilia_reticulata"), 20);
    }

    #[test]
    fn validator_rejects_duplicates_and_zero_price() {
        let mut c = Catalog::bundled().unwrap();
        c.species[1].id = c.species[0].id.clone();
        assert!(c.validate().is_err());
        let mut c = Catalog::bundled().unwrap();
        c.species[0].price = 0;
        assert!(c.validate().is_err());
    }
}
