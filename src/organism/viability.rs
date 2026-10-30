//! Viability requirements for a physically realized organism.
//!
//! These requirements deliberately do not define a canonical body plan. A
//! organism target is one possible realization chosen by a genome; the
//! realized physical structure must satisfy the viability contract regardless
//! of how many pieces or what arrangement produced it.

use crate::cavity::analyze_genome_cavity;
use crate::resources::BaseResource;
use crate::structure::OrganismStructure;

#[derive(Clone, Copy, Debug)]
pub struct OrganismViabilityRequirements {
    pub require_sealed_genome_cavity: bool,
    pub require_extra_structure: bool,
}

impl Default for OrganismViabilityRequirements {
    fn default() -> Self {
        Self {
            require_sealed_genome_cavity: true,
            require_extra_structure: true,
        }
    }
}

/// Validate organism viability from authoritative realized geometry and graph.
/// This does not require any fixed piece count, material recipe, shape, or
/// topology. The genome is identified by the realized physical cavity rather
/// than by a predefined set of constituents.
pub fn validate_realized_organism(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    requirements: OrganismViabilityRequirements,
) -> Result<(), String> {
    let cavity = if requirements.require_sealed_genome_cavity {
        Some(analyze_genome_cavity(structure, catalog)?.ok_or_else(|| {
            "organism genome cavity is not sealed and sufficiently large".to_string()
        })?)
    } else {
        analyze_genome_cavity(structure, catalog)?
    };

    if requirements.require_extra_structure {
        let boundary_count = cavity
            .as_ref()
            .map(|value| value.boundary_units.len())
            .unwrap_or(0);
        if structure.units.len() <= boundary_count {
            return Err(
                "organism has no realized structure outside its genome cavity boundary".into(),
            );
        }
    }
    Ok(())
}

/// Validate one identified genome inside a graph that may contain other
/// physical components or genome cores. Geometry and cavity topology are still
/// derived from the live graph; the supplied IDs only select which core is
/// being validated.
pub(crate) fn validate_realized_organism_for_core(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    requirements: OrganismViabilityRequirements,
    core_ids: &[crate::structure::PhysicalConstituentId],
) -> Result<(), String> {
    let surviving_core_index = core_ids
        .iter()
        .find_map(|id| structure.unit_index(*id))
        .ok_or_else(|| "organism genome core has no surviving physical constituents".to_string())?;
    let cavity = if requirements.require_sealed_genome_cavity {
        Some(
            crate::cavity::analyze_genome_cavity_for_core(structure, catalog, core_ids)?
                .ok_or_else(|| {
                    "organism genome cavity is not sealed and sufficiently large".to_string()
                })?,
        )
    } else {
        crate::cavity::analyze_genome_cavity_for_core(structure, catalog, core_ids)?
    };

    if requirements.require_extra_structure {
        let component_seed = cavity
            .as_ref()
            .and_then(|value| value.boundary_units.first().copied())
            .unwrap_or(surviving_core_index);
        let component = structure.connected_component_containing(component_seed);
        let boundary_count = cavity
            .as_ref()
            .map(|value| value.boundary_units.len())
            .unwrap_or(0);
        if component.len() <= boundary_count {
            return Err(
                "organism has no realized structure outside its genome cavity boundary".into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::juvenile::confirmed_seed_baseline;
    use crate::resources::default_catalog;

    #[test]
    fn core_specific_viability_fails_closed_when_core_identity_is_missing() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let unit = structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let core_id = structure.physical_id(unit).unwrap();
        let requirements = OrganismViabilityRequirements {
            require_sealed_genome_cavity: false,
            require_extra_structure: false,
        };
        assert!(validate_realized_organism_for_core(
            &structure,
            &catalog,
            requirements,
            &[core_id],
        )
        .is_ok());
        assert!(validate_realized_organism_for_core(
            &structure,
            &catalog,
            requirements,
            &[crate::structure::PhysicalConstituentId(u64::MAX)],
        )
        .is_err());
    }

    #[test]
    fn cavity_seed_is_not_yet_a_viable_organism() {
        let catalog = default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).unwrap();
        let (structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();

        assert!(crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .is_some());
        assert!(
            validate_realized_organism(
                &structure,
                &catalog,
                OrganismViabilityRequirements::default(),
            )
            .is_err(),
            "the cavity seed is a milestone, not a viable organism without extra realized structure"
        );
    }

    #[test]
    fn unsealed_genome_fails_viability() {
        let catalog = default_catalog();
        let blueprint = confirmed_seed_baseline(&catalog).unwrap();
        let (mut structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog).unwrap();
        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .expect("fixture begins with a sealed cavity");
        let core_unit = *cavity
            .boundary_units
            .first()
            .expect("sealed cavity must have a boundary unit");
        let core_id = structure.units[core_unit].physical_id;
        let mut opened = false;

        for bond_index in cavity.boundary_bond_indices(&structure, &catalog) {
            let mut trial = structure.clone();
            trial.bonds.remove(bond_index);
            let remains_connected =
                trial.connected_component_containing(core_unit).len() == trial.units.len();
            let cavity_is_open = crate::cavity::analyze_genome_cavity(&trial, &catalog)
                .unwrap()
                .is_none();
            if remains_connected && cavity_is_open {
                structure = trial;
                opened = true;
                break;
            }
        }

        assert!(
            opened,
            "fixture must admit removal of a boundary bond while retaining one connected structure"
        );
        assert!(structure.unit_index(core_id).is_some());
        assert!(crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .unwrap()
            .is_none());
        assert!(validate_realized_organism(
            &structure,
            &catalog,
            OrganismViabilityRequirements::default(),
        )
        .is_err());
    }
}
