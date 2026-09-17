use crate::material_geometry::PlacedMaterialPart;
use crate::state::{Environment, Organism, Simulation};
use crate::structure::{Placement, StructuralUnit};

impl Simulation {
    pub(crate) fn update_movement(
        organism: &mut Organism,
        environment: &Environment,
        other_organisms: &[Organism],
    ) -> bool {
        let memory_strength_trait = organism.genome.memory_strength();
        let movement_efficiency = organism.genome.movement_efficiency();
        let perception_weight = 1.0 - (0.5 + memory_strength_trait * 0.5);
        let memory_weight = 1.0 - perception_weight;
        let (px, py) = match organism.occupied_cells.first() { Some(p) => (p.x, p.y), None => return false };

        let mut mx = 0.0;
        let mut my = 0.0;
        let mut total = 0.0;
        for point in &organism.memory {
            let dx = point.x - px;
            let dy = point.y - py;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance <= f64::EPSILON { continue; }
            let weight = point.strength / distance;
            mx += (dx / distance) * weight;
            my += (dy / distance) * weight;
            total += weight;
        }
        if total > 0.0 { mx /= total; my /= total; }
        if organism.active_transformation_id.is_some() { return false; }

        let mut move_x = memory_weight * mx + perception_weight * organism.resource_sense.direction_x;
        let mut move_y = memory_weight * my + perception_weight * organism.resource_sense.direction_y;
        let magnitude = (move_x * move_x + move_y * move_y).sqrt();
        if magnitude <= f64::EPSILON { return false; }
        move_x /= magnitude;
        move_y /= magnitude;
        let step = 5.0 * movement_efficiency;
        Self::try_move_cell(organism, environment, other_organisms, move_x * step, move_y * step)
    }

    /// Canonical movement commit boundary. A proposed displacement is atomic:
    /// derived geometry must be collision-free before any world position changes.
    pub(crate) fn try_move_cell(
        organism: &mut Organism,
        environment: &Environment,
        other_organisms: &[Organism],
        delta_x: f64,
        delta_y: f64,
    ) -> bool {
        if !delta_x.is_finite() || !delta_y.is_finite() { return false; }
        let (old_x, old_y) = match organism.occupied_cells.first() { Some(p) => (p.x, p.y), None => return false };
        let new_x = (old_x + delta_x).clamp(0.0, environment.width);
        let new_y = (old_y + delta_y).clamp(0.0, environment.height);
        let dx = new_x - old_x;
        let dy = new_y - old_y;
        if dx.abs() <= f64::EPSILON && dy.abs() <= f64::EPSILON { return false; }

        if movement_collides(organism, environment, other_organisms, dx, dy) { return false; }

        let cell = organism.occupied_cells.first_mut().expect("checked above");
        cell.x = new_x;
        cell.y = new_y;
        for unit in &mut organism.structure.units {
            unit.placement.x += dx;
            unit.placement.y += dy;
        }
        true
    }
}

fn movement_collides(
    organism: &Organism,
    environment: &Environment,
    other_organisms: &[Organism],
    dx: f64,
    dy: f64,
) -> bool {
    for unit in &organism.structure.units {
        let Some(shape) = unit.shape(&environment.catalog) else { continue; };
        let moved = PlacedMaterialPart {
            part_index: 0,
            form: shape.form.clone(),
            placement: Placement { x: unit.placement.x + dx, y: unit.placement.y + dy, rotation_radians: unit.placement.rotation_radians },
        };

        for other in other_organisms {
            for blocker_unit in &other.structure.units {
                let Some(blocker_shape) = blocker_unit.shape(&environment.catalog) else { continue; };
                let blocker = PlacedMaterialPart {
                    part_index: 1,
                    form: blocker_shape.form.clone(),
                    placement: blocker_unit.placement,
                };
                if crate::material_geometry::placed_forms_overlap(&moved, &blocker, 0.0) { return true; }
            }
        }

        let radius = moved.form.bounding_radius().max(0.0);
        let min_x = (moved.placement.x - radius).max(0.0);
        let max_x = (moved.placement.x + radius).max(0.0);
        let min_y = (moved.placement.y - radius).max(0.0);
        let max_y = (moved.placement.y + radius).max(0.0);
        let min_col = (min_x / environment.field.cell_size).floor() as usize;
        let max_col = ((max_x / environment.field.cell_size).floor() as usize).min(environment.field.width_cells.saturating_sub(1));
        let min_row = (min_y / environment.field.cell_size).floor() as usize;
        let max_row = ((max_y / environment.field.cell_size).floor() as usize).min(environment.field.height_cells.saturating_sub(1));
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let index = row * environment.field.width_cells + col;
                let cell = &environment.field.cells[index];
                if cell.materials.iter().any(|m| m.is_valid() && !m.is_empty() && m.has_internal_structure()) { return true; }
                for physical in &cell.physical_materials {
                    if !physical.is_realized() || physical.material.is_empty() { continue; }
                    let Some(placements) = &physical.placements else { continue; };
                    for ((name, amount), placement) in physical.material.parts.iter().zip(placements.iter()) {
                        if (*amount - 1.0).abs() > 1e-9 { continue; }
                        let Some(base) = environment.catalog.iter().find(|b| b.name == *name) else { continue; };
                        let blocker = PlacedMaterialPart { part_index: 1, form: base.shape.form.clone(), placement: *placement };
                        if crate::material_geometry::placed_forms_overlap(&moved, &blocker, 0.0) { return true; }
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

    #[test]
    fn try_move_cell_translates_anchor_and_structure() {
        let simulation = Simulation::new(7, 20.0);
        let environment = simulation.environment.clone();
        let mut organism = simulation.organisms[0].clone();
        let anchor = organism.occupied_cells[0].clone();
        let placements: Vec<_> = organism.structure.units.iter().map(|u| (u.placement.x, u.placement.y)).collect();
        assert!(Simulation::try_move_cell(&mut organism, &environment, &[], 12.0, -7.0));
        assert!((organism.occupied_cells[0].x - (anchor.x + 12.0)).abs() < 1e-9);
        assert!((organism.occupied_cells[0].y - (anchor.y - 7.0)).abs() < 1e-9);
        for (unit, (x, y)) in organism.structure.units.iter().zip(placements) {
            assert!((unit.placement.x - (x + 12.0)).abs() < 1e-9);
            assert!((unit.placement.y - (y - 7.0)).abs() < 1e-9);
        }
    }

    #[test]
    fn try_move_cell_rejects_boundary_zero_displacement() {
        let simulation = Simulation::new(7, 20.0);
        let environment = simulation.environment.clone();
        let mut organism = simulation.organisms[0].clone();
        organism.occupied_cells[0].x = 0.0;
        organism.occupied_cells[0].y = 0.0;
        assert!(!Simulation::try_move_cell(&mut organism, &environment, &[], -10.0, -10.0));
    }

    #[test]
    fn try_move_cell_rejects_non_finite_displacement() {
        let simulation = Simulation::new(7, 20.0);
        let environment = simulation.environment.clone();
        let mut organism = simulation.organisms[0].clone();
        let original = organism.occupied_cells[0].clone();
        assert!(!Simulation::try_move_cell(&mut organism, &environment, &[], f64::NAN, 1.0));
        assert_eq!(organism.occupied_cells[0], original);
    }

    #[test]
    fn try_move_cell_rejects_overlap_with_another_organism() {
        let simulation = Simulation::new(7, 20.0);
        let environment = simulation.environment.clone();
        let mut organism = simulation.organisms[0].clone();
        let mut blocker = simulation.organisms[0].clone();
        blocker.occupied_cells[0].x += 8.0;
        for unit in &mut blocker.structure.units { unit.placement.x += 8.0; }
        let original_x = organism.occupied_cells[0].x;
        assert!(!Simulation::try_move_cell(&mut organism, &environment, &[blocker], 8.0, 0.0));
        assert_eq!(organism.occupied_cells[0].x, original_x);
    }
}
