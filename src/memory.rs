use crate::state::{
    Environment, MemoryPoint, Organism, Simulation, MEMORY_DECAY_PER_TICK, MEMORY_MERGE_RADIUS,
    MEMORY_PRUNE_THRESHOLD,
};

pub(crate) const MEMORY_CAPACITY_GROWTH_EXPONENT: f64 = 0.5;

fn qualifying_genome_cavity(
    organism: &mut Organism,
    environment: &Environment,
) -> Option<crate::cavity::GenomeCavity> {
    organism.genome_cavity_cached(&environment.catalog)
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
            organism.memory.sort_by(|a, b| {
                b.strength
                    .partial_cmp(&a.strength)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            organism.memory.truncate(capacity);
        }
    }

    pub(crate) fn remember_perception(
        organism: &mut Organism,
        sx: f64,
        sy: f64,
        memory_strength: f64,
        capacity: usize,
        spectrum: &crate::harmonics::ToneSpectrum,
    ) {
        let merged_index = organism.memory.iter().position(|p| {
            let dx = p.x - sx;
            let dy = p.y - sy;
            (dx * dx + dy * dy).sqrt() < MEMORY_MERGE_RADIUS
        });

        if let Some(index) = merged_index {
            let existing = &mut organism.memory[index];
            existing.x = sx;
            existing.y = sy;
            existing.strength = (existing.strength + memory_strength).min(1.0);
            existing.spectrum.merge_from(spectrum, 1.0);
        } else if organism.memory.len() < capacity {
            organism.memory.push(MemoryPoint {
                x: sx,
                y: sy,
                strength: memory_strength,
                spectrum: spectrum.clone(),
                consequence: None,
            });
        } else if let Some(weakest) = organism
            .memory
            .iter_mut()
            .min_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
        {
            if memory_strength > weakest.strength && weakest.consequence.is_none() {
                *weakest = MemoryPoint {
                    x: sx,
                    y: sy,
                    strength: memory_strength,
                    spectrum: spectrum.clone(),
                    consequence: None,
                };
            }
        }
    }

    pub(crate) fn reinforce_memory_point(
        organism: &mut Organism,
        sx: f64,
        sy: f64,
        memory_strength: f64,
        capacity: usize,
        spectrum: &crate::harmonics::ToneSpectrum,
        consequence: crate::decision::ActionConsequence,
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
                existing.spectrum.merge_from(spectrum, 1.0);
                existing.consequence = Some(consequence);
            }
            None => {
                if organism.memory.len() < capacity {
                    organism.memory.push(MemoryPoint {
                        x: sx,
                        y: sy,
                        strength: memory_strength,
                        spectrum: spectrum.clone(),
                        consequence: Some(consequence),
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
                            spectrum: spectrum.clone(),
                            consequence: Some(consequence),
                        };
                    }
                }
            }
        }
    }
}

pub(crate) fn remember_perception(
    organism: &mut Organism,
    sx: f64,
    sy: f64,
    memory_strength: f64,
    capacity: usize,
    spectrum: &crate::harmonics::ToneSpectrum,
) {
    Simulation::remember_perception(organism, sx, sy, memory_strength, capacity, spectrum);
}

pub(crate) fn reinforce_memory_point(
    organism: &mut Organism,
    sx: f64,
    sy: f64,
    memory_strength: f64,
    capacity: usize,
    spectrum: &crate::harmonics::ToneSpectrum,
    consequence: crate::decision::ActionConsequence,
) {
    Simulation::reinforce_memory_point(
        organism,
        sx,
        sy,
        memory_strength,
        capacity,
        spectrum,
        consequence,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harmonic_experience_is_stored_with_its_consequence() {
        let mut organism = Simulation::create_initial_organism();
        let spectrum = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: 440.0,
                amplitude: 0.75,
                phase_radians: 0.25,
            }],
        };

        reinforce_memory_point(
            &mut organism,
            12.0,
            34.0,
            0.5,
            1,
            &spectrum,
            crate::decision::ActionConsequence {
                energy_delta: 1.0,
                stress_delta: 0.0,
                developmental_delta: 0.0,
            },
        );

        assert_eq!(organism.memory.len(), 1);
        assert_eq!(organism.memory[0].spectrum, spectrum);
        assert_eq!(
            organism.memory[0].consequence,
            Some(crate::decision::ActionConsequence {
                energy_delta: 1.0,
                stress_delta: 0.0,
                developmental_delta: 0.0,
            })
        );
    }

    #[test]
    fn perception_memory_is_level_one_until_an_outcome_occurs() {
        let mut organism = Simulation::create_initial_organism();
        let spectrum = crate::harmonics::ToneSpectrum::empty();

        Simulation::remember_perception(&mut organism, 12.0, 34.0, 0.5, 1, &spectrum);
        assert_eq!(organism.memory.len(), 1);
        assert_eq!(organism.memory[0].consequence, None);
        let perception_strength = organism.memory[0].strength;

        reinforce_memory_point(
            &mut organism,
            12.0,
            34.0,
            0.5,
            1,
            &spectrum,
            crate::decision::ActionConsequence {
                energy_delta: -1.0,
                stress_delta: 1.0,
                developmental_delta: 0.0,
            },
        );

        assert_eq!(
            organism.memory[0].consequence,
            Some(crate::decision::ActionConsequence {
                energy_delta: -1.0,
                stress_delta: 1.0,
                developmental_delta: 0.0,
            })
        );
        assert!(organism.memory[0].strength > perception_strength);
    }

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
