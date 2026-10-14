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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn cavity_contract_does_not_depend_on_a_predefined_piece_count_or_core() {
        let catalog = default_catalog();
        let structure =
            crate::initial_organism_constructor::sealed_nitrogen_ring_fixture(&catalog)
                .expect("test ring should be physically realized and bonded");
        validate_realized_organism(
            &structure,
            &catalog,
            OrganismViabilityRequirements {
                require_sealed_genome_cavity: true,
                // This test isolates the cavity contract. Extra processing
                // structure is a separate viability requirement.
                require_extra_structure: false,
            },
        )
        .expect("any realized topology meeting the cavity contract should qualify");
    }

    #[test]
    fn unsealed_genome_fails_viability() {
        let catalog = default_catalog();
        let mut structure =
            crate::initial_organism_constructor::sealed_nitrogen_ring_fixture(&catalog)
                .expect("test ring should be physically realized and bonded");
        structure.bonds.pop();
        let result = validate_realized_organism(
            &structure,
            &catalog,
            OrganismViabilityRequirements::default(),
        );
        assert!(
            result
                .as_ref()
                .is_err_and(|error| error.contains("genome cavity is not sealed")),
            "a single missing perimeter bond must invalidate the genome cavity; result={result:?}"
        );
    }
}
