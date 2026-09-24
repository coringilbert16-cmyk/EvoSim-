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
                outcome: None,
            });
        } else if let Some(weakest) = organism
            .memory
            .iter_mut()
            .min_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
        {
            if memory_strength > weakest.strength && weakest.outcome.is_none() {
                *weakest = MemoryPoint {
                    x: sx,
                    y: sy,
                    strength: memory_strength,
                    spectrum: spectrum.clone(),
                    outcome: None,
                };
            }
        }
    }

    pub(crate) fn remember_nearby_harmonics(organism: &mut Organism, environment: &Environment) {
        let Some(cavity) = qualifying_genome_cavity(organism, environment) else {
            return;
        };
    let capacity = memory_capacity(&cavity);
    for (x, y, spectrum) in
        crate::harmonics::nearby_environmental_spectra(organism, environment)
    {
        let Some((sx, sy)) = organism.occupied_cells.first().map(|p| (p.x, p.y)) else {
            continue;
        };
        let distance = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
        let amplitude = spectrum
            .components
            .iter()
            .map(|component| component.amplitude)
            .fold(0.0, f64::max);
        let strength = (amplitude / (1.0 + distance)).clamp(0.0, 1.0);
        if strength > f64::EPSILON {
            Simulation::remember_perception(organism, x, y, strength, capacity, &spectrum);
        }
    }
}

pub(crate) fn reinforce_acquired_material(
    organism: &mut Organism,
    environment: &Environment,
    x: f64,
    y: f64,
    material: &crate::resources::Material,
) {
    let Some(cavity) = qualifying_genome_cavity(organism, environment) else {
        return;
    };
    let spectrum = crate::harmonics::material_spectrum(material, &environment.catalog);
    if spectrum.components.is_empty() {
        return;
    }
    let matches_perceived_signal = |point: &MemoryPoint| {
        let dx = point.x - x;
        let dy = point.y - y;
        if (dx * dx + dy * dy).sqrt() >= MEMORY_MERGE_RADIUS {
            return false;
        }
        point.spectrum.components.iter().any(|observed| {
            spectrum.components.iter().any(|acquired| {
                (observed.frequency_hz - acquired.frequency_hz).abs() <= 1e-6
                    })
            })
    };
    let Some(index) = organism.memory.iter().position(matches_perceived_signal) else {
        return;
    };
    let point = organism.memory[index].clone();
    Simulation::reinforce_memory_point(
        organism,
        point.x,
        point.y,
        organism.genome.memory_strength().clamp(0.0, 1.0),
        memory_capacity(&cavity),
        &spectrum,
        crate::decision::OutcomeKind::Beneficial,
    );
}

    pub(crate) fn reinforce_memory_point(
        organism: &mut Organism,
        sx: f64,
        sy: f64,
        memory_strength: f64,
        capacity: usize,
        spectrum: &crate::harmonics::ToneSpectrum,
        outcome: crate::decision::OutcomeKind,
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
                existing.outcome = Some(outcome);
            }
            None => {
                if organism.memory.len() < capacity {
                    organism.memory.push(MemoryPoint {
                        x: sx,
                        y: sy,
                        strength: memory_strength,
                        spectrum: spectrum.clone(),
                        outcome: Some(outcome),
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
                            outcome: Some(outcome),
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
    outcome: crate::decision::OutcomeKind,
) {
    Simulation::reinforce_memory_point(
        organism,
        sx,
        sy,
        memory_strength,
        capacity,
        spectrum,
        outcome,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harmonic_experience_is_stored_with_its_outcome() {
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
            crate::decision::OutcomeKind::Beneficial,
        );

        assert_eq!(organism.memory.len(), 1);
        assert_eq!(organism.memory[0].spectrum, spectrum);
        assert_eq!(
            organism.memory[0].outcome,
            Some(crate::decision::OutcomeKind::Beneficial)
        );
    }

    #[test]
    fn perception_memory_is_level_one_until_an_outcome_occurs() {
        let mut organism = Simulation::create_initial_organism();
        let spectrum = crate::harmonics::ToneSpectrum::empty();

        Simulation::remember_perception(&mut organism, 12.0, 34.0, 0.5, 1, &spectrum);
        assert_eq!(organism.memory.len(), 1);
        assert_eq!(organism.memory[0].outcome, None);
        let perception_strength = organism.memory[0].strength;

        reinforce_memory_point(
            &mut organism,
            12.0,
            34.0,
            0.5,
            1,
            &spectrum,
            crate::decision::OutcomeKind::Harmful,
        );

        assert_eq!(
            organism.memory[0].outcome,
            Some(crate::decision::OutcomeKind::Harmful)
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
