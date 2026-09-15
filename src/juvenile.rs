//! Deterministic construction of one valid juvenile realization.
//!
//! The default genome supplies one known-good blueprint for initialization. It
//! is not the definition of juvenile viability; viability is checked against
//! the realized physical structure.
use crate::combine_runtime::combine_specific_pair;
use crate::contact::ConnectionCompatibilityCache;
use crate::juvenile_requirements::{validate_realized_juvenile, JuvenileViabilityRequirements};
use crate::resources::BaseResource;
use crate::state::EnergyLedger;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::{OrganismStructure, StructuralUnit};

pub(crate) const JUVENILE_INITIAL_ENERGY_RESERVE: f64 = 16.0;
const TRIAL_ENERGY: f64 = 1.0e12;
const EPS: f64 = 1e-8;

/// Compatibility wrapper for callers that still use the original initial-
/// condition API. New lifecycle code should use `realize_initial_with_reserve`.
pub(crate) fn realize_initial(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    realize_initial_with_reserve(blueprint, catalog, JUVENILE_INITIAL_ENERGY_RESERVE)
}

pub(crate) fn realize_initial_with_reserve(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    reserve_energy: f64,
) -> Result<(OrganismStructure, EnergyLedger, f64), String> {
    if !blueprint.is_valid() {
        return Err("juvenile blueprint is invalid".into());
    }
    if blueprint.core_elements.is_empty() {
        return Err("juvenile blueprint has no genome core".into());
    }
    if !reserve_energy.is_finite() || reserve_energy <= 0.0 {
        return Err("juvenile energy reserve must be finite and positive".into());
    }
    let base = realize_declared_units(blueprint, catalog)?;
    let (required_initial_energy, _) =
        form_declared_bonds(base.clone(), blueprint, catalog, TRIAL_ENERGY)?;
    let initial_energy = required_initial_energy + reserve_energy;
    let (structure, ledger, remaining) =
        form_declared_bonds(base, blueprint, catalog, initial_energy)?;
    validate_realized_juvenile(
        &structure,
        catalog,
        &blueprint.core_elements,
        JuvenileViabilityRequirements::default(),
    )?;
    if remaining + EPS < reserve_energy {
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
        let c = default_catalog();
        let g = initial_genome();
        let (s, _, e) =
            realize_initial_with_reserve(&g.juvenile_blueprint, &c, g.juvenile_energy_reserve)
                .unwrap();
        assert_eq!(s.bonds.len(), g.juvenile_blueprint.connections.len());
        assert!(e >= g.juvenile_energy_reserve - EPS);
    }
    #[test]
    fn nonpositive_reserve_is_rejected() {
        let g = initial_genome();
        assert!(
            realize_initial_with_reserve(&g.juvenile_blueprint, &default_catalog(), 0.0).is_err()
        );
    }
}
