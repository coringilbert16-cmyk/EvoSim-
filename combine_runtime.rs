use crate::combine::{effective_reactivity, evaluate_formation, experimental_interaction};
use crate::contact::{connection_pair_candidates, try_add_bond};
use crate::geometry::Position;
use crate::resources::Material;
use crate::state::{Organism, WorldState};

const EPSILON: f64 = 1.0e-9;
const SURPLUS_BOND_FRACTION: f64 = 0.5;
const SURPLUS_USABLE_FRACTION: f64 = 0.4;
const SURPLUS_HEAT_FRACTION: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CombineAttempt {
    pub(crate) unit_a: usize,
    pub(crate) point_a: usize,
    pub(crate) unit_b: usize,
    pub(crate) point_b: usize,
    pub(crate) energy_released: f64,
    pub(crate) energy_paid: f64,
    pub(crate) usable_energy_gained: f64,
    pub(crate) heat_dissipated: f64,
    pub(crate) bond_energy: f64,
    pub(crate) bond_strength: f64,
}

fn instantiate_one_unit(
    organism: &mut Organism,
    material: &Material,
    position: Position,
) -> Option<usize> {
    let part = material.parts.first()?.0.clone();
    if material.bonded || material.parts.len() != 1 {
        return None;
    }

    let index = organism.structure.units.len();
    organism.structure.units.push(crate::structure::StructuralUnit {
        resource_name: part,
        placement: crate::structure::Placement {
            x: position.x,
            y: position.y,
            rotation_radians: 0.0,
        },
    });
    Some(index)
}

pub(crate) fn try_combine(
    world: &mut WorldState,
    organism_index: usize,
) -> Option<CombineAttempt> {
    let organism = world.organisms.get_mut(organism_index)?;
    let candidates = connection_pair_candidates(&organism.structure, &world.catalog);
    let evaluation = candidates.first()?.clone();

    let material_a = organism
        .structure
        .unit_material(evaluation.unit_a, &world.catalog)?;
    let material_b = organism
        .structure
        .unit_material(evaluation.unit_b, &world.catalog)?;
    let props_a = world.catalog.properties(&material_a)?;
    let props_b = world.catalog.properties(&material_b)?;

    let water_field = organism
        .stored_unbonded
        .parts
        .iter()
        .filter(|(name, _)| name == "Water")
        .map(|(_, amount)| *amount)
        .sum::<f64>();
    let interaction =
        experimental_interaction(props_a, props_b, evaluation.candidate, water_field);
    if interaction.direction <= 0.0 || interaction.magnitude <= EPSILON {
        return None;
    }

    let formation = evaluate_formation(props_a, props_b, evaluation.candidate);
    let effective_reactivity = effective_reactivity(props_a, props_b, water_field);
    let potential = (props_a.potential_energy + props_b.potential_energy).max(0.0);
    let interaction_modifier =
        (interaction.magnitude / effective_reactivity.max(EPSILON)).clamp(0.0, 1.0);
    let energy_released = potential * interaction_modifier;

    let energy_paid = (formation.work_cost - energy_released).max(0.0);
    if organism.usable_energy + EPSILON < energy_paid {
        return None;
    }

    organism.usable_energy -= energy_paid;
    let surplus = (energy_released - formation.work_cost).max(0.0);
    let bond_energy = surplus * SURPLUS_BOND_FRACTION;
    let usable_energy_gained = surplus * SURPLUS_USABLE_FRACTION;
    let heat_dissipated = surplus * SURPLUS_HEAT_FRACTION;
    organism.usable_energy += usable_energy_gained;

    let bond_strength = crate::combine::bond_strength_from_surplus(
        surplus,
        props_a.cohesion,
        props_b.cohesion,
    );

    let bond = crate::structure::Bond {
        unit_a: evaluation.unit_a,
        point_a: evaluation.point_a,
        unit_b: evaluation.unit_b,
        point_b: evaluation.point_b,
        strength: bond_strength,
        bond_energy,
    };
    try_add_bond(&mut organism.structure, bond).ok()?;

    Some(CombineAttempt {
        unit_a: evaluation.unit_a,
        point_a: evaluation.point_a,
        unit_b: evaluation.unit_b,
        point_b: evaluation.point_b,
        energy_released,
        energy_paid,
        usable_energy_gained,
        heat_dissipated,
        bond_energy,
        bond_strength,
    })
}
