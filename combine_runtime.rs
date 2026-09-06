//! Runtime COMBINE boundary.
//!
//! This module bridges organism-owned material storage, structural units,
//! contact, and the experimental COMBINE equations. ACQUIRE does not call into
//! this module; COMBINE is the operation that changes material or structure.

use crate::combine::{bond_strength, eligible_candidates};
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::{BaseResource, ConnectionSites, Material};
use crate::state::{Environment, Organism};
use crate::structure::{Placement, StructuralUnit};

const EPSILON: f64 = 1e-12;
const COMBINE_CONTACT_TOLERANCE: f64 = 1.0;

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

/// Convert one stored, unstructured material object into one physical
/// structural unit. Validation happens before inventory is consumed.
pub(crate) fn instantiate_one_unit(
    organism: &mut Organism,
    catalog: &[BaseResource],
) -> Option<usize> {
    let material = organism.stored_material.peek_one_unstructured()?;
    let resource_name = material
        .parts
        .first()
        .filter(|(_, amount)| (*amount - 1.0).abs() <= EPSILON)
        .map(|(name, _)| name.clone())?;
    if material.parts.len() != 1 || catalog.iter().all(|base| base.name != resource_name) {
        return None;
    }
    let material = organism
        .stored_material
        .take_one_unstructured_named(&resource_name)?;
    let (x, y) = organism
        .occupied_cells
        .first()
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    debug_assert_eq!(material.total_amount(), 1.0);
    Some(organism.structure.add_unit(StructuralUnit::new(
        resource_name,
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        },
    )))
}

fn water_field_amount(environment: &Environment, organism: &Organism) -> f64 {
    organism
        .occupied_cells
        .first()
        .and_then(|position| environment.field.index_for_position(position.x, position.y))
        .map(|index| {
            environment.field.cells[index]
                .materials
                .iter()
                .flat_map(|material| material.parts.iter())
                .filter(|(name, _)| name == "Water")
                .map(|(_, amount)| *amount)
                .sum::<f64>()
        })
        .unwrap_or(0.0)
}

/// Find the placement that puts a stored free unit into physical contact with
/// an existing available connection site. The new unit is rotated so its
/// selected site faces the existing site's outward normal.
fn placement_for_connection(
    existing: &crate::structure::StructuralUnit,
    existing_point: crate::resources::ConnectionPoint,
    new_point: crate::resources::ConnectionPoint,
) -> Placement {
    let existing_world = crate::contact::world_connection_point(existing_point, existing);
    let existing_angle = existing_world.normal_y.atan2(existing_world.normal_x);
    let rotation = existing_angle + std::f64::consts::PI - new_point.direction_radians;
    let (s, c) = rotation.sin_cos();
    let rotated_x = new_point.x * c - new_point.y * s;
    let rotated_y = new_point.x * s + new_point.y * c;
    Placement {
        x: existing_world.x - rotated_x,
        y: existing_world.y - rotated_y,
        rotation_radians: rotation,
    }
}

/// Build and bond one stored free material unit to the existing physical
/// structure. The operation is transactional: storage is only consumed after
/// a complete candidate bond and energy requirement have been established.
pub(crate) fn try_combine_stored_unit(
    organism: &mut Organism,
    environment: &Environment,
    compatibility_cache: &mut ConnectionCompatibilityCache,
) -> Option<CombineAttempt> {
    let raw = organism.stored_material.peek_one_unstructured()?;
    if raw.parts.len() != 1
        || raw.internal_bonds.len() != 0
        || (raw.parts[0].1 - 1.0).abs() > EPSILON
        || environment.catalog.iter().all(|base| base.name != raw.parts[0].0)
    {
        return None;
    }
    let resource_name = raw.parts[0].0.clone();
    let new_sites = match environment
        .catalog
        .iter()
        .find(|base| base.name == resource_name)
        .map(|base| base.shape.connection_sites())
    {
        Some(ConnectionSites::Corners(points)) => points,
        _ => return None,
    };

    let mut best: Option<(
        usize,
        usize,
        Placement,
        crate::combine::FormationEvaluation,
        crate::combine::ExperimentalInteraction,
        f64,
        f64,
    )> = None;
    let water_field = water_field_amount(environment, organism);

    for unit_a in 0..organism.structure.units.len() {
        let Some(ConnectionSites::Corners(existing_sites)) = organism.structure.units[unit_a]
            .connection_sites(&environment.catalog)
        else {
            continue;
        };
        for (point_a, &existing_point) in existing_sites.iter().enumerate() {
            if organism.structure.connection_count(unit_a, point_a) != 0 {
                continue;
            }
            for &new_point in &new_sites {
                let placement = placement_for_connection(
                    &organism.structure.units[unit_a],
                    existing_point,
                    new_point,
                );
                let mut hypothetical = organism.structure.clone();
                let unit_b = hypothetical.add_unit(StructuralUnit::new(
                    resource_name.clone(),
                    placement,
                ));
                let candidates = crate::contact::connection_pair_candidates_cached(
                    &hypothetical,
                    unit_a,
                    unit_b,
                    &environment.catalog,
                    compatibility_cache,
                );
                for candidate in candidates.into_iter().filter(|candidate| {
                    candidate.point_a == point_a
                        && candidate.distance <= COMBINE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                }) {
                    let props_a = hypothetical.units[unit_a].properties(&environment.catalog)?;
                    let props_b = hypothetical.units[unit_b].properties(&environment.catalog)?;
                    let evaluation = crate::combine::evaluate_formation(
                        candidate,
                        props_a.cohesion,
                        props_b.cohesion,
                    );
                    let (interaction, work_cost, energy_paid) =
                        crate::structural_combine::required_investment(
                            *props_a,
                            *props_b,
                            evaluation,
                            water_field,
                        )
                        .ok()?;
                    if best
                        .as_ref()
                        .map(|current| energy_paid < current.5)
                        .unwrap_or(true)
                    {
                        best = Some((
                            unit_a,
                            point_a,
                            placement,
                            evaluation,
                            interaction,
                            work_cost,
                            energy_paid,
                        ));
                    }
                }
            }
        }
    }

    let (
        unit_a,
        point_a,
        placement,
        evaluation,
        interaction,
        work_cost,
        energy_paid,
    ) = best?;
    if organism.usable_energy + EPSILON < energy_paid {
        return None;
    }

    // Recreate the exact candidate on the live structure after all planning
    // and energy checks have succeeded. No inventory is consumed before this.
    let material = organism
        .stored_material
        .take_one_unstructured_named(&resource_name)?;
    let unit_b = organism.structure.add_unit(StructuralUnit::new(
        resource_name,
        placement,
    ));
    let bond_strength = {
        let props_a = organism.structure.units[unit_a].properties(&environment.catalog)?;
        let props_b = organism.structure.units[unit_b].properties(&environment.catalog)?;
        bond_strength(*props_a, *props_b)
    };
    let bond_energy = (energy_paid - evaluation.threshold).max(0.0);
    let bond = crate::structure::Bond {
        unit_a,
        point_a,
        unit_b,
        point_b: evaluation.candidate.point_b,
        strength: bond_strength,
        bond_energy,
    };
    let bond_index = match crate::contact::try_add_bond(
        &mut organism.structure,
        bond,
        &environment.catalog,
    ) {
        Ok(index) => index,
        Err(_) => {
            organism.structure.units.pop();
            debug_assert!(organism.store_material(material));
            return None;
        }
    };
    debug_assert_eq!(bond_index, organism.structure.bonds.len() - 1);
    organism.usable_energy -= energy_paid;

    Some(CombineAttempt {
        unit_a,
        unit_b,
        point_a,
        point_b: evaluation.candidate.point_b,
        work_cost,
        energy_paid,
        interaction_direction: interaction.direction,
        interaction_magnitude: interaction.magnitude,
        formation_threshold: evaluation.threshold,
        surplus: energy_paid - evaluation.threshold,
        bond_strength,
        bond_energy,
    })
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
            .filter(|candidate| candidate.distance <= COMBINE_CONTACT_TOLERANCE)
            .filter_map(|candidate| {
                let a = organism.structure.units[unit_a].properties(catalog)?;
                let b = organism.structure.units[unit_b].properties(catalog)?;
                Some(crate::combine::evaluate_formation(
                    candidate,
                    a.cohesion,
                    b.cohesion,
                ))
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
    let water_field = water_field_amount(environment, organism);

    let (interaction, work_cost, energy_paid) =
        crate::structural_combine::required_investment(props_a, props_b, evaluation, water_field)
            .ok()?;
    let surplus = energy_paid - evaluation.threshold;
    if organism.usable_energy + EPSILON < energy_paid {
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
    !raw.has_internal_structure()
        && raw.parts.iter().all(|(name, amount)| {
            *amount >= 0.0 && catalog.iter().any(|r| r.name == *name)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::state::Position;

    #[test]
    fn stored_unit_is_not_lost_when_validation_fails() {
        let mut organism = Organism::default();
        organism
            .stored_material
            .store(Material::free_base("NotInCatalog", 1.0));
        let catalog = default_catalog();
        assert!(instantiate_one_unit(&mut organism, &catalog).is_none());
        assert_eq!(organism.stored_material.count_unstructured(), 1);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn stored_free_material_can_become_a_bonded_structural_unit() {
        let catalog = default_catalog();
        let mut organism = Organism::default();
        organism.occupied_cells.push(Position { x: 0.0, y: 0.0 });
        organism.usable_energy = 1_000.0;
        organism
            .structure
            .add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        organism
            .stored_material
            .store(Material::free_base("Carbon", 1.0));

        let environment = Environment {
            catalog,
            ..Default::default()
        };
        let mut cache = ConnectionCompatibilityCache::new();
        let result = try_combine_stored_unit(&mut organism, &environment, &mut cache);

        assert!(result.is_some());
        assert_eq!(organism.structure.units.len(), 2);
        assert_eq!(organism.structure.bonds.len(), 1);
        assert_eq!(organism.stored_material.count_unstructured(), 0);
        assert!(organism.structure.bonds[0].bond_energy >= 0.0);
    }
}
