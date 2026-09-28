#![expect(
    dead_code,
    reason = "New associative memory is staged during runtime migration"
)]

use serde::{Deserialize, Serialize};

use crate::state::{Environment, Organism};

pub(crate) const MEMORY_CAPACITY_GROWTH_EXPONENT: f64 = 0.5;

/// EXPERIMENTAL: the minimum continuous spectral similarity used to merge
/// generalized spectral memories. The similarity itself remains the weight
/// used for reinforcement; this only prevents unrelated spectra from being
/// forced into one prototype.
pub(crate) const SPECTRAL_MEMORY_MATCH_FLOOR: f64 = 0.25;

/// EXPERIMENTAL: memories below this strength are removed during decay.
pub(crate) const EXPERIENCE_MEMORY_PRUNE_THRESHOLD: f64 = 0.01;

/// EXPERIMENTAL: calibration scales for converting heterogeneous physical
/// consequences into a dimensionless learned value.
pub(crate) const CONSEQUENCE_ENERGY_SCALE: f64 = 1.0;
pub(crate) const CONSEQUENCE_DEVELOPMENT_SCALE: f64 = 0.1;
pub(crate) const CONSEQUENCE_MATERIAL_SCALE: f64 = 1.0;
pub(crate) const CONSEQUENCE_STRESS_SCALE: f64 = 1.0;
pub(crate) const CONSEQUENCE_DAMAGE_SCALE: f64 = 1.0;
pub(crate) const CONSEQUENCE_CONSUMED_MATERIAL_SCALE: f64 = 1.0;

/// State-dependent valuation of an already observed physical consequence.
/// The consequence is not changed; current need pressure only changes how
/// useful or harmful that remembered outcome is right now.
pub(crate) fn consequence_value(
    consequence: &MemoryConsequence,
    needs: crate::decision::CurrentNeeds,
) -> f64 {
    let survival = needs.survival.clamp(0.0, 1.0);
    let reproduction = needs.reproduction.clamp(0.0, 1.0);
    let development = needs.development.clamp(0.0, 1.0);

    let energy_weight = 1.0 + survival;
    let development_weight = 1.0 + development + 0.5 * reproduction;
    let acquisition_weight = 1.0 + survival;
    let stress_penalty = 1.0 + 2.0 * survival;
    let damage_penalty = 1.0 + 2.0 * survival;
    let consumption_penalty = 0.25 + 0.75 * survival;

    let raw = consequence.energy_delta / CONSEQUENCE_ENERGY_SCALE * energy_weight
        + consequence.developmental_delta / CONSEQUENCE_DEVELOPMENT_SCALE * development_weight
        + consequence.material_acquired / CONSEQUENCE_MATERIAL_SCALE * acquisition_weight
        - consequence.stress_delta / CONSEQUENCE_STRESS_SCALE * stress_penalty
        - consequence.damage_delta / CONSEQUENCE_DAMAGE_SCALE * damage_penalty
        - consequence.material_consumed / CONSEQUENCE_CONSUMED_MATERIAL_SCALE * consumption_penalty;

    raw.tanh()
}

/// Record one completed experience against all spatially attributed
/// resonance signals involved in it. Location and spectral associations are
/// reinforced independently; the encounter record keeps the linkage.
pub(crate) fn memory_consequence_from_action(
    consequence: crate::decision::ActionConsequence,
) -> MemoryConsequence {
    MemoryConsequence {
        energy_delta: consequence.energy_delta,
        stress_delta: consequence.stress_delta,
        damage_delta: 0.0,
        developmental_delta: consequence.developmental_delta,
        material_acquired: 0.0,
        material_consumed: 0.0,
    }
}

pub(crate) fn record_experience(
    memory: &mut ExperienceMemory,
    perceptions: &[crate::harmonics::ResonancePerception],
    action: crate::decision::ActionKind,
    consequence: MemoryConsequence,
    needs: crate::decision::CurrentNeeds,
    capacity: usize,
    formation_strength: f64,
) {
    if perceptions.is_empty() || capacity == 0 {
        return;
    }

    let association = consequence_value(&consequence, needs).tanh();
    let raw_magnitude = [
        consequence.energy_delta / CONSEQUENCE_ENERGY_SCALE,
        consequence.developmental_delta / CONSEQUENCE_DEVELOPMENT_SCALE,
        consequence.material_acquired / CONSEQUENCE_MATERIAL_SCALE,
        consequence.stress_delta / CONSEQUENCE_STRESS_SCALE,
        consequence.damage_delta / CONSEQUENCE_DAMAGE_SCALE,
        consequence.material_consumed / CONSEQUENCE_CONSUMED_MATERIAL_SCALE,
    ]
    .into_iter()
    .map(f64::abs)
    .sum::<f64>();
    let experience_magnitude = (1.0 - (-raw_magnitude).exp())
        .mul_add(formation_strength.clamp(0.0, 1.0), 0.0)
        .clamp(0.0, 1.0);
    if experience_magnitude <= f64::EPSILON {
        return;
    }

    let total_perception_magnitude = perceptions
        .iter()
        .map(|perception| perception.magnitude.max(0.0))
        .sum::<f64>()
        .max(f64::EPSILON);

    for perception in perceptions {
        let share = perception.magnitude.max(0.0) / total_perception_magnitude;
        let weight = experience_magnitude * share;
        if weight <= f64::EPSILON {
            continue;
        }
        reinforce_spatial_memory(
            memory,
            perception.source_x,
            perception.source_y,
            perception.extent,
            association,
            weight,
        );
        reinforce_spectral_memory(&mut *memory, &perception.spectrum, association, weight);
        reinforce_encounter_memory(
            memory,
            perception.source_x,
            perception.source_y,
            perception.extent,
            &perception.spectrum,
            action,
            consequence.clone(),
            weight,
        );
    }

    memory.spatial.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    memory.spectral.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    memory.encounters.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    memory.spatial.truncate(capacity);
    memory.spectral.truncate(capacity);
    memory.encounters.truncate(capacity);
}

fn bounded_strength_after_experience(current: f64, experience_magnitude: f64) -> f64 {
    let current = current.clamp(0.0, 1.0);
    let magnitude = experience_magnitude.max(0.0);
    (current + magnitude).clamp(0.0, 1.0)
}

fn update_association(
    current: f64,
    current_weight: f64,
    observed: f64,
    observed_weight: f64,
) -> (f64, f64) {
    let current_weight = current_weight.max(0.0);
    let observed_weight = observed_weight.max(0.0);
    let total_weight = current_weight + observed_weight;
    if total_weight <= f64::EPSILON {
        return (observed.clamp(-1.0, 1.0), 0.0);
    }
    let association =
        ((current * current_weight + observed * observed_weight) / total_weight).clamp(-1.0, 1.0);
    (association, total_weight)
}

fn spatial_memories_overlap(a: &SpatialMemory, x: f64, y: f64, extent: f64) -> bool {
    let dx = a.x - x;
    let dy = a.y - y;
    let combined_extent = a.extent.max(0.0) + extent.max(0.0);
    dx * dx + dy * dy <= combined_extent * combined_extent
}

fn best_spectral_memory(
    memories: &[SpectralMemory],
    spectrum: &crate::harmonics::ToneSpectrum,
) -> Option<(usize, f64)> {
    memories
        .iter()
        .enumerate()
        .map(|(index, memory)| {
            (
                index,
                crate::harmonics::spectral_similarity(&memory.spectrum, spectrum),
            )
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
}

pub(crate) fn decay_experience_memory(memory: &mut ExperienceMemory, decay: f64) {
    let decay = decay.clamp(0.0, 1.0);
    for spatial in &mut memory.spatial {
        spatial.strength *= decay;
        spatial.association_weight *= decay;
    }
    for spectral in &mut memory.spectral {
        spectral.strength *= decay;
        spectral.association_weight *= decay;
    }
    for encounter in &mut memory.encounters {
        encounter.strength *= decay;
        encounter.experience_weight *= decay;
    }

    memory
        .spatial
        .retain(|entry| entry.strength > EXPERIENCE_MEMORY_PRUNE_THRESHOLD);
    memory
        .spectral
        .retain(|entry| entry.strength > EXPERIENCE_MEMORY_PRUNE_THRESHOLD);
    memory
        .encounters
        .retain(|entry| entry.strength > EXPERIENCE_MEMORY_PRUNE_THRESHOLD);
}

pub(crate) fn reinforce_spatial_memory(
    memory: &mut ExperienceMemory,
    x: f64,
    y: f64,
    extent: f64,
    association: f64,
    experience_magnitude: f64,
) {
    let experience_magnitude = experience_magnitude.max(0.0);
    if let Some(existing) = memory
        .spatial
        .iter_mut()
        .find(|entry| spatial_memories_overlap(entry, x, y, extent))
    {
        let (association, weight) = update_association(
            existing.association,
            existing.association_weight,
            association,
            experience_magnitude,
        );
        existing.association = association;
        existing.association_weight = weight;
        existing.strength =
            bounded_strength_after_experience(existing.strength, experience_magnitude);
        return;
    }

    memory.spatial.push(SpatialMemory {
        x,
        y,
        extent: extent.max(0.0),
        association: association.clamp(-1.0, 1.0),
        association_weight: experience_magnitude,
        strength: experience_magnitude.clamp(0.0, 1.0),
    });
}

pub(crate) fn reinforce_spectral_memory(
    memory: &mut ExperienceMemory,
    spectrum: &crate::harmonics::ToneSpectrum,
    association: f64,
    experience_magnitude: f64,
) {
    let experience_magnitude = experience_magnitude.max(0.0);
    let Some((index, similarity)) = best_spectral_memory(&memory.spectral, spectrum) else {
        memory.spectral.push(SpectralMemory {
            spectrum: spectrum.clone(),
            association: association.clamp(-1.0, 1.0),
            association_weight: experience_magnitude,
            strength: experience_magnitude.clamp(0.0, 1.0),
        });
        return;
    };

    if similarity < SPECTRAL_MEMORY_MATCH_FLOOR {
        memory.spectral.push(SpectralMemory {
            spectrum: spectrum.clone(),
            association: association.clamp(-1.0, 1.0),
            association_weight: experience_magnitude,
            strength: experience_magnitude.clamp(0.0, 1.0),
        });
        return;
    }

    let existing = &mut memory.spectral[index];
    let weighted_magnitude = experience_magnitude * similarity;
    let previous_weight = existing.association_weight.max(0.0);
    let (association, weight) = update_association(
        existing.association,
        previous_weight,
        association,
        weighted_magnitude,
    );
    existing.association = association;
    existing.association_weight = weight;
    existing.strength = bounded_strength_after_experience(existing.strength, weighted_magnitude);

    let blend = if weight <= f64::EPSILON {
        0.0
    } else {
        (weighted_magnitude / weight).clamp(0.0, 1.0)
    };
    for component in &spectrum.components {
        let best = existing
            .spectrum
            .components
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                (
                    index,
                    (candidate.frequency_hz / component.frequency_hz).ln().abs(),
                )
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((component_index, distance)) = best {
            let frequency_similarity = (-distance / crate::harmonics::SPECTRAL_MATCH_SIGMA).exp();
            if frequency_similarity >= SPECTRAL_MEMORY_MATCH_FLOOR {
                let existing_component = &mut existing.spectrum.components[component_index];
                existing_component.frequency_hz = ((1.0 - blend)
                    * existing_component.frequency_hz.ln()
                    + blend * component.frequency_hz.ln())
                .exp();
                existing_component.amplitude =
                    existing_component.amplitude * (1.0 - blend) + component.amplitude * blend;
                continue;
            }
        }

        existing
            .spectrum
            .components
            .push(crate::harmonics::ToneComponent {
                frequency_hz: component.frequency_hz,
                amplitude: component.amplitude * blend,
                phase_radians: component.phase_radians,
            });
    }
    existing.spectrum.retain_strongest();
}

pub(crate) fn reinforce_encounter_memory(
    memory: &mut ExperienceMemory,
    x: f64,
    y: f64,
    extent: f64,
    spectrum: &crate::harmonics::ToneSpectrum,
    action: crate::decision::ActionKind,
    consequence: MemoryConsequence,
    experience_magnitude: f64,
) {
    let experience_magnitude = experience_magnitude.max(0.0);
    let Some((index, similarity)) = memory
        .encounters
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let spatial = spatial_memories_overlap(
                &SpatialMemory {
                    x: entry.x,
                    y: entry.y,
                    extent: entry.extent,
                    association: 0.0,
                    association_weight: 0.0,
                    strength: 0.0,
                },
                x,
                y,
                extent,
            );
            let spectral = crate::harmonics::spectral_similarity(&entry.spectrum, spectrum);
            (index, spatial, spectral)
        })
        .filter(|(index, spatial, spectral)| {
            memory.encounters[*index].action == action
                && *spatial
                && *spectral >= SPECTRAL_MEMORY_MATCH_FLOOR
        })
        .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, _, spectral)| (index, spectral))
    else {
        memory.encounters.push(EncounterMemory {
            x,
            y,
            extent: extent.max(0.0),
            spectrum: spectrum.clone(),
            action,
            consequence,
            experience_weight: experience_magnitude,
            strength: experience_magnitude.clamp(0.0, 1.0),
        });
        return;
    };

    let existing = &mut memory.encounters[index];
    let weight = experience_magnitude * similarity;
    existing.experience_weight += weight;
    existing.strength = bounded_strength_after_experience(existing.strength, weight);
    let previous_weight = existing.experience_weight - weight;
    existing.consequence.energy_delta = weighted_average(
        existing.consequence.energy_delta,
        previous_weight,
        consequence.energy_delta,
        weight,
    );
    existing.consequence.stress_delta = weighted_average(
        existing.consequence.stress_delta,
        previous_weight,
        consequence.stress_delta,
        weight,
    );
    existing.consequence.damage_delta = weighted_average(
        existing.consequence.damage_delta,
        previous_weight,
        consequence.damage_delta,
        weight,
    );
    existing.consequence.developmental_delta = weighted_average(
        existing.consequence.developmental_delta,
        previous_weight,
        consequence.developmental_delta,
        weight,
    );
    existing.consequence.material_acquired = weighted_average(
        existing.consequence.material_acquired,
        previous_weight,
        consequence.material_acquired,
        weight,
    );
    existing.consequence.material_consumed = weighted_average(
        existing.consequence.material_consumed,
        previous_weight,
        consequence.material_consumed,
        weight,
    );
}

fn weighted_average(current: f64, current_weight: f64, observed: f64, observed_weight: f64) -> f64 {
    let total = current_weight.max(0.0) + observed_weight.max(0.0);
    if total <= f64::EPSILON {
        return observed;
    }
    (current * current_weight.max(0.0) + observed * observed_weight.max(0.0)) / total
}

/// A physically grounded spatial association. The region is the portion of
/// the existing resonance geometry involved in the experience; it is not an
/// authored perception radius.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SpatialMemory {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) extent: f64,
    pub(crate) association: f64,
    pub(crate) association_weight: f64,
    pub(crate) strength: f64,
}

/// A generalized learned association for a recurring received spectrum.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SpectralMemory {
    pub(crate) spectrum: crate::harmonics::ToneSpectrum,
    pub(crate) association: f64,
    pub(crate) association_weight: f64,
    pub(crate) strength: f64,
}

/// The physical result retained by an experience before current-state
/// valuation interprets it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct MemoryConsequence {
    pub(crate) energy_delta: f64,
    pub(crate) stress_delta: f64,
    pub(crate) damage_delta: f64,
    pub(crate) developmental_delta: f64,
    pub(crate) material_acquired: f64,
    pub(crate) material_consumed: f64,
}

/// A specific experience linking what was perceived, where it was perceived,
/// what action occurred, and what physically happened afterward.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct EncounterMemory {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) extent: f64,
    pub(crate) spectrum: crate::harmonics::ToneSpectrum,
    pub(crate) action: crate::decision::ActionKind,
    pub(crate) consequence: MemoryConsequence,
    pub(crate) experience_weight: f64,
    pub(crate) strength: f64,
}

/// A movement experience waits for the following environmental transfer pass so
/// material acquired by overlap is attributed to the movement that caused it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PendingMovementExperience {
    pub(crate) perceptions: Vec<crate::harmonics::ResonancePerception>,
    pub(crate) consequence: MemoryConsequence,
    pub(crate) needs: crate::decision::CurrentNeeds,
}

/// A processing experience waits for a multi-tick BREAK transformation to
/// resolve before its physical consequence is assigned.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct PendingTransformationExperience {
    pub(crate) perceptions: Vec<crate::harmonics::ResonancePerception>,
    pub(crate) needs: crate::decision::CurrentNeeds,
    pub(crate) before_energy: f64,
    pub(crate) before_stress: f64,
    pub(crate) before_developmental_realization: f64,
}

/// The organism's associative memory. This is the new memory authority;
/// capacity and decay remain derived from the physical genome cavity.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct ExperienceMemory {
    pub(crate) spatial: Vec<SpatialMemory>,
    pub(crate) spectral: Vec<SpectralMemory>,
    pub(crate) encounters: Vec<EncounterMemory>,
}

fn qualifying_genome_cavity<'a>(
    organism: &'a mut Organism,
    environment: &Environment,
) -> Option<&'a crate::cavity::GenomeCavity> {
    organism.genome_cavity_cached_ref(&environment.catalog)
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

pub(crate) fn update_experience_memory(organism: &mut Organism, environment: &Environment) {
    let Some((capacity, decay)) = qualifying_genome_cavity(organism, environment)
        .map(|cavity| (memory_capacity(cavity), memory_decay_for_cavity(cavity)))
    else {
        organism.experience_memory = ExperienceMemory::default();
        return;
    };

    decay_experience_memory(&mut organism.experience_memory, decay);
    organism.experience_memory.spatial.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    organism.experience_memory.spectral.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    organism.experience_memory.encounters.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    organism.experience_memory.spatial.truncate(capacity);
    organism.experience_memory.spectral.truncate(capacity);
    organism.experience_memory.encounters.truncate(capacity);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experience_decay_reduces_strength_and_weight_and_prunes() {
        let mut memory = ExperienceMemory {
            spatial: vec![SpatialMemory {
                x: 0.0,
                y: 0.0,
                extent: 1.0,
                association: 0.5,
                association_weight: 1.0,
                strength: 0.5,
            }],
            spectral: Vec::new(),
            encounters: Vec::new(),
        };
        decay_experience_memory(&mut memory, 0.5);
        assert_eq!(memory.spatial[0].strength, 0.25);
        assert_eq!(memory.spatial[0].association_weight, 0.5);

        decay_experience_memory(&mut memory, 0.0);
        assert!(memory.spatial.is_empty());
    }

    #[test]
    fn spatial_reinforcement_keeps_location_association_independent() {
        let mut memory = ExperienceMemory::default();
        reinforce_spatial_memory(&mut memory, 10.0, 10.0, 2.0, 1.0, 1.0);
        reinforce_spatial_memory(&mut memory, 10.5, 10.0, 2.0, -1.0, 1.0);
        assert_eq!(memory.spatial.len(), 1);
        assert!(memory.spatial[0].association.abs() < f64::EPSILON);
        assert_eq!(memory.spatial[0].association_weight, 2.0);
    }

    #[test]
    fn spectral_reinforcement_generalizes_by_similarity() {
        let mut memory = ExperienceMemory::default();
        let a = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: 440.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let b = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: 460.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        reinforce_spectral_memory(&mut memory, &a, 1.0, 1.0);
        reinforce_spectral_memory(&mut memory, &b, -1.0, 1.0);
        assert_eq!(memory.spectral.len(), 1);
        assert!(memory.spectral[0].association < 1.0);
        assert!(memory.spectral[0].association > -1.0);
        let prototype_frequency = memory.spectral[0].spectrum.components[0].frequency_hz;
        assert!(prototype_frequency > 440.0 && prototype_frequency < 460.0);
    }

    #[test]
    fn unrelated_spectra_remain_separate_memories() {
        let mut memory = ExperienceMemory::default();
        let a = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: 440.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let b = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: 1760.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        reinforce_spectral_memory(&mut memory, &a, 1.0, 1.0);
        reinforce_spectral_memory(&mut memory, &b, -1.0, 1.0);
        assert_eq!(memory.spectral.len(), 2);
    }

    #[test]
    fn record_experience_keeps_location_and_spectrum_associations_independent() {
        let spectrum = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: 440.0,
                amplitude: 1.0,
                phase_radians: 0.0,
            }],
        };
        let perception_a = crate::harmonics::ResonancePerception {
            source_x: 10.0,
            source_y: 10.0,
            extent: 2.0,
            spectrum: spectrum.clone(),
            magnitude: 1.0,
        };
        let perception_b = crate::harmonics::ResonancePerception {
            source_x: 100.0,
            source_y: 100.0,
            extent: 2.0,
            spectrum,
            magnitude: 1.0,
        };
        let mut memory = ExperienceMemory::default();
        let needs = crate::decision::CurrentNeeds {
            survival: 0.5,
            ..Default::default()
        };

        record_experience(
            &mut memory,
            &[perception_a],
            crate::decision::ActionKind::Move,
            MemoryConsequence {
                energy_delta: 1.0,
                ..Default::default()
            },
            needs,
            8,
            1.0,
        );
        record_experience(
            &mut memory,
            &[perception_b],
            crate::decision::ActionKind::Move,
            MemoryConsequence {
                stress_delta: 1.0,
                ..Default::default()
            },
            needs,
            8,
        );

        assert_eq!(memory.spatial.len(), 2);
        assert_eq!(memory.spectral.len(), 1);
        assert!(memory.spectral[0].association.abs() < 1.0);
        assert_eq!(memory.encounters.len(), 2);
    }

    #[test]
    fn consequence_value_changes_with_current_need() {
        let consequence = MemoryConsequence {
            energy_delta: 1.0,
            ..Default::default()
        };
        let low_survival = consequence_value(
            &consequence,
            crate::decision::CurrentNeeds {
                survival: 0.0,
                ..Default::default()
            },
        );
        let high_survival = consequence_value(
            &consequence,
            crate::decision::CurrentNeeds {
                survival: 1.0,
                ..Default::default()
            },
        );
        assert!(high_survival > low_survival);
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
