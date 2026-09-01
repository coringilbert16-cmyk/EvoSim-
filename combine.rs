//! COMBINE support: deterministic recipe caching, locked formation threshold,
//! and explicitly EXPERIMENTAL interaction / bond-strength equations.
//!
//! The equations in this module are experimental balancing infrastructure.
//! They implement the locked architectural decisions without claiming that
//! their numerical form is final:
//!
//! - potential-energy difference establishes interaction direction;
//! - reactivity modifies interaction magnitude nonlinearly;
//! - geometry/contact modifies interaction magnitude;
//! - formation still requires the locked cohesion/load threshold;
//! - surplus produces bond strength through a capped diminishing-returns
//!   curve.
//!
//! These equations are tested in isolation before being wired into organism
//! evolution.

use std::collections::HashMap;

use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::math::exponential_influence;
use crate::resources::{combine_materials, effective_reactivity, BaseResource, Material, ResourceProperties};
use crate::structure::{formation_threshold, OrganismStructure};

const EPSILON: f64 = 1e-12;

/// Experimental scale used by the surplus -> bond-strength curve.
/// This is deliberately a named tuning parameter rather than a hidden
/// constant so later balancing can change it without redesigning the API.
pub const EXPERIMENTAL_BOND_STRENGTH_SCALE: f64 = 1.0;

/// Maximum bond strength produced by the current experimental curve.
pub const EXPERIMENTAL_MAX_BOND_STRENGTH: f64 = 1.0;

/// Experimental interaction result.
///
/// `direction` is -1, 0, or +1 and identifies the potential-energy gradient
/// from material A toward material B. Swapping A and B reverses it.
/// `magnitude` contains the nonlinear reactivity and geometric modifiers.
/// `signed_value` combines both and is therefore useful for diagnostics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExperimentalInteraction {
    pub direction: f64,
    pub magnitude: f64,
    pub signed_value: f64,
}

/// Experimental COMBINE interaction model.
///
/// Potential energy establishes direction. Reactivity and geometry establish
/// how strongly that potential difference matters. Water is represented as a
/// dilution field through the existing effective-reactivity rule.
///
/// Geometry uses the contact candidate's facing and distance:
/// - facing is remapped from [-1, 1] to [0, 1];
/// - distance uses a bounded inverse falloff;
/// - the two are multiplied so poor contact suppresses the interaction.
///
/// This function is intentionally symmetric in magnitude and antisymmetric
/// in direction when A and B are swapped.
pub fn experimental_interaction(
    a: ResourceProperties,
    b: ResourceProperties,
    candidate: ConnectionPairCandidate,
    water_field: f64,
) -> ExperimentalInteraction {
    let potential_delta = b.potential_energy - a.potential_energy;
    let direction = if potential_delta > EPSILON {
        1.0
    } else if potential_delta < -EPSILON {
        -1.0
    } else {
        0.0
    };

    let reactivity = (
        exponential_influence(effective_reactivity(a.reactivity.max(0.0), water_field))
            + exponential_influence(effective_reactivity(b.reactivity.max(0.0), water_field))
    ) / 2.0;

    let facing = ((candidate.facing.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
    let distance = if candidate.distance.is_finite() {
        candidate.distance.max(0.0)
    } else {
        f64::INFINITY
    };
    let distance_factor = if distance.is_finite() {
        1.0 / (1.0 + distance)
    } else {
        0.0
    };

    let geometry = facing * distance_factor;
    let magnitude = potential_delta.abs() * reactivity * geometry;

    ExperimentalInteraction {
        direction,
        magnitude,
        signed_value: direction * magnitude,
    }
}

/// Experimental positive work cost for COMBINE.
///
/// The potential-energy interaction contributes through its magnitude, while
/// the direction remains available separately in `ExperimentalInteraction`.
/// The cost is always non-negative and therefore cannot be changed merely by
/// reversing the order in which the two participating materials are supplied.
/// Complexity and cohesion provide the baseline construction burden.
pub fn experimental_combine_work_cost(
    a: ResourceProperties,
    b: ResourceProperties,
    candidate: ConnectionPairCandidate,
    water_field: f64,
) -> f64 {
    let interaction = experimental_interaction(a, b, candidate, water_field);
    let complexity_factor = 1.0 + ((a.mass.max(0.0) + b.mass.max(0.0)) * 0.5).sqrt();
    let cohesion_factor = 1.0 + ((a.cohesion.clamp(0.0, 1.0) + b.cohesion.clamp(0.0, 1.0)) * 0.5);

    // A small irreducible work floor prevents zero-cost COMBINE for equal-
    // potential, perfectly aligned materials. The interaction term is bounded
    // by the same physical quantities that establish its direction.
    (0.25 + interaction.magnitude) * complexity_factor * cohesion_factor
}

/// Experimental capped diminishing-returns mapping from formation surplus to
/// bond strength.
///
/// At zero surplus the resulting strength is zero. Additional investment
/// increases strength monotonically but approaches the configured cap rather
/// than growing without bound:
///
///     strength = max_strength * (1 - exp(-surplus / scale))
///
/// Negative, NaN, and infinite surplus do not produce a bond strength.
pub fn experimental_bond_strength(surplus: f64) -> f64 {
    if !surplus.is_finite() || surplus <= 0.0 {
        return 0.0;
    }
    let scale = EXPERIMENTAL_BOND_STRENGTH_SCALE.max(EPSILON);
    let max_strength = EXPERIMENTAL_MAX_BOND_STRENGTH.max(0.0);
    max_strength * (1.0 - (-surplus / scale).exp())
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialRecipeKey {
    bonded: Vec<bool>,
    parts: Vec<(String, u64)>,
}

impl MaterialRecipeKey {
    pub fn from_material(material: &Material) -> Self {
        Self {
            bonded: vec![material.bonded],
            parts: material.parts.iter()
                .filter(|(_, amount)| *amount > EPSILON)
                .map(|(name, amount)| (name.clone(), amount.to_bits()))
                .collect(),
        }
    }

    pub fn from_inputs(inputs: &[Material]) -> Self {
        let mut parts = Vec::new();
        let mut bonded = Vec::with_capacity(inputs.len());

        for material in inputs {
            bonded.push(material.bonded);
            parts.extend(material.parts.iter()
                .filter(|(_, amount)| *amount > EPSILON)
                .map(|(name, amount)| (name.clone(), amount.to_bits())));
        }

        // Bonded/unbonded classification is part of Material state and must
        // therefore participate in the cache key. Parts remain order-independent.
        parts.sort_by(|a, b| a.cmp(b));
        bonded.sort_unstable();

        Self { bonded, parts }
    }
}

#[derive(Default)]
pub struct CombineCache {
    results: HashMap<MaterialRecipeKey, Material>,
}

impl CombineCache {
    pub fn new() -> Self { Self::default() }

    pub fn combine(&mut self, inputs: &[Material]) -> Material {
        let key = MaterialRecipeKey::from_inputs(inputs);
        if let Some(existing) = self.results.get(&key) {
            return existing.clone();
        }
        let result = combine_materials(inputs);
        self.results.insert(key, result.clone());
        result
    }

    pub fn len(&self) -> usize { self.results.len() }
    pub fn is_empty(&self) -> bool { self.results.is_empty() }
    pub fn clear(&mut self) { self.results.clear(); }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormationEvaluation {
    pub candidate: ConnectionPairCandidate,
    pub threshold: f64,
}

/// Evaluate only the locked formation-threshold equation.
pub fn evaluate_formation(
    candidate: ConnectionPairCandidate,
    cohesion_a: f64,
    cohesion_b: f64,
) -> FormationEvaluation {
    FormationEvaluation {
        candidate,
        threshold: formation_threshold(
            cohesion_a,
            cohesion_b,
            candidate.load_a,
            candidate.load_b,
        ),
    }
}

pub fn formation_surplus(evaluation: FormationEvaluation, investment: f64) -> f64 {
    if !investment.is_finite() {
        return f64::NAN;
    }
    investment - evaluation.threshold
}

pub fn formation_succeeds(evaluation: FormationEvaluation, investment: f64) -> bool {
    let surplus = formation_surplus(evaluation, investment);
    surplus.is_finite() && surplus >= 0.0
}

/// Evaluate formation and map successful surplus to the experimental bond
/// strength. This remains a non-mutating calculation; actual structure
/// mutation belongs to the later structural-placement step.
pub fn evaluate_bond_strength(evaluation: FormationEvaluation, investment: f64) -> Option<f64> {
    if !formation_succeeds(evaluation, investment) {
        return None;
    }
    Some(experimental_bond_strength(formation_surplus(evaluation, investment)))
}

/// Non-mutating bridge from contact candidates to the formation-threshold stage.
/// Static connection topology is cached; geometry and bond load remain dynamic.
pub fn evaluate_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<FormationEvaluation> {
    let candidates = crate::contact::connection_pair_candidates_cached(
        structure, unit_a, unit_b, catalog, cache,
    );

    let Some(unit_a_ref) = structure.units.get(unit_a) else { return Vec::new() };
    let Some(unit_b_ref) = structure.units.get(unit_b) else { return Vec::new() };
    let Some(cohesion_a) = unit_a_ref.properties(catalog).map(|p| p.cohesion) else { return Vec::new() };
    let Some(cohesion_b) = unit_b_ref.properties(catalog).map(|p| p.cohesion) else { return Vec::new() };

    candidates.into_iter()
        .map(|candidate| evaluate_formation(candidate, cohesion_a, cohesion_b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carbon(amount: f64) -> Material { Material::free_base("Carbon", amount) }
    fn methane(amount: f64) -> Material { Material::free_base("Methane", amount) }

    fn props(potential_energy: f64, reactivity: f64, cohesion: f64) -> ResourceProperties {
        ResourceProperties {
            mass: 1.0,
            potential_energy,
            reactivity,
            cohesion,
        }
    }

    fn candidate(load_a: f64, load_b: f64) -> ConnectionPairCandidate {
        ConnectionPairCandidate {
            point_a: 0, point_b: 0, distance: 0.0, facing: 1.0, load_a, load_b,
        }
    }

    #[test]
    fn recipe_key_is_independent_of_input_order() {
        assert_eq!(MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]), MaterialRecipeKey::from_inputs(&[methane(2.0), carbon(1.0)]));
    }

    #[test]
    fn different_quantities_do_not_collide() {
        assert_ne!(MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]), MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(3.0)]));
    }

    #[test]
    fn bonded_state_participates_in_cache_key() {
        let free = carbon(1.0);
        let mut bonded = carbon(1.0);
        bonded.bonded = true;
        assert_ne!(MaterialRecipeKey::from_material(&free), MaterialRecipeKey::from_material(&bonded));
    }

    #[test]
    fn cache_reuses_same_recipe() {
        let mut cache = CombineCache::new();
        let inputs = [carbon(1.0), methane(2.0)];
        let first = cache.combine(&inputs);
        let second = cache.combine(&inputs);
        assert_eq!(first.parts, second.parts);
        assert!(first.bonded);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cached_result_matches_existing_definition() {
        let mut cache = CombineCache::new();
        let inputs = [carbon(1.0), methane(2.0)];
        let cached = cache.combine(&inputs);
        let direct = combine_materials(&inputs);
        assert_eq!(cached.parts, direct.parts);
        assert_eq!(cached.bonded, direct.bonded);
    }

    #[test]
    fn clear_removes_cached_recipes() {
        let mut cache = CombineCache::new();
        cache.combine(&[carbon(1.0), methane(2.0)]);
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn evaluation_uses_locked_threshold() {
        let e = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        assert!((e.threshold - 0.6).abs() < 1e-12);
    }

    #[test]
    fn existing_load_raises_threshold() {
        let free = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        let loaded = evaluate_formation(candidate(1.0, 0.0), 0.8, 0.4);
        assert!(loaded.threshold > free.threshold);
    }

    #[test]
    fn surplus_is_zero_at_threshold() {
        let e = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        assert!(formation_surplus(e, e.threshold).abs() < 1e-12);
    }

    #[test]
    fn surplus_is_negative_below_threshold() {
        let e = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        assert!(formation_surplus(e, e.threshold - 0.25) < 0.0);
    }

    #[test]
    fn investment_must_meet_threshold() {
        let e = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        assert!(!formation_succeeds(e, e.threshold - 1e-9));
        assert!(formation_succeeds(e, e.threshold));
        assert!(formation_succeeds(e, e.threshold + 1.0));
    }

    #[test]
    fn non_finite_investment_cannot_form() {
        let e = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        assert!(!formation_succeeds(e, f64::NAN));
        assert!(!formation_succeeds(e, f64::INFINITY));
        assert!(formation_surplus(e, f64::NAN).is_nan());
    }

    #[test]
    fn potential_energy_sets_interaction_direction() {
        let low = props(1.0, 1.0, 0.5);
        let high = props(10.0, 1.0, 0.5);
        let e = experimental_interaction(low, high, candidate(0.0, 0.0), 0.0);
        assert_eq!(e.direction, 1.0);
        assert!(e.signed_value > 0.0);

        let reversed = experimental_interaction(high, low, candidate(0.0, 0.0), 0.0);
        assert_eq!(reversed.direction, -1.0);
        assert!((e.magnitude - reversed.magnitude).abs() < 1e-12);
        assert!((e.signed_value + reversed.signed_value).abs() < 1e-12);
    }

    #[test]
    fn equal_potential_has_no_direction() {
        let a = props(5.0, 1.0, 0.5);
        let b = props(5.0, 1.0, 0.5);
        let e = experimental_interaction(a, b, candidate(0.0, 0.0), 0.0);
        assert_eq!(e.direction, 0.0);
        assert!(e.magnitude.abs() < 1e-12);
    }

    #[test]
    fn higher_reactivity_increases_interaction_magnitude() {
        let low = props(1.0, 0.1, 0.5);
        let high = props(10.0, 4.0, 0.5);
        let low_reac = props(10.0, 0.1, 0.5);
        let high_reac = props(10.0, 4.0, 0.5);
        let a = experimental_interaction(low, low_reac, candidate(0.0, 0.0), 0.0);
        let b = experimental_interaction(low, high_reac, candidate(0.0, 0.0), 0.0);
        assert!(b.magnitude > a.magnitude);
    }

    #[test]
    fn water_dilution_reduces_reactivity_effect() {
        let a = props(1.0, 4.0, 0.5);
        let b = props(10.0, 4.0, 0.5);
        let dry = experimental_interaction(a, b, candidate(0.0, 0.0), 0.0);
        let wet = experimental_interaction(a, b, candidate(0.0, 0.0), 10.0);
        assert!(wet.magnitude < dry.magnitude);
    }

    #[test]
    fn poor_geometry_reduces_interaction_magnitude() {
        let a = props(1.0, 4.0, 0.5);
        let b = props(10.0, 4.0, 0.5);
        let close = candidate(0.0, 0.0);
        let far = ConnectionPairCandidate { distance: 9.0, ..close };
        let misaligned = ConnectionPairCandidate { facing: -1.0, ..close };
        let close_value = experimental_interaction(a, b, close, 0.0).magnitude;
        let far_value = experimental_interaction(a, b, far, 0.0).magnitude;
        let misaligned_value = experimental_interaction(a, b, misaligned, 0.0).magnitude;
        assert!(close_value > far_value);
        assert!(close_value > misaligned_value);
        assert!(misaligned_value.abs() < 1e-12);
    }

    #[test]
    fn combine_work_cost_is_order_independent_and_positive() {
        let a = props(1.0, 2.0, 0.7);
        let b = props(20.0, 3.0, 0.2);
        let c = candidate(0.0, 0.0);
        let ab = experimental_combine_work_cost(a, b, c, 0.0);
        let ba = experimental_combine_work_cost(b, a, c, 0.0);
        assert!(ab > 0.0);
        assert!((ab - ba).abs() < 1e-12);
    }

    #[test]
    fn bond_strength_is_zero_at_and_below_no_surplus() {
        assert_eq!(experimental_bond_strength(-1.0), 0.0);
        assert_eq!(experimental_bond_strength(0.0), 0.0);
    }

    #[test]
    fn bond_strength_has_diminishing_returns_and_is_capped() {
        let a = experimental_bond_strength(0.5);
        let b = experimental_bond_strength(1.0);
        let c = experimental_bond_strength(2.0);
        assert!(a > 0.0 && a < b && b < c);
        assert!(c < EXPERIMENTAL_MAX_BOND_STRENGTH);
        assert!(experimental_bond_strength(1000.0) <= EXPERIMENTAL_MAX_BOND_STRENGTH);
        assert!((experimental_bond_strength(2.0) - experimental_bond_strength(1.0))
            < (experimental_bond_strength(1.0) - experimental_bond_strength(0.5)));
    }

    #[test]
    fn successful_formation_maps_surplus_to_strength() {
        let e = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        assert_eq!(evaluate_bond_strength(e, e.threshold - 0.01), None);
        assert_eq!(evaluate_bond_strength(e, e.threshold), Some(0.0));
        assert!(evaluate_bond_strength(e, e.threshold + 1.0).unwrap() > 0.0);
    }
}
