//! Validation of physical organism structure against its inherited blueprint.
//!
//! This is an organism-level invariant boundary. Low-level structural
//! mechanics may construct synthetic units for isolated tests, but an
//! inherited organism structure must remain traceable to its blueprint.

use crate::resources::BaseResource;
use crate::state::Organism;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::OrganismStructure;

/// Validate a complete physical structure against its inherited blueprint.
///
/// Every blueprint element must have exactly one physical unit, and every
/// blueprint connection must have exactly one corresponding physical bond.
/// Bond strength is deliberately not compared with the stored value: the
/// authoritative intrinsic strength is recomputed by `OrganismStructure`.
pub(crate) fn validate_complete(
    structure: &OrganismStructure,
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<(), String> {
    blueprint.validate()?;
    if structure.units.len() != blueprint.elements.len() {
        return Err("physical unit count does not match blueprint element count".into());
    }

    validate_unit_identity(structure, blueprint)?;
    validate_authored_connections(structure, blueprint, catalog, true)
}

/// Validate a partial structure during blueprint-authorized growth or repair.
///
/// Existing units must still match their authored blueprint elements, but
/// missing elements and their not-yet-materialized connections are allowed.
pub(crate) fn validate_partial(
    structure: &OrganismStructure,
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
) -> Result<(), String> {
    blueprint.validate()?;
    if structure.units.len() > blueprint.elements.len() {
        return Err("physical structure contains more units than the blueprint".into());
    }

    validate_unit_identity(structure, blueprint)?;
    validate_authored_connections(structure, blueprint, catalog, false)
}

/// Validate the inherited blueprint identity carried by every physical unit.
pub(crate) fn validate_organism(
    organism: &Organism,
    catalog: &[BaseResource],
) -> Result<(), String> {
    validate_partial(
        &organism.structure,
        &organism.genome.structural_blueprint,
        catalog,
    )
}

fn validate_unit_identity(
    structure: &OrganismStructure,
    blueprint: &StructuralBlueprint,
) -> Result<(), String> {
    for (unit_index, unit) in structure.units.iter().enumerate() {
        let Some(blueprint_index) = unit.blueprint_index else {
            return Err(format!("physical unit {unit_index} has no inherited blueprint identity"));
        };
        let Some(element) = blueprint.elements.get(blueprint_index) else {
            return Err(format!("physical unit {unit_index} references invalid blueprint element {blueprint_index}"));
        };
        if structure
            .units
            .iter()
            .enumerate()
            .any(|(other_index, other)| other_index != unit_index && other.blueprint_index == Some(blueprint_index))
        {
            return Err(format!("blueprint element {blueprint_index} is represented by multiple physical units"));
        }
        if unit.material != element.material {
            return Err(format!("physical unit {unit_index} material differs from its blueprint element"));
        }
        if unit.geometry != element.geometry {
            return Err(format!("physical unit {unit_index} geometry differs from its blueprint element"));
        }
        if unit.placement != element.placement {
            return Err(format!("physical unit {unit_index} placement differs from its blueprint element"));
        }
    }
    Ok(())
}

fn validate_authored_connections(
    structure: &OrganismStructure,
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    require_complete: bool,
) -> Result<(), String> {
    if require_complete && structure.bonds.len() != blueprint.connections.len() {
        return Err("physical bond count does not match blueprint connection count".into());
    }
    if !require_complete && structure.bonds.len() > blueprint.connections.len() {
        return Err("physical structure contains more bonds than the blueprint".into());
    }

    for bond in &structure.bonds {
        if !structure.is_valid_bond(bond, catalog) {
            return Err("physical structure contains an invalid bond".into());
        }
        let Some(authored) = blueprint.connections.iter().find(|connection| {
            let Some(unit_a) = structure.units.iter().position(|unit| unit.blueprint_index == Some(connection.element_a)) else { return false; };
            let Some(unit_b) = structure.units.iter().position(|unit| unit.blueprint_index == Some(connection.element_b)) else { return false; };
            (bond.unit_a == unit_a && bond.point_a == connection.point_a && bond.unit_b == unit_b && bond.point_b == connection.point_b)
                || (bond.unit_a == unit_b && bond.point_a == connection.point_b && bond.unit_b == unit_a && bond.point_b == connection.point_a)
        }) else {
            return Err("physical structure contains a bond not authored by the blueprint".into());
        };
        let _ = authored;
    }

    for (connection_index, connection) in blueprint.connections.iter().enumerate() {
        let Some(unit_a) = structure.units.iter().position(|unit| unit.blueprint_index == Some(connection.element_a)) else {
            if require_complete { return Err("complete structure is missing a blueprint connection endpoint".into()); }
            continue;
        };
        let Some(unit_b) = structure.units.iter().position(|unit| unit.blueprint_index == Some(connection.element_b)) else {
            if require_complete { return Err("complete structure is missing a blueprint connection endpoint".into()); }
            continue;
        };
        let matches = structure.bonds.iter().filter(|bond| {
            (bond.unit_a == unit_a && bond.point_a == connection.point_a && bond.unit_b == unit_b && bond.point_b == connection.point_b)
                || (bond.unit_a == unit_b && bond.point_a == connection.point_b && bond.unit_b == unit_a && bond.point_b == connection.point_a)
        }).count();
        if matches > 1 {
            return Err(format!("blueprint connection {connection_index} is represented by multiple physical bonds"));
        }
        if require_complete && matches == 0 {
            return Err("complete structure is missing a blueprint-authored connection".into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;
    use crate::reproduction::instantiate_blueprint;

    #[test]
    fn complete_inherited_structure_matches_blueprint() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let structure = instantiate_blueprint(&genome.structural_blueprint, &catalog).unwrap();
        assert!(validate_complete(&structure, &genome.structural_blueprint, &catalog).is_ok());
    }

    #[test]
    fn complete_structure_rejects_missing_blueprint_identity() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let mut structure = instantiate_blueprint(&genome.structural_blueprint, &catalog).unwrap();
        structure.units[0].blueprint_index = None;
        assert!(validate_complete(&structure, &genome.structural_blueprint, &catalog).is_err());
    }

    #[test]
    fn complete_structure_rejects_unowned_extra_bond() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let mut structure = instantiate_blueprint(&genome.structural_blueprint, &catalog).unwrap();
        let original = structure.bonds[0];
        structure.bonds[0].point_a = (original.point_a + 1) % structure.units[0].geometry.connection_regions.len();
        assert!(validate_complete(&structure, &genome.structural_blueprint, &catalog).is_err());
    }

    #[test]
    fn complete_structure_rejects_duplicate_physical_bond_for_one_authored_connection() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let mut structure = instantiate_blueprint(&genome.structural_blueprint, &catalog).unwrap();
        let duplicate = structure.bonds[0];
        structure.add_bond(duplicate);
        assert!(validate_complete(&structure, &genome.structural_blueprint, &catalog).is_err());
    }

    #[test]
    fn partial_structure_may_omit_not_yet_built_elements() {
        let catalog = default_catalog();
        let genome = initial_genome();
        let mut structure = OrganismStructure::new();
        let first = &genome.structural_blueprint.elements[0];
        structure.add_unit(crate::structure::StructuralUnit::from_blueprint_indexed(
            first.material.clone(),
            first.geometry.clone(),
            first.placement,
            0,
        ));
        assert!(validate_partial(&structure, &genome.structural_blueprint, &catalog).is_ok());
    }
}
