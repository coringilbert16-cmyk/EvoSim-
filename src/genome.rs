use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::resources::Material;
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
};

const JUVENILE_LINEAR_SCALE: f64 = 0.40;

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
    /// The complete mature organism architecture. This is the sole
    /// developmental structural authority.
    #[serde(default = "default_structural_blueprint")]
    pub structural_blueprint: StructuralBlueprint,
    /// Deterministically derived juvenile realization target. This is derived
    /// developmental data, not an independent biological architecture.
    #[serde(default = "default_juvenile_blueprint")]
    pub juvenile_blueprint: StructuralBlueprint,
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
        let mut probability_sum = 0.0;
        let mut sigma_sum = 0.0;
        for t in &mut self.traits {
            if rng.gen::<f64>() < t.mutation_probability.clamp(1e-6, 0.25) {
                t.value += rng.gen_range(-1.0..1.0) * t.mutation_sigma.max(0.0);
            }
            if rng.gen::<f64>() < 0.001 {
                t.mutation_probability = (t.mutation_probability * rng.gen_range(0.5..1.5)).clamp(1e-6, 0.1);
            }
            probability_sum += t.mutation_probability;
            sigma_sum += t.mutation_sigma.max(0.0);
        }
        let count = self.traits.len().max(1) as f64;
        self.mutate_structural_blueprint(
            rng,
            (probability_sum / count).clamp(1e-6, 0.25),
            (sigma_sum / count).clamp(1e-6, 1.0),
        );
    }

    /// Structural mutation acts on the adult architecture. The juvenile is
    /// always regenerated from that result, preventing two competing body
    /// plans from evolving independently.
    fn mutate_structural_blueprint(&mut self, rng: &mut ChaCha8Rng, mutation_probability: f64, mutation_sigma: f64) {
        let original = self.structural_blueprint.clone();
        let probability = mutation_probability.clamp(0.0, 1.0);
        let sigma = mutation_sigma.max(0.0);
        for element in &mut self.structural_blueprint.elements {
            if rng.gen::<f64>() >= probability { continue; }
            element.placement.x += rng.gen_range(-1.0..1.0) * sigma;
            element.placement.y += rng.gen_range(-1.0..1.0) * sigma;
            element.placement.rotation_radians += rng.gen_range(-1.0..1.0) * sigma;
        }
        if !self.structural_blueprint.is_valid() {
            self.structural_blueprint = original;
            return;
        }
        self.juvenile_blueprint = derive_scaled_juvenile(&self.structural_blueprint);
    }
}

fn default_juvenile_reserve() -> Material { Material::free_base("Hydrogen", 1.0) }
fn default_juvenile_energy_reserve() -> f64 { 16.0 }
fn trait_def(name: &str, value: f64, sigma: f64) -> TraitDef {
    TraitDef { name: name.into(), value, mutation_probability: 0.001, mutation_sigma: sigma }
}
fn seed_wall_material() -> Material { Material::free_base("Nitrogen", 1.0) }
fn seed_interface_material() -> Material { Material::free_base("Hydrogen", 1.0) }

fn add_core_shell(elements: &mut Vec<BlueprintElement>, connections: &mut Vec<BlueprintConnection>) {
    let side = 1.511_858;
    let thickness = 0.330_719;
    let offset = (side + thickness) / 2.0;
    elements.extend([
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: 0.0, y: offset, rotation_radians: 0.0 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: -offset, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: offset, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: 0.0, y: -offset, rotation_radians: 0.0 } },
    ]);
    connections.extend([
        BlueprintConnection { element_a: 0, element_b: 1 },
        BlueprintConnection { element_a: 0, element_b: 2 },
        BlueprintConnection { element_a: 1, element_b: 3 },
        BlueprintConnection { element_a: 2, element_b: 3 },
    ]);
}

fn add_outer_shell(elements: &mut Vec<BlueprintElement>, connections: &mut Vec<BlueprintConnection>) {
    let half_segment = 1.511_858 / 2.0;
    let offset = 1.677_2175;
    let start = elements.len();
    elements.extend([
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: -half_segment, y: offset, rotation_radians: 0.0 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: half_segment, y: offset, rotation_radians: 0.0 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: -offset, y: -half_segment, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: -offset, y: half_segment, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: offset, y: -half_segment, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: offset, y: half_segment, rotation_radians: std::f64::consts::FRAC_PI_2 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: -half_segment, y: -offset, rotation_radians: 0.0 } },
        BlueprintElement { material: seed_wall_material(), placement: BlueprintPlacement { x: half_segment, y: -offset, rotation_radians: 0.0 } },
    ]);
    connections.extend([
        BlueprintConnection { element_a: start, element_b: start + 1 },
        BlueprintConnection { element_a: start + 1, element_b: start + 5 },
        BlueprintConnection { element_a: start + 5, element_b: start + 4 },
        BlueprintConnection { element_a: start + 4, element_b: start + 7 },
        BlueprintConnection { element_a: start + 7, element_b: start + 6 },
        BlueprintConnection { element_a: start + 6, element_b: start + 2 },
        BlueprintConnection { element_a: start + 2, element_b: start + 3 },
        BlueprintConnection { element_a: start + 3, element_b: start },
    ]);
}

fn add_interface_connectors(elements: &mut Vec<BlueprintElement>, connections: &mut Vec<BlueprintConnection>) {
    let inner_outer = 1.086_648;
    let outer_inner = 1.511_858;
    let length = 0.797_884;
    let gap = outer_inner - inner_outer;
    let tangent = (length * length - gap * gap).sqrt();
    let center = (inner_outer + outer_inner) / 2.0;
    let start = elements.len();
    elements.extend([
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: 0.0, y: center, rotation_radians: gap.atan2(-tangent) } },
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: -center, y: 0.0, rotation_radians: (-tangent).atan2(-gap) } },
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: center, y: 0.0, rotation_radians: tangent.atan2(gap) } },
        BlueprintElement { material: seed_interface_material(), placement: BlueprintPlacement { x: 0.0, y: -center, rotation_radians: (-gap).atan2(tangent) } },
    ]);
    connections.extend([
        BlueprintConnection { element_a: 0, element_b: start },
        BlueprintConnection { element_a: 4, element_b: start },
        BlueprintConnection { element_a: 1, element_b: start + 1 },
        BlueprintConnection { element_a: 6, element_b: start + 1 },
        BlueprintConnection { element_a: 2, element_b: start + 2 },
        BlueprintConnection { element_a: 8, element_b: start + 2 },
        BlueprintConnection { element_a: 3, element_b: start + 3 },
        BlueprintConnection { element_a: 10, element_b: start + 3 },
    ]);
}

/// Derive the juvenile from the complete adult architecture. Rigid constituent
/// geometry is never scaled or mutated. Only the developmental target layout is
/// contracted; the physical construction solver must realize those immutable
/// pieces through actual contact and COMBINE rules.
fn derive_scaled_juvenile(adult: &StructuralBlueprint) -> StructuralBlueprint {
    let mut juvenile = adult.clone();
    for element in &mut juvenile.elements {
        element.placement.x *= JUVENILE_LINEAR_SCALE;
        element.placement.y *= JUVENILE_LINEAR_SCALE;
    }
    juvenile
}

fn default_structural_blueprint() -> StructuralBlueprint {
    let mut elements = Vec::new();
    let mut connections = Vec::new();
    add_core_shell(&mut elements, &mut connections);
    add_outer_shell(&mut elements, &mut connections);
    add_interface_connectors(&mut elements, &mut connections);
    StructuralBlueprint::with_core_elements(elements, connections, vec![0, 1, 2, 3])
}

fn default_juvenile_blueprint() -> StructuralBlueprint {
    derive_scaled_juvenile(&default_structural_blueprint())
}

pub fn initial_genome() -> Genome {
    let structural_blueprint = default_structural_blueprint();
    let juvenile_blueprint = derive_scaled_juvenile(&structural_blueprint);
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
        structural_blueprint,
        juvenile_blueprint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cavity::analyze_genome_cavity;
    use crate::resources::default_catalog;
    use rand::SeedableRng;

    #[test]
    fn adult_is_the_complete_developmental_authority() {
        let g = initial_genome();
        assert!(g.structural_blueprint.is_valid());
        assert!(g.structural_blueprint.realize(&default_catalog()).is_ok());
    }

    #[test]
    fn juvenile_is_derived_from_adult_without_deleting_architecture() {
        let g = initial_genome();
        assert_eq!(g.juvenile_blueprint.elements.len(), g.structural_blueprint.elements.len());
        assert_eq!(g.juvenile_blueprint.connections, g.structural_blueprint.connections);
        assert_eq!(g.juvenile_blueprint.core_elements, g.structural_blueprint.core_elements);
        for (adult, juvenile) in g.structural_blueprint.elements.iter().zip(&g.juvenile_blueprint.elements) {
            assert_eq!(adult.material, juvenile.material);
            assert!((juvenile.placement.x - adult.placement.x * JUVENILE_LINEAR_SCALE).abs() < 1e-12);
            assert!((juvenile.placement.y - adult.placement.y * JUVENILE_LINEAR_SCALE).abs() < 1e-12);
            assert_eq!(adult.placement.rotation_radians, juvenile.placement.rotation_radians);
        }
    }

    #[test]
    fn juvenile_retains_the_same_enclosing_architecture() {
        let g = initial_genome();
        let c = default_catalog();
        assert!(g.juvenile_blueprint.is_connected());
        let s = g.juvenile_blueprint.realize(&c).unwrap();
        assert!(analyze_genome_cavity(&s, &c, &g.juvenile_blueprint.core_elements)
            .unwrap()
            .is_some_and(|x| x.qualifies()));
    }

    #[test]
    fn structural_mutation_regenerates_juvenile_from_adult() {
        let mut g = initial_genome();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        g.mutate_structural_blueprint(&mut rng, 1.0, 0.001);
        assert!(g.structural_blueprint.is_valid());
        for (adult, juvenile) in g.structural_blueprint.elements.iter().zip(&g.juvenile_blueprint.elements) {
            assert!((juvenile.placement.x - adult.placement.x * JUVENILE_LINEAR_SCALE).abs() < 1e-12);
            assert!((juvenile.placement.y - adult.placement.y * JUVENILE_LINEAR_SCALE).abs() < 1e-12);
        }
    }

    #[test]
    fn juvenile_reserve_is_not_structural_authority() {
        let g = initial_genome();
        assert!(g.juvenile_reserve.is_valid());
        assert_eq!(g.juvenile_reserve.total_amount(), 1.0);
        assert!(g.juvenile_energy_reserve.is_finite() && g.juvenile_energy_reserve > 0.0);
    }

    #[test]
    fn invalid_structural_mutation_is_rejected_transactionally() {
        let mut g = initial_genome();
        let before_adult = g.structural_blueprint.clone();
        let before_juvenile = g.juvenile_blueprint.clone();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        g.mutate_structural_blueprint(&mut rng, 1.0, f64::INFINITY);
        assert_eq!(g.structural_blueprint, before_adult);
        assert_eq!(g.juvenile_blueprint, before_juvenile);
    }
}
