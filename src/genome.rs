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

fn seed_wall_material() -> Material {
    Material {
        parts: vec![("Nitrogen".into(), 1.0)],
        internal_bonds: Vec::new(),
    }
}

fn default_structural_blueprint() -> StructuralBlueprint {
    // The ancestral phenotype is a four-wall ring made from ordinary rigid
    // material. The enclosed region is the first physically meaningful place
    // where the inherited genome can reside; there is no special membrane/core
    // geometry and no seed-only viability exception.
    let half_wall = 1.511_858 / 2.0;
    let half_thickness = 0.330_719 / 2.0;
    let center_offset = half_wall + half_thickness;
    StructuralBlueprint::with_core_elements(
        vec![
            BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: 0.0, y: center_offset, rotation_radians: 0.0 } },
            BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: -center_offset, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_2 } },
            BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: center_offset, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_2 } },
            BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: 0.0, y: -center_offset, rotation_radians: 0.0 } },
        ],
        vec![
            BlueprintConnection { element_a: 0, element_b: 1 },
            BlueprintConnection { element_a: 0, element_b: 2 },
            BlueprintConnection { element_a: 1, element_b: 3 },
            BlueprintConnection { element_a: 2, element_b: 3 },
        ],
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
    fn seed_blueprint_has_a_four_wall_genome_bearing_phenotype() {
        let g = initial_genome();
        let b = &g.structural_blueprint;
        assert_eq!(b.elements.len(), 4);
        assert_eq!(b.connections.len(), 4);
        assert_eq!(b.core_elements, vec![0]);
        assert!(b.validate().is_ok());
        assert!(b.is_connected());
        assert!(b.elements.iter().all(|element| element.material.parts.len() == 1));
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
