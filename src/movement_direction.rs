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

    let x = memory_x;
    let y = memory_y;
    let magnitude = (x * x + y * y).sqrt();
    if magnitude <= f64::EPSILON {
        None
    } else {
        Some((x / magnitude, y / magnitude))
    }
}
