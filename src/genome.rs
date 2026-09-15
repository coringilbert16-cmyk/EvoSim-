use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::architecture::{default_architecture, OrganismArchitecture, JUVENILE_LINEAR_SCALE};
use crate::resources::Material;
use crate::structural_blueprint::StructuralBlueprint;

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
    #[serde(default = "default_juvenile_reserve")]
    pub juvenile_reserve: Material,
    #[serde(default = "default_juvenile_energy_reserve")]
    pub juvenile_energy_reserve: f64,
    /// Sole inherited structural authority. Region-level intent only; no physical
    /// bonds or constituent coordinates are inherited.
    #[serde(default = "default_architecture")]
    pub architecture: OrganismArchitecture,
}

impl Genome {
    pub fn trait_value(&self, name: &str, default: f64) -> f64 {
        self.traits
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.value)
            .unwrap_or(default)
    }
    pub fn mass_affinity(&self) -> f64 {
        self.trait_value("mass_affinity", 0.0).clamp(-1.0, 1.0)
    }
    pub fn potential_energy_affinity(&self) -> f64 {
        self.trait_value("potential_energy_affinity", 0.0)
            .clamp(-1.0, 1.0)
    }
    pub fn reactivity_affinity(&self) -> f64 {
        self.trait_value("reactivity_affinity", 0.0)
            .clamp(-1.0, 1.0)
    }
    pub fn cohesion_affinity(&self) -> f64 {
        self.trait_value("cohesion_affinity", 0.0).clamp(-1.0, 1.0)
    }
    pub fn memory_strength(&self) -> f64 {
        self.trait_value("memory_strength", 0.5).clamp(0.0, 1.0)
    }
    pub fn perception_radius(&self) -> f64 {
        self.trait_value("perception_radius", 100.0).max(0.0)
    }
    pub fn sensory_resolution(&self) -> f64 {
        self.trait_value("sensory_resolution", 0.5).clamp(0.0, 1.0)
    }
    pub fn directional_resolution(&self) -> f64 {
        self.trait_value("directional_resolution", 1.0)
            .clamp(0.0, 1.0)
    }
    pub fn processing_efficiency(&self) -> f64 {
        self.trait_value("processing_efficiency", 0.8)
            .clamp(0.05, 1.0)
    }
    pub fn movement_efficiency(&self) -> f64 {
        self.trait_value("movement_efficiency", 0.8)
            .clamp(0.05, 1.0)
    }
    pub fn reproductive_investment(&self) -> f64 {
        self.trait_value("reproductive_investment", 0.5)
            .clamp(0.15, 1.0)
    }

    pub fn mature_construction_target(&self) -> Result<StructuralBlueprint, String> {
        self.architecture.adult_construction_target()
    }

    pub fn developmental_construction_target(
        &self,
        catalog: &[crate::resources::BaseResource],
    ) -> Result<StructuralBlueprint, String> {
        self.architecture
            .developmental_target(JUVENILE_LINEAR_SCALE, catalog)
    }

    pub fn mutate(&mut self, rng: &mut ChaCha8Rng) {
        let mut probability_sum = 0.0;
        let mut sigma_sum = 0.0;
        for t in &mut self.traits {
            if rng.gen::<f64>() < t.mutation_probability.clamp(1e-6, 0.25) {
                t.value += rng.gen_range(-1.0..1.0) * t.mutation_sigma.max(0.0);
            }
            if rng.gen::<f64>() < 0.001 {
                t.mutation_probability =
                    (t.mutation_probability * rng.gen_range(0.5..1.5)).clamp(1e-6, 0.1);
            }
            probability_sum += t.mutation_probability;
            sigma_sum += t.mutation_sigma.max(0.0);
        }
        let count = self.traits.len().max(1) as f64;
        self.mutate_architecture(
            rng,
            (probability_sum / count).clamp(1e-6, 0.25),
            (sigma_sum / count).clamp(1e-6, 1.0),
        );
    }

    fn mutate_architecture(
        &mut self,
        rng: &mut ChaCha8Rng,
        mutation_probability: f64,
        mutation_sigma: f64,
    ) {
        let original = self.architecture.clone();
        let probability = mutation_probability.clamp(0.0, 1.0);
        let sigma = mutation_sigma.max(0.0);
        for region in &mut self.architecture.regions {
            if rng.gen::<f64>() >= probability {
                continue;
            }
            region.center_x += rng.gen_range(-1.0..1.0) * sigma;
            region.center_y += rng.gen_range(-1.0..1.0) * sigma;
            let factor = (1.0 + rng.gen_range(-1.0..1.0) * sigma * 0.1).max(0.01);
            region.extent_x *= factor;
            region.extent_y *= factor;
        }
        if self.architecture.validate().is_err() {
            self.architecture = original;
        }
    }
}

fn default_juvenile_reserve() -> Material {
    Material::free_base("Hydrogen", 1.0)
}

fn default_juvenile_energy_reserve() -> f64 {
    16.0
}

fn trait_def(name: &str, value: f64, sigma: f64) -> TraitDef {
    TraitDef {
        name: name.into(),
        value,
        mutation_probability: 0.001,
        mutation_sigma: sigma,
    }
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
        juvenile_reserve: default_juvenile_reserve(),
        juvenile_energy_reserve: default_juvenile_energy_reserve(),
        architecture: default_architecture(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn genome_architecture_is_the_serialized_structural_authority() {
        let genome = initial_genome();
        assert!(genome.architecture.validate().is_ok());
        assert_eq!(genome.architecture.regions.len(), 3);
    }

    #[test]
    fn construction_targets_are_derived_from_architecture() {
        let genome = initial_genome();
        assert!(genome.mature_construction_target().unwrap().is_valid());
        assert!(genome
            .developmental_construction_target(&crate::resources::default_catalog())
            .unwrap()
            .is_valid());
    }

    #[test]
    fn structural_mutation_rolls_back_invalid_architecture() {
        let mut genome = initial_genome();
        let before = genome.architecture.clone();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        genome.mutate_architecture(&mut rng, 1.0, f64::INFINITY);
        assert_eq!(genome.architecture, before);
    }

    #[test]
    fn reserves_remain_genome_defined() {
        let genome = initial_genome();
        assert!(genome.juvenile_reserve.is_valid());
        assert_eq!(genome.juvenile_reserve.total_amount(), 1.0);
        assert!(genome.juvenile_energy_reserve.is_finite() && genome.juvenile_energy_reserve > 0.0);
    }
}
