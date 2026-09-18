use crate::state::{
    Environment, MemoryPoint, Organism, Simulation, MEMORY_DECAY_PER_TICK, MEMORY_MERGE_RADIUS,
    MEMORY_PRUNE_THRESHOLD,
};

pub(crate) const MEMORY_CAPACITY_GROWTH_EXPONENT: f64 = 0.5;

fn qualifying_genome_cavity(
    organism: &Organism,
    environment: &Environment,
) -> Option<crate::cavity::GenomeCavity> {
    crate::cavity::analyze_genome_cavity(&organism.structure, &environment.catalog)
        .ok()
        .flatten()
        .filter(|cavity| cavity.qualifies())
}

pub(crate) fn memory_capacity(cavity: &crate::cavity::GenomeCavity) -> usize {
    let area_ratio = (cavity.area / cavity.minimum_area).max(1.0);
    area_ratio
        .powf(MEMORY_CAPACITY_GROWTH_EXPONENT)
        .floor()
        .max(1.0) as usize
}

fn memory_decay_for_cavity(cavity: &crate::cavity::GenomeCavity) -> f64 {
    let area_ratio = (cavity.minimum_area / cavity.area.max(cavity.minimum_area)).sqrt();
    MEMORY_DECAY_PER_TICK
        .powf(area_ratio)
        .clamp(MEMORY_DECAY_PER_TICK, 1.0)
}

impl Simulation {
    pub(crate) fn update_memory_from_sources(organism: &mut Organism, environment: &Environment) {
        let Some(cavity) = qualifying_genome_cavity(organism, environment) else {
            organism.memory.clear();
            return;
        };
        let capacity = memory_capacity(&cavity);
        let decay = memory_decay_for_cavity(&cavity);

        for point in &mut organism.memory {
            point.strength *= decay;
        }
        organism
            .memory
            .retain(|p| p.strength > MEMORY_PRUNE_THRESHOLD);
        if organism.memory.len() > capacity {
            organism
                .memory
                .sort_by(|a, b| {
                    b.strength
                        .partial_cmp(&a.strength)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            organism.memory.truncate(capacity);
        }

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
            for material in &cell.materials {
                let perceived_amount = Self::perceived_amount(material, sensory_resolution);
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
        reinforce_memory_point(organism, sx, sy, memory_strength, capacity);
    }

    pub(crate) fn reinforce_memory_point(
        organism: &mut Organism,
        sx: f64,
        sy: f64,
        memory_strength: f64,
        capacity: usize,
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
                if organism.memory.len() < capacity {
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
    fn memory_capacity_uses_diminishing_area_returns() {
        let minimum = crate::cavity::GenomeCavity {
            area: 10.0,
            boundary_units: Vec::new(),
            minimum_area: 10.0,
        };
        let four_times = crate::cavity::GenomeCavity {
            area: 40.0,
            boundary_units: Vec::new(),
            minimum_area: 10.0,
        };
        let sixteen_times = crate::cavity::GenomeCavity {
            area: 160.0,
            boundary_units: Vec::new(),
            minimum_area: 10.0,
        };
        assert_eq!(memory_capacity(&minimum), 1);
        assert_eq!(memory_capacity(&four_times), 2);
        assert_eq!(memory_capacity(&sixteen_times), 4);
    }

    #[test]
    fn larger_cavity_has_longer_but_diminishing_memory_persistence() {
        let minimum = crate::cavity::GenomeCavity {
            area: 10.0,
            boundary_units: Vec::new(),
            minimum_area: 10.0,
        };
        let four_times = crate::cavity::GenomeCavity {
            area: 40.0,
            boundary_units: Vec::new(),
            minimum_area: 10.0,
        };
        let sixteen_times = crate::cavity::GenomeCavity {
            area: 160.0,
            boundary_units: Vec::new(),
            minimum_area: 10.0,
        };
        let base = memory_decay_for_cavity(&minimum);
        let medium = memory_decay_for_cavity(&four_times);
        let large = memory_decay_for_cavity(&sixteen_times);
        assert!(base < medium && medium < large && large < 1.0);
        assert!((medium - base) > (large - medium));
    }
}
