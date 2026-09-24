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
) -> Option<(f64, f64)> {
    let (px, py) = organism.occupied_cells.first().map(|p| (p.x, p.y))?;

    let mut memory_x = 0.0;
    let mut memory_y = 0.0;
    let mut total = 0.0;
    for point in &organism.memory {
        let dx = point.x - px;
        let dy = wrapped_delta(point.y, py, environment_height);
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= f64::EPSILON {
            continue;
        }
        let weight = point.strength / distance;
        memory_x += dx / distance * weight;
        memory_y += dy / distance * weight;
        total += weight;
    }
    if total > 0.0 {
        memory_x /= total;
        memory_y /= total;
    }

    let (x, y) = if total <= 0.0 {
        // A zero-memory organism still needs an initial direction. Use a
        // stable pseudo-random angle derived from the organism id so the
        // initial push is soft/randomized without changing simulation state.
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in organism.id.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let angle = (hash as f64 / u64::MAX as f64) * std::f64::consts::TAU;
        (angle.cos(), angle.sin())
    } else {
        (memory_x, memory_y)
    };
    let magnitude = (x * x + y * y).sqrt();
    if magnitude <= f64::EPSILON {
        None
    } else {
        Some((x / magnitude, y / magnitude))
    }
}
