//! Structural COMBINE execution boundary.
//!
//! Raw material is bulk/theoretical until instantiated. Only structural
//! units and bonds are physical. Placement is supplied by the organism.

use crate::combine::{
    bond_strength, evaluate_formation, experimental_combine_work_cost,
    experimental_interaction, ExperimentalInteraction, FormationEvaluation,
};
use crate::contact::{connection_pair_candidates_cached, ConnectionCompatibilityCache};
use crate::resources::{BaseResource, ResourceProperties};
use crate::structure::{Bond, OrganismStructure, Placement, StructuralUnit};

const COMBINE_CONTACT_TOLERANCE: f64 = 1.0;
const EPSILON: f64 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructuralCombineResult {
    pub unit_a: usize,
    pub unit_b: usize,
    pub point_a: usize,
    pub point_b: usize,
    pub interaction: ExperimentalInteraction,
    pub work_cost: f64,
    pub formation_threshold: f64,
    pub investment: f64,
    pub surplus: f64,
    pub bond_strength: f64,
    pub bond_energy: f64,
    pub bond_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuralCombineError {
    MissingUnit,
    NoGeometricallyEligibleCandidate,
    NonFiniteInvestment,
    InsufficientInvestment,
    NonFiniteWorkCost,
    NonFiniteBondStrength,
    BondGeometryRejected,
    UnfavorableInteraction,
}

/// Calculate the exact minimum investment required by the authoritative
/// COMBINE equations. Runtime and reproductive construction share this
/// calculation so neither path invents or duplicates an energy-payment rule.
pub(crate) fn required_investment(
    props_a: ResourceProperties,
    props_b: ResourceProperties,
    evaluation: FormationEvaluation,
    water_field: f64,
) -> Result<(ExperimentalInteraction, f64, f64), StructuralCombineError> {
    let interaction = experimental_interaction(props_a, props_b, evaluation.candidate, water_field);
    if interaction.direction <= 0.0 || interaction.magnitude <= EPSILON {
        return Err(StructuralCombineError::UnfavorableInteraction);
    }

    let work_cost =
        experimental_combine_work_cost(props_a, props_b, evaluation.candidate, water_field);
    if !work_cost.is_finite() {
        return Err(StructuralCombineError::NonFiniteWorkCost);
    }

    let energy_paid = work_cost.max(evaluation.threshold);
    if !energy_paid.is_finite() || energy_paid < 0.0 {
        return Err(StructuralCombineError::NonFiniteWorkCost);
    }
    Ok((interaction, work_cost, energy_paid))
}

pub fn execute(
    structure: &mut OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    investment: f64,
    water_field: f64,
) -> Result<StructuralCombineResult, StructuralCombineError> {
    if structure.units.get(unit_a).is_none() || structure.units.get(unit_b).is_none() {
        return Err(StructuralCombineError::MissingUnit);
    }
    if unit_a == unit_b || !investment.is_finite() {
        return if unit_a == unit_b {
            Err(StructuralCombineError::NoGeometricallyEligibleCandidate)
        } else {
            Err(StructuralCombineError::NonFiniteInvestment)
        };
    }

    let candidate = *connection_pair_candidates_cached(structure, unit_a, unit_b, catalog, cache)
        .iter()
        .filter(|c| c.distance <= COMBINE_CONTACT_TOLERANCE && c.available_a && c.available_b)
        .max_by(|a, b| {
            a.facing
                .partial_cmp(&b.facing)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.distance
                        .partial_cmp(&a.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| b.point_a.cmp(&a.point_a))
                .then_with(|| b.point_b.cmp(&a.point_b))
        })
        .ok_or(StructuralCombineError::NoGeometricallyEligibleCandidate)?;

    let a = structure.units[unit_a]
        .properties(catalog)
        .ok_or(StructuralCombineError::MissingUnit)?;
    let b = structure.units[unit_b]
        .properties(catalog)
        .ok_or(StructuralCombineError::MissingUnit)?;
    let formation = evaluate_formation(candidate, a.cohesion, b.cohesion);
    let (interaction, work_cost, _required) =
        required_investment(*a, *b, formation, water_field)?;

    if investment < formation.threshold.max(work_cost) {
        return Err(StructuralCombineError::InsufficientInvestment);
    }
    let surplus = investment - formation.threshold;
    let bond_strength = bond_strength(*a, *b);
    if !bond_strength.is_finite() {
        return Err(StructuralCombineError::NonFiniteBondStrength);
    }

    // Surplus investment becomes stored bond energy. It is now part of the
    // structural bond state and is never reconstructed from resource potential
    // energy during BREAK.
    let bond_energy = surplus.max(0.0);
    let bond_index = crate::contact::try_add_bond(
        structure,
        Bond {
            unit_a,
            point_a: candidate.point_a,
            unit_b,
            point_b: candidate.point_b,
            strength: bond_strength,
            bond_energy,
        },
        catalog,
    )
    .map_err(|_| StructuralCombineError::BondGeometryRejected)?;

    Ok(StructuralCombineResult {
        unit_a,
        unit_b,
        point_a: candidate.point_a,
        point_b: candidate.point_b,
        interaction,
        work_cost,
        formation_threshold: formation.threshold,
        investment,
        surplus,
        bond_strength,
        bond_energy,
        bond_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn failed_combine_does_not_mutate_structure() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let mut cache = ConnectionCompatibilityCache::new();
        let result = execute(&mut structure, a, a, &catalog, &mut cache, 100.0, 0.0);
        assert_eq!(
            result,
            Err(StructuralCombineError::NoGeometricallyEligibleCandidate)
        );
        assert!(structure.bonds.is_empty());
    }

    #[test]
    fn combine_rejects_connection_points_more_than_one_structural_unit_apart() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 3.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let mut cache = ConnectionCompatibilityCache::new();
        let result = execute(&mut structure, a, b, &catalog, &mut cache, 100.0, 0.0);
        assert_eq!(
            result,
            Err(StructuralCombineError::NoGeometricallyEligibleCandidate)
        );
        assert!(structure.bonds.is_empty());
    }
}
