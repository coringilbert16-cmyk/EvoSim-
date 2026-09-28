use crate::state::Organism;

fn wrapped_delta(target: f64, origin: f64, period: f64) -> f64 {
    let direct = target - origin;
    if period > 0.0 {
        return (direct + period * 0.5).rem_euclid(period) - period * 0.5;
    }
    direct
}

pub(crate) fn movement_direction_periodic(
    organism: &Organism,
    environment_height: f64,
    perceptions: &[crate::harmonics::ResonancePerception],
) -> Option<(f64, f64)> {
    let (px, py) = organism.occupied_cells.first().map(|p| (p.x, p.y))?;
    let curiosity = organism.genome.curiosity();

    let mut direction_x = 0.0;
    let mut direction_y = 0.0;
    let mut current_signal = 0.0;

    for perception in perceptions {
        if !perception.magnitude.is_finite() || perception.magnitude <= f64::EPSILON {
            continue;
        }
        let dx = perception.source_x - px;
        let dy = wrapped_delta(perception.source_y, py, environment_height);
        let distance = dx.hypot(dy);
        if distance <= f64::EPSILON {
            continue;
        }

        let spatial = organism
            .experience_memory
            .spatial
            .iter()
            .filter_map(|memory| {
                let memory_distance = (memory.x - perception.source_x).hypot(wrapped_delta(
                    memory.y,
                    perception.source_y,
                    environment_height,
                ));
                (memory_distance <= memory.extent.max(0.0) + perception.extent.max(0.0))
                    .then_some(memory)
            })
            .max_by(|a, b| {
                a.strength
                    .partial_cmp(&b.strength)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let spectral = organism
            .experience_memory
            .spectral
            .iter()
            .filter_map(|memory| {
                let similarity =
                    crate::harmonics::spectral_similarity(&memory.spectrum, &perception.spectrum);
                similarity.is_finite().then_some((memory, similarity))
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b));

        let spatial_confidence = spatial
            .map(|memory| memory.strength.max(0.0) / (1.0 + memory.strength.max(0.0)))
            .unwrap_or(0.0);
        let spatial_utility = spatial
            .map(|memory| {
                spatial_confidence * memory.association + (1.0 - spatial_confidence) * curiosity
            })
            .unwrap_or(curiosity);

        let spectral_confidence = spectral
            .map(|(memory, similarity)| {
                similarity * memory.strength.max(0.0) / (1.0 + memory.strength.max(0.0))
            })
            .unwrap_or(0.0);
        let spectral_utility = spectral
            .map(|(memory, _)| {
                spectral_confidence * memory.association + (1.0 - spectral_confidence) * curiosity
            })
            .unwrap_or(curiosity);

        let stimulus = perception.magnitude.max(0.0) * (spatial_utility + spectral_utility);
        direction_x += dx / distance * stimulus;
        direction_y += dy / distance * stimulus;
        current_signal += perception.magnitude.max(0.0);
    }

    // When current perception is absent, remembered spatial associations can
    // still guide a return. This is deliberately weaker than live perception.
    if current_signal <= f64::EPSILON {
        for memory in &organism.experience_memory.spatial {
            let dx = memory.x - px;
            let dy = wrapped_delta(memory.y, py, environment_height);
            let distance = dx.hypot(dy);
            if distance <= f64::EPSILON {
                continue;
            }
            let weight = memory.strength.max(0.0) * memory.association;
            direction_x += dx / distance * weight;
            direction_y += dy / distance * weight;
        }
    }

    let magnitude = direction_x.hypot(direction_y);
    if magnitude <= f64::EPSILON {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in organism.id.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let angle = (hash as f64 / u64::MAX as f64) * std::f64::consts::TAU;
        return Some((angle.cos(), angle.sin()));
    }

    Some((direction_x / magnitude, direction_y / magnitude))
}
