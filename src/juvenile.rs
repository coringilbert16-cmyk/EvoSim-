//! Deterministic construction of one valid juvenile realization.
//!
//! The default genome currently supplies a fixed, known-good blueprint so the
//! initial-condition path is deterministic and fast. That blueprint is not
//! the definition of a juvenile: viability is checked separately against the
//! realized physical structure.

use crate::combine_runtime::combine_specific_pair;
use crate::contact::ConnectionCompatibilityCache;
use crate::energy_ledger::EnergyLedger;
use crate::juvenile_requirements::{
    validate_realized_juvenile, JuvenileViabilityRequirements,
};
use crate::resources::BaseResource;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::{OrganismStructure, StructuralUnit};

pub(crate) const JUVENILE_INITIAL_ENERGY_RESERVE: f64 = 16.0;
const TRIAL_ENERGY: f64 = 1.0e12;
const EPS: f64 = 1e-8;

pub(crate) fn realize_initial(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    if !blueprint.is_valid() {
        return Err("juvenile blueprint is invalid".into());
    }
    if blueprint.core_elements.is_empty() {
        return Err("juvenile blueprint has no genome core".into());
    }

    let base = realize_declared_units(blueprint, catalog)?;
    let (required_initial_energy, _) =
        form_declared_bonds(base.clone(), blueprint, catalog, TRIAL_ENERGY)?;
    let initial_energy = required_initial_energy + JUVENILE_INITIAL_ENERGY_RESERVE;
    let (structure, ledger, remaining) =
        form_declared_bonds(base, blueprint, catalog, initial_energy)?;

    validate_realized_juvenile(
        &structure,
        catalog,
        &blueprint.core_elements,
        JuvenileViabilityRequirements::default(),
    )?;

    if remaining + EPS < JUVENILE_INITIAL_ENERGY_RESERVE {
        return Err(format!(
            "juvenile initialization could not preserve its reserve: remaining={remaining}"
        ));
    }
    Ok((structure, ledger, remaining))
}

fn realize_declared_units(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<OrganismStructure, String> {
    let mut structure = OrganismStructure::new();
    for element in &blueprint.elements {
        let mut unit = StructuralUnit::from_material(
            element.material.clone(),
            crate::structure::Placement {
                x: element.placement.x,
                y: element.placement.y,
                rotation_radians: element.placement.rotation_radians,
            },
        )
        .ok_or_else(|| "juvenile blueprint contains invalid material".to_string())?;
        if !unit.realize_default_geometry(catalog) {
            return Err("juvenile blueprint contains unrealizable material geometry".into());
        }
        structure.add_unit(unit);
    }
    Ok(structure)
}

fn form_declared_bonds(
    mut structure: OrganismStructure,
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    mut energy: f64,
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    let mut ledger = EnergyLedger::default();
    let mut cache = ConnectionCompatibilityCache::new();
    for connection in &blueprint.connections {
        combine_specific_pair(
            &mut structure,
            connection.element_a,
            connection.element_b,
            catalog,
            0.0,
            &mut cache,
            &mut ledger,
            &mut energy,
        )
        .ok_or_else(|| {
            format!(
                "juvenile blueprint bond could not be realized: {}-{}",
                connection.element_a, connection.element_b
            )
        })?;
    }
    Ok((structure, ledger, energy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;

    #[test]
    fn juvenile_initialization_is_a_valid_physical_realization() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let (structure, _ledger, energy) =
            realize_initial(&genome.juvenile_blueprint, &catalog).unwrap();
        assert_eq!(
            structure.bonds.len(),
            genome.juvenile_blueprint.connections.len()
        );
        assert!(energy >= JUVENILE_INITIAL_ENERGY_RESERVE - EPS);
    }
}
