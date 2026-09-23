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

    pub fn memory_strength(&self) -> f64 {
        self.trait_value("memory_strength", 0.5).clamp(0.0, 1.0)
    }

    /// Inherited developmental-size preference.
    ///
    /// The normalized value is the inherited authority. Preferred mass is derived
    /// from it; actual mass always belongs to the realized physical structure.
    pub fn size_preference(&self) -> f64 {
        self.trait_value("size_preference", 0.5).clamp(0.0, 1.0)
    }

    /// Preferred structural mass derived from the inherited size preference.
    ///
    /// M_MIN and M_MAX are experimental P6 parameter values, not permanent
    /// biological constants. The logarithmic mapping is the approved equation.
    pub fn adult_mass(&self) -> f64 {
        const M_MIN: f64 = 4.0; // EXPERIMENTAL: P6 developmental-size bound.
        const M_MAX: f64 = 225.0; // EXPERIMENTAL: chosen so default s=0.5 preserves 30.0.
        M_MIN * (M_MAX / M_MIN).powf(self.size_preference())
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
                let delta = gaussian_unit(rng) * t.mutation_sigma.max(0.0);
                t.value = if t.name == "size_preference" {
                    // Bell-shaped mutation around the parent's value, bounded to [0, 1].
                    (t.value + delta).clamp(0.0, 1.0)
                } else if t.name == "adult_mass" {
                    // Legacy serialized genomes may still contain this trait. It is no
                    // longer an authority and must not affect developmental size.
                    t.value
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
    Material {
        parts: vec![
            ("Carbon".into(), 1.0),
            ("Carbon".into(), 1.0),
            ("Sulfur".into(), 1.0),
        ],
        internal_bonds: vec![
            crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            },
            crate::resources::InternalBond {
                part_a: 1,
                part_b: 2,
            },
        ],
    }
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
    #[test]
    fn developmental_blueprint_is_the_serialized_structural_authority() {
        let genome = initial_genome();
        assert!(genome.developmental_blueprint.validate().is_ok());
    }

    #[test]
    fn size_preference_is_the_inherited_size_authority() {
        let genome = initial_genome();
        assert!((genome.size_preference() - 0.5).abs() < f64::EPSILON);
        assert!((genome.adult_mass() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn reserves_remain_genome_defined() {
        let genome = initial_genome();
        assert!(genome.juvenile_reserve.is_valid());
        assert_eq!(
            genome.juvenile_reserve.parts,
            vec![
                ("Carbon".into(), 1.0),
                ("Carbon".into(), 1.0),
                ("Sulfur".into(), 1.0),
            ]
        );
        assert_eq!(genome.juvenile_reserve.internal_bonds.len(), 2);
        assert_eq!(genome.juvenile_reserve.total_amount(), 3.0);
        assert!(genome.juvenile_energy_reserve.is_finite() && genome.juvenile_energy_reserve > 0.0);
    }
}
