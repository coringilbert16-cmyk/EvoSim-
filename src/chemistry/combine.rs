#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! COMBINE support: deterministic recipe caching, locked formation threshold, and
//! resource-derived bond strength.
use crate::chemistry::{attraction, interaction_potential, CHEMICAL_D_MAX, CHEMICAL_K};
use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::resources::{combine_materials, BaseResource, Material, ResourceProperties};
use crate::structure::{formation_threshold, OrganismStructure};
use std::collections::HashMap;
const EPSILON: f64 = 1e-12;
pub const EXPERIMENTAL_BOND_STRENGTH_SCALE: f64 = 1.0;
pub const EXPERIMENTAL_MAX_BOND_STRENGTH: f64 = 1.0;
/// Chemistry-facing pair interaction. The tuning constants are supplied by the caller
/// so this layer does not invent biological chemistry parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChemicalInteraction {
    pub static_potential: f64,
    pub attraction: f64,
}

pub fn chemical_interaction(
    a: ResourceProperties,
    b: ResourceProperties,
    candidate: ConnectionPairCandidate,
    k: f64,
    d_max: f64,
    contact_radius: f64,
    max_force: f64,
) -> Option<ChemicalInteraction> {
    let position_a = a.chemical_position?;
    let position_b = b.chemical_position?;
    let static_potential = interaction_potential(position_a, position_b, k, d_max)?;
    let attraction = attraction(
        position_a,
        position_b,
        candidate.distance.max(0.0),
        k,
        d_max,
        contact_radius,
        max_force,
    )?;
    Some(ChemicalInteraction {
        static_potential,
        attraction,
    })
}

/// Remaining physical work required to bring an eligible chemical interface
/// from its current separation to contact.
///
/// Attraction is a force, not an energy deposit. Work is therefore the integral
/// of that force over the remaining approach distance. If the constituents are
/// already touching, no additional formation work is charged; the work that
/// occurred during their earlier approach belongs to physical motion.
pub fn formation_work_cost(a: ResourceProperties, b: ResourceProperties, distance: f64) -> f64 {
    if !distance.is_finite() || distance < 0.0 {
        return f64::NAN;
    }
    let (Some(position_a), Some(position_b)) = (a.chemical_position, b.chemical_position) else {
        return 0.0;
    };
    let Some(static_potential) =
        interaction_potential(position_a, position_b, CHEMICAL_K, CHEMICAL_D_MAX)
    else {
        return f64::NAN;
    };
    let d = distance.min(crate::chemistry::CHEMICAL_CONTACT_RADIUS);
    let radius = crate::chemistry::CHEMICAL_CONTACT_RADIUS;
    let force_scale = crate::chemistry::CHEMICAL_MAX_FORCE;
    // F(x) = static_potential * force_scale * (1 - x/radius)^2.
    // Integrate from x=0 to x=d to obtain the work still available to be
    // performed by attraction while the interface closes.
    let integral = d - (d * d / radius) + (d * d * d / (3.0 * radius * radius));
    let work = static_potential * force_scale * integral;
    if work.is_finite() {
        work.max(0.0)
    } else {
        f64::NAN
    }
}

/// Intrinsic potential stored by a newly formed structural bond.
///
/// Chemical materials derive this potential from their static chemical
/// interaction. Materials without a chemical position fall back to their
/// physically defined cohesion-derived bond strength. This is stored potential,
/// not formation work, and is therefore accounted for separately.
pub fn intrinsic_bond_potential(
    a: ResourceProperties,
    b: ResourceProperties,
    strength: f64,
) -> f64 {
    if !strength.is_finite() || strength < 0.0 {
        return f64::NAN;
    }
    let normalized = match (a.chemical_position, b.chemical_position) {
        (Some(position_a), Some(position_b)) => {
            interaction_potential(position_a, position_b, CHEMICAL_K, CHEMICAL_D_MAX).unwrap_or(0.0)
        }
        _ => strength.clamp(0.0, 1.0),
    };
    crate::chemistry::normalized_chemistry_to_energy(normalized).unwrap_or(f64::NAN)
}
pub fn experimental_bond_strength(surplus: f64) -> f64 {
    if !surplus.is_finite() || surplus <= 0.0 {
        return 0.0;
    }
    let scale = EXPERIMENTAL_BOND_STRENGTH_SCALE.max(EPSILON);
    let max_strength = EXPERIMENTAL_MAX_BOND_STRENGTH.max(0.0);
    max_strength * (1.0 - (-surplus / scale).exp())
}
pub fn bond_strength(a: ResourceProperties, b: ResourceProperties) -> f64 {
    if !a.cohesion.is_finite() || !b.cohesion.is_finite() {
        return 0.0;
    }
    (a.cohesion.clamp(0.0, 1.0) * b.cohesion.clamp(0.0, 1.0)).sqrt()
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MaterialIdentityKey {
    parts: Vec<(String, u64)>,
    internal_bonds: Vec<(usize, usize)>,
}
impl MaterialIdentityKey {
    fn from_material(material: &Material) -> Self {
        let parts = material
            .parts
            .iter()
            .filter(|(_, amount)| *amount > EPSILON)
            .map(|(name, amount)| (name.clone(), amount.to_bits()))
            .collect();
        let mut internal_bonds = material
            .internal_bonds
            .iter()
            .map(|bond| (bond.part_a.min(bond.part_b), bond.part_a.max(bond.part_b)))
            .collect::<Vec<_>>();
        internal_bonds.sort_unstable();
        Self {
            parts,
            internal_bonds,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialRecipeKey {
    materials: Vec<MaterialIdentityKey>,
}
impl MaterialRecipeKey {
    pub fn from_material(material: &Material) -> Self {
        Self {
            materials: vec![MaterialIdentityKey::from_material(material)],
        }
    }
    pub fn from_inputs(inputs: &[Material]) -> Self {
        let mut materials = inputs
            .iter()
            .map(MaterialIdentityKey::from_material)
            .collect::<Vec<_>>();
        materials.sort_by(|a, b| {
            a.parts
                .cmp(&b.parts)
                .then_with(|| a.internal_bonds.cmp(&b.internal_bonds))
        });
        Self { materials }
    }
}
#[derive(Default)]
pub struct CombineCache {
    results: HashMap<MaterialRecipeKey, Material>,
}
impl CombineCache {
    pub fn new() -> Self {
        Self::default()
    }
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
        self.results.clear()
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormationEvaluation {
    pub candidate: ConnectionPairCandidate,
    pub threshold: f64,
}
pub fn evaluate_formation(
    candidate: ConnectionPairCandidate,
    cohesion_a: f64,
    cohesion_b: f64,
) -> FormationEvaluation {
    FormationEvaluation {
        candidate,
        threshold: formation_threshold(cohesion_a, cohesion_b, candidate.load_a, candidate.load_b),
    }
}
pub fn formation_succeeds(evaluation: FormationEvaluation, investment: f64) -> bool {
    investment.is_finite() && evaluation.threshold.is_finite() && investment >= evaluation.threshold
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombineEvaluationError {
    NonFiniteWorkCost,
    InvalidFormationThreshold,
}
pub fn formation_cost(
    a: ResourceProperties,
    b: ResourceProperties,
    evaluation: FormationEvaluation,
) -> Result<(f64, f64), CombineEvaluationError> {
    let work = formation_work_cost(a, b, evaluation.candidate.distance);
    if !work.is_finite() || work < 0.0 {
        return Err(CombineEvaluationError::NonFiniteWorkCost);
    }
    if !evaluation.threshold.is_finite() || evaluation.threshold < 0.0 {
        return Err(CombineEvaluationError::InvalidFormationThreshold);
    }
    Ok((work, evaluation.threshold))
}
pub fn eligible_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<ConnectionPairCandidate> {
    let Some(a) = structure.units.get(unit_a) else {
        return Vec::new();
    };
    let Some(b) = structure.units.get(unit_b) else {
        return Vec::new();
    };
    let is_water_only = |material: &Material| {
        !material.parts.is_empty()
            && material.parts.iter().all(|(name, amount)| {
                *amount > EPSILON
                    && catalog
                        .iter()
                        .find(|resource| resource.name == *name)
                        .is_some_and(|resource| {
                            resource.name == "Water"
                                && resource.physical_state == crate::resources::PhysicalState::Fluid
                        })
            })
    };
    let both_fluid = is_water_only(&a.material) && is_water_only(&b.material);
    if both_fluid {
        return Vec::new();
    }
    crate::contact::connection_pair_candidates_cached(structure, unit_a, unit_b, catalog, cache)
        .into_iter()
        .filter(|candidate| candidate.available_a && candidate.available_b)
        .collect()
}
pub fn evaluate_candidates(
    structure: &OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
) -> Vec<FormationEvaluation> {
    let Some(unit_a_ref) = structure.units.get(unit_a) else {
        return Vec::new();
    };
    let Some(unit_b_ref) = structure.units.get(unit_b) else {
        return Vec::new();
    };
    let Some(cohesion_a) = unit_a_ref.properties(catalog).map(|p| p.cohesion) else {
        return Vec::new();
    };
    let Some(cohesion_b) = unit_b_ref.properties(catalog).map(|p| p.cohesion) else {
        return Vec::new();
    };
    eligible_candidates(structure, unit_a, unit_b, catalog, cache)
        .into_iter()
        .map(|candidate| evaluate_formation(candidate, cohesion_a, cohesion_b))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::{
        Bond, BondEndpoint, ConnectionEndpoint, PhysicalConstituentId, Placement, StructuralUnit,
    };
    fn carbon(amount: f64) -> Material {
        Material::free_base("Carbon", amount)
    }
    fn methane(amount: f64) -> Material {
        Material::free_base("Methane", amount)
    }
    fn candidate(load_a: f64, load_b: f64) -> ConnectionPairCandidate {
        ConnectionPairCandidate {
            endpoint_a: ConnectionEndpoint::Corner { point_index: 0 },
            endpoint_b: ConnectionEndpoint::Corner { point_index: 0 },
            distance: 0.0,
            facing: 1.0,
            load_a,
            load_b,
            available_a: true,
            available_b: true,
        }
    }
    fn bond(a: usize, ap: usize, b: usize, bp: usize) -> Bond {
        Bond {
            endpoint_a: BondEndpoint::new(
                PhysicalConstituentId(a as u64 + 1),
                ConnectionEndpoint::Corner { point_index: ap },
            ),
            endpoint_b: BondEndpoint::new(
                PhysicalConstituentId(b as u64 + 1),
                ConnectionEndpoint::Corner { point_index: bp },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        }
    }
    #[test]
    fn recipe_key_is_order_independent() {
        assert_eq!(
            MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]),
            MaterialRecipeKey::from_inputs(&[methane(2.0), carbon(1.0)])
        )
    }
    #[test]
    fn different_quantities_do_not_collide() {
        assert_ne!(
            MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(2.0)]),
            MaterialRecipeKey::from_inputs(&[carbon(1.0), methane(3.0)])
        )
    }
    #[test]
    fn chemical_interaction_uses_catalog_positions_not_potential_energy() {
        let a = ResourceProperties {
            mass: 1.0,
            potential_energy: 100.0,
            reactivity: 999.0,
            chemical_position: Some(1.5),
            cohesion: 0.5,
        };
        let b = ResourceProperties {
            potential_energy: 0.0,
            reactivity: 0.0,
            chemical_position: Some(12.5),
            ..a
        };
        let result = chemical_interaction(
            a,
            b,
            candidate(0.0, 0.0),
            CHEMICAL_K,
            CHEMICAL_D_MAX,
            1.0,
            10.0,
        )
        .unwrap();
        assert!(result.static_potential > 0.0);
        assert!((result.attraction - 10.0 * result.static_potential).abs() < 1e-12);
    }

    #[test]
    fn formation_work_is_remaining_chemical_approach_work() {
        let a = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 0.0,
            chemical_position: Some(1.5),
            cohesion: 0.5,
        };
        let b = ResourceProperties {
            mass: 2.0,
            potential_energy: 99.0,
            reactivity: 999.0,
            chemical_position: Some(12.5),
            cohesion: 0.8,
        };
        let at_contact = formation_work_cost(a, b, 0.0);
        let separated = formation_work_cost(a, b, 0.5);
        assert_eq!(at_contact, 0.0);
        assert!(separated > 0.0);
    }

    #[test]
    fn formation_work_is_independent_of_nonchemical_energy_properties() {
        let a = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 0.0,
            chemical_position: Some(1.5),
            cohesion: 0.5,
        };
        let b = ResourceProperties {
            mass: 2.0,
            potential_energy: 99.0,
            reactivity: 999.0,
            chemical_position: Some(12.5),
            cohesion: 0.8,
        };
        let mut altered = b;
        altered.potential_energy = -500.0;
        altered.reactivity = 0.01;
        assert_eq!(
            formation_work_cost(a, b, 0.5),
            formation_work_cost(a, altered, 0.5)
        );
    }

    #[test]
    fn intrinsic_bond_potential_is_separate_from_formation_work() {
        let a = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 0.0,
            chemical_position: Some(1.5),
            cohesion: 0.5,
        };
        let b = ResourceProperties {
            mass: 2.0,
            potential_energy: 99.0,
            reactivity: 999.0,
            chemical_position: Some(12.5),
            cohesion: 0.8,
        };
        let strength = bond_strength(a, b);
        let potential = intrinsic_bond_potential(a, b, strength);
        assert!(potential.is_finite());
        assert!(potential >= 0.0);
    }

    #[test]
    fn intrinsic_bond_strength_is_cohesion_only() {
        let a = ResourceProperties {
            mass: 1.0,
            potential_energy: 1.0,
            reactivity: 0.1,
            chemical_position: None,
            cohesion: 0.8,
        };
        let b = ResourceProperties {
            potential_energy: 10.0,
            reactivity: 4.0,
            chemical_position: None,
            cohesion: 0.2,
            ..a
        };
        assert!((bond_strength(a, b) - (0.8_f64 * 0.2).sqrt()).abs() < 1e-12)
    }
    #[test]
    fn formation_threshold_is_only_an_eligibility_gate() {
        let free = evaluate_formation(candidate(0.0, 0.0), 0.8, 0.4);
        let loaded = evaluate_formation(candidate(1.0, 0.0), 0.8, 0.4);
        assert!((free.threshold - 0.6).abs() < 1e-12);
        assert!(loaded.threshold > free.threshold);
        assert!(!formation_succeeds(free, free.threshold - 1e-9));
        assert!(formation_succeeds(free, free.threshold))
    }
    #[test]
    fn physical_endpoint_bond_round_trips() {
        let b = bond(0, 0, 1, 2);
        let encoded = serde_json::to_string(&b).unwrap();
        let decoded: Bond = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, b)
    }
    #[test]
    fn water_water_bond_candidates_are_forbidden() {
        let catalog = default_catalog();
        let mut s = OrganismStructure::new();
        s.add_unit(StructuralUnit::new(
            "Water",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        s.add_unit(StructuralUnit::new(
            "Water",
            Placement {
                x: 0.8,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let mut cache = ConnectionCompatibilityCache::new();
        assert!(eligible_candidates(&s, 0, 1, &catalog, &mut cache).is_empty());
    }

    #[test]
    fn continuous_candidates_are_supported_by_combine() {
        let catalog = default_catalog();
        let mut s = OrganismStructure::new();
        s.add_unit(StructuralUnit::new(
            "Hydrogen",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        s.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.5,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let mut cache = ConnectionCompatibilityCache::new();
        assert!(eligible_candidates(&s, 0, 1, &catalog, &mut cache)
            .iter()
            .all(|c| matches!(c.endpoint_a, ConnectionEndpoint::LineEndpoint { .. })))
    }
}
