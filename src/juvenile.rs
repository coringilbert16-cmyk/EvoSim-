//! Deterministic construction of the minimum viable juvenile initial condition.
//!
//! The juvenile blueprint is a fixed initial architecture, not a developmental
//! search problem. We therefore realize its declared physical placements
//! directly, then admit each declared bond through the normal COMBINE bond
//! authority. This keeps initial construction physical and authoritative while
//! avoiding combinatorial blueprint search during seed/offspring creation.

use crate::combine_runtime::combine_specific_pair;
use crate::contact::ConnectionCompatibilityCache;
use crate::energy_ledger::EnergyLedger;
use crate::resources::BaseResource;
use crate::state::JUVENILE_INITIAL_ENERGY_RESERVE;
use crate::structure::{OrganismStructure, StructuralUnit};
use crate::structural_blueprint::StructuralBlueprint;

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
    let (required_initial_energy, _) = form_declared_bonds(
        base.clone(),
        blueprint,
        catalog,
        TRIAL_ENERGY,
    )?;
    let initial_energy = required_initial_energy + JUVENILE_INITIAL_ENERGY_RESERVE;
    let (structure, ledger, remaining) =
        form_declared_bonds(base, blueprint, catalog, initial_energy)?;

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
    use crate::cavity::analyze_genome_cavity;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;

    #[test]
    fn juvenile_initialization_is_fast_path_and_physically_sealed() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let (structure, _ledger, energy) =
            realize_initial(&genome.juvenile_blueprint, &catalog).unwrap();
        assert_eq!(structure.units.len(), 16);
        assert_eq!(structure.bonds.len(), genome.juvenile_blueprint.connections.len());
        assert!(energy >= JUVENILE_INITIAL_ENERGY_RESERVE - EPS);
        let cavity = analyze_genome_cavity(
            &structure,
            &catalog,
            &genome.juvenile_blueprint.core_elements,
        )
        .unwrap()
        .expect("juvenile genome cavity must be sealed");
        assert!(cavity.qualifies());
    }
}
