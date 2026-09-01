//! Structural COMBINE execution boundary.
//!
//! This file intentionally contains the execution primitive separately from
//! the existing experimental equation/cache module. Raw material is bulk and
//! theoretical until instantiated; only StructuralUnits and Bonds are
//! physical. Placement is chosen by the organism.

use crate::combine::{experimental_bond_strength, experimental_combine_work_cost, experimental_interaction, evaluate_formation, evaluate_bond_strength, FormationEvaluation};
use crate::contact::{eligible_connection_candidates, ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, OrganismStructure, Placement, StructuralUnit};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructuralCombineResult {
    pub unit_a: usize,
    pub unit_b: usize,
    pub point_a: usize,
    pub point_b: usize,
    pub interaction: crate::combine::ExperimentalInteraction,
    pub work_cost: f64,
    pub formation_threshold: f64,
    pub investment: f64,
    pub surplus: f64,
    pub bond_strength: f64,
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
}

/// Execute COMBINE on two already-physical structural units.
///
/// The caller supplies the investment and local water field. This function
/// does not debit organism energy; the caller owns the ledger. The structure
/// is mutated only after all eligibility and formation checks pass.
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
    if unit_a == unit_b {
        return Err(StructuralCombineError::NoGeometricallyEligibleCandidate);
    }
    if !investment.is_finite() {
        return Err(StructuralCombineError::NonFiniteInvestment);
    }

    let candidates = eligible_connection_candidates(structure, unit_a, unit_b, catalog, cache);
    let candidate = *candidates.iter().max_by(|a, b| {
        a.facing
            .partial_cmp(&b.facing)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.distance.partial_cmp(&a.distance).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| b.point_a.cmp(&a.point_a))
            .then_with(|| b.point_b.cmp(&a.point_b))
    }).ok_or(StructuralCombineError::NoGeometricallyEligibleCandidate)?;

    let a = structure.units[unit_a].properties(catalog).ok_or(StructuralCombineError::MissingUnit)?;
    let b = structure.units[unit_b].properties(catalog).ok_or(StructuralCombineError::MissingUnit)?;

    let interaction = experimental_interaction(*a, *b, candidate, water_field);
    let work_cost = experimental_combine_work_cost(*a, *b, candidate, water_field);
    if !work_cost.is_finite() {
        return Err(StructuralCombineError::NonFiniteWorkCost);
    }

    let formation = evaluate_formation(candidate, a.cohesion, b.cohesion);
    if investment < formation.threshold.max(work_cost) {
        return Err(StructuralCombineError::InsufficientInvestment);
    }

    let surplus = investment - formation.threshold;
    let strength = evaluate_bond_strength(formation, investment)
        .ok_or(StructuralCombineError::InsufficientInvestment)?;
    if !strength.is_finite() {
        return Err(StructuralCombineError::NonFiniteBondStrength);
    }

    let bond = Bond {
        unit_a,
        point_a: candidate.point_a,
        unit_b,
        point_b: candidate.point_b,
        strength,
    };
    let bond_index = crate::contact::try_add_bond(structure, bond, catalog)
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
        bond_strength: strength,
        bond_index,
    })
}

/// Turn one unit of theoretical raw stock into one physical structural unit.
/// The organism chooses the placement and rotation. Exactly 1.0 bulk quantity
/// is consumed because StructuralUnit represents one physical unit and does
/// not store a bulk amount.
pub fn instantiate_raw_unit(
    structure: &mut OrganismStructure,
    raw: &mut Material,
    resource_name: &str,
    placement: Placement,
    catalog: &[BaseResource],
) -> Result<usize, &'static str> {
    if raw.bonded {
        return Err("raw material must be unbonded");
    }
    if catalog.iter().all(|r| r.name != resource_name) {
        return Err("resource type is not in the catalog");
    }
    if raw.parts.iter().find(|(n, a)| n == resource_name && *a >= 1.0).is_none() {
        return Err("insufficient raw material");
    }

    let mut remaining = Vec::with_capacity(raw.parts.len());
    for (name, amount) in std::mem::take(&mut raw.parts) {
        if name == resource_name {
            let next = amount - 1.0;
            if next > 1e-12 {
                remaining.push((name, next));
            }
        } else {
            remaining.push((name, amount));
        }
    }
    raw.parts = remaining;
    Ok(structure.add_unit(StructuralUnit::new(resource_name, placement)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact::ConnectionCompatibilityCache;
    use crate::resources::default_catalog;

    #[test]
    fn raw_stock_becomes_physical_unit_at_organism_chosen_position() {
        let catalog = default_catalog();
        let mut raw = Material::free_base("Carbon", 3.0);
        let mut structure = OrganismStructure::new();
        let index = instantiate_raw_unit(
            &mut structure,
            &mut raw,
            "Carbon",
            Placement { x: 10.0, y: 20.0, rotation_radians: 0.5 },
            &catalog,
        ).unwrap();
        assert_eq!(index, 0);
        assert_eq!(structure.units[0].placement.x, 10.0);
        assert_eq!(structure.units[0].placement.y, 20.0);
        assert!((raw.total_amount() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn failed_combine_does_not_mutate_structure() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 1000.0, y: 1000.0, rotation_radians: 0.0 }));
        let mut cache = ConnectionCompatibilityCache::new();
        let result = execute(&mut structure, a, b, &catalog, &mut cache, 100.0, 0.0);
        assert_eq!(result, Err(StructuralCombineError::NoGeometricallyEligibleCandidate));
        assert!(structure.bonds.is_empty());
    }
}
