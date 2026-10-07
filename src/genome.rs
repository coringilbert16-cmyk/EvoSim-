#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::developmental_blueprint::{
    default_developmental_blueprint, DevelopmentalFieldBlueprint,
};

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
    /// Inherited energy allocation available to a newly constructed offspring during budding.
    #[serde(default = "default_reproductive_energy_allocation", alias = "juvenile_energy_reserve")]
    pub reproductive_energy_allocation: f64,
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

    /// Inherited drive to investigate unfamiliar perceived stimuli.
    /// EXPERIMENTAL: behavioral calibration value; mutation is inherited.
    pub fn curiosity(&self) -> f64 {
        self.trait_value("curiosity", 0.5).clamp(0.0, 1.0)
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
    pub fn preferred_mass(&self) -> f64 {
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

    /// Movement cadence is the number of ticks between one-unit movement steps.
    /// The inherited value is a continuous speed preference; the physical operation
    /// resolves it to the discrete cadence the simulator can execute.
    pub fn movement_step_interval(&self) -> u64 {
        let speed = self.trait_value("movement_speed", 1.0).clamp(0.25, 1.0);
        (1.0 / speed).round().clamp(1.0, 4.0) as u64
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
        self.developmental_blueprint.mutate(rng);

        for t in &mut self.traits {
            if rng.gen::<f64>() < t.mutation_probability.clamp(1e-6, 0.25) {
                let delta = gaussian_unit(rng) * t.mutation_sigma.max(0.0);
                t.value = if t.name == "size_preference" {
                    // Bell-shaped mutation around the parent's value, bounded to [0, 1].
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
            trait_def("curiosity", 0.5, 0.05),
            trait_def("size_preference", 0.5, 0.05),
            trait_def("processing_efficiency", 0.8, 0.05),
            trait_def("movement_efficiency", 0.8, 0.05),
            trait_def("movement_speed", 1.0, 0.05),
            trait_def("reproductive_investment", 0.5, 0.05),
        ],
        reproductive_energy_allocation: default_reproductive_energy_allocation(),
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
        assert!((genome.preferred_mass() - 30.0).abs() < 1e-9);
    }

}
