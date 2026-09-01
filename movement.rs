use crate::state::{Environment, Organism, Simulation};

impl Simulation {
    /// Execute the already-selected MOVE action. This function performs only
    /// movement mechanics; action selection happens in decision_runtime.rs.
    pub(crate) fn update_movement(
        organism: &mut Organism,
        environment: &Environment,
    ) -> bool {
        let memory_strength_trait = organism.genome.memory_strength();
        let movement_efficiency = organism.genome.movement_efficiency();
        let perception_weight = 1.0 - (0.5 + memory_strength_trait * 0.5);
        let memory_weight = 1.0 - perception_weight;

        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
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

        let mut move_x = memory_weight * memory_dir_x
            + perception_weight * organism.resource_sense.direction_x;
        let mut move_y = memory_weight * memory_dir_y
            + perception_weight * organism.resource_sense.direction_y;
        let magnitude = (move_x * move_x + move_y * move_y).sqrt();
        if magnitude <= f64::EPSILON {
            return false;
        }
        move_x /= magnitude;
        move_y /= magnitude;

        const STEP_DISTANCE: f64 = 5.0;
        let step = STEP_DISTANCE * movement_efficiency;
        let cell = &mut organism.occupied_cells[0];
        let old_x = cell.x;
        let old_y = cell.y;
        cell.x = (cell.x + move_x * step).clamp(0.0, environment.width);
        cell.y = (cell.y + move_y * step).clamp(0.0, environment.height);
        (cell.x - old_x).abs() > f64::EPSILON || (cell.y - old_y).abs() > f64::EPSILON
    }
}
