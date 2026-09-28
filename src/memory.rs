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
pub(crate) const CONSEQUENCE_TRANSFORMED_MATERIAL_SCALE: f64 = 1.0;

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

    let association = consequence_value(&consequence, needs);
    let raw_magnitude = [
        consequence.energy_delta / CONSEQUENCE_ENERGY_SCALE,
        consequence.developmental_delta / CONSEQUENCE_DEVELOPMENT_SCALE,
        consequence.material_acquired / CONSEQUENCE_MATERIAL_SCALE,
        consequence.stress_delta / CONSEQUENCE_STRESS_SCALE,
        consequence.damage_delta / CONSEQUENCE_DAMAGE_SCALE,
        consequence.material_consumed / CONSEQUENCE_CONSUMED_MATERIAL_SCALE,
        consequence.material_transformed / CONSEQUENCE_TRANSFORMED_MATERIAL_SCALE,
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