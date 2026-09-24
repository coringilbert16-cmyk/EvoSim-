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

fn water_field_amount(environment: &Environment, position: &Position) -> f64 {
    environment
        .field
        .index_for_position(position.x, position.y)
        .map(|index| {
            environment.field.cells[index]
                .materials
                .iter()
                .flat_map(|material| material.parts.iter())
                .filter(|(name, _)| name == "Water")
                .map(|(_, amount)| *amount)
                .sum()
        })
        .unwrap_or(0.0)
}

pub(crate) fn resolve_one_bond_with_ledger(
    body: &mut DecomposingBody,
    environment: &Environment,
    ledger: &mut EnergyLedger,
) -> Option<DecompositionStep> {
    let target = *body.structure.bonds.first()?;
    let ia = body
        .structure
        .unit_index(target.endpoint_a.constituent_id)?;
    let ib = body
        .structure
        .unit_index(target.endpoint_b.constituent_id)?;
    let a = body
        .structure
        .units
        .get(ia)?
        .properties(&environment.catalog)?;
    let b = body
        .structure
        .units
        .get(ib)?
        .properties(&environment.catalog)?;

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
    let (gross, usable, heat) = crate::transformation::break_energy_yield(
        a,
        b,
        water_field_amount(environment, &body.position),
        1.0,
    )?;
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

    #[test]
    fn retains_structure_budget_and_position() {
        let blueprint = crate::juvenile::confirmed_seed_baseline(&default_catalog()).unwrap();
        let structure = blueprint.realize(&default_catalog()).unwrap();
        let body = DecomposingBody::new(structure, 4.0, Position { x: 2.0, y: 3.0 }).unwrap();
        assert!(!body.structure.units.is_empty());
        assert_eq!(body.energy_budget, 4.0);
        assert_eq!(body.position, Position { x: 2.0, y: 3.0 })
    }

    #[test]
    fn zero_bond_structure_is_finished() {
        let blueprint = crate::juvenile::confirmed_seed_baseline(&default_catalog()).unwrap();
        let mut structure = blueprint.realize(&default_catalog()).unwrap();
        structure.bonds.clear();
        let body = DecomposingBody::new(structure, 0.0, Position { x: 0.0, y: 0.0 }).unwrap();
        assert!(body.is_finished())
    }
}
