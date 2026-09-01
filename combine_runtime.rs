//! Runtime COMBINE boundary.
//!
//! This module is intentionally the bridge between decision selection and the
//! existing COMBINE/contact/structure mechanics. It does not own chemistry or
//! geometry rules. Those remain in `combine.rs`, `contact.rs`, and
//! `structure.rs`.
//!
//! Phase 6 starts by making the previously disconnected physical path
//! explicit: bulk raw material can be instantiated into discrete structural
//! units, and eligible structural units can then be evaluated by the existing
//! COMBINE formation pipeline. Energy payment and final surplus allocation are
//! kept at this boundary so the chemistry/geometry modules remain reusable.

use crate::combine::{eligible_candidates, evaluate_formation, experimental_combine_work_cost, experimental_interaction};
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::{BaseResource, Material};
use crate::state::{Environment, Organism};
use crate::structure::{Bond, StructuralUnit, Placement};

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
}

/// Move one whole base-resource amount from bulk raw storage into a discrete
/// structural unit. Only pure single-resource material can be instantiated;
/// mixed bulk remains bulk until a future explicit decomposition rule exists.
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

/// Attempt the physical COMBINE stage for an organism that already owns at
/// least two instantiated structural units.
///
/// The current phase deliberately uses the existing experimental interaction
/// and work equations rather than duplicating them. It also refuses to create
/// a bond when the formation threshold cannot be met. The exact surplus-energy
/// allocation remains a separate energy-system concern and is not invented
/// here.
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
            .map(|candidate| {
                let props_a = organism.structure.units[unit_a].properties(catalog)?;
                let props_b = organism.structure.units[unit_b].properties(catalog)?;
                Some(evaluate_formation(candidate, props_a.cohesion, props_b.cohesion))
            })
            .flatten()
            {
                if best
                    .as_ref()
                    .map(|(_, _, current)| evaluation.candidate.distance < current.candidate.distance)
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

    let interaction = experimental_interaction(
        props_a,
        props_b,
        evaluation.candidate,
        water_field,
    );
    if interaction.direction <= 0.0 || interaction.magnitude <= EPSILON {
        return None;
    }

    let material = Material {
        parts: vec![
            (organism.structure.units[unit_a].resource_name.clone(), 1.0),
            (organism.structure.units[unit_b].resource_name.clone(), 1.0),
        ],
        bonded: false,
    };
    let work_cost = experimental_combine_work_cost(
        props_a,
        props_b,
        evaluation.candidate,
        water_field,
    );

    // Formation deficit is an explicit organism payment. The interaction
    // itself is the physical investment; no arbitrary extra surplus is
    // fabricated in this phase.
    let surplus = interaction.magnitude - evaluation.threshold;
    if surplus < 0.0 {
        let deficit = -surplus;
        let total_payment = work_cost + deficit;
        if organism.usable_energy + EPSILON < total_payment {
            return None;
        }
        organism.usable_energy -= total_payment;
    } else {
        if organism.usable_energy + EPSILON < work_cost {
            return None;
        }
        organism.usable_energy -= work_cost;
    }

    let bond_strength = crate::combine::experimental_bond_strength(surplus.max(0.0));
    let bond = Bond {
        unit_a,
        point_a: evaluation.candidate.point_a,
        unit_b,
        point_b: evaluation.candidate.point_b,
        strength: bond_strength,
    };

    if crate::contact::try_add_bond(&mut organism.structure, bond, catalog).is_err() {
        return None;
    }

    let _ = material;

    Some(CombineAttempt {
        unit_a,
        unit_b,
        point_a: evaluation.candidate.point_a,
        point_b: evaluation.candidate.point_b,
        work_cost,
        energy_paid: if surplus < 0.0 { work_cost - surplus } else { work_cost },
        interaction_direction: interaction.direction,
        interaction_magnitude: interaction.magnitude,
        formation_threshold: evaluation.threshold,
        surplus,
        bond_strength,
    })
}
