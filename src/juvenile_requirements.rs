//! Viability requirements for a physically realized juvenile.
//!
//! These requirements deliberately do not define a canonical body plan. A
//! juvenile target is one possible realization chosen by a genome; the
//! realized physical structure must satisfy the viability contract regardless
//! of how many pieces or what arrangement produced it.

use crate::cavity::analyze_genome_cavity;
use crate::resources::BaseResource;
use crate::structure::OrganismStructure;

#[derive(Clone, Copy, Debug)]
pub struct JuvenileViabilityRequirements {
    pub require_sealed_genome_cavity: bool,
    pub require_extra_structure: bool,
}

impl Default for JuvenileViabilityRequirements {
    fn default() -> Self {
        Self {
            require_sealed_genome_cavity: true,
            require_extra_structure: true,
        }
    }
}

/// Validate juvenile viability from authoritative realized geometry and graph.
/// This does not require any fixed piece count, material recipe, shape, or
/// topology. The genome is identified by the realized physical cavity rather
/// than by a predefined set of constituents.
pub fn validate_realized_juvenile(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    requirements: JuvenileViabilityRequirements,
) -> Result<(), String> {
    let cavity = if requirements.require_sealed_genome_cavity {
        Some(
            analyze_genome_cavity(structure, catalog)?
                .ok_or_else(|| "juvenile genome cavity is not sealed and sufficiently large".to_string())?,
        )
    } else {
        analyze_genome_cavity(structure, catalog)?
    };

    if requirements.require_extra_structure {
        let boundary_count = cavity
            .as_ref()
            .map(|value| value.boundary_units.len())
            .unwrap_or(0);
        if structure.units.len() <= boundary_count {
            return Err("juvenile has no realized structure outside its genome cavity boundary".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;

    #[test]
    fn viability_does_not_depend_on_a_predefined_piece_count_or_core() {
        let genome = initial_genome();
        let catalog = default_catalog();
        let blueprint = genome.developmental_construction_target(&catalog).unwrap();
        let structure = blueprint.realize(&catalog).unwrap();
        validate_realized_juvenile(
            &structure,
            &catalog,
            JuvenileViabilityRequirements::default(),
        )
        .unwrap();
    }

    #[test]
    fn unsealed_genome_fails_viability() {
        let genome = initial_genome();
        let catalog = default_catalog();
        let blueprint = genome.developmental_construction_target(&catalog).unwrap();
        let mut structure = blueprint.realize(&catalog).unwrap();
        structure.bonds.clear();
        assert!(validate_realized_juvenile(
            &structure,
            &catalog,
            JuvenileViabilityRequirements::default(),
        )
        .is_err());
    }
}
