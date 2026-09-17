use crate::material_geometry::PlacedMaterialPart;
use crate::state::{Environment, Organism, Simulation};
use crate::structure::Placement;

impl Simulation {
    pub(crate) fn update_movement(
        organism: &mut Organism,
        environment: &mut Environment,
        other_organisms: &mut [Organism],
    ) -> bool {
        let memory_strength = organism.genome.memory_strength();
        let movement_efficiency = organism.genome.movement_efficiency();
        let perception_weight = 1.0 - (0.5 + memory_strength * 0.5);
        let memory_weight = 1.0 - perception_weight;
        let (px, py) = match organism.occupied_cells.first() {
            Some(p) => (p.x, p.y),
            None => return false,
        };
        let mut mx = 0.0;
        let mut my = 0.0;
        let mut total = 0.0;
        for point in &organism.memory {
            let dx = point.x - px;
            let dy = point.y - py;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance <= f64::EPSILON {
                continue;
            }
            let weight = point.strength / distance;
            mx += dx / distance * weight;
            my += dy / distance * weight;
            total += weight;
        }
        if total > 0.0 {
            mx /= total;
            my /= total;
        }
        if organism.active_transformation_id.is_some() {
            return false;
        }
        let mut x = memory_weight * mx + perception_weight * organism.resource_sense.direction_x;
        let mut y = memory_weight * my + perception_weight * organism.resource_sense.direction_y;
        let magnitude = (x * x + y * y).sqrt();
        if magnitude <= f64::EPSILON {
            return false;
        }
        x /= magnitude;
        y /= magnitude;
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
        let new_y = (old_y + delta_y).clamp(0.0, environment.height);
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

        organism.occupied_cells[0].x = new_x;
        organism.occupied_cells[0].y = new_y;
        for unit in &mut organism.structure.units {
            unit.placement.x += dx;
            unit.placement.y += dy;
        }
        for (original, trial) in other_organisms.iter_mut().zip(trial_organisms) {
            original.occupied_cells = trial.occupied_cells;
            original.structure = trial.structure;
        }
        *environment = trial_environment;
        true
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
    push_organism_blockers(
        moving,
        other_organisms,
        environment,
        dx,
        dy,
        &mut organism_visited,
        &mut physical_visited,
    )
}

fn push_organism_blockers(
    moving: &Organism,
    other_organisms: &mut [Organism],
    environment: &mut Environment,
    dx: f64,
    dy: f64,
    organism_visited: &mut [bool],
    physical_visited: &mut std::collections::HashSet<(usize, usize)>,
) -> bool {
    for index in 0..other_organisms.len() {
        if organism_visited[index]
            || !organism_overlaps_after(&other_organisms[index], moving, dx, dy, environment)
        {
            continue;
        }
        if !can_translate_organism(&other_organisms[index], environment, dx, dy) {
            return false;
        }
        organism_visited[index] = true;
        let blocker = other_organisms[index].clone();
        if !push_organism_blockers(
            &blocker,
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
    push_physical_blockers(moving, environment, dx, dy, physical_visited)
}

fn push_physical_blockers(
    moving: &Organism,
    environment: &mut Environment,
    dx: f64,
    dy: f64,
    physical_visited: &mut std::collections::HashSet<(usize, usize)>,
) -> bool {
    for cell_index in 0..environment.field.cells.len() {
        let material_count = environment.field.cells[cell_index].physical_materials.len();
        for material_index in 0..material_count {
            if physical_visited.contains(&(cell_index, material_index)) {
                continue;
            }
            let physical = &environment.field.cells[cell_index].physical_materials[material_index];
            if !physical.is_realized()
                || physical.material.is_empty()
                || !physical_overlaps_after(physical, moving, dx, dy, environment)
            {
                continue;
            }
            if !can_translate_physical(physical, environment, dx, dy) {
                return false;
            }
            let mut pushed = physical.clone();
            translate_physical(&mut pushed, dx, dy);
            physical_visited.insert((cell_index, material_index));
            environment.field.cells[cell_index].physical_materials[material_index] = pushed;
        }
    }
    true
}

fn can_translate_physical(
    physical: &crate::physical_material::PhysicalMaterial,
    environment: &Environment,
    dx: f64,
    dy: f64,
) -> bool {
    let Some(placements) = &physical.placements else {
        return false;
    };
    placements.iter().all(|p| {
        let x = p.x + dx;
        let y = p.y + dy;
        x.is_finite()
            && y.is_finite()
            && x >= 0.0
            && y >= 0.0
            && x <= environment.width
            && y <= environment.height
    })
}

fn translate_physical(physical: &mut crate::physical_material::PhysicalMaterial, dx: f64, dy: f64) {
    if let Some(placements) = physical.placements.as_mut() {
        for placement in placements {
            placement.x += dx;
            placement.y += dy;
        }
    }
}
fn organism_overlaps_after(
    blocker: &Organism,
    moving: &Organism,
    dx: f64,
    dy: f64,
    environment: &Environment,
) -> bool {
    for blocker_unit in &blocker.structure.units {
        let Some(blocker_shape) = blocker_unit.shape(&environment.catalog) else {
            continue;
        };
        let blocker_part = PlacedMaterialPart {
            part_index: 1,
            form: blocker_shape.form.clone(),
            placement: blocker_unit.placement,
        };
        for moving_unit in &moving.structure.units {
            let Some(moving_shape) = moving_unit.shape(&environment.catalog) else {
                continue;
            };
            let moved_part = PlacedMaterialPart {
                part_index: 0,
                form: moving_shape.form.clone(),
                placement: Placement {
                    x: moving_unit.placement.x + dx,
                    y: moving_unit.placement.y + dy,
                    rotation_radians: moving_unit.placement.rotation_radians,
                },
            };
            if crate::material_geometry::placed_forms_penetrate(&moved_part, &blocker_part, 0.0) {
                return true;
            }
        }
    }
    false
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
            && y - radius >= 0.0
            && x + radius <= environment.width
            && y + radius <= environment.height
    })
}

fn translate_organism(organism: &mut Organism, dx: f64, dy: f64) {
    for point in &mut organism.occupied_cells {
        point.x += dx;
        point.y += dy;
    }
    for unit in &mut organism.structure.units {
        unit.placement.x += dx;
        unit.placement.y += dy;
    }
}

fn physical_overlaps_after(
    physical: &crate::physical_material::PhysicalMaterial,
    moving: &Organism,
    dx: f64,
    dy: f64,
    environment: &Environment,
) -> bool {
    let Some(placements) = &physical.placements else {
        return false;
    };
    for ((name, amount), placement) in physical.material.parts.iter().zip(placements.iter()) {
        if (*amount - 1.0).abs() > 1e-9 {
            continue;
        }
        let Some(base) = environment.catalog.iter().find(|b| b.name == *name) else {
            continue;
        };
        let blocker = PlacedMaterialPart {
            part_index: 1,
            form: base.shape.form.clone(),
            placement: *placement,
        };
        for unit in &moving.structure.units {
            let Some(shape) = unit.shape(&environment.catalog) else {
                continue;
            };
            let moved = PlacedMaterialPart {
                part_index: 0,
                form: shape.form.clone(),
                placement: Placement {
                    x: unit.placement.x + dx,
                    y: unit.placement.y + dy,
                    rotation_radians: unit.placement.rotation_radians,
                },
            };
            if crate::material_geometry::placed_forms_penetrate(&moved, &blocker, 0.0) {
                return true;
            }
        }
    }
    false
}

fn movement_collides(
    organism: &Organism,
    environment: &Environment,
    other_organisms: &[Organism],
    dx: f64,
    dy: f64,
) -> bool {
    for unit in &organism.structure.units {
        let Some(shape) = unit.shape(&environment.catalog) else {
            continue;
        };
        let moved = PlacedMaterialPart {
            part_index: 0,
            form: shape.form.clone(),
            placement: Placement {
                x: unit.placement.x + dx,
                y: unit.placement.y + dy,
                rotation_radians: unit.placement.rotation_radians,
            },
        };
        for other in other_organisms {
            for blocker_unit in &other.structure.units {
                let Some(blocker_shape) = blocker_unit.shape(&environment.catalog) else {
                    continue;
                };
                let blocker = PlacedMaterialPart {
                    part_index: 1,
                    form: blocker_shape.form.clone(),
                    placement: blocker_unit.placement,
                };
                if crate::material_geometry::placed_forms_penetrate(&moved, &blocker, 0.0) {
                    return true;
                }
            }
        }
        let radius = moved.form.bounding_radius().max(0.0);
        let min_col =
            ((moved.placement.x - radius).max(0.0) / environment.field.cell_size).floor() as usize;
        let max_col =
            ((moved.placement.x + radius).max(0.0) / environment.field.cell_size).floor() as usize;
        let min_row =
            ((moved.placement.y - radius).max(0.0) / environment.field.cell_size).floor() as usize;
        let max_row =
            ((moved.placement.y + radius).max(0.0) / environment.field.cell_size).floor() as usize;
        let max_col = max_col.min(environment.field.width_cells.saturating_sub(1));
        let max_row = max_row.min(environment.field.height_cells.saturating_sub(1));
        if min_col >= environment.field.width_cells || min_row >= environment.field.height_cells {
            continue;
        }
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let cell = &environment.field.cells[row * environment.field.width_cells + col];
                if cell
                    .materials
                    .iter()
                    .any(|m| m.is_valid() && !m.is_empty() && m.has_internal_structure())
                {
                    return true;
                }
                for physical in &cell.physical_materials {
                    if !physical.is_realized() || physical.material.is_empty() {
                        continue;
                    }
                    let Some(placements) = &physical.placements else {
                        continue;
                    };
                    for ((name, amount), placement) in
                        physical.material.parts.iter().zip(placements.iter())
                    {
                        if (*amount - 1.0).abs() > 1e-9 {
                            continue;
                        }
                        let Some(base) = environment.catalog.iter().find(|b| b.name == *name)
                        else {
                            continue;
                        };
                        let blocker = PlacedMaterialPart {
                            part_index: 1,
                            form: base.shape.form.clone(),
                            placement: *placement,
                        };
                        if crate::material_geometry::placed_forms_penetrate(&moved, &blocker, 0.0) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
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
    fn movement_is_blocked_by_another_organism() {
        let simulation = Simulation::new(7, 20.0);
        let mut environment = empty_environment(&simulation);
        let mut organism = simulation.organisms[0].clone();
        let mut blocker = simulation.organisms[0].clone();
        blocker.id = "blocker".to_string();
        let x = organism.structure.units[0].placement.x;
        let y = organism.structure.units[0].placement.y;
        for unit in &mut blocker.structure.units {
            unit.placement.x = x + 4.0;
            unit.placement.y = y;
        }
        let old_anchor = organism.occupied_cells[0].clone();
        assert!(!Simulation::try_move_cell(
            &mut organism,
            &mut environment,
            &mut [blocker],
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
            unit.placement.x = x + 7.0;
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
}
