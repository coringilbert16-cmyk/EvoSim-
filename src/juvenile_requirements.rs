//! Viability requirements for a physically realized juvenile.
//!
//! These requirements deliberately do not define a canonical body plan. A
//! juvenile blueprint is one possible realization chosen by a genome; the
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
        Self { require_sealed_genome_cavity: true, require_extra_structure: true }
    }
}

/// Validate juvenile viability from authoritative realized geometry and graph.
/// This does not require any fixed piece count, material recipe, shape, or
/// topology. The default genome's four-piece Nitrogen cavity is only one
/// possible realization.
pub fn validate_realized_juvenile(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
    core_units: &[usize],
    requirements: JuvenileViabilityRequirements,
) -> Result<(), String> {
    if core_units.is_empty() { return Err("juvenile has no physical genome-core anchor".into()); }
    if requirements.require_sealed_genome_cavity {
        let cavity = analyze_genome_cavity(structure, catalog, core_units)?
            .ok_or_else(|| "juvenile genome cavity is not sealed and sufficiently large".to_string())?;
        if !cavity.qualifies() { return Err("juvenile genome cavity is below the minimum capacity".into()); }
    }
    if requirements.require_extra_structure && structure.units.len() <= core_units.len() {
        return Err("juvenile has no realized structure outside its genome core".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;

    #[test]
    fn viability_does_not_depend_on_default_piece_count() {
        let genome = initial_genome();
        let catalog = default_catalog();
        let structure = genome.juvenile_blueprint.realize(&catalog).unwrap();
        validate_realized_juvenile(&structure, &catalog, &genome.juvenile_blueprint.core_elements, JuvenileViabilityRequirements::default()).unwrap();
    }

    #[test]
    fn unsealed_genome_fails_viability() {
        let genome = initial_genome();
        let catalog = default_catalog();
        let mut structure = genome.juvenile_blueprint.realize(&catalog).unwrap();
        structure.bonds.clear();
        assert!(validate_realized_juvenile(&structure, &catalog, &genome.juvenile_blueprint.core_elements, JuvenileViabilityRequirements::default()).is_err());
    }
}
