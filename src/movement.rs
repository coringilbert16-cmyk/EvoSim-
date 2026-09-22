use crate::material_geometry::PlacedMaterialPart;
use crate::state::{Environment, Organism, Simulation};
use crate::structure::Placement;

impl Simulation {
    pub(crate) fn update_movement(
        organism: &mut Organism,
        environment: &mut Environment,
        other_organisms: &mut [Organism],
    ) -> bool {
        let movement_efficiency = organism.genome.movement_efficiency();
        let (x, y) = match movement_direction_periodic(organism, environment.height) {
            Some(direction) => direction,
            None => return false,
        };
        if organism.active_transformation_id.is_some() {
            return false;
        }
        let step = 5.0 * movement_efficiency;
        Self::try_move_cell(organism, environment, other_organisms, x * step, y * step)
    }

    pub(crate) fn try_move_cell(
        organism: &mut Organism,
        environment: &mut Environment,
        other_organisms: &mut [Organism],
        delta_x: f64,
        delta_y: f64,
    ) -> bool {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return false;
        }
        let (old_x, old_y) = match organism.occupied_cells.first() {
            Some(p) => (p.x, p.y),
            None => return false,
        };
        let new_x = (old_x + delta_x).clamp(0.0, environment.width);
        let new_y = wrap_y(old_y + delta_y, environment.height);
        let dx = new_x - old_x;
        let dy = new_y - old_y;
        if dx.abs() <= f64::EPSILON && dy.abs() <= f64::EPSILON {
            return false;
        }

        let mut trial_environment = environment.clone();
        let mut trial_organisms = other_organisms.to_vec();
        if !resolve_push_chain(
            organism,
            &mut trial_organisms,
            &mut trial_environment,
            dx,
            dy,
        ) {
            return false;
        }
        reindex_physical_materials(&mut trial_environment);

        organism.occupied_cells[0].x = new_x;
        organism.occupied_cells[0].y = new_y;
        organism.developmental_origin.x += dx;
        organism.developmental_origin.y =
            wrap_y(organism.developmental_origin.y + dy, environment.height);
        for unit in &mut organism.structure.units {
            unit.placement.x += dx;
            unit.placement.y = wrap_y(unit.placement.y + dy, environment.height);
        }
        translate_reproductive_construction(organism, dx, dy, environment.height);
        for (original, trial) in other_organisms.iter_mut().zip(trial_organisms) {
            original.occupied_cells = trial.occupied_cells;
            original.structure = trial.structure;
        }
        *environment = trial_environment;
        true
    }
}

fn movement_direction(organism: &Organism) -> Option<(f64, f64)> {
    movement_direction_periodic(organism, 0.0)
}

pub(crate) fn movement_direction_periodic(
    organism: &Organism,
    environment_height: f64,
) -> Option<(f64, f64)> {
    let memory_strength = organism.genome.memory_strength();
    let perception_weight = 1.0 - (0.5 + memory_strength * 0.5);
    let memory_weight = 1.0 - perception_weight;
    let (px, py) = organism.occupied_cells.first().map(|p| (p.x, p.y))?;

    let mut memory_x = 0.0;
    let mut memory_y = 0.0;
    let mut total = 0.0;
    for point in &organism.memory {
        let dx = point.x - px;
        let dy = wrapped_delta(point.y, py, environment_height);
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= f64::EPSILON {
            continue;
        }
        let weight = point.strength / distance;
        memory_x += dx / distance * weight;
        memory_y += dy / distance * weight;
        total += weight;
    }
    if total > 0.0 {
        memory_x /= total;
        memory_y /= total;
    }

    let x = memory_weight * memory_x + perception_weight * organism.resource_sense.direction_x;
    let y = memory_weight * memory_y + perception_weight * organism.resource_sense.direction_y;
    let magnitude = (x * x + y * y).sqrt();
    if magnitude <= f64::EPSILON {
        None
    } else {
        Some((x / magnitude, y / magnitude))
    }
}

fn reindex_physical_materials(environment: &mut Environment) {
    let mut physical_materials = Vec::new();
    for cell in &mut environment.field.cells {
        physical_materials.append(&mut cell.physical_materials);
    }
    for physical in physical_materials {
        let Some(placement) = physical
            .placements
            .as_ref()
            .and_then(|placements| placements.first())
        else {
            continue;
        };
        let Some(index) = environment
            .field
            .index_for_position(placement.x, placement.y)
        else {
            continue;
        };
        environment.field.cells[index]
            .physical_materials
            .push(physical);
    }
}

fn resolve_push_chain(
    moving: &Organism,
    other_organisms: &mut [Organism],
    environment: &mut Environment,
    dx: f64,
    dy: f64,
) -> bool {
    let mut organism_visited = vec![false; other_organisms.len()];
    let mut physical_visited = std::collections::HashSet::new();
    let moving_destination = organism_parts_at(moving, environment, dx, dy);
    if static_material_blocks(&moving_destination, environment) {
        return false;
    }
    push_blockers_for_parts(
        &moving_destination,
        other_organisms,
        environment,
        dx,
        dy,
        &mut organism_visited,
        &mut physical_visited,
    )
}

fn push_blockers_for_parts(
    moving_destination: &[PlacedMaterialPart],
    other_organisms: &mut [Organism],
    environment: &mut Environment,
    dx: f64,
    dy: f64,
    organism_visited: &mut [bool],
    physical_visited: &mut std::collections::HashSet<(usize, usize)>,
) -> bool {
    for index in 0..other_organisms.len() {
        if organism_visited[index] {
            continue;
        }
        let candidate = other_organisms[index].clone();
        let candidate_parts = organism_parts_at(&candidate, environment, 0.0, 0.0);
        if !parts_penetrate(moving_destination, &candidate_parts, environment.height) {
            continue;
        }
        if !can_translate_organism(&candidate, environment, dx, dy) {
            return false;
        }
        let destination = organism_parts_at(&candidate, environment, dx, dy);
        if static_material_blocks(&destination, environment) {
            return false;
        }
        organism_visited[index] = true;
        if !push_blockers_for_parts(
            &destination,
            other_organisms,
            environment,
            dx,
            dy,
            organism_visited,
            physical_visited,
        ) {
            return false;
        }
        translate_organism(&mut other_organisms[index], dx, dy);
    }

    for cell_index in 0..environment.field.cells.len() {
        let material_count = environment.field.cells[cell_index].physical_materials.len();
        for material_index in 0..material_count {
            let key = (cell_index, material_index);
            if physical_visited.contains(&key) {
                continue;
            }
            let candidate =
                environment.field.cells[cell_index].physical_materials[material_index].clone();
            if !candidate.is_realized() || candidate.material.is_empty() {
                continue;
            }
            let candidate_parts = physical_parts_at(&candidate, environment, 0.0, 0.0);
            if !parts_penetrate(moving_destination, &candidate_parts, environment.height) {
                continue;
            }
            if !can_translate_physical(&candidate, environment, dx, dy) {
                return false;
            }
            let destination = physical_parts_at(&candidate, environment, dx, dy);
            if static_material_blocks(&destination, environment) {
                return false;
            }
            physical_visited.insert(key);
            if !push_blockers_for_parts(
                &destination,
                other_organisms,
                environment,
                dx,
                dy,
                organism_visited,
                physical_visited,
            ) {
                return false;
            }
            let mut pushed = candidate;
            translate_physical(&mut pushed, dx, dy, environment.height);
            environment.field.cells[cell_index].physical_materials[material_index] = pushed;
        }
    }
    true
}

fn organism_parts_at(
    organism: &Organism,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> Vec<PlacedMaterialPart> {
    organism
        .structure
        .units
        .iter()
        .filter_map(|unit| {
            let shape = unit.shape(&environment.catalog)?;
            Some(PlacedMaterialPart {
                part_index: 0,
                form: shape.form.clone(),
                placement: Placement {
                    x: unit.placement.x + dx,
                    y: unit.placement.y + dy,
                    rotation_radians: unit.placement.rotation_radians,
                },
            })
        })
        .collect()
}

fn physical_parts_at(
    physical: &crate::physical_material::PhysicalMaterial,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> Vec<PlacedMaterialPart> {
    let Some(placements) = &physical.placements else {
        return Vec::new();
    };
    physical
        .material
        .parts
        .iter()
        .zip(placements.iter())
        .enumerate()
        .filter_map(|(part_index, ((name, amount), placement))| {
            if (*amount - 1.0).abs() > 1e-9 {
                return None;
            }
            let base = environment.catalog.iter().find(|b| b.name == *name)?;
            Some(PlacedMaterialPart {
                part_index,
                form: base.shape.form.clone(),
                placement: Placement {
                    x: placement.x + dx,
                    y: placement.y + dy,
                    rotation_radians: placement.rotation_radians,
                },
            })
        })
        .collect()
}

fn parts_penetrate(
    a: &[PlacedMaterialPart],
    b: &[PlacedMaterialPart],
    environment_height: f64,
) -> bool {
    a.iter().any(|part_a| {
        b.iter().any(|part_b| {
            if crate::material_geometry::placed_forms_penetrate(part_a, part_b, 0.0) {
                return true;
            }
            // The active field is vertically periodic. Test the two wrapped
            // images needed to detect contact across the seam without making
            // the seam itself a physical wall.
            if environment_height <= 0.0 {
                return false;
            }
            let mut wrapped = part_b.clone();
            wrapped.placement.y += environment_height;
            if crate::material_geometry::placed_forms_penetrate(part_a, &wrapped, 0.0) {
                return true;
            }
            wrapped.placement.y -= 2.0 * environment_height;
            crate::material_geometry::placed_forms_penetrate(part_a, &wrapped, 0.0)
        })
    })
}

fn wrapped_delta(target: f64, origin: f64, period: f64) -> f64 {
    let direct = target - origin;
    if period > 0.0 {
        return (direct + period * 0.5).rem_euclid(period) - period * 0.5;
    }
    direct
}

fn wrap_y(y: f64, height: f64) -> f64 {
    if height > 0.0 {
        y.rem_euclid(height)
    } else {
        y
    }
}

fn static_material_blocks(parts: &[PlacedMaterialPart], environment: &Environment) -> bool {
    // The field grid is an index, not the physical authority. Structured
    // aggregate Material values therefore cannot act as collision geometry;
    // only already-realized physical material can block movement.
    for cell in &environment.field.cells {
        for physical in &cell.physical_materials {
            let candidate_parts = physical_parts_at(physical, environment, 0.0, 0.0);
            if !candidate_parts.is_empty()
                && parts_penetrate(parts, &candidate_parts, environment.height)
            {
                return true;
            }
        }
    }
    false
}

fn can_translate_physical(
    physical: &crate::physical_material::PhysicalMaterial,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> bool {
    let parts = physical_parts_at(physical, environment, dx, dy);
    !parts.is_empty()
        && parts.iter().all(|part| {
            let radius = part.form.bounding_radius();
            let x = part.placement.x;
            let y = part.placement.y;
            x.is_finite()
                && y.is_finite()
                && x - radius >= 0.0
                && x + radius <= environment.width
        })
}

fn translate_physical(
    physical: &mut crate::physical_material::PhysicalMaterial,
    dx: f64,
    dy: f64,
    environment_height: f64,
) {
    if let Some(placements) = physical.placements.as_mut() {
        for placement in placements {
            placement.x += dx;
            placement.y = wrap_y(placement.y + dy, environment_height);
        }
    }
}

fn can_translate_organism(
    organism: &Organism,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> bool {
    organism.structure.units.iter().all(|unit| {
        let Some(shape) = unit.shape(&environment.catalog) else {
            return false;
        };
        let radius = shape.form.bounding_radius();
        let x = unit.placement.x + dx;
        let y = unit.placement.y + dy;
        x.is_finite()
            && y.is_finite()
            && x - radius >= 0.0
            && x + radius <= environment.width
    })
}

fn translate_reproductive_construction(
    organism: &mut Organism,
    dx: f64,
    dy: f64,
    environment_height: f64,
) {
    if let Some(construction) = organism.reproductive_construction.as_mut() {
        construction.developmental_origin.x += dx;
        construction.developmental_origin.y =
            wrap_y(construction.developmental_origin.y + dy, environment_height);
        for unit in &mut construction.developing_structure.units {
            unit.placement.x += dx;
            unit.placement.y = wrap_y(unit.placement.y + dy, environment_height);
        }
    }
}

fn translate_organism(
    organism: &mut Organism,
    dx: f64,
    dy: f64,
    environment_height: f64,
) {
    organism.developmental_origin.x += dx;
    organism.developmental_origin.y =
        wrap_y(organism.developmental_origin.y + dy, environment_height);
    for point in &mut organism.occupied_cells {
        point.x += dx;
        point.y = wrap_y(point.y + dy, environment_height);
    }
    for unit in &mut organism.structure.units {
        unit.placement.x += dx;
        unit.placement.y = wrap_y(unit.placement.y + dy, environment_height);
    }
    translate_reproductive_construction(organism, dx, dy, environment_height);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_environment(simulation: &Simulation) -> Environment {
        let mut environment = simulation.environment.clone();
        for cell in &mut environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        environment
    }

    #[test]
    fn movement_direction_uses_existing_memory_and_resource_sense() {
        let simulation = Simulation::new(7, 20.0);
        let mut organism = simulation.organisms[0].clone();
        organism.resource_sense.direction_x = 1.0;
        organism.resource_sense.direction_y = 0.0;
        organism.memory.push(crate::state::MemoryPoint {
            x: organism.occupied_cells[0].x,
            y: organism.occupied_cells[0].y - 20.0,
            strength: 1.0,
        });
        let (x, y) = movement_direction(&organism).expect("direction should exist");
        assert!(x > 0.0);
        assert!(y < 0.0);
    }

    #[test]
    fn movement_direction_without_inputs_is_rejected() {
        let simulation = Simulation::new(7, 20.0);
        let organism = simulation.organisms[0].clone();
        assert!(movement_direction(&organism).is_none());
    }

    #[test]
    fn move_translates_developing_offspring_with_parent() {
        let mut simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms.remove(0);
        organism.development_stage = crate::state::DevelopmentStage::Adult;
        let mut ledger = crate::state::EnergyLedger::default();
        assert!(crate::reproduction::begin_reproduction(
            &mut organism,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let before = organism
            .reproductive_construction
            .as_ref()
            .unwrap()
            .developmental_origin
            .clone();
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            12.0,
            -7.0,
        ));
        let after = &organism
            .reproductive_construction
            .as_ref()
            .unwrap()
            .developmental_origin;
        assert!((after.x - before.x - 12.0).abs() < 1e-9);
        assert!((after.y - before.y + 7.0).abs() < 1e-9);
    }

    #[test]
    fn move_translates_anchor_and_structure() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let anchor = organism.occupied_cells[0].clone();
        let placements: Vec<_> = organism
            .structure
            .units
            .iter()
            .map(|u| (u.placement.x, u.placement.y))
            .collect();
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            12.0,
            -7.0
        ));
        assert!((organism.occupied_cells[0].x - anchor.x - 12.0).abs() < 1e-9);
        assert!((organism.occupied_cells[0].y - anchor.y + 7.0).abs() < 1e-9);
        for (unit, (x, y)) in organism.structure.units.iter().zip(placements) {
            assert!((unit.placement.x - x - 12.0).abs() < 1e-9);
            assert!((unit.placement.y - y + 7.0).abs() < 1e-9);
        }
    }

    #[test]
    fn blocked_zero_move_and_nonfinite_move_are_rejected() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = simulation.environment.clone();
        let mut organism = simulation.organisms[0].clone();
        organism.occupied_cells[0].x = 0.0;
        organism.occupied_cells[0].y = 0.0;
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            -10.0,
            -10.0
        ));
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            f64::NAN,
            1.0
        ));
    }

    #[test]
    fn movement_is_blocked_by_realized_physical_material() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        let placement = crate::structure::Placement {
            x: x + 5.0,
            y,
            rotation_radians: 0.0,
        };
        let physical = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                placement.clone(),
                crate::structure::Placement {
                    x: x + 6.0,
                    y,
                    rotation_radians: 0.0,
                },
            ],
            &environment.catalog,
        )
        .expect("realized composite should be valid");
        let index = environment
            .field
            .index_for_position(placement.x, placement.y)
            .expect("physical material must be in bounds");
        assert!(environment.field.deposit_physical_at_index(index, physical));
        let old_anchor = organism.occupied_cells[0].clone();
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            5.0,
            0.0
        ));
        assert_eq!(organism.occupied_cells[0], old_anchor);
    }

    #[test]
    fn movement_pushes_another_organism_atomically() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut blocker = simulation.organisms[0].clone();
        blocker.id = "pushed".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut blocker.structure.units {
            unit.placement.x = x + 6.0;
            unit.placement.y = y;
        }
        let mut others = vec![blocker];
        let blocker_before = others[0].structure.units[0].placement.x;
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut others,
            5.0,
            0.0
        ));
        assert!((organism.structure.units[0].placement.x - (x + 5.0)).abs() < 1e-9);
        assert!((others[0].structure.units[0].placement.x - (blocker_before + 5.0)).abs() < 1e-9);
    }

    #[test]
    fn movement_propagates_an_organism_push_chain_atomically() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut first = simulation.organisms[0].clone();
        let mut second = simulation.organisms[0].clone();
        first.id = "first".to_string();
        second.id = "second".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut first.structure.units {
            unit.placement.x = x + 6.0;
            unit.placement.y = y;
        }
        for unit in &mut second.structure.units {
            unit.placement.x = x + 10.0;
            unit.placement.y = y;
        }
        let first_before = first.structure.units[0].placement.x;
        let second_before = second.structure.units[0].placement.x;
        let mut others = vec![first, second];
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut others,
            5.0,
            0.0
        ));
        assert_eq!(organism.structure.units[0].placement.x, x + 5.0);
        assert_eq!(others[0].structure.units[0].placement.x, first_before + 5.0);
        assert_eq!(
            others[1].structure.units[0].placement.x,
            second_before + 5.0
        );
    }

    #[test]
    fn movement_pushes_realized_physical_material_and_reindexes_it() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        let placement = crate::structure::Placement {
            x: x + 6.0,
            y,
            rotation_radians: 0.0,
        };
        let physical = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![placement],
            &environment.catalog,
        )
        .expect("single carbon should have a valid physical realization");
        let original_index = environment
            .field
            .index_for_position(placement.x, placement.y)
            .expect("physical material must be in bounds");
        assert!(environment
            .field
            .deposit_physical_at_index(original_index, physical));
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            5.0,
            0.0
        ));
        let moved_x = x + 11.0;
        let moved_index = environment
            .field
            .index_for_position(moved_x, y)
            .expect("pushed material must remain in bounds");
        let physical = environment.field.cells[moved_index]
            .physical_materials
            .first()
            .expect("pushed physical material must be reindexed");
        assert_eq!(physical.placements.as_ref().unwrap()[0].x, moved_x);
    }

    #[test]
    fn touching_another_organism_does_not_block_movement() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut blocker = simulation.organisms[0].clone();
        blocker.id = "touching".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut blocker.structure.units {
            unit.placement.x = x + 10.0;
            unit.placement.y = y;
        }
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [blocker],
            5.0,
            0.0
        ));
    }

    #[test]
    fn failed_push_chain_is_atomic_for_all_affected_objects() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut first = simulation.organisms[0].clone();
        let mut second = simulation.organisms[0].clone();
        first.id = "first".to_string();
        second.id = "second".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut first.structure.units {
            unit.placement.x = x + 6.0;
            unit.placement.y = y;
        }
        for unit in &mut second.structure.units {
            unit.placement.x = x + 12.0;
            unit.placement.y = y;
        }
        let aggregate_x = x + 17.0;
        environment.field.deposit(
            aggregate_x,
            y,
            crate::resources::Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
        );
        let organism_before = organism.structure.clone();
        let first_before = first.structure.clone();
        let second_before = second.structure.clone();
        let mut others = vec![first, second];
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut others,
            5.0,
            0.0
        ));
        assert_eq!(organism.structure.units, organism_before.units);
        assert_eq!(organism.structure.bonds, organism_before.bonds);
        assert_eq!(others[0].structure.units, first_before.units);
        assert_eq!(others[0].structure.bonds, first_before.bonds);
        assert_eq!(others[1].structure.units, second_before.units);
        assert_eq!(others[1].structure.bonds, second_before.bonds);
    }

    #[test]
    fn moving_realized_material_preserves_intrinsic_realization() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        let material = crate::resources::Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let placements = vec![
            crate::structure::Placement {
                x: x + 4.0,
                y,
                rotation_radians: 0.0,
            },
            crate::structure::Placement {
                x: x + 5.0,
                y,
                rotation_radians: 0.25,
            },
        ];
        let physical = crate::physical_material::PhysicalMaterial::realized(
            material,
            placements.clone(),
            &environment.catalog,
        )
        .expect("realized bonded material should be valid");
        let original = physical.clone();
        let original_index = environment
            .field
            .index_for_position(placements[0].x, placements[0].y)
            .expect("material must be in bounds");
        assert!(environment
            .field
            .deposit_physical_at_index(original_index, physical));
        assert!(Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [],
            5.0,
            0.0
        ));
        let moved_index = environment
            .field
            .index_for_position(x + 9.0, y)
            .expect("material must remain in bounds");
        let moved = environment.field.cells[moved_index]
            .physical_materials
            .iter()
            .find(|candidate| candidate.material.parts == original.material.parts)
            .expect("realized material should remain present");
        assert_eq!(moved.material, original.material);
        assert_eq!(moved.internal_connections, original.internal_connections);
        let moved_placements = moved.placements.as_ref().expect("placements must remain");
        let original_placements = original.placements.as_ref().expect("placements must exist");
        assert_eq!(moved_placements.len(), original_placements.len());
        for (moved, original) in moved_placements.iter().zip(original_placements) {
            assert!((moved.x - original.x - 5.0).abs() < 1e-9);
            assert!((moved.y - original.y).abs() < 1e-9);
            assert_eq!(moved.rotation_radians, original.rotation_radians);
        }
    }
}
