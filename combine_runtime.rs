//! Runtime COMBINE boundary.
//!
//! This module bridges organism-owned structural units, contact geometry, and
//! the experimental COMBINE equations. Chemistry and geometry remain in their
//! authoritative modules.

use crate::combine::{eligible_candidates, evaluate_formation, experimental_interaction};
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::{BaseResource, Material};
use crate::state::{Environment, Organism};
use crate::structure::{Placement, StructuralUnit};

const MATERIAL_UNIT_AMOUNT: f64 = 1.0;
const EPSILON: f64 = 1e-12;

// Experimental partition of energy left after formation work. These are not
// chemistry constants; they are the first explicit resolution of the energy
// ledger and can later become evolvable if that proves useful.
const SURPLUS_TO_BOND: f64 = 0.50;
const SURPLUS_TO_USABLE: f64 = 0.40;
const SURPLUS_TO_HEAT: f64 = 0.10;

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
    pub usable_energy_gained: f64,
    pub heat_dissipated: f64,
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
            .filter(|candidate| candidate.distance <= 1.0)
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

    let interaction =
        experimental_interaction(props_a, props_b, evaluation.candidate, water_field);
    if interaction.direction <= 0.0 || interaction.magnitude <= EPSILON {
        return None;
    }

    // Potential energy establishes the direction. Reactivity and geometry
    // modify how much of the participating potential is released into this
    // interaction. This is deliberately distinct from formation work.
    let potential_sum =
        (props_a.potential_energy.max(0.0) + props_b.potential_energy.max(0.0)).max(0.0);
    let potential_delta = (props_b.potential_energy - props_a.potential_energy).abs();
    if !potential_sum.is_finite() || !potential_delta.is_finite() || potential_delta <= EPSILON {
        return None;
    }
    let interaction_modifier = interaction.magnitude / potential_delta;
    let potential_energy_released = potential_sum * interaction_modifier;
    if !potential_energy_released.is_finite() || potential_energy_released <= EPSILON {
        return None;
    }

    let formation_threshold = evaluation.threshold;
    if !formation_threshold.is_finite() || formation_threshold < 0.0 {
        return None;
    }

    // If the interaction cannot pay formation work, the organism subsidizes
    // exactly the deficit. The temporary starting energy is therefore a real
    // payment source, not an input to the interaction itself.
    let deficit = (formation_threshold - potential_energy_released).max(0.0);
    if organism.usable_energy + EPSILON < deficit {
        return None;
    }

    let surplus = (potential_energy_released - formation_threshold).max(0.0);
    let bond_energy = surplus * SURPLUS_TO_BOND;
    let usable_energy_gained = surplus * SURPLUS_TO_USABLE;
    let heat_dissipated = surplus * SURPLUS_TO_HEAT;
    let partition_total = bond_energy + usable_energy_gained + heat_dissipated;
    if (partition_total - surplus).abs() > 1e-9 * surplus.max(1.0) {
        return None;
    }

    let bond_strength = crate::combine::experimental_bond_strength(bond_energy);
    if !bond_strength.is_finite() {
        return None;
    }

    let bond = crate::structure::Bond {
        unit_a,
        point_a: evaluation.candidate.point_a,
        unit_b,
        point_b: evaluation.candidate.point_b,
        strength: bond_strength,
        bond_energy,
    };
    crate::contact::try_add_bond(&mut organism.structure, bond, catalog).ok()?;

    organism.usable_energy -= deficit;
    organism.usable_energy += usable_energy_gained;

    Some(CombineAttempt {
        unit_a,
        unit_b,
        point_a: evaluation.candidate.point_a,
        point_b: evaluation.candidate.point_b,
        work_cost: formation_threshold,
        energy_paid: deficit,
        interaction_direction: interaction.direction,
        interaction_magnitude: interaction.magnitude,
        formation_threshold,
        surplus,
        bond_strength,
        bond_energy,
        usable_energy_gained,
        heat_dissipated,
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
