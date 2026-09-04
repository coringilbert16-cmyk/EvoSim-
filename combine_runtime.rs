//! Runtime structural-combination boundary.
//!
//! Organisms do not use COMBINE to redesign themselves. Lifetime structural
//! change is constrained to inherited blueprint growth and repair. The
//! low-level chemistry/structural COMBINE implementation remains available to
//! those controlled lifecycle paths, but this organism decision boundary is
//! intentionally inert until a blueprint-authorized caller is introduced.

use crate::contact::ConnectionCompatibilityCache;
use crate::state::{Environment, Organism};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CombineAttempt {
    pub unit_a: usize,
    pub unit_b: usize,
    pub point_a: usize,
    pub point_b: usize,
    pub work_cost: f64,
    pub energy_paid: f64,
    pub interaction_direction: f64,
    pub interaction_magnitude: f64,
    pub formation_threshold: f64,
    pub surplus: f64,
    pub bond_strength: f64,
    pub bond_energy: f64,
}

/// Organisms may not instantiate arbitrary structural material. Construction
/// is performed only from an inherited blueprint by the growth/reproduction
/// lifecycle.
pub(crate) fn instantiate_one_unit(
    _organism: &mut Organism,
    _catalog: &[crate::resources::BaseResource],
) -> Option<usize> {
    None
}

/// Organism-directed COMBINE is prohibited by the locked lifecycle rule:
/// existing structural bonds cannot be intentionally created to redesign the
/// body. Blueprint construction and repair use dedicated lifecycle paths.
pub(crate) fn try_combine(
    _organism: &mut Organism,
    _environment: &Environment,
    _compatibility_cache: &mut ConnectionCompatibilityCache,
) -> Option<CombineAttempt> {
    None
}
