use crate::state::{
    Environment, MemoryPoint, Organism, Simulation, MAX_MEMORY_POINTS, MEMORY_DECAY_PER_TICK,
    MEMORY_MERGE_RADIUS, MEMORY_PRUNE_THRESHOLD,
};

impl Simulation {
    pub(crate) fn update_memory_from_sources(organism: &mut Organism, environment: &Environment) {
        for point in &mut organism.memory {
            point.strength *= MEMORY_DECAY_PER_TICK;
        }
        organism
            .memory
            .retain(|p| p.strength > MEMORY_PRUNE_THRESHOLD);

        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };
        let perception_radius = organism.genome.perception_radius();
        let sensory_resolution = organism.genome.sensory_resolution();
        let baselines = crate::resources::ResourceBaselines::from_catalog(&environment.catalog);
        let ranges = crate::resources::property_ranges(&environment.catalog);

        let mut strongest_source: Option<(f64, f64, f64)> = None;
        for cell_index in environment
            .field
            .cells_within_radius(px, py, perception_radius)
        {
            let (cell_x, cell_y) = environment.field.cell_center(cell_index);
            let cell = &environment.field.cells[cell_index];
            for material in [&cell.bonded, &cell.unbonded] {
                let perceived_amount = material.total_amount() * sensory_resolution;
                if perceived_amount <= 0.0 {
                    continue;
                }
                let properties = material.weighted_properties(&environment.catalog);
                let (_, _, _, _, _, desirability) = Self::calculate_desirability(
                    organism,
                    &properties,
                    perceived_amount,
                    &baselines,
                    &ranges,
                );
                if desirability <= 0.0 {
                    continue;
                }
                if strongest_source
                    .map(|(_, _, current)| desirability > current)
                    .unwrap_or(true)
                {
                    strongest_source = Some((cell_x, cell_y, desirability));
                }
            }
        }

        let Some((sx, sy, desirability)) = strongest_source else {
            return;
        };
        let memory_strength = (desirability * organism.genome.memory_strength()).clamp(0.0, 1.0);
        if memory_strength <= 0.0 {
            return;
        }
        Self::reinforce_memory_point(organism, sx, sy, memory_strength);
    }

    pub(crate) fn reinforce_memory_point(
        organism: &mut Organism,
        sx: f64,
        sy: f64,
        memory_strength: f64,
    ) {
        let merged = organism.memory.iter_mut().find(|p| {
            let dx = p.x - sx;
            let dy = p.y - sy;
            (dx * dx + dy * dy).sqrt() < MEMORY_MERGE_RADIUS
        });

        match merged {
            Some(existing) => {
                existing.x = sx;
                existing.y = sy;
                existing.strength = (existing.strength + memory_strength).min(1.0);
            }
            None => {
                if organism.memory.len() < MAX_MEMORY_POINTS {
                    organism.memory.push(MemoryPoint {
                        x: sx,
                        y: sy,
                        strength: memory_strength,
                    });
                } else if let Some(weakest) = organism
                    .memory
                    .iter_mut()
                    .min_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
                {
                    if memory_strength > weakest.strength {
                        *weakest = MemoryPoint {
                            x: sx,
                            y: sy,
                            strength: memory_strength,
                        };
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reinforcement_is_bounded_and_replaces_only_weaker_points() {
        let mut organism = Simulation::create_initial_organism();
        for i in 0..MAX_MEMORY_POINTS {
            organism.memory.push(MemoryPoint {
                x: i as f64 * 100.0,
                y: 0.0,
                strength: 0.1 + i as f64 * 0.1,
            });
        }

        Simulation::reinforce_memory_point(&mut organism, 999.0, 999.0, 0.9);

        assert_eq!(organism.memory.len(), MAX_MEMORY_POINTS);
        assert!(organism.memory.iter().any(|p| {
            (p.x - 999.0).abs() < f64::EPSILON && (p.y - 999.0).abs() < f64::EPSILON
        }));
        assert!(organism
            .memory
            .iter()
            .all(|p| p.strength.is_finite() && (0.0..=1.0).contains(&p.strength)));
    }

    #[test]
    fn memory_strength_decays_without_new_sources() {
        let mut sim = Simulation::new(101, 10.0);
        sim.organisms[0].memory.push(MemoryPoint {
            x: 500.0,
            y: 500.0,
            strength: 0.5,
        });
        let expected = 0.5 * MEMORY_DECAY_PER_TICK;
        let environment = sim.environment.clone();

        Simulation::update_memory_from_sources(&mut sim.organisms[0], &environment);

        assert_eq!(sim.organisms[0].memory.len(), 1);
        assert!((sim.organisms[0].memory[0].strength - expected).abs() < 1e-12);
    }

    #[test]
    fn weak_memory_points_are_pruned_after_decay() {
        let mut sim = Simulation::new(103, 10.0);
        sim.organisms[0].memory.push(MemoryPoint {
            x: 500.0,
            y: 500.0,
            strength: MEMORY_PRUNE_THRESHOLD,
        });
        let environment = sim.environment.clone();

        Simulation::update_memory_from_sources(&mut sim.organisms[0], &environment);

        assert!(sim.organisms[0].memory.is_empty());
    }

    #[test]
    fn reinforcement_merges_nearby_points_without_exceeding_capacity() {
        let mut organism = Simulation::create_initial_organism();
        organism.memory.push(MemoryPoint {
            x: 100.0,
            y: 100.0,
            strength: 0.2,
        });

        Simulation::reinforce_memory_point(&mut organism, 110.0, 110.0, 0.3);

        assert_eq!(organism.memory.len(), 1);
        assert!((organism.memory[0].strength - 0.5).abs() < 1e-12);
        assert!((organism.memory[0].x - 110.0).abs() < f64::EPSILON);
        assert!((organism.memory[0].y - 110.0).abs() < f64::EPSILON);
    }
}
