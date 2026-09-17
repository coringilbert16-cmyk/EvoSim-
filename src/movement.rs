use crate::state::{Environment, Organism, Simulation};

impl Simulation {
    /// Execute the already-selected MOVE action. This function performs only
    /// movement intent generation; action selection happens in
    /// decision_runtime.rs and physical movement is committed by
    /// `try_move_cell`.
    pub(crate) fn update_movement(organism: &mut Organism, environment: &Environment) -> bool {
        let memory_strength_trait = organism.genome.memory_strength();
        let movement_efficiency = organism.genome.movement_efficiency();
        let perception_weight = 1.0 - (0.5 + memory_strength_trait * 0.5);
        let memory_weight = 1.0 - perception_weight;

        let (px, py) = match organism.occupied_cells.first() {
            Some(p) => (p.x, p.y),
            None => return false,
        };

        let mut memory_dir_x = 0.0;
        let mut memory_dir_y = 0.0;
        let mut memory_total_weight = 0.0;
        for point in &organism.memory {
            let dx = point.x - px;
            let dy = point.y - py;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance <= f64::EPSILON {
                continue;
            }
            let weight = point.strength / distance;
            memory_dir_x += (dx / distance) * weight;
            memory_dir_y += (dy / distance) * weight;
            memory_total_weight += weight;
        }
        if memory_total_weight > 0.0 {
            memory_dir_x /= memory_total_weight;
            memory_dir_y /= memory_total_weight;
        }

        if organism.active_transformation_id.is_some() {
            return false;
        }

        let mut move_x =
            memory_weight * memory_dir_x + perception_weight * organism.resource_sense.direction_x;
        let mut move_y =
            memory_weight * memory_dir_y + perception_weight * organism.resource_sense.direction_y;
        let magnitude = (move_x * move_x + move_y * move_y).sqrt();
        if magnitude <= f64::EPSILON {
            return false;
        }
        move_x /= magnitude;
        move_y /= magnitude;

        const STEP_DISTANCE: f64 = 5.0;
        let step = STEP_DISTANCE * movement_efficiency;
        Self::try_move_cell(organism, environment, move_x * step, move_y * step)
    }

    /// Canonical movement boundary for an organism's locomotion anchor.
    ///
    /// This is intentionally a narrow Phase 4.1 boundary: it applies the
    /// already-computed displacement and translates the realized physical
    /// structure by the same world-space delta. Collision/contact resolution,
    /// pushing, and other movement mechanics belong in later Phase 4 steps.
    pub(crate) fn try_move_cell(
        organism: &mut Organism,
        environment: &Environment,
        delta_x: f64,
        delta_y: f64,
    ) -> bool {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return false;
        }

        let (old_x, old_y, new_x, new_y) = match organism.occupied_cells.first_mut() {
            Some(cell) => {
                let old_x = cell.x;
                let old_y = cell.y;
                cell.x = (cell.x + delta_x).clamp(0.0, environment.width);
                cell.y = (cell.y + delta_y).clamp(0.0, environment.height);
                (old_x, old_y, cell.x, cell.y)
            }
            None => return false,
        };

        let applied_delta_x = new_x - old_x;
        let applied_delta_y = new_y - old_y;
        if applied_delta_x.abs() <= f64::EPSILON && applied_delta_y.abs() <= f64::EPSILON {
            return false;
        }

        // Structural-unit placements are world-space physical geometry. When
        // the organism moves, its body must translate with its locomotion
        // anchor; otherwise the realized physical body would remain behind.
        for unit in &mut organism.structure.units {
            unit.placement.x += applied_delta_x;
            unit.placement.y += applied_delta_y;
        }

        true
    }
}
