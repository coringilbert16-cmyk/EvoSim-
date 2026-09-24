use crate::state::Organism;

fn wrapped_delta(target: f64, origin: f64, period: f64) -> f64 {
    let direct = target - origin;
    if period > 0.0 {
        return (direct + period * 0.5).rem_euclid(period) - period * 0.5;
    }
    direct
}

pub(crate) fn resource_seek_direction(
    organism: &Organism,
    environment: &crate::state::Environment,
    resource_names: &[String],
) -> Option<(f64, f64)> {
    if resource_names.is_empty() {
        return None;
    }
    let (px, py) = organism.occupied_cells.first().map(|p| (p.x, p.y))?;
    let radius = organism.genome.perception_radius().max(0.0);
    let indices = environment.field.cells_within_radius(px, py, radius);
    let mut dx_total = 0.0;
    let mut dy_total = 0.0;
    let mut weight_total = 0.0;
    for index in indices {
        let (cx, cy) = environment.field.cell_center(index);
        let mut amount = 0.0;
        let cell = &environment.field.cells[index];
        for material in &cell.materials {
            amount += material
                .parts
                .iter()
                .filter(|(name, value)| {
                    *value > 0.0 && resource_names.iter().any(|wanted| wanted == name)
                })
                .map(|(_, value)| *value)
                .sum::<f64>();
        }
        for physical in &cell.physical_materials {
            amount += physical
                .material
                .parts
                .iter()
                .filter(|(name, value)| {
                    *value > 0.0 && resource_names.iter().any(|wanted| wanted == name)
                })
                .map(|(_, value)| *value)
                .sum::<f64>();
        }
        if amount <= 0.0 {
            continue;
        }
        let dx = cx - px;
        let direct_dy = cy - py;
        let field_height = environment.height.max(0.0);
        let dy = if field_height > 0.0 {
            (direct_dy + field_height * 0.5).rem_euclid(field_height) - field_height * 0.5
        } else {
            direct_dy
        };
        let distance = dx.hypot(dy);
        if distance <= f64::EPSILON {
            continue;
        }
        let weight = amount / distance;
        dx_total += dx / distance * weight;
        dy_total += dy / distance * weight;
        weight_total += weight;
    }
    if weight_total <= f64::EPSILON {
        return None;
    }
    let magnitude = dx_total.hypot(dy_total);
    if magnitude <= f64::EPSILON {
        None
    } else {
        Some((dx_total / magnitude, dy_total / magnitude))
    }
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
