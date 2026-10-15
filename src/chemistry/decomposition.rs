#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::physical_material::PhysicalMaterial;
use crate::state::{EnergyLedger, Environment, Organism, Position};
use crate::structure::OrganismStructure;

#[derive(Clone, Debug)]
pub(crate) struct DecomposingBody {
    pub(crate) structure: OrganismStructure,
    pub(crate) energy_budget: f64,
    pub(crate) position: Position,
}

impl DecomposingBody {
    pub(crate) fn new(
        structure: OrganismStructure,
        energy_budget: f64,
        position: Position,
    ) -> Option<Self> {
        if !energy_budget.is_finite() || energy_budget < 0.0 {
            return None;
        }
        Some(Self {
            structure,
            energy_budget,
            position,
        })
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.structure.bonds.is_empty()
    }

    pub(crate) fn release_finished_material(
        &mut self,
        catalog: &[crate::resources::BaseResource],
    ) -> Vec<PhysicalMaterial> {
        if !self.is_finished() {
            return Vec::new();
        }
        self.structure
            .units
            .iter()
            .filter_map(|unit| {
                PhysicalMaterial::realized(unit.material.clone(), vec![unit.placement], catalog)
            })
            .collect()
    }
}

pub(crate) struct DecompositionStep {
    pub(crate) net_energy: f64,
    pub(crate) potential_energy: f64,
    pub(crate) heat: f64,
    pub(crate) released_material: Option<Vec<PhysicalMaterial>>,
}

pub(crate) fn resolve_one_bond_with_ledger(
    body: &mut DecomposingBody,
    environment: &Environment,
    ledger: &mut EnergyLedger,
) -> Option<DecompositionStep> {
    let target = *body.structure.bonds.first()?;
    // A decomposition step still breaks an existing physical bond. Its
    // energy source is therefore the bond's stored intrinsic potential, not
    // the constituent material potential (which was never consumed by
    // formation).
    let mut trial_structure = body.structure.clone();
    trial_structure.break_matching_bond(target)?;
    let released_material = if trial_structure.bonds.is_empty() {
        Some(
            trial_structure
                .units
                .iter()
                .filter_map(|unit| {
                    PhysicalMaterial::realized(
                        unit.material.clone(),
                        vec![unit.placement],
                        &environment.catalog,
                    )
                })
                .collect(),
        )
    } else {
        None
    };

    let before = body.energy_budget;
    let (gross, usable, heat) =
        crate::transformation::bond_break_energy_yield(target.bond_energy, 1.0)?;
    let transaction = EnergyTransaction {
        reason: EnergyReason::Decomposition,
        potential_released: gross,
        usable_delta: usable,
        structural_delta: 0.0,
        heat_dissipated: heat,
    };
    if !ledger.settle_transaction(&mut body.energy_budget, transaction) {
        return None;
    }
    let net = body.energy_budget - before;
    body.structure = trial_structure;
    Some(DecompositionStep {
        net_energy: net,
        potential_energy: gross,
        heat,
        released_material,
    })
}

pub(crate) fn harvestable_decomposition_energy(
    organisms: &[Organism],
    position: &Position,
) -> Option<usize> {
    organisms
        .iter()
        .enumerate()
        .filter_map(|(index, organism)| {
            let organism_position = organism.occupied_cells.first()?;
            let distance =
                (organism_position.x - position.x).hypot(organism_position.y - position.y);
            Some((index, distance))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};

    fn minimal_realized_structure() -> OrganismStructure {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let mut unit = StructuralUnit::new(
            "Carbon".to_string(),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        assert!(unit.realize_default_geometry(&catalog));
        structure.add_unit(unit);
        structure
    }

    #[test]
    fn retains_structure_budget_and_position() {
        let structure = minimal_realized_structure();
        let body = DecomposingBody::new(structure, 4.0, Position { x: 2.0, y: 3.0 }).unwrap();
        assert_eq!(body.structure.units.len(), 1);
        assert_eq!(body.energy_budget, 4.0);
        assert_eq!(body.position, Position { x: 2.0, y: 3.0 });
    }

    #[test]
    fn zero_bond_structure_is_finished() {
        let structure = minimal_realized_structure();
        let body = DecomposingBody::new(structure, 0.0, Position { x: 0.0, y: 0.0 }).unwrap();
        assert!(body.is_finished());
    }
}
