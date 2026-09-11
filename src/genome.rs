use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::resources::{InternalBond, Material};
use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TraitDef {
    pub name: String,
    pub value: f64,
    pub mutation_probability: f64,
    pub mutation_sigma: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Genome {
    pub traits: Vec<TraitDef>,
    #[serde(default = "default_structural_blueprint")]
    pub structural_blueprint: StructuralBlueprint,
}

impl Genome {
    pub fn trait_value(&self, name: &str, default: f64) -> f64 {
        self.traits.iter().find(|t| t.name == name).map(|t| t.value).unwrap_or(default)
    }
    pub fn mass_affinity(&self) -> f64 { self.trait_value("mass_affinity", 0.0).clamp(-1.0, 1.0) }
    pub fn potential_energy_affinity(&self) -> f64 { self.trait_value("potential_energy_affinity", 0.0).clamp(-1.0, 1.0) }
    pub fn reactivity_affinity(&self) -> f64 { self.trait_value("reactivity_affinity", 0.0).clamp(-1.0, 1.0) }
    pub fn cohesion_affinity(&self) -> f64 { self.trait_value("cohesion_affinity", 0.0).clamp(-1.0, 1.0) }
    pub fn memory_strength(&self) -> f64 { self.trait_value("memory_strength", 0.5).clamp(0.0, 1.0) }
    pub fn perception_radius(&self) -> f64 { self.trait_value("perception_radius", 100.0).max(0.0) }
    pub fn sensory_resolution(&self) -> f64 { self.trait_value("sensory_resolution", 0.5).clamp(0.0, 1.0) }
    pub fn directional_resolution(&self) -> f64 { self.trait_value("directional_resolution", 1.0).clamp(0.0, 1.0) }
    pub fn processing_efficiency(&self) -> f64 { self.trait_value("processing_efficiency", 0.8).clamp(0.05, 1.0) }
    pub fn movement_efficiency(&self) -> f64 { self.trait_value("movement_efficiency", 0.8).clamp(0.05, 1.0) }
    pub fn reproductive_investment(&self) -> f64 { self.trait_value("reproductive_investment", 0.5).clamp(0.15, 1.0) }
    pub fn mutate(&mut self, rng: &mut ChaCha8Rng) {
        for t in &mut self.traits {
            if rng.gen::<f64>() < t.mutation_probability.clamp(1e-6, 0.25) {
                t.value += rng.gen_range(-1.0..1.0) * t.mutation_sigma.max(0.0);
            }
            if rng.gen::<f64>() < 0.001 {
                t.mutation_probability = (t.mutation_probability * rng.gen_range(0.5..1.5)).clamp(1e-6, 0.1);
            }
        }
    }
}

fn trait_def(name: &str, value: f64, sigma: f64) -> TraitDef {
    TraitDef { name: name.into(), value, mutation_probability: 0.001, mutation_sigma: sigma }
}

fn core_material() -> Material {
    Material {
        parts: vec![("Carbon".into(), 1.0), ("Nitrogen".into(), 1.0)],
        internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
    }
}

fn soft_interior_material() -> Material {
    Material {
        parts: vec![("Hydrogen".into(), 1.0), ("Sulfur".into(), 1.0)],
        internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
    }
}

fn membrane_material() -> Material {
    Material {
        parts: vec![("Carbon".into(), 1.0), ("Phosphorus".into(), 1.0)],
        internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
    }
}

fn default_structural_blueprint() -> StructuralBlueprint {
    StructuralBlueprint::with_core_elements(
        vec![
            BlueprintElement { material: core_material(), placement: BlueprintPlacement { x: 0.0, y: 0.0, rotation_radians: 0.0 } },
            BlueprintElement { material: soft_interior_material(), placement: BlueprintPlacement { x: 2.0, y: 0.0, rotation_radians: 0.0 } },
            BlueprintElement { material: membrane_material(), placement: BlueprintPlacement { x: 4.0, y: 0.0, rotation_radians: 0.0 } },
        ],
        vec![BlueprintConnection { element_a: 0, element_b: 1 }, BlueprintConnection { element_a: 1, element_b: 2 }],
        vec![0],
    )
}

pub fn initial_genome() -> Genome {
    Genome {
        traits: vec![
            trait_def("memory_strength", 0.5, 0.05),
            trait_def("perception_radius", 100.0, 1.0),
            trait_def("sensory_resolution", 0.5, 0.05),
            trait_def("directional_resolution", 1.0, 0.05),
            trait_def("mass_affinity", 0.0, 0.05),
            trait_def("potential_energy_affinity", 0.5, 0.05),
            trait_def("reactivity_affinity", 0.0, 0.05),
            trait_def("cohesion_affinity", 0.0, 0.05),
            trait_def("processing_efficiency", 0.8, 0.05),
            trait_def("movement_efficiency", 0.8, 0.05),
            trait_def("reproductive_investment", 0.5, 0.05),
        ],
        structural_blueprint: default_structural_blueprint(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn seed_blueprint_has_core_soft_interior_and_membrane() {
        let g = initial_genome();
        let b = &g.structural_blueprint;
        assert_eq!(b.elements.len(), 3);
        assert_eq!(b.connections.len(), 2);
        assert_eq!(b.core_elements, vec![0]);
        assert!(b.validate().is_ok());
        assert!(b.is_connected());
        assert_eq!(b.elements[0].material.parts.len(), 2);
        assert_eq!(b.elements[1].material.parts.len(), 2);
        assert_eq!(b.elements[2].material.parts.len(), 2);
        assert!(b.realize(&default_catalog()).is_ok());
    }

    #[test]
    fn seed_blueprint_is_connected() {
        assert!(initial_genome().structural_blueprint.is_connected());
    }

    #[test]
    fn seed_genome_core_is_connected() {
        let b = &initial_genome().structural_blueprint;
        assert_eq!(b.core_elements, vec![0]);
        assert!(b.core_elements.iter().all(|&i| i < b.elements.len()));
    }
}
