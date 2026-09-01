use super::*;
use super::vents::{apply_vents, Vent};
use super::settling::{apply_settling, DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS};
use crate::resources::Material;

fn make_bonded(name: &str, amount: f64) -> Material { Material { parts: vec![(name.to_string(), amount)], bonded: true } }
fn make_unbonded(name: &str, amount: f64) -> Material { Material { parts: vec![(name.to_string(), amount)], bonded: false } }

