//! COMBINE support: deterministic recipe caching and formation evaluation.
//!
//! Energy accounting lives in `energy.rs` so COMBINE and BREAK have one
//! authoritative energetic implementation. The exact interaction-energy,
//! bond-strength, and surplus-partition equations remain injectable/testable.

use std::collections::HashMap;

use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::resources::{combine_materials, BaseResource, Material};
use crate::structure::{formation_threshold, Bond, OrganismStructure};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialRecipeKey {
    bonded: Vec<bool>,
    parts: Vec<(String, u64)>,
}

impl MaterialRecipeKey {
    pub fn from_material(material: &Material) -> Self {
        Self {
            bonded: vec![material.bonded],
            parts: material.parts.iter().filter(|(_, amount)| *amount > 1e-12)
                .map(|(name, amount)| (name.clone(), amount.to_bits())).collect(),
        }
    }

    pub fn from_inputs(inputs: &[Material]) -> Self {
        let mut parts = Vec::new();
        let mut bonded = Vec::with_capacity(inputs.len());
        for material in inputs {
            bonded.push(material.bonded);
            parts.extend(material.parts.iter().filter(|(_, amount)| *amount > 1e-12)
                .map(|(name, amount)| (name.clone(), amount.to_bits())));
        }
        parts.sort_by(|a, b| a.cmp(b));
        bonded.sort_unstable();
        Self { bonded, parts }
    }
}

#[derive(Default)]
pub struct CombineCache { results: HashMap<MaterialRecipeKey, Material> }

impl CombineCache {
    pub fn new() -> Self { Self::default() }
    pub fn combine(&mut self, inputs: &[Material]) -> Material {
        let key = MaterialRecipeKey::from_inputs(inputs);
        if let Some(existing) = self.results.get(&key) { return existing.clone(); }
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

pub fn evaluate_formation(candidate: ConnectionPairCandidate, cohesion_a: f64, cohesion_b: f64) -> FormationEvaluation {
    FormationEvaluation { candidate, threshold: formation_threshold(cohesion_a, cohesion_b, candidate.load_a, candidate.load_b) }
}

pub fn formation_surplus(evaluation: FormationEvaluation, investment: f64) -> f64 {
    if !investment.is_finite() { return f64::NAN; }
    investment - evaluation.threshold
}

pub fn formation_succeeds(evaluation: FormationEvaluation, investment: f64) -> bool {
    let surplus = formation_surplus(evaluation, investment);
    surplus.is_finite() && surplus >= 0.0
}

/// Transfer resolved COMBINE bond energy into mutable bond state.
pub fn assign_combine_bond_energy(bond: &mut Bond, bond_energy: f64, bond_strength: f64) {
    bond.bond_energy = bond_energy.max(0.0);
    bond.strength = bond_strength.clamp(0.0, 1.0);
}

pub fn evaluate_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<FormationEvaluation> {
    let candidates = crate::contact::connection_pair_candidates_cached(structure, unit_a, unit_b, catalog, cache);
    let Some(unit_a_ref) = structure.units.get(unit_a) else { return Vec::new() };
    let Some(unit_b_ref) = structure.units.get(unit_b) else { return Vec::new() };
    let Some(cohesion_a) = unit_a_ref.properties(catalog).map(|p| p.cohesion) else { return Vec::new() };
    let Some(cohesion_b) = unit_b_ref.properties(catalog).map(|p| p.cohesion) else { return Vec::new() };
    candidates.into_iter().map(|candidate| evaluate_formation(candidate, cohesion_a, cohesion_b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn carbon(amount: f64) -> Material { Material::free_base("Carbon", amount) }
    fn methane(amount: f64) -> Material { Material::free_base("Methane", amount) }
    fn candidate(load_a: f64, load_b: f64) -> ConnectionPairCandidate {
        ConnectionPairCandidate { point_a: 0, point_b: 0, distance: 0.0, facing: 1.0, load_a, load_b }
    }
    #[test] fn recipe_key_is_independent_of_input_order() { assert_eq!(MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]), MaterialRecipeKey::from_inputs(&[methane(2.0), carbon(1.0)])); }
    #[test] fn different_quantities_do_not_collide() { assert_ne!(MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]), MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(3.0)])); }
    #[test] fn bonded_state_participates_in_cache_key() { let free=carbon(1.0); let mut bonded=carbon(1.0); bonded.bonded=true; assert_ne!(MaterialRecipeKey::from_material(&free), MaterialRecipeKey::from_material(&bonded)); }
    #[test] fn cache_reuses_same_recipe() { let mut cache=CombineCache::new(); let inputs=[carbon(1.0),methane(2.0)]; let first=cache.combine(&inputs); let second=cache.combine(&inputs); assert_eq!(first.parts,second.parts); assert!(first.bonded); assert_eq!(cache.len(),1); }
    #[test] fn cached_result_matches_existing_definition() { let mut cache=CombineCache::new(); let inputs=[carbon(1.0),methane(2.0)]; let cached=cache.combine(&inputs); let direct=combine_materials(&inputs); assert_eq!(cached.parts,direct.parts); assert_eq!(cached.bonded,direct.bonded); }
    #[test] fn clear_removes_cached_recipes() { let mut cache=CombineCache::new(); cache.combine(&[carbon(1.0),methane(2.0)]); assert!(!cache.is_empty()); cache.clear(); assert!(cache.is_empty()); }
    #[test] fn evaluation_uses_locked_threshold() { let e=evaluate_formation(candidate(0.0,0.0),0.8,0.4); assert!((e.threshold-0.6).abs()<1e-12); }
    #[test] fn existing_load_raises_threshold() { let free=evaluate_formation(candidate(0.0,0.0),0.8,0.4); let loaded=evaluate_formation(candidate(1.0,0.0),0.8,0.4); assert!(loaded.threshold>free.threshold); }
    #[test] fn surplus_is_zero_at_threshold() { let e=evaluate_formation(candidate(0.0,0.0),0.8,0.4); assert!(formation_surplus(e,e.threshold).abs()<1e-12); }
    #[test] fn surplus_is_negative_below_threshold() { let e=evaluate_formation(candidate(0.0,0.0),0.8,0.4); assert!(formation_surplus(e,e.threshold-0.25)<0.0); }
    #[test] fn investment_must_meet_threshold() { let e=evaluate_formation(candidate(0.0,0.0),0.8,0.4); assert!(!formation_succeeds(e,e.threshold-1e-9)); assert!(formation_succeeds(e,e.threshold)); assert!(formation_succeeds(e,e.threshold+1.0)); }
    #[test] fn non_finite_investment_cannot_form() { let e=evaluate_formation(candidate(0.0,0.0),0.8,0.4); assert!(!formation_succeeds(e,f64::NAN)); assert!(!formation_succeeds(e,f64::INFINITY)); assert!(formation_surplus(e,f64::NAN).is_nan()); }
}
