//! Compatibility wrappers for non-simulation callers.

use crate::construction_runtime::realize_material_with_context;
use crate::resources::BaseResource;
use crate::state::EnergyLedger;
use crate::structural_blueprint::BlueprintElement;
use crate::structure::OrganismStructure;

pub(crate) fn realize_material(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
) -> Result<Vec<usize>, String> {
    let mut ledger = EnergyLedger::default();
    let mut energy = 1_000_000.0;
    Ok(
        realize_material_with_context(structure, element, catalog, &mut ledger, &mut energy, &[])?
            .0,
    )
}

pub(crate) fn realize_material_with_constraints(
    structure: &mut OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    external: &[Vec<usize>],
) -> Result<Vec<usize>, String> {
    let mut ledger = EnergyLedger::default();
    let mut energy = 1_000_000.0;
    Ok(realize_material_with_context(
        structure,
        element,
        catalog,
        &mut ledger,
        &mut energy,
        external,
    )?
    .0)
}
