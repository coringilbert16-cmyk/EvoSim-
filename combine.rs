//! COMBINE support that is safe to implement before the unresolved
//! interaction-energy and bond-strength equations are finalized.
//!
//! The simulator can encounter the same material recipe repeatedly.
//! This module gives those repeated recipes a canonical, deterministic
//! key and caches the already-constructed bonded Material result.
//! It does NOT decide whether a combine should happen, how much energy
//! the interaction releases/consumes, or what structural bond strength
//! results. Those remain separate decisions.

use std::collections::HashMap;

use crate::resources::{combine_materials, Material};

/// Exact, order-independent key for a material composition.
///
/// Quantities use their IEEE-754 bit representation rather than a lossy
/// decimal rounding rule. That means the cache never silently treats two
/// numerically different material quantities as identical. The individual
/// parts are sorted by resource name so equivalent material stacks with
/// different internal ordering produce the same key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialRecipeKey(Vec<(String, u64)>);

impl MaterialRecipeKey {
    pub fn from_material(material: &Material) -> Self {
        let mut parts: Vec<(String, u64)> = material
            .parts
            .iter()
            .filter(|(_, amount)| *amount > 1e-12)
            .map(|(name, amount)| (name.clone(), amount.to_bits()))
            .collect();
        parts.sort_by(|a, b| a.cmp(b));
        Self(parts)
    }

    /// Builds one key for the complete set of COMBINE inputs. Input order
    /// is deliberately irrelevant: the same recipe produces the same key.
    pub fn from_inputs(inputs: &[Material]) -> Self {
        let mut parts = Vec::new();
        for material in inputs {
            parts.extend(
                material
                    .parts
                    .iter()
                    .filter(|(_, amount)| *amount > 1e-12)
                    .map(|(name, amount)| (name.clone(), amount.to_bits())),
            );
        }
        parts.sort_by(|a, b| a.cmp(b));
        Self(parts)
    }
}

/// A small per-simulation cache for repeated COMBINE recipes.
///
/// This cache only memoizes the deterministic material construction already
/// defined by `resources::combine_materials`. It does not memoize energetic
/// consequences, because those may depend on environmental state (for
/// example water field) and the actual interaction equation is intentionally
/// unresolved.
#[derive(Default)]
pub struct CombineCache {
    results: HashMap<MaterialRecipeKey, Material>,
}

impl CombineCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the deterministic bonded-material result for this recipe,
    /// constructing it only the first time this exact recipe is encountered.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn carbon(amount: f64) -> Material {
        Material::free_base("Carbon", amount)
    }

    fn methane(amount: f64) -> Material {
        Material::free_base("Methane", amount)
    }

    #[test]
    fn recipe_key_is_independent_of_input_order() {
        let a = MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]);
        let b = MaterialRecipeKey::from_inputs(&[methane(2.0), carbon(1.0)]);
        assert_eq!(a, b);
    }

    #[test]
    fn different_quantities_do_not_collide() {
        let a = MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]);
        let b = MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(3.0)]);
        assert_ne!(a, b);
    }

    #[test]
    fn cache_reuses_the_same_recipe() {
        let mut cache = CombineCache::new();
        let inputs = [carbon(1.0), methane(2.0)];

        let first = cache.combine(&inputs);
        let second = cache.combine(&inputs);

        assert_eq!(first.parts, second.parts);
        assert!(first.bonded);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cached_result_matches_existing_combine_definition() {
        let mut cache = CombineCache::new();
        let inputs = [carbon(1.0), methane(2.0)];
        let cached = cache.combine(&inputs);
        let direct = combine_materials(&inputs);
        assert_eq!(cached.parts, direct.parts);
        assert_eq!(cached.bonded, direct.bonded);
    }

    #[test]
    fn clear_removes_cached_recipes_without_changing_material_definition() {
        let mut cache = CombineCache::new();
        let inputs = [carbon(1.0), methane(2.0)];
        cache.combine(&inputs);
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }
}
