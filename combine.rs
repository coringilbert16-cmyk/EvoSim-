//! COMBINE support: deterministic recipe caching, locked formation threshold,
//! and explicitly experimental interaction / bond-strength equations.
//!
//! Potential energy establishes direction; reactivity and geometry modify
//! magnitude. Formation uses the locked cohesion/load threshold. Surplus maps
//! to bond strength through capped diminishing returns.

use std::collections::HashMap;

use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::math::exponential_influence;
use crate::resources::{
    combine_materials, effective_reactivity, BaseResource, Material, ResourceProperties,
};
use crate::structure::{formation_threshold, OrganismStructure};

const EPSILON: f64 = 1e-12;
pub const EXPERIMENTAL_BOND_STRENGTH_SCALE: f64 = 1.0;
pub const EXPERIMENTAL_MAX_BOND_STRENGTH: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExperimentalInteraction {
    pub direction: f64,
    pub magnitude: f64,
    pub signed_value: f64,
}

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
    let reactivity =
        (exponential_influence(effective_reactivity(a.reactivity.max(0.0), water_field))
            + exponential_influence(effective_reactivity(b.reactivity.max(0.0), water_field)))
            / 2.0;
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
    let magnitude = potential_delta.abs() * reactivity * facing * distance_factor;
    ExperimentalInteraction {
        direction,
        magnitude,
        signed_value: direction * magnitude,
    }
}

pub fn experimental_combine_work_cost(
    a: ResourceProperties,
    b: ResourceProperties,
    candidate: ConnectionPairCandidate,
    water_field: f64,
) -> f64 {
    let interaction = experimental_interaction(a, b, candidate, water_field);
    let complexity_factor = 1.0 + ((a.mass.max(0.0) + b.mass.max(0.0)) * 0.5).sqrt();
    let cohesion_factor = 1.0 + ((a.cohesion.clamp(0.0, 1.0) + b.cohesion.clamp(0.0, 1.0)) * 0.5);
    (0.25 + interaction.magnitude) * complexity_factor * cohesion_factor
}

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
            parts: material
                .parts
                .iter()
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
            parts.extend(
                material
                    .parts
                    .iter()
                    .filter(|(_, amount)| *amount > EPSILON)
                    .map(|(name, amount)| (name.clone(), amount.to_bits())),
            );
        }
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
    pub fn new() -> Self {
        Self::default()
    }
    pub fn combine(&mut self, inputs: &[Material]) -> Material {
        let key = MaterialRecipeKey::from_inputs(inputs);
        if let Some(existing) = self.results.get(&key) {
            return existing.clone();
        }
        let result = combine_materials(inputs);
        self.results.insert(key, result.clone());
        result
    }
    pub fn len(&self) -> usize {
        self.results.len()
    }
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
    pub fn clear(&mut self) {
        self.results.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormationEvaluation {
    pub candidate: ConnectionPairCandidate,
    pub threshold: f64,
}

pub fn evaluate_formation(
    candidate: ConnectionPairCandidate,
    cohesion_a: f64,
    cohesion_b: f64,
) -> FormationEvaluation {
    FormationEvaluation {
        candidate,
        threshold: formation_threshold(cohesion_a, cohesion_b, candidate.load_a, candidate.load_b),
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

pub fn evaluate_bond_strength(evaluation: FormationEvaluation, investment: f64) -> Option<f64> {
    if !formation_succeeds(evaluation, investment) {
        return None;
    }
    Some(experimental_bond_strength(formation_surplus(
        evaluation, investment,
    )))
}

pub fn eligible_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<ConnectionPairCandidate> {
    crate::contact::connection_pair_candidates_cached(structure, unit_a, unit_b, catalog, cache)
        .into_iter()
        .filter(|candidate| candidate.available_a && candidate.available_b)
        .collect()
}

pub fn evaluate_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<FormationEvaluation> {
    let Some(unit_a_ref) = structure.units.get(unit_a) else {
        return Vec::new();
    };
    let Some(unit_b_ref) = structure.units.get(unit_b) else {
        return Vec::new();
    };
    let Some(cohesion_a) = unit_a_ref.properties(catalog).map(|p| p.cohesion) else {
        return Vec::new();
    };
    let Some(cohesion_b) = unit_b_ref.properties(catalog).map(|p| p.cohesion) else {
        return Vec::new();
    };
    eligible_candidates(structure, unit_a, unit_b, catalog, cache)
        .into_iter()
        .map(|candidate| evaluate_formation(candidate, cohesion_a, cohesion_b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::{Bond, Placement, StructuralUnit};

    fn carbon(amount: f64) -> Material {
        Material::free_base("Carbon", amount)
    }
    fn methane(amount: f64) -> Material {
        Material::free_base("Methane", amount)
    }
    fn props(p: f64, r: f64, c: f64) -> ResourceProperties {
        ResourceProperties {
            mass: 1.0,
            potential_energy: p,
            reactivity: r,
            cohesion: c,
        }
    }
    fn candidate(load_a: f64, load_b: f64) -> ConnectionPairCandidate {
        ConnectionPairCandidate {
            point_a: 0,
            point_b: 0,
            distance: 0.0,
            facing: 1.0,
            load_a,
            load_b,
            available_a: true,
            available_b: true,
        }
    }
    fn unit(s: &mut OrganismStructure, name: &str, x: f64, y: f64) -> usize {
        s.add_unit(StructuralUnit::new(
            name,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        ))
    }
    fn bond(a: usize, ap: usize, b: usize, bp: usize) -> Bond {
        Bond {
            unit_a: a,
            point_a: ap,
            unit_b: b,
            point_b: bp,
            strength: 0.5,
            bond_energy: 1.0,
        }
    }

    #[test]
    fn recipe_key_is_order_independent() {
        assert_eq!(
            MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]),
            MaterialRecipeKey::from_inputs(&[methane(2.0), carbon(1.0)])
        );
    }
    #[test]
    fn different_quantities_do_not_collide() {
        assert_ne!(
            MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]),
            MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(3.0)])
        );
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
    fn potential_energy_sets_direction() {
        let low = props(1.0, 1.0, 0.5);
        let high = props(10.0, 1.0, 0.5);
        let e = experimental_interaction(low, high, candidate(0.0, 0.0), 0.0);
        let r = experimental_interaction(high, low, candidate(0.0, 0.0), 0.0);
        assert_eq!(e.direction, 1.0);
        assert_eq!(r.direction, -1.0);
        assert!((e.magnitude - r.magnitude).abs() < 1e-12);
    }
    #[test]
    fn equal_potential_has_no_direction() {
        let e = experimental_interaction(
            props(5.0, 1.0, 0.5),
            props(5.0, 1.0, 0.5),
            candidate(0.0, 0.0),
            0.0,
        );
        assert_eq!(e.direction, 0.0);
        assert!(e.magnitude.abs() < 1e-12);
    }
    #[test]
    fn reactivity_and_water_modify_magnitude() {
        let low = props(1.0, 0.1, 0.5);
        let high = props(10.0, 4.0, 0.5);
        let a = experimental_interaction(low, props(10.0, 0.1, 0.5), candidate(0.0, 0.0), 0.0);
        let b = experimental_interaction(low, high, candidate(0.0, 0.0), 0.0);
        let wet = experimental_interaction(low, high, candidate(0.0, 0.0), 10.0);
        assert!(b.magnitude > a.magnitude);
        assert!(wet.magnitude < b.magnitude);
    }
    #[test]
    fn poor_geometry_reduces_interaction() {
        let a = props(1.0, 4.0, 0.5);
        let b = props(10.0, 4.0, 0.5);
        let close = candidate(0.0, 0.0);
        let far = ConnectionPairCandidate {
            distance: 9.0,
            ..close
        };
        let misaligned = ConnectionPairCandidate {
            facing: -1.0,
            ..close
        };
        assert!(
            experimental_interaction(a, b, close, 0.0).magnitude
                > experimental_interaction(a, b, far, 0.0).magnitude
        );
        assert!(
            experimental_interaction(a, b, misaligned, 0.0)
                .magnitude
                .abs()
                < 1e-12
        );
    }
    #[test]
    fn bond_strength_has_capped_diminishing_returns() {
        let a = experimental_bond_strength(0.5);
        let b = experimental_bond_strength(1.0);
        let c = experimental_bond_strength(2.0);
        assert!(a > 0.0 && a < b && b < c);
        assert!(c < EXPERIMENTAL_MAX_BOND_STRENGTH);
        assert!(experimental_bond_strength(1000.0) <= EXPERIMENTAL_MAX_BOND_STRENGTH);
        assert!((c - b) < (b - a));
    }
    #[test]
    fn formation_uses_load_and_surplus() {
        let free = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        let loaded = evaluate_formation(candidate(1.0, 0.0), 0.8, 0.4);
        assert!((free.threshold - 0.6).abs() < 1e-12);
        assert!(loaded.threshold > free.threshold);
        assert!(!formation_succeeds(free, free.threshold - 1e-9));
        assert!(formation_succeeds(free, free.threshold));
        assert!(evaluate_bond_strength(free, free.threshold).unwrap().abs() < 1e-12);
        assert!(evaluate_bond_strength(free, free.threshold + 1.0).unwrap() > 0.0);
    }
    #[test]
    fn non_finite_investment_cannot_form() {
        let e = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        assert!(!formation_succeeds(e, f64::NAN));
        assert!(!formation_succeeds(e, f64::INFINITY));
    }
    #[test]
    fn geometry_gate_rejects_candidates_marked_unavailable() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = unit(&mut structure, "Carbon", 0.0, 0.0);
        let b = unit(&mut structure, "Carbon", 1.0, 0.0);
        let mut cache = ConnectionCompatibilityCache::new();
        let first = eligible_candidates(&structure, a, b, &catalog, &mut cache);
        assert!(!first.is_empty());
        let point = first[0].point_a;
        let other = first[0].point_b;
        assert!(
            crate::contact::try_add_bond(&mut structure, bond(a, point, b, other), &catalog)
                .is_ok()
        );
        let second = eligible_candidates(&structure, a, b, &catalog, &mut cache);
        assert!(second.iter().all(|c| c.available_a && c.available_b));
    }
    #[test]
    fn occupied_point_is_no_longer_eligible() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = unit(&mut structure, "Carbon", 0.0, 0.0);
        let b = unit(&mut structure, "Carbon", 1.0, 0.0);
        let c = unit(&mut structure, "Carbon", 0.0, 1.0);
        assert!(crate::contact::try_add_bond(&mut structure, bond(a, 0, b, 0), &catalog).is_ok());
        let mut cache = ConnectionCompatibilityCache::new();
        let candidates = eligible_candidates(&structure, a, c, &catalog, &mut cache);
        assert!(candidates.iter().all(|candidate| candidate.point_a != 0));
    }
}
