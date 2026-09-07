use crate::structure::OrganismStructure;

/// Physical material formerly owned by a living organism.
///
/// The biological organism is gone, but its physical structure remains so
/// decomposition can dismantle organism-level bonds progressively.
#[derive(Clone, Debug)]
pub(crate) struct DecomposingBody {
    pub(crate) structure: OrganismStructure,
    pub(crate) energy_budget: f64,
}

impl DecomposingBody {
    pub(crate) fn new(structure: OrganismStructure, energy_budget: f64) -> Option<Self> {
        if !energy_budget.is_finite() || energy_budget < 0.0 {
            return None;
        }
        Some(Self { structure, energy_budget })
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.structure.bonds.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;

    #[test]
    fn retains_structure_and_decomposition_budget() {
        let genome = initial_genome();
        let structure = genome.structural_blueprint.realize(&default_catalog()).unwrap();
        let body = DecomposingBody::new(structure, 4.0).unwrap();
        assert!(!body.structure.units.is_empty());
        assert_eq!(body.energy_budget, 4.0);
    }

    #[test]
    fn zero_bond_structure_is_finished() {
        let genome = initial_genome();
        let mut structure = genome.structural_blueprint.realize(&default_catalog()).unwrap();
        structure.bonds.clear();
        let body = DecomposingBody::new(structure, 0.0).unwrap();
        assert!(body.is_finished());
    }
}
