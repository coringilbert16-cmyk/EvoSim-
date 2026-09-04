use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::resources::{ConnectionPoint, Shape};
use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, BlueprintGeometry, ConnectionRegion, StructuralBlueprint};
use crate::structural_material::StructuralMaterial;
use crate::structure::Placement;

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
    /// The inherited physical body plan. This is genotype-level structure,
    /// not a procedural construction hint or a description of current state.
    #[serde(default)]
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
    pub fn juvenile_mass(&self) -> f64 { self.trait_value("juvenile_mass", 4.0).clamp(1.0, 40.0) }
    pub fn adult_mass(&self) -> f64 { self.trait_value("adult_mass", 16.0).clamp(4.0, 80.0) }

    pub fn mutate(&mut self, rng: &mut ChaCha8Rng) {
        for t in &mut self.traits {
            if rng.gen::<f64>() < t.mutation_probability.clamp(1e-6, 0.25) {
                let delta = rng.gen_range(-1.0..1.0) * t.mutation_sigma.max(0.0);
                t.value += delta;
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

/// Produce a completed, physically realizable seed body plan. The seed index
/// changes the orientation of the water-bearing attachment, giving different
/// valid initial configurations without introducing anatomical role tags.
pub fn initial_blueprint(seed_variant: u64) -> StructuralBlueprint {
    let catalog = crate::resources::default_catalog();
    let carbon_shape = catalog.iter().find(|r| r.name == "Carbon").map(|r| r.shape.clone()).unwrap_or(Shape { form: crate::resources::Form::RegularPolygon { sides: 6, radius: 0.438_691 } });
    let water_shape = catalog.iter().find(|r| r.name == "Water").map(|r| r.shape.clone()).unwrap_or(Shape { form: crate::resources::Form::Fluid { nominal_area: crate::resources::NOMINAL_UNIT_AREA } });
    let carbon_radius = carbon_shape.bounding_radius();
    let water_radius = water_shape.bounding_radius();
    let angle = (seed_variant % 3) as f64 * std::f64::consts::TAU / 3.0;
    let ux = angle.cos();
    let uy = angle.sin();
    let carbon_point = ConnectionPoint { x: carbon_radius * ux, y: carbon_radius * uy, direction_radians: angle };
    let water_point = ConnectionPoint { x: -water_radius * ux, y: -water_radius * uy, direction_radians: angle + std::f64::consts::PI };
    let water_placement = Placement { x: (carbon_radius + water_radius) * ux, y: (carbon_radius + water_radius) * uy, rotation_radians: 0.0 };

    let mut carbon_geometry = BlueprintGeometry::single(carbon_shape);
    let mut water_geometry = BlueprintGeometry::single(water_shape);
    carbon_geometry.connection_regions.push(ConnectionRegion { point: carbon_point });
    water_geometry.connection_regions.push(ConnectionRegion { point: water_point });

    StructuralBlueprint::new(
        vec![
            BlueprintElement { material: StructuralMaterial::single("Carbon"), geometry: carbon_geometry, placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 } },
            BlueprintElement { material: StructuralMaterial::single("Water"), geometry: water_geometry, placement: water_placement },
        ],
        vec![BlueprintConnection { element_a: 0, point_a: 6, element_b: 1, point_b: 0 }],
    )
}

pub fn initial_genome() -> Genome {
    initial_genome_variant(0)
}

pub fn initial_genome_variant(seed_variant: u64) -> Genome {
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
            trait_def("juvenile_mass", 4.0, 0.2),
            trait_def("adult_mass", 16.0, 0.4),
        ],
        structural_blueprint: initial_blueprint(seed_variant),
    }
}
