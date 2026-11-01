//! Deterministic ambient transport for realized environmental material.
//!
//! The flow is deliberately a bounded approximation, not a fluid solver. It
//! supplies a smooth S-shaped current, local recirculation, mass-dependent
//! mobility, and coarse obstacle wakes while leaving chemistry to the normal
//! simulation tick.

use crate::environment::ActiveMaterialField;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{BaseResource, Form};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Velocity {
    x: f64,
    y: f64,
}

impl Velocity {
    fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    fn normalized(self) -> Self {
        let length = self.length();
        if length > f64::EPSILON {
            self.scale(1.0 / length)
        } else {
            Self { x: 1.0, y: 0.0 }
        }
    }
}

const FLOW_BIN_SIZE: f64 = 5.0;

#[derive(Clone, Copy, Debug)]
struct Obstacle {
    source_cell: usize,
    source_index: usize,
    x: f64,
    y: f64,
    radius: f64,
    solidity: f64,
}

fn resource_for_part<'a>(name: &str, catalog: &'a [BaseResource]) -> Option<&'a BaseResource> {
    catalog.iter().find(|resource| resource.name == name)
}

fn material_center(material: &PhysicalMaterial, height: f64) -> Option<(f64, f64)> {
    let placements = material.placements.as_ref()?;
    if placements.is_empty()
        || !height.is_finite()
        || height <= 0.0
        || placements
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return None;
    }
    let count = placements.len() as f64;
    let x = placements.iter().map(|p| p.x).sum::<f64>() / count;
    // Vertical coordinates are bounded, not periodic. Keep the ordinary
    // center of mass so objects cannot appear across the top/bottom seam.
    let y = placements.iter().map(|p| p.y).sum::<f64>() / count;
    Some((x, y))
}

fn shape_radius(form: &Form) -> f64 {
    match form {
        Form::Circle { radius } => *radius,
        Form::Line { length } => length * 0.5,
        Form::Rectangle { width, height } => (width * width + height * height).sqrt() * 0.5,
        Form::RegularPolygon { radius, .. } => *radius,
        Form::Polygon { vertices } => vertices
            .iter()
            .map(|(x, y)| x.hypot(*y))
            .fold(0.0, f64::max),
        Form::Fluid { nominal_area, .. } => (nominal_area.max(0.0) / std::f64::consts::PI).sqrt(),
    }
}

/// Approximate each constituent's intrinsic planar moment from its
/// realized geometry. Compound inertia adds this to the point-mass contribution
/// from each constituent's offset from the compound center.
fn intrinsic_moment_of_inertia(form: &Form, mass: f64) -> f64 {
    match form {
        Form::Circle { radius } => 0.5 * mass * radius * radius,
        Form::Line { length } => mass * length * length / 12.0,
        Form::Rectangle { width, height } => mass * (width * width + height * height) / 12.0,
        Form::RegularPolygon { radius, .. } => 0.5 * mass * radius * radius,
        Form::Polygon { vertices } => {
            let mean_radius_sq = vertices.iter().map(|(x, y)| x * x + y * y).sum::<f64>()
                / vertices.len().max(1) as f64;
            0.5 * mass * mean_radius_sq
        }
        Form::Fluid { nominal_area, .. } => {
            let radius_sq = nominal_area.max(0.0) / std::f64::consts::PI;
            0.5 * mass * radius_sq
        }
    }
}

fn obstacle_for(
    source_cell: usize,
    source_index: usize,
    material: &PhysicalMaterial,
    catalog: &[BaseResource],
    height: f64,
) -> Option<Obstacle> {
    let (x, y) = material_center(material, height)?;
    let placements = material.placements.as_ref()?;
    let mut radius: f64 = 0.5;
    let mut weighted_cohesion = 0.0;
    let mut total = 0.0;
    for (index, (name, amount)) in material.material.parts.iter().enumerate() {
        let resource = resource_for_part(name, catalog)?;
        let placement = placements.get(index)?;
        radius = radius
            .max((placement.x - x).hypot(placement.y - y) + shape_radius(&resource.shape.form));
        weighted_cohesion += resource.properties.cohesion * *amount;
        total += *amount;
    }
    if !weighted_cohesion.is_finite() || total <= 0.0 {
        return None;
    }
    let cohesion = weighted_cohesion / total;
    let permeability = crate::resources::permeability_from_cohesion(cohesion, catalog);
    Some(Obstacle {
        source_cell,
        source_index,
        x,
        y,
        radius,
        solidity: (1.0 - permeability).clamp(0.0, 1.0),
    })
}

/// Linear mobility with a strictly positive floor. This affects response to
/// the current, not the current itself; mass never makes an object immovable.
fn mobility(mass: f64) -> f64 {
    if !mass.is_finite() || mass < 0.0 {
        return 0.2;
    }
    (1.0 - 0.4 * mass).clamp(0.2, 1.0)
}

/// Smooth S-shaped background flow with outward speed falloff and two small
/// vortices at the bends. The same query is used for every material type.
fn background_velocity(x: f64, y: f64, width: f64, height: f64) -> Velocity {
    if ![x, y, width, height].iter().all(|v| v.is_finite()) || width <= 0.0 || height <= 0.0 {
        return Velocity::default();
    }
    let wrapped_x = x.rem_euclid(width);
    let phase = std::f64::consts::TAU * wrapped_x / width;
    let center_y = height * 0.5 + height * 0.23 * phase.sin();
    let slope = height * 0.23 * std::f64::consts::TAU / width * phase.cos();
    let vertical_delta = y - center_y;
    let width_scale = (height * 0.16).max(1.0);
    let envelope = (-0.5 * (vertical_delta / width_scale).powi(2)).exp();
    let tangent = Velocity { x: 1.0, y: slope }.normalized();
    let mut velocity = tangent.scale(0.12 + 0.88 * envelope);

    // The flow field is periodic left-to-right and bounded top-to-bottom.
    for (fraction, sign) in [(0.25, 1.0), (0.75, -1.0)] {
        let cx = width * fraction;
        let cy = height * 0.5 + height * 0.23 * (std::f64::consts::TAU * fraction).sin();
        let dx = (wrapped_x - cx + width * 0.5).rem_euclid(width) - width * 0.5;
        let dy = y - cy;
        let radial_sq = dx * dx + dy * dy;
        let vortex_radius = (height * 0.13).max(1.0);
        let weight = (-radial_sq / (2.0 * vortex_radius * vortex_radius)).exp() * sign * 1.2;
        velocity = velocity.add(Velocity {
            x: -dy / vortex_radius * weight,
            y: dx / vortex_radius * weight,
        });
    }
    velocity
}

fn obstacle_adjusted_velocity(
    mut velocity: Velocity,
    x: f64,
    y: f64,
    source_cell: usize,
    source_index: usize,
    obstacles: &[Obstacle],
    width: f64,
) -> Velocity {
    for obstacle in obstacles {
        if obstacle.source_cell == source_cell && obstacle.source_index == source_index {
            continue;
        }
        if obstacle.solidity <= 1e-6 {
            continue;
        }
        let dx = (x - obstacle.x + width * 0.5).rem_euclid(width) - width * 0.5;
        let dy = y - obstacle.y;
        let distance = dx.hypot(dy).max(1e-9);
        let radius = obstacle.radius.max(0.5);
        if distance < radius * 2.5 {
            let influence = obstacle.solidity * (1.0 - distance / (radius * 2.5)).powi(2);
            let side = if dy >= 0.0 { 1.0 } else { -1.0 };
            velocity = velocity.add(Velocity {
                x: -dx.signum() * influence * 0.22,
                y: side * influence * 0.65,
            });
        }
        if dx > 0.0 && dx < radius * 6.0 && dy.abs() < radius * 2.0 {
            let longitudinal = 1.0 - dx / (radius * 6.0);
            let lateral = 1.0 - dy.abs() / (radius * 2.0);
            let attenuation = (obstacle.solidity * longitudinal * lateral * 0.72).clamp(0.0, 0.85);
            velocity = velocity.scale(1.0 - attenuation);
        }
    }
    velocity
}

fn shape_vertical_extent(form: &Form, rotation_radians: f64) -> f64 {
    let (sin, cos) = rotation_radians.sin_cos();
    match form {
        Form::Circle { radius } => *radius,
        Form::Line { length } => length * 0.5 * sin.abs(),
        Form::Rectangle { width, height } => width * 0.5 * sin.abs() + height * 0.5 * cos.abs(),
        Form::RegularPolygon { radius, .. } => *radius,
        Form::Polygon { vertices } => vertices
            .iter()
            .map(|(x, y)| (x * sin + y * cos).abs())
            .fold(0.0, f64::max),
        Form::Fluid {
            nominal_area,
            arc_boundary,
            ..
        } => {
            let equivalent_radius = (nominal_area.max(0.0) / std::f64::consts::PI).sqrt();
            arc_boundary
                .as_ref()
                .map_or(equivalent_radius, |arc| equivalent_radius.max(arc.radius))
        }
    }
}

fn clamp_vertical_displacement(
    material: &PhysicalMaterial,
    requested_dy: f64,
    height: f64,
    catalog: &[BaseResource],
) -> Option<f64> {
    let placements = material.placements.as_ref()?;
    if placements.is_empty()
        || placements.len() != material.material.parts.len()
        || !height.is_finite()
        || height <= 0.0
    {
        return None;
    }
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (index, placement) in placements.iter().enumerate() {
        let (name, _) = material.material.parts.get(index)?;
        let form = material
            .realized_shapes
            .as_ref()
            .and_then(|shapes| shapes.get(index))
            .map(|shape| &shape.form)
            .or_else(|| resource_for_part(name, catalog).map(|resource| &resource.shape.form))?;
        let extent = shape_vertical_extent(form, placement.rotation_radians);
        if !placement.y.is_finite() || !extent.is_finite() || extent < 0.0 {
            return None;
        }
        min_y = min_y.min(placement.y - extent);
        max_y = max_y.max(placement.y + extent);
    }
    let world_max = (height - 1e-6).max(0.0);
    let min_dy = -min_y;
    let max_dy = world_max - max_y;
    (min_dy <= max_dy).then_some(requested_dy.clamp(min_dy, max_dy))
}
fn translated(
    material: &PhysicalMaterial,
    dx: f64,
    dy: f64,
    height: f64,
    catalog: &[BaseResource],
) -> Option<PhysicalMaterial> {
    let dy = clamp_vertical_displacement(material, dy, height, catalog)?;
    let mut moved = material.clone();
    let placements = moved.placements.as_mut()?;
    if placements.is_empty() {
        return None;
    }
    for placement in placements {
        placement.x += dx;
        placement.y += dy;
        if !placement.x.is_finite()
            || !placement.y.is_finite()
            || !placement.rotation_radians.is_finite()
        {
            return None;
        }
    }
    Some(moved)
}

fn angular_displacement(
    material: &PhysicalMaterial,
    catalog: &[BaseResource],
    width: f64,
    height: f64,
    obstacles: &[Obstacle],
    source_cell: usize,
    source_index: usize,
) -> f64 {
    let Some(placements) = material.placements.as_ref() else {
        return 0.0;
    };
    if placements.is_empty() {
        return 0.0;
    }
    let total_mass = material.material.mass(catalog);
    if !total_mass.is_finite()
        || total_mass <= 0.0
        || placements.len() != material.material.parts.len()
    {
        return 0.0;
    }
    let anchor_y = placements[0].y;
    let mut weighted_x = 0.0;
    let mut weighted_y = 0.0;
    for (index, placement) in placements.iter().enumerate() {
        let Some((name, amount)) = material.material.parts.get(index) else {
            return 0.0;
        };
        let Some(resource) = resource_for_part(name, catalog) else {
            return 0.0;
        };
        let part_mass = resource.properties.mass * *amount;
        if !part_mass.is_finite() || part_mass <= 0.0 {
            return 0.0;
        }
        weighted_x += placement.x * part_mass;
        weighted_y += (placement.y - anchor_y) * part_mass;
    }
    let center_x = weighted_x / total_mass;
    let center_y = anchor_y + weighted_y / total_mass;

    let center_velocity = obstacle_adjusted_velocity(
        background_velocity(center_x, center_y, width, height),
        center_x,
        center_y,
        source_cell,
        source_index,
        obstacles,
        width,
    );
    let mut torque = 0.0;
    let mut moment = 0.0;
    for (index, placement) in placements.iter().enumerate() {
        let Some((name, amount)) = material.material.parts.get(index) else {
            continue;
        };
        let Some(resource) = resource_for_part(name, catalog) else {
            continue;
        };
        let part_mass = resource.properties.mass * *amount;
        if !part_mass.is_finite() || part_mass <= 0.0 {
            continue;
        }
        let rx = placement.x - center_x;
        let ry = placement.y - center_y;
        let radius_sq = rx * rx + ry * ry;
        let form = material
            .realized_shapes
            .as_ref()
            .and_then(|shapes| shapes.get(index))
            .map(|shape| &shape.form)
            .unwrap_or(&resource.shape.form);
        moment += part_mass * radius_sq + intrinsic_moment_of_inertia(form, part_mass);
        if radius_sq <= f64::EPSILON {
            continue;
        }
        let local_velocity = obstacle_adjusted_velocity(
            background_velocity(placement.x, placement.y, width, height),
            placement.x,
            placement.y,
            source_cell,
            source_index,
            obstacles,
            width,
        );
        torque += part_mass
            * (rx * (local_velocity.y - center_velocity.y)
                - ry * (local_velocity.x - center_velocity.x));
    }

    let response = if placements.len() > 1 && moment > f64::EPSILON {
        torque / moment
    } else {
        // A single constituent has no center-of-mass lever arm. Its shape's
        // anisotropy determines whether flow vorticity can turn its orientation.
        let shape_form = material
            .realized_shapes
            .as_ref()
            .and_then(|shapes| shapes.first())
            .map(|shape| &shape.form)
            .or_else(|| {
                resource_for_part(&material.material.parts[0].0, catalog)
                    .map(|resource| &resource.shape.form)
            });
        let anisotropy = match shape_form {
            Some(Form::Circle { .. }) | None => 0.0,
            Some(Form::Fluid {
                boundary,
                arc_boundary,
                multi_contact_boundary,
                ..
            }) => {
                if boundary.is_some() || arc_boundary.is_some() || multi_contact_boundary.is_some()
                {
                    1.0
                } else {
                    0.0
                }
            }
            Some(Form::Line { .. })
            | Some(Form::Rectangle { .. })
            | Some(Form::RegularPolygon { .. })
            | Some(Form::Polygon { .. }) => 1.0,
        };
        if anisotropy == 0.0 {
            return 0.0;
        }
        let Some(shape_form) = shape_form else {
            return 0.0;
        };
        let epsilon = (width.min(height) * 1e-4).max(1e-3);
        let right = obstacle_adjusted_velocity(
            background_velocity(center_x + epsilon, center_y, width, height),
            center_x + epsilon,
            center_y,
            source_cell,
            source_index,
            obstacles,
            width,
        );
        let left = obstacle_adjusted_velocity(
            background_velocity(center_x - epsilon, center_y, width, height),
            center_x - epsilon,
            center_y,
            source_cell,
            source_index,
            obstacles,
            width,
        );
        let up = obstacle_adjusted_velocity(
            background_velocity(center_x, center_y + epsilon, width, height),
            center_x,
            center_y + epsilon,
            source_cell,
            source_index,
            obstacles,
            width,
        );
        let down = obstacle_adjusted_velocity(
            background_velocity(center_x, center_y - epsilon, width, height),
            center_x,
            center_y - epsilon,
            source_cell,
            source_index,
            obstacles,
            width,
        );
        let dv_y_dx = (right.y - left.y) / (2.0 * epsilon);
        let dv_x_dy = (up.x - down.x) / (2.0 * epsilon);
        let characteristic_radius_sq = shape_radius(shape_form).powi(2);
        let intrinsic_moment = intrinsic_moment_of_inertia(shape_form, total_mass);
        if intrinsic_moment <= f64::EPSILON {
            0.0
        } else {
            (dv_y_dx - dv_x_dy) * 0.5 * anisotropy * total_mass * characteristic_radius_sq
                / intrinsic_moment
        }
    };

    let angle = response * mobility(total_mass) * 0.15;
    if angle.is_finite() {
        angle
    } else {
        0.0
    }
}

/// Rotate a complete realized object around its mass-center while preserving
/// every constituent's local orientation offset and all bond metadata.
fn rotated(
    material: &PhysicalMaterial,
    angle: f64,
    catalog: &[BaseResource],
    _width: f64,
    height: f64,
) -> Option<PhysicalMaterial> {
    if !angle.is_finite() {
        return None;
    }
    let mut result = material.clone();
    {
        let placements = result.placements.as_mut()?;
        if placements.is_empty() || placements.len() != material.material.parts.len() {
            return None;
        }
        let anchor_y = placements[0].y;
        let mut total_mass = 0.0;
        let mut weighted_x = 0.0;
        let mut weighted_y = 0.0;
        for (index, placement) in placements.iter().enumerate() {
            let (name, amount) = material.material.parts.get(index)?;
            let resource = resource_for_part(name, catalog)?;
            let mass = resource.properties.mass * *amount;
            if !mass.is_finite() || mass <= 0.0 {
                return None;
            }
            total_mass += mass;
            weighted_x += placement.x * mass;
            weighted_y += (placement.y - anchor_y) * mass;
        }
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return None;
        }
        let center_x = weighted_x / total_mass;
        let center_y = anchor_y + weighted_y / total_mass;
        let (sin, cos) = angle.sin_cos();
        for placement in placements.iter_mut() {
            let dx = placement.x - center_x;
            let dy = placement.y - center_y;
            placement.x = center_x + dx * cos - dy * sin;
            placement.y = center_y + dx * sin + dy * cos;
            placement.rotation_radians =
                (placement.rotation_radians + angle).rem_euclid(std::f64::consts::TAU);
            if !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
            {
                return None;
            }
        }
    }

    let vertical_shift = clamp_vertical_displacement(&result, 0.0, height, catalog)?;
    for placement in result.placements.as_mut()? {
        placement.y += vertical_shift;
    }
    Some(result)
}

/// Advance the field by one small deterministic transport slice. All candidate
/// destinations are computed from a snapshot, then committed together so
/// iteration order cannot change which current an object samples.
pub(crate) fn transport_field(
    field: &mut ActiveMaterialField,
    catalog: &[BaseResource],
    world_width: f64,
    world_height: f64,
) -> usize {
    let width = world_width.min(field.width_cells as f64 * field.cell_size);
    let height = world_height.min(field.height_cells as f64 * field.cell_size);
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return 0;
    }

    let snapshot: Vec<(usize, usize, PhysicalMaterial)> = field
        .cells
        .iter()
        .enumerate()
        .flat_map(|(cell_index, cell)| {
            cell.physical_materials
                .iter()
                .cloned()
                .enumerate()
                .map(move |(material_index, material)| (cell_index, material_index, material))
        })
        .collect();
    // Use finer bins than the material field. Spatial indexing narrows each
    // query to the local region, but every obstacle in that region contributes;
    // do not truncate physical influence with a per-object candidate cap.
    let obstacles: Vec<Obstacle> = snapshot
        .iter()
        .filter_map(|(cell, index, material)| {
            obstacle_for(*cell, *index, material, catalog, height)
        })
        .collect();
    let flow_cols = (width / FLOW_BIN_SIZE).ceil().max(1.0) as usize;
    let flow_rows = (height / FLOW_BIN_SIZE).ceil().max(1.0) as usize;
    let mut obstacle_bins = vec![Vec::<Obstacle>::new(); flow_cols * flow_rows];
    for obstacle in obstacles {
        let radius = obstacle.radius.max(0.5).min(width.max(height));
        let first_col = ((obstacle.x - radius * 6.0) / FLOW_BIN_SIZE).floor() as isize;
        let last_col = ((obstacle.x + radius * 6.0) / FLOW_BIN_SIZE).floor() as isize;
        let col_count = last_col.saturating_sub(first_col).saturating_add(1) as usize;
        let cols: Vec<usize> = if col_count >= flow_cols {
            (0..flow_cols).collect()
        } else {
            (0..col_count)
                .map(|offset| (first_col + offset as isize).rem_euclid(flow_cols as isize) as usize)
                .collect()
        };
        let first_row =
            (((obstacle.y - radius * 2.5) / FLOW_BIN_SIZE).floor() as isize).max(0) as usize;
        let last_row = (((obstacle.y + radius * 2.5) / FLOW_BIN_SIZE).floor() as isize)
            .min(flow_rows.saturating_sub(1) as isize)
            .max(0) as usize;
        if first_row > last_row {
            continue;
        }
        for row in first_row..=last_row {
            for &col in &cols {
                obstacle_bins[row * flow_cols + col].push(obstacle);
            }
        }
    }
    let mut staged = Vec::new();

    for (source_cell, source_index, material) in snapshot {
        let Some((x, y)) = material_center(&material, height) else {
            continue;
        };
        let mass = material.material.mass(catalog);
        let flow_col =
            ((x / FLOW_BIN_SIZE).floor() as isize).rem_euclid(flow_cols as isize) as usize;
        let flow_row = ((y / FLOW_BIN_SIZE).floor() as isize)
            .clamp(0, flow_rows.saturating_sub(1) as isize) as usize;
        let nearby = &obstacle_bins[flow_row * flow_cols + flow_col];
        let velocity = obstacle_adjusted_velocity(
            background_velocity(x, y, width, height),
            x,
            y,
            source_cell,
            source_index,
            nearby,
            width,
        );
        let scale = mobility(mass) * 0.15;
        let dx = velocity.x * scale;
        let dy = velocity.y * scale;
        let angular_step = angular_displacement(
            &material,
            catalog,
            width,
            height,
            nearby,
            source_cell,
            source_index,
        );
        if dx.abs() + dy.abs() <= 1e-12 && angular_step.abs() <= 1e-12 {
            continue;
        }
        let Some(translated_material) = translated(&material, dx, dy, height, catalog) else {
            continue;
        };
        let Some(moved) = rotated(&translated_material, angular_step, catalog, width, height)
        else {
            continue;
        };
        let Some(anchor) = moved.placements.as_ref().and_then(|p| p.first()) else {
            continue;
        };
        let Some(destination_cell) = field.index_for_position(anchor.x, anchor.y) else {
            continue;
        };
        if destination_cell != source_cell || moved != material {
            staged.push((source_cell, source_index, destination_cell, moved));
        }
    }
    if staged.is_empty() {
        return 0;
    }

    let moved_count = staged.len();
    let mut changed = vec![false; field.cells.len()];
    for (source_cell, source_index, _, _) in &staged {
        if *source_cell >= field.cells.len()
            || *source_index >= field.cells[*source_cell].physical_materials.len()
        {
            return 0;
        }
    }
    // Remove from each source in reverse index order before any destination
    // insertions. Snapshot indices are stable until this removal phase ends.
    let mut removals: Vec<(usize, usize)> = staged
        .iter()
        .map(|(cell, index, _, _)| (*cell, *index))
        .collect();
    removals.sort_unstable_by(|a, b| b.cmp(a));
    for (cell, index) in removals {
        field.cells[cell].physical_materials.remove(index);
        changed[cell] = true;
    }
    for (_, _, destination, material) in staged {
        field.cells[destination].physical_materials.push(material);
        changed[destination] = true;
    }
    for (index, did_change) in changed.into_iter().enumerate() {
        if did_change {
            field.mark_changed_at_index(index);
        }
    }
    moved_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_flow_is_finite_deterministic_and_left_to_right() {
        let first = background_velocity(20.0, 30.0, 100.0, 80.0);
        let second = background_velocity(20.0, 30.0, 100.0, 80.0);
        assert_eq!(first, second);
        assert!(first.x.is_finite() && first.y.is_finite());
        assert!(first.x > 0.0);
    }

    #[test]
    fn current_has_a_speed_gradient_and_vortices() {
        let core = background_velocity(25.0, 40.0 + 80.0 * 0.23, 100.0, 80.0).length();
        let outer = background_velocity(25.0, 5.0, 100.0, 80.0).length();
        assert!(core > outer);
        let vortex = background_velocity(25.0, 40.0 + 80.0 * 0.23, 100.0, 80.0);
        assert!(vortex.x.is_finite() && vortex.y.is_finite());
        let first_bend_counter_flow =
            background_velocity(25.0, 40.0 + 80.0 * 0.23 + 1.5 * (80.0 * 0.13), 100.0, 80.0);
        assert!(
            first_bend_counter_flow.x < 0.0,
            "the first bend must create a local counter-flow pocket"
        );

        let second_bend_center = 40.0 - 80.0 * 0.23;
        let second_bend_counter_flow =
            background_velocity(75.0, second_bend_center - 1.5 * (80.0 * 0.13), 100.0, 80.0);
        assert!(
            second_bend_counter_flow.x < 0.0,
            "the second bend must create a local counter-flow pocket"
        );
    }

    #[test]
    fn mobility_decreases_linearly_until_positive_floor() {
        assert!(mobility(1.5) < mobility(0.5));
        assert_eq!(mobility(100.0), 0.2);
        assert!(mobility(100.0) > 0.0);
    }

    #[test]
    fn actual_material_cohesion_sets_obstacle_permeability() {
        let catalog = crate::resources::default_catalog();
        let make_material = |name: &str| {
            PhysicalMaterial::realized(
                crate::resources::Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x: 10.0,
                    y: 10.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("catalog material should realize")
        };
        let water = make_material("Water");
        let carbon = make_material("Carbon");
        let water_obstacle = obstacle_for(0, 0, &water, &catalog, 100.0).unwrap();
        let carbon_obstacle = obstacle_for(0, 1, &carbon, &catalog, 100.0).unwrap();

        assert_eq!(water_obstacle.solidity, 0.0);
        assert!(
            carbon_obstacle.solidity > 0.9,
            "dense carbon geometry should strongly disturb local flow"
        );
        assert!(
            carbon_obstacle.radius > water_obstacle.radius,
            "the obstacle footprint should come from the material's actual catalog shape"
        );
    }

    #[test]
    fn porous_obstacle_does_not_create_a_solid_wake() {
        let porous = Obstacle {
            source_cell: 9,
            source_index: 9,
            x: 10.0,
            y: 10.0,
            radius: 3.0,
            solidity: 0.0,
        };
        let free = Velocity { x: 1.0, y: 0.0 };
        assert_eq!(
            obstacle_adjusted_velocity(free, 15.0, 10.0, 1, 1, &[porous], 100.0),
            free
        );
    }

    #[test]
    fn transport_moves_complete_material_and_preserves_inventory() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let material = PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 24.999,
                y: 200.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("catalog carbon should realize");
        let original = material.clone();
        assert!(field.deposit_physical(24.999, 200.0, material, &catalog));
        let inventory_before = field.total_material();
        let revision_before = field.revision;
        let source_cell = field.index_for_position(24.999, 200.0).unwrap();
        let source_cell_revision = field.cell_revisions[source_cell];
        let destination_cell = field.index_for_position(25.001, 200.0).unwrap();
        let destination_cell_revision = field.cell_revisions[destination_cell];

        assert_eq!(transport_field(&mut field, &catalog, 1000.0, 1000.0), 1);

        assert_eq!(field.total_material(), inventory_before);
        assert!(field.revision > revision_before);
        assert!(field.cell_revisions[source_cell] > source_cell_revision);
        assert!(field.cell_revisions[destination_cell] > destination_cell_revision);
        let objects: Vec<_> = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(objects.len(), 1);
        assert!(objects[0].placements.as_ref().unwrap()[0].x > 25.0);
        assert_ne!(
            field.index_for_position(objects[0].placements.as_ref().unwrap()[0].x, 200.0),
            Some(source_cell)
        );
        let (center_x, center_y) = material_center(&original, 1000.0).unwrap();
        let velocity = background_velocity(center_x, center_y, 1000.0, 1000.0);
        let displacement_scale = mobility(original.material.mass(&catalog)) * 0.15;
        let translated_expected = translated(
            &original,
            velocity.x * displacement_scale,
            velocity.y * displacement_scale,
            1000.0,
            &catalog,
        )
        .unwrap();
        let angular_step = angular_displacement(&original, &catalog, 1000.0, 1000.0, &[], 0, 0);
        let expected =
            rotated(&translated_expected, angular_step, &catalog, 1000.0, 1000.0).unwrap();
        assert_eq!(
            objects[0], &expected,
            "transport may change only world placement and orientation"
        );
    }

    #[test]
    fn environmental_transport_crosses_the_horizontal_seam_without_relocation_loss() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let material = PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 999.99,
                y: 500.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("catalog carbon should realize");
        let source = field.index_for_position(999.99, 500.0).unwrap();
        assert!(field.deposit_physical(999.99, 500.0, material, &catalog));

        assert_eq!(transport_field(&mut field, &catalog, 1000.0, 1000.0), 1);

        let objects: Vec<_> = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(objects.len(), 1);
        let placement = &objects[0].placements.as_ref().unwrap()[0];
        assert!(
            placement.x >= 1000.0,
            "the element should cross the right edge"
        );
        let destination = field.index_for_position(placement.x, placement.y).unwrap();
        assert_ne!(destination, source);
        assert_eq!(
            destination,
            field
                .index_for_position(placement.x - 1000.0, placement.y)
                .unwrap()
        );
        assert_eq!(field.total_material(), vec![("Carbon".to_string(), 1.0)]);
    }

    #[test]
    fn horizontal_boundary_wraps_complete_compounds_without_changing_geometry() {
        let catalog = crate::resources::default_catalog();
        let material = PhysicalMaterial::realized(
            crate::resources::Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                crate::structure::Placement {
                    x: 999.2,
                    y: 500.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 999.8,
                    y: 500.0,
                    rotation_radians: 0.25,
                },
            ],
            &catalog,
        )
        .expect("realized carbon-hydrogen compound should realize");

        let moved = translated(&material, 0.5, 0.0, 1000.0, &catalog).unwrap();
        let placements = moved.placements.as_ref().unwrap();
        assert!(placements.iter().all(|p| p.x >= 999.0));
        assert!(placements.iter().any(|p| p.x >= 1000.0));
        assert_eq!(
            ActiveMaterialField::new(1000.0, 1000.0, 25.0)
                .index_for_position(placements[0].x, placements[0].y),
            ActiveMaterialField::new(1000.0, 1000.0, 25.0)
                .index_for_position(placements[0].x - 1000.0, placements[0].y)
        );
        let original_placements = material.placements.as_ref().unwrap();
        let original_separation = original_placements[1].x - original_placements[0].x;
        let moved_separation = placements[1].x - placements[0].x;
        assert!(
            (moved_separation - original_separation).abs() < 1e-9,
            "wrapping must preserve the object's internal geometry"
        );
    }

    #[test]
    fn vertical_boundary_is_a_hard_wall_not_a_wraparound_seam() {
        let catalog = crate::resources::default_catalog();
        let material = PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 500.0,
                y: 999.999,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("catalog carbon should realize");
        let moved = translated(&material, 0.0, 1.0, 1000.0, &catalog).unwrap();
        let placement = &moved.placements.as_ref().unwrap()[0];
        let carbon_form = &catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .expect("carbon must exist in the canonical catalog")
            .shape
            .form;
        let extent = shape_vertical_extent(carbon_form, 0.0);
        assert!(placement.y - extent >= 0.0);
        assert!(placement.y + extent <= 1000.0 - 1e-6 + 1e-9);
        assert!(
            placement.y < 999.999,
            "the element's full geometry, not only its center, must remain inside the wall"
        );

        let top_material = PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 500.0,
                y: 0.001,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("catalog carbon should realize");
        let top_moved = translated(&top_material, 0.0, -1.0, 1000.0, &catalog).unwrap();
        let top_placement = &top_moved.placements.as_ref().unwrap()[0];
        assert!(top_placement.y - extent >= 0.0);
        assert!(
            top_placement.y > 0.0,
            "the top wall must constrain full element geometry"
        );

        let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        assert!(field.index_for_position(500.0, 1000.0).is_none());
        assert!(field.index_for_position(500.0, -0.001).is_none());
    }

    #[test]
    fn intrinsic_rotation_inertia_scales_with_mass_and_geometry() {
        let unit_line = intrinsic_moment_of_inertia(&Form::Line { length: 1.0 }, 1.0);
        let double_line = intrinsic_moment_of_inertia(&Form::Line { length: 2.0 }, 1.0);
        let double_mass = intrinsic_moment_of_inertia(&Form::Line { length: 1.0 }, 2.0);
        assert!((double_line / unit_line - 4.0).abs() < 1e-12);
        assert!((double_mass / unit_line - 2.0).abs() < 1e-12);
    }

    #[test]
    fn angular_response_is_finite_for_realized_resources_and_compounds() {
        let catalog = crate::resources::default_catalog();
        let resource = PhysicalMaterial::realized(
            crate::resources::Material::free_base("Hydrogen", 1.0),
            vec![crate::structure::Placement {
                x: 25.0,
                y: 40.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("catalog resource should realize");
        let angle = angular_displacement(&resource, &catalog, 100.0, 80.0, &[], 0, 0);
        assert!(angle.is_finite());
        assert!(
            angle.abs() > 1e-12,
            "an anisotropic resource should respond to local flow vorticity"
        );

        let compound = PhysicalMaterial::realized(
            crate::resources::Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                crate::structure::Placement {
                    x: 25.0,
                    y: 40.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 26.0,
                    y: 40.0,
                    rotation_radians: 0.2,
                },
            ],
            &catalog,
        )
        .expect("compound should realize");
        let angle = angular_displacement(&compound, &catalog, 100.0, 80.0, &[], 0, 1);
        assert!(angle.is_finite());
        assert!(
            angle.abs() > 1e-12,
            "a compound spanning a flow gradient should experience angular response"
        );
    }

    #[test]
    fn rotation_preserves_compound_distances_and_bond_state() {
        let catalog = crate::resources::default_catalog();
        let compound = PhysicalMaterial::realized(
            crate::resources::Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                crate::structure::Placement {
                    x: 10.0,
                    y: 10.0,
                    rotation_radians: 0.1,
                },
                crate::structure::Placement {
                    x: 11.0,
                    y: 10.0,
                    rotation_radians: 0.4,
                },
            ],
            &catalog,
        )
        .expect("compound should realize");
        let rotated = rotated(
            &compound,
            std::f64::consts::FRAC_PI_2,
            &catalog,
            100.0,
            100.0,
        )
        .expect("finite rotation should succeed");
        let before = compound.placements.as_ref().unwrap();
        let after = rotated.placements.as_ref().unwrap();
        let before_distance = (before[0].x - before[1].x).hypot(before[0].y - before[1].y);
        let after_distance = (after[0].x - after[1].x).hypot(after[0].y - after[1].y);
        assert!((before_distance - after_distance).abs() < 1e-9);
        assert!(
            (after[0].rotation_radians - before[0].rotation_radians - std::f64::consts::FRAC_PI_2)
                .abs()
                < 1e-12
        );
        assert_eq!(rotated.material, compound.material);
        assert_eq!(rotated.internal_connections, compound.internal_connections);
        assert_eq!(rotated.realized_shapes, compound.realized_shapes);
    }

    #[test]
    fn dense_obstacle_bin_transport_has_a_generous_runtime_ceiling() {
        use std::time::{Duration, Instant};

        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        // Keep all objects inside one fine flow bin so this exercises the
        // intentionally uncapped dense-obstacle path, not the easy sparse case.
        for index in 0..512 {
            let x = 10.0 + (index % 32) as f64 * 0.1;
            let y = 10.0 + (index / 32) as f64 * 0.1;
            let material = PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("catalog carbon should realize");
            assert!(field.deposit_physical(x, y, material, &catalog));
        }

        let before = field.total_material();
        let started = Instant::now();
        let moved = transport_field(&mut field, &catalog, 1000.0, 1000.0);
        let elapsed = started.elapsed();

        assert_eq!(
            field.total_material(),
            before,
            "dense transport must conserve inventory"
        );
        let retained_objects = field
            .cells
            .iter()
            .map(|cell| cell.physical_materials.len())
            .sum::<usize>();
        assert_eq!(
            retained_objects, 512,
            "dense transport must retain every realized object"
        );
        assert!(
            moved > 0,
            "dense obstacles should not immobilize the entire field"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "512-object dense-bin transport took {elapsed:?}; investigate obstacle-query cost"
        );
    }

    #[test]
    fn dense_obstacle_shelters_its_downstream_wake() {
        let obstacle = Obstacle {
            source_cell: 9,
            source_index: 9,
            x: 10.0,
            y: 10.0,
            radius: 3.0,
            solidity: 1.0,
        };
        let free = Velocity { x: 1.0, y: 0.0 };
        let wake = obstacle_adjusted_velocity(free, 15.0, 10.0, 1, 1, &[obstacle], 100.0);
        assert!(wake.length() < free.length());
        let upstream = obstacle_adjusted_velocity(free, 0.0, 10.0, 1, 1, &[obstacle], 100.0);
        assert_eq!(upstream, free);
    }
}
