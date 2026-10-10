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

    let mut selected_score = f64::NEG_INFINITY;
    let mut selected_direction = None;

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
            .filter(|memory| {
                let memory_distance = (memory.x - perception.source_x).hypot(wrapped_delta(
                    memory.y,
                    perception.source_y,
                    environment_height,
                ));
                memory_distance <= memory.extent.max(0.0) + perception.extent.max(0.0)
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
            .map(|memory| {
                let similarity =
                    crate::harmonics::spectral_similarity(&memory.spectrum, &perception.spectrum);
                (memory, similarity)
            })
            .filter(|(_, similarity)| similarity.is_finite())
            .max_by(|(_, a), (_, b)| a.total_cmp(b));

        let spatial_confidence = spatial
            .map(|memory| memory.strength.max(0.0) / (1.0 + memory.strength.max(0.0)))
            .unwrap_or(0.0);
        let spatial_utility = spatial
            .map(|memory| spatial_confidence * memory.association)
            .unwrap_or(0.0);

        let spectral_confidence = spectral
            .map(|(memory, similarity)| {
                similarity * memory.strength.max(0.0) / (1.0 + memory.strength.max(0.0))
            })
            .unwrap_or(0.0);
        let spectral_utility = spectral
            .map(|(memory, _)| spectral_confidence * memory.association)
            .unwrap_or(0.0);

        let desirability = spatial_utility + spectral_utility;
        if !desirability.is_finite() || desirability.abs() <= f64::EPSILON {
            continue;
        }
        let score = perception.magnitude.max(0.0) * desirability;
        if !score.is_finite() || score <= selected_score {
            continue;
        }

        selected_score = score;
        let sign = if desirability < 0.0 { -1.0 } else { 1.0 };
        selected_direction = Some((sign * dx / distance, sign * dy / distance));
    }

    if let Some((direction_x, direction_y)) = selected_direction {
        return Some((direction_x, direction_y));
    }

    let mut hash = 0xcbf29ce484222325_u64;
    for byte in organism.id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let angle = (hash as f64 / u64::MAX as f64) * std::f64::consts::TAU;
    Some((angle.cos(), angle.sin()))
}
