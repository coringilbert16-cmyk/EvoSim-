#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::developmental_blueprint::{
    default_developmental_blueprint, DevelopmentalFieldBlueprint,
};
use crate::resources::Material;

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
    /// Sole inherited structural-developmental authority. This stores continuous
    /// developmental tendencies, never exact constituent instances or topology.
    #[serde(default = "default_developmental_blueprint")]
    pub developmental_blueprint: DevelopmentalFieldBlueprint,
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

    /// Inherited developmental preference for adult scale, centered at the
    /// parent's value when offspring mutation is sampled.
    pub fn size_preference(&self) -> f64 {
        self.trait_value("size_preference", 0.5).clamp(0.0, 1.0)
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

    pub fn preferred_developmental_scale(&self) -> f64 {
        DevelopmentalFieldBlueprint::preferred_developmental_scale(self.size_preference())
    }

    /// Compatibility accessor for callers that need the adult developmental
    /// realization. The returned StructuralBlueprint is transient solver state;
    /// the genome stores only the developmental field blueprint.
    pub fn mature_construction_target(
        &self,
    ) -> Result<crate::structural_blueprint::StructuralBlueprint, String> {
        self.developmental_construction_target(&crate::resources::default_catalog(), false)
    }

    pub fn developmental_construction_target(
        &self,
        catalog: &[crate::resources::BaseResource],
        juvenile: bool,
    ) -> Result<crate::structural_blueprint::StructuralBlueprint, String> {
        let juvenile_scale = if juvenile {
            crate::architecture::JUVENILE_LINEAR_SCALE
        } else {
            1.0
        };
        self.developmental_blueprint.construction_candidate(
            catalog,
            self.preferred_developmental_scale(),
            juvenile_scale,
        )
    }

    pub fn mutate(&mut self, rng: &mut ChaCha8Rng) {
        if !self
            .traits
            .iter()
            .any(|trait_def| trait_def.name == "size_preference")
        {
            self.traits.push(trait_def("size_preference", 0.5, 0.05));
        }

        for t in &mut self.traits {
            if rng.gen::<f64>() < t.mutation_probability.clamp(1e-6, 0.25) {
                let delta = if t.name == "size_preference" {
                    gaussian_unit(rng) * t.mutation_sigma.max(0.0)
                } else {
                    rng.gen_range(-1.0..1.0) * t.mutation_sigma.max(0.0)
                };
                t.value = if t.name == "size_preference" {
                    (t.value + delta).clamp(0.0, 1.0)
                } else {
                    t.value + delta
                };
            }
            if rng.gen::<f64>() < 0.001 {
                t.mutation_probability =
                    (t.mutation_probability * rng.gen_range(0.5..1.5)).clamp(1e-6, 0.1);
            }
        }
    }
}

fn default_juvenile_reserve() -> Material {
    Material::free_base("Hydrogen", 1.0)
}

fn default_juvenile_energy_reserve() -> f64 {
    16.0
}

fn gaussian_unit(rng: &mut ChaCha8Rng) -> f64 {
    let u1 = rng.gen_range(f64::MIN_POSITIVE..1.0);
    let u2 = rng.gen_range(0.0..1.0);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
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
            trait_def("size_preference", 0.5, 0.05),
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
        developmental_blueprint: default_developmental_blueprint(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn developmental_blueprint_is_the_serialized_structural_authority() {
        let genome = initial_genome();
        assert!(genome.developmental_blueprint.validate().is_ok());
    }

    #[test]
    fn construction_targets_are_transient_artifacts_of_developmental_fields() {
        let genome = initial_genome();
        let catalog = crate::resources::default_catalog();
        assert!(genome
            .developmental_construction_target(&catalog, true)
            .unwrap()
            .is_valid());
        assert!(genome
            .developmental_construction_target(&catalog, false)
            .unwrap()
            .is_valid());
    }

    #[test]
    fn reserves_remain_genome_defined() {
        let genome = initial_genome();
        assert!(genome.juvenile_reserve.is_valid());
        assert_eq!(genome.juvenile_reserve.total_amount(), 1.0);
        assert!(genome.juvenile_energy_reserve.is_finite() && genome.juvenile_energy_reserve > 0.0);
    }
}

#[cfg(test)]
mod size_preference_tests {
    use super::*;
    use rand::SeedableRng;
    #[test]
    fn size_preference_defaults_to_center() {
        assert_eq!(initial_genome().size_preference(), 0.5);
    }

    #[test]
    fn size_preference_mutation_stays_bounded() {
        let mut genome = initial_genome();
        genome
            .traits
            .iter_mut()
            .find(|t| t.name == "size_preference")
            .unwrap()
            .mutation_probability = 1.0;
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        for _ in 0..1000 {
            genome.mutate(&mut rng);
            assert!((0.0..=1.0).contains(&genome.size_preference()));
        }
    }

    #[test]
    fn size_preference_mutation_is_parent_centered_in_distribution() {
        let mut above = 0;
        let mut below = 0;
        for seed in 0..200 {
            let mut genome = initial_genome();
            genome
                .traits
                .iter_mut()
                .find(|t| t.name == "size_preference")
                .unwrap()
                .mutation_probability = 1.0;
            genome
                .traits
                .iter_mut()
                .find(|t| t.name == "size_preference")
                .unwrap()
                .value = 0.5;
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            genome.mutate(&mut rng);
            if genome.size_preference() > 0.5 {
                above += 1;
            } else if genome.size_preference() < 0.5 {
                below += 1;
            }
        }
        assert!(above > 50 && below > 50);
    }
}
