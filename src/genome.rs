use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::resources::Material;
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
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
    #[serde(default = "default_juvenile_blueprint")]
    pub juvenile_blueprint: StructuralBlueprint,
    #[serde(default = "default_structural_blueprint")]
    pub structural_blueprint: StructuralBlueprint,
}

impl Genome {
    pub fn trait_value(&self, name: &str, default: f64) -> f64 {
        self.traits
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.value)
            .unwrap_or(default)
    }
    pub fn mass_affinity(&self) -> f64 { self.trait_value("mass_affinity", 0.0).clamp(-1.0, 1.0) }
    pub fn potential_energy_affinity(&self) -> f64 {
        self.trait_value("potential_energy_affinity", 0.0).clamp(-1.0, 1.0)
    }
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
        let mut mutation_probability_sum = 0.0;
        let mut mutation_sigma_sum = 0.0;
        for t in &mut self.traits {
            if rng.gen::<f64>() < t.mutation_probability.clamp(1e-6, 0.25) {
                t.value += rng.gen_range(-1.0..1.0) * t.mutation_sigma.max(0.0);
            }
            if rng.gen::<f64>() < 0.001 {
                t.mutation_probability =
                    (t.mutation_probability * rng.gen_range(0.5..1.5)).clamp(1e-6, 0.1);
            }
            mutation_probability_sum += t.mutation_probability;
            mutation_sigma_sum += t.mutation_sigma.max(0.0);
        }
        let count = self.traits.len().max(1) as f64;
        let structural_probability = (mutation_probability_sum / count).clamp(1e-6, 0.25);
        let structural_sigma = (mutation_sigma_sum / count).clamp(1e-6, 1.0);
        self.mutate_structural_blueprint(rng, structural_probability, structural_sigma);
    }

    fn mutate_structural_blueprint(
        &mut self,
        rng: &mut ChaCha8Rng,
        mutation_probability: f64,
        mutation_sigma: f64,
    ) {
        let original = self.structural_blueprint.clone();
        let probability = mutation_probability.clamp(0.0, 1.0);
        let sigma = mutation_sigma.max(0.0);
        for element in &mut self.structural_blueprint.elements {
            if rng.gen::<f64>() >= probability { continue; }
            element.placement.x += rng.gen_range(-1.0..1.0) * sigma;
            element.placement.y += rng.gen_range(-1.0..1.0) * sigma;
            element.placement.rotation_radians += rng.gen_range(-1.0..1.0) * sigma;
        }
        if !self.structural_blueprint.is_valid() { self.structural_blueprint = original; }
    }
}

fn trait_def(name: &str, value: f64, sigma: f64) -> TraitDef {
    TraitDef { name: name.into(), value, mutation_probability: 0.001, mutation_sigma: sigma }
}

fn seed_wall_material() -> Material {
    Material { parts: vec![("Nitrogen".into(), 1.0)], internal_bonds: Vec::new() }
}

fn seed_interface_material() -> Material {
    Material { parts: vec![("Hydrogen".into(), 1.0)], internal_bonds: Vec::new() }
}

fn square_shell(elements: &mut Vec<BlueprintElement>, side: f64, thickness: f64) {
    let half_wall = side / 2.0;
    let half_thickness = thickness / 2.0;
    let offset = half_wall + half_thickness;
    elements.extend([
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: 0.0, y: offset, rotation_radians: 0.0 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: -offset, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: offset, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: 0.0, y: -offset, rotation_radians: 0.0 } },
    ]);
}

fn shell_connections(start: usize, connections: &mut Vec<BlueprintConnection>) {
    connections.extend([
        BlueprintConnection { element_a: start, element_b: start + 1 },
        BlueprintConnection { element_a: start, element_b: start + 2 },
        BlueprintConnection { element_a: start + 1, element_b: start + 3 },
        BlueprintConnection { element_a: start + 2, element_b: start + 3 },
    ]);
}

fn interface_connectors(
    elements: &mut Vec<BlueprintElement>,
    inner_offset: f64,
    outer_offset: f64,
) {
    let center = (inner_offset + outer_offset) / 2.0;
    elements.extend([
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: 0.0, y: center, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: -center, y: 0.0, rotation_radians: 0.0 } },
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: center, y: 0.0, rotation_radians: 0.0 } },
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: 0.0, y: -center, rotation_radians: std::f64::consts::FRAC_PI_2 } },
    ]);
}

fn default_juvenile_blueprint() -> StructuralBlueprint {
    let thickness = 0.330_719;
    let core_side = 1.511_858;
    let connector_length = 0.797_884;
    let core_offset = (core_side + thickness) / 2.0;
    let core_outer = core_offset + thickness / 2.0;
    let outer_inner = core_outer + connector_length;
    let outer_side = 2.0 * (outer_inner + thickness / 2.0) - thickness;
    let outer_offset = (outer_side + thickness) / 2.0;

    let mut elements = Vec::new();
    let mut connections = Vec::new();
    square_shell(&mut elements, core_side, thickness);
    square_shell(&mut elements, outer_side, thickness);
    shell_connections(0, &mut connections);
    shell_connections(4, &mut connections);
    interface_connectors(&mut elements, core_outer, outer_inner);
    connections.extend([
        BlueprintConnection { element_a: 0, element_b: 8 },
        BlueprintConnection { element_a: 4, element_b: 8 },
        BlueprintConnection { element_a: 1, element_b: 9 },
        BlueprintConnection { element_a: 5, element_b: 9 },
        BlueprintConnection { element_a: 2, element_b: 10 },
        BlueprintConnection { element_a: 6, element_b: 10 },
        BlueprintConnection { element_a: 3, element_b: 11 },
        BlueprintConnection { element_a: 7, element_b: 11 },
    ]);
    let _ = outer_offset;
    StructuralBlueprint::with_core_elements(elements, connections, vec![0, 1, 2, 3])
}

fn default_structural_blueprint() -> StructuralBlueprint {
    let mut elements = default_juvenile_blueprint().elements;
    let mut connections = default_juvenile_blueprint().connections;
    let thickness = 0.330_719;
    let first_outer_side = 3.769_064;
    let connector_length = 0.797_884;
    let first_outer_offset = (first_outer_side + thickness) / 2.0;
    let first_outer_outer = first_outer_offset + thickness / 2.0;
    let second_outer_inner = first_outer_outer + connector_length;
    let second_outer_side = 2.0 * (second_outer_inner + thickness / 2.0) - thickness;
    let start = elements.len();
    square_shell(&mut elements, second_outer_side, thickness);
    shell_connections(start, &mut connections);
    interface_connectors(&mut elements, first_outer_outer, second_outer_inner);
    connections.extend([
        BlueprintConnection { element_a: 4, element_b: start + 8 },
        BlueprintConnection { element_a: start, element_b: start + 8 },
        BlueprintConnection { element_a: 5, element_b: start + 9 },
        BlueprintConnection { element_a: start + 1, element_b: start + 9 },
        BlueprintConnection { element_a: 6, element_b: start + 10 },
        BlueprintConnection { element_a: start + 2, element_b: start + 10 },
        BlueprintConnection { element_a: 7, element_b: start + 11 },
        BlueprintConnection { element_a: start + 3, element_b: start + 11 },
    ]);
    StructuralBlueprint::with_core_elements(elements, connections, vec![0, 1, 2, 3])
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
        juvenile_blueprint: default_juvenile_blueprint(),
        structural_blueprint: default_structural_blueprint(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::analyze_genome_cavity;
    use crate::resources::default_catalog;
    use rand::SeedableRng;

    #[test]
    fn juvenile_blueprint_has_sealed_genome_and_extracore_structure() {
        let g = initial_genome();
        let b = &g.juvenile_blueprint;
        assert_eq!(b.elements.len(), 12);
        assert_eq!(b.core_elements, vec![0, 1, 2, 3]);
        assert!(b.validate().is_ok());
        let catalog = default_catalog();
        let structure = b.realize(&catalog).unwrap();
        let cavity = analyze_genome_cavity(&structure, &catalog, &b.core_elements)
            .unwrap()
            .expect("juvenile genome cavity must be sealed");
        assert!(cavity.qualifies());
        assert!(structure.units.len() > b.core_elements.len());
    }

    #[test]
    fn mature_blueprint_contains_the_viable_juvenile_architecture() {
        let g = initial_genome();
        assert!(g.structural_blueprint.elements.len() > g.juvenile_blueprint.elements.len());
        assert_eq!(g.structural_blueprint.core_elements, g.juvenile_blueprint.core_elements);
        assert!(g.structural_blueprint.validate().is_ok());
    }

    #[test]
    fn juvenile_blueprint_is_connected() {
        assert!(initial_genome().juvenile_blueprint.is_connected());
    }

    #[test]
    fn structural_mutation_preserves_developmental_blueprint_validity() {
        let mut genome = initial_genome();
        let before = genome.structural_blueprint.clone();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        genome.mutate_structural_blueprint(&mut rng, 1.0, 0.05);
        assert!(genome.structural_blueprint.is_valid());
        assert_ne!(genome.structural_blueprint, before);
        assert_eq!(genome.juvenile_blueprint.core_elements, vec![0, 1, 2, 3]);
    }

    #[test]
    fn invalid_structural_mutation_is_rejected_transactionally() {
        let mut genome = initial_genome();
        let before = genome.structural_blueprint.clone();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        genome.mutate_structural_blueprint(&mut rng, 1.0, f64::INFINITY);
        assert_eq!(genome.structural_blueprint, before);
    }
}
