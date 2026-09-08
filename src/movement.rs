use crate::state::{Environment, Organism, Simulation};

impl Simulation {
    /// Execute the already-selected MOVE action. This function performs only
    /// movement mechanics; action selection happens in decision_runtime.rs.
    pub(crate) fn update_movement(organism: &mut Organism, environment: &Environment) -> bool {
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

        // Organisms are another perceptual source, using the same directional
        // movement channel as resource perception. There is deliberately no
        // social need, aggression rule, or organism-specific action here.
        let mut organism_dir_x = 0.0;
        let mut organism_dir_y = 0.0;
        for observation in &organism.resource_sense.sensed_organisms {
            organism_dir_x += observation.direction_x;
            organism_dir_y += observation.direction_y;
        }
        let organism_magnitude = (organism_dir_x * organism_dir_x + organism_dir_y * organism_dir_y).sqrt();
        if organism_magnitude > f64::EPSILON {
            organism_dir_x /= organism_magnitude;
            organism_dir_y /= organism_magnitude;
        }

        let perception_dir_x = organism.resource_sense.direction_x + organism_dir_x;
        let perception_dir_y = organism.resource_sense.direction_y + organism_dir_y;
        let perception_magnitude = (perception_dir_x * perception_dir_x + perception_dir_y * perception_dir_y).sqrt();
        let (perception_dir_x, perception_dir_y) = if perception_magnitude > f64::EPSILON {
            (perception_dir_x / perception_magnitude, perception_dir_y / perception_magnitude)
        } else {
            (0.0, 0.0)
        };

        let mut move_x = memory_weight * memory_dir_x + perception_weight * perception_dir_x;
        let mut move_y = memory_weight * memory_dir_y + perception_weight * perception_dir_y;
        let magnitude = (move_x * move_x + move_y * move_y).sqrt();
        if magnitude <= f64::EPSILON {
            return false;
        }
        move_x /= magnitude;
        move_y /= magnitude;

        const STEP_DISTANCE: f64 = 5.0;
        let step = STEP_DISTANCE * movement_efficiency;
        let (old_x, old_y, new_x, new_y) = {
            let cell = &mut organism.occupied_cells[0];
            let old_x = cell.x;
            let old_y = cell.y;
            cell.x = (cell.x + move_x * step).clamp(0.0, environment.width);
            cell.y = (cell.y + move_y * step).clamp(0.0, environment.height);
            (old_x, old_y, cell.x, cell.y)
        };

        let delta_x = new_x - old_x;
        let delta_y = new_y - old_y;
        if delta_x.abs() <= f64::EPSILON && delta_y.abs() <= f64::EPSILON {
            return false;
        }

        // Structural-unit placements are world-space physical geometry. When
        // the organism moves, its body must translate with its locomotion
        // anchor; otherwise the derived physical body would remain behind.
        for unit in &mut organism.structure.units {
            unit.placement.x += delta_x;
            unit.placement.y += delta_y;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::OrganismObservation;

    #[test]
    fn organism_perception_uses_same_radius_and_records_geometry_size() {
        let environment = Simulation::create_environment();
        let mut observer = Simulation::create_initial_organism();
        let mut other = Simulation::create_initial_organism();
        other.id = "2".into();
        let offset = observer.genome.perception_radius() * 0.5;
        let origin = observer.occupied_cells[0].clone();
        other.occupied_cells[0].x = origin.x + offset;
        other.occupied_cells[0].y = origin.y;
        let mut organisms = vec![observer.clone(), other];

        Simulation::update_organism_perception(&mut observer, &organisms, &environment);

        assert_eq!(observer.resource_sense.sensed_organisms.len(), 1);
        let observation = &observer.resource_sense.sensed_organisms[0];
        assert_eq!(observation.id, "2");
        assert!((observation.distance - offset).abs() < 1e-9);
        assert!((observation.direction_x - 1.0).abs() < 1e-9);
        assert!(observation.direction_y.abs() < 1e-9);
        assert!(observation.size > 0.0);

        organisms[1].occupied_cells[0].x = origin.x + observer.genome.perception_radius() + 1.0;
        Simulation::update_organism_perception(&mut observer, &organisms, &environment);
        assert!(observer.resource_sense.sensed_organisms.is_empty());
    }

    #[test]
    fn sensed_organism_direction_affects_movement() {
        let environment = Simulation::create_environment();
        let mut organism = Simulation::create_initial_organism();
        let old_x = organism.occupied_cells[0].x;
        let old_y = organism.occupied_cells[0].y;
        organism.resource_sense.sensed_organisms = vec![OrganismObservation {
            id: "2".into(),
            distance: 10.0,
            direction_x: 1.0,
            direction_y: 0.0,
            size: 1.0,
        }];
        organism.resource_sense.direction_x = 0.0;
        organism.resource_sense.direction_y = 0.0;

        assert!(Simulation::update_movement(&mut organism, &environment));
        assert!(organism.occupied_cells[0].x > old_x);
        assert!((organism.occupied_cells[0].y - old_y).abs() < 1e-9);
    }
}
