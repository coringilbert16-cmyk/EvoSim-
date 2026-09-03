//! Runtime COMBINE boundary.
//!
//! This module bridges organism-owned raw material, structural units, contact,
//! and the experimental COMBINE equations. Chemistry and geometry remain in
//! their authoritative modules.

use crate::combine::{
    bond_strength, eligible_candidates, evaluate_formation, experimental_combine_work_cost,
    experimental_interaction,
};
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::{BaseResource, Material};
use crate::state::{Environment, Organism};
use crate::structure::{Placement, StructuralUnit};

const MATERIAL_UNIT_AMOUNT: f64 = 1.0;
const EPSILON: f64 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CombineAttempt {
    pub unit_a: usize,
    pub unit_b: usize,
    pub point_a: usize,
    pub point_b: usize,
    pub work_cost: f64,
    pub energy_paid: f64,
    pub interaction_direction: f64,
    pub interaction_magnitude: f64,
    pub formation_threshold: f64,
    pub surplus: f64,
    pub bond_strength: f64,
    pub bond_energy: f64,
}

pub(crate) fn instantiate_one_unit(
    organism: &mut Organism,
    catalog: &[BaseResource],
) -> Option<usize> {
    let index = organism
        .stored_unbonded
        .parts
        .iter()
        .position(|(_, amount)| *amount >= MATERIAL_UNIT_AMOUNT - EPSILON)?;
    let resource_name = organism.stored_unbonded.parts[index].0.clone();
    if catalog.iter().all(|base| base.name != resource_name) {
        return None;
    }
    organism.stored_unbonded.parts[index].1 -= MATERIAL_UNIT_AMOUNT;
    organism
        .stored_unbonded
        .parts
        .retain(|(_, amount)| *amount > EPSILON);
    let (x, y) = organism
        .occupied_cells
        .first()
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    Some(organism.structure.add_unit(StructuralUnit::new(
        resource_name,
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        },
    )))
}

pub(crate) fn try_combine(
    organism: &mut Organism,
    environment: &Environment,
    compatibility_cache: &mut ConnectionCompatibilityCache,
) -> Option<CombineAttempt> {
    if organism.structure.units.len() < 2 {
        return None;
    }
    let catalog = &environment.catalog;
    let mut best: Option<(usize, usize, crate::combine::FormationEvaluation)> = None;

    for unit_a in 0..organism.structure.units.len() {
        for unit_b in (unit_a + 1)..organism.structure.units.len() {
            for evaluation in eligible_candidates(
                &organism.structure,
                unit_a,
                unit_b,
                catalog,
                compatibility_cache,
            )
            .into_iter()
            .filter_map(|candidate| {
                let a = organism.structure.units[unit_a].properties(catalog)?;
                let b = organism.structure.units[unit_b].properties(catalog)?;
                Some(evaluate_formation(candidate, a.cohesion, b.cohesion))
            }) {
                if best
                    .as_ref()
                    .map(|(_, _, current)| {
                        evaluation.candidate.distance < current.candidate.distance
                    })
                    .unwrap_or(true)
                {
                    best = Some((unit_a, unit_b, evaluation));
                }
            }
        }
    }

    let (unit_a, unit_b, evaluation) = best?;
    let props_a = *organism.structure.units[unit_a].properties(catalog)?;
    let props_b = *organism.structure.units[unit_b].properties(catalog)?;
    let water_field = environment
        .field
        .index_for_position(
            organism.occupied_cells.first()?.x,
            organism.occupied_cells.first()?.y,
        )
        .map(|index| {
            environment.field.cells[index]
                .bonded
                .parts
                .iter()
                .chain(environment.field.cells[index].unbonded.parts.iter())
                .filter(|(name, _)| name == "Water")
                .map(|(_, amount)| *amount)
                .sum::<f64>()
        })
        .unwrap_or(0.0);

    let interaction = experimental_interaction(props_a, props_b, evaluation.candidate, water_field);
    if interaction.direction <= 0.0 || interaction.magnitude <= EPSILON {
        return None;
    }
    let work_cost =
        experimental_combine_work_cost(props_a, props_b, evaluation.candidate, water_field);
    if !work_cost.is_finite() {
        return None;
    }

    // Investment still controls whether formation succeeds. Surplus becomes
    // stored bond energy; it does not determine intrinsic bond strength.
    let energy_paid = work_cost.max(evaluation.threshold);
    let surplus = energy_paid - evaluation.threshold;
    if !energy_paid.is_finite()
        || energy_paid < 0.0
        || organism.usable_energy + EPSILON < energy_paid
    {
        return None;
    }

    let bond_strength = bond_strength(props_a, props_b);
    let bond_energy = surplus;
    let bond = crate::structure::Bond {
        unit_a,
        point_a: evaluation.candidate.point_a,
        unit_b,
        point_b: evaluation.candidate.point_b,
        strength: bond_strength,
        bond_energy,
    };
    crate::contact::try_add_bond(&mut organism.structure, bond, catalog).ok()?;
    organism.usable_energy -= energy_paid;

    Some(CombineAttempt {
        unit_a,
        unit_b,
        point_a: evaluation.candidate.point_a,
        point_b: evaluation.candidate.point_b,
        work_cost,
        energy_paid,
        interaction_direction: interaction.direction,
        interaction_magnitude: interaction.magnitude,
        formation_threshold: evaluation.threshold,
        surplus,
        bond_strength,
        bond_energy,
    })
}

#[allow(dead_code)]
fn _raw_material_type_check(raw: &Material, catalog: &[BaseResource]) -> bool {
    !raw.bonded
        && raw
            .parts
            .iter()
            .all(|(name, amount)| *amount >= 0.0 && catalog.iter().any(|r| r.name == *name))
}
