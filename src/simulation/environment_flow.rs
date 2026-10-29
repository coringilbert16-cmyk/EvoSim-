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
    let anchor = placements.first()?;
    if !height.is_finite()
        || height <= 0.0
        || placements
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return None;
    }
    let count = placements.len() as f64;
    let x = placements.iter().map(|p| p.x).sum::<f64>() / count;
    // Average vertical offsets relative to one constituent so composites
    // crossing the wrapped top/bottom seam keep a compact center.
    let y_offset = placements
        .iter()
        .map(|p| (p.y - anchor.y + height * 0.5).rem_euclid(height) - height * 0.5)
        .sum::<f64>()
        / count;
    Some((x, (anchor.y + y_offset).rem_euclid(height)))
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
    let phase = std::f64::consts::TAU * x / width;
    let center_y = height * 0.5 + height * 0.23 * phase.sin();
    let slope = height * 0.23 * std::f64::consts::TAU / width * phase.cos();
    let vertical_delta = (y - center_y + height * 0.5).rem_euclid(height) - height * 0.5;
    let width_scale = (height * 0.16).max(1.0);
    let envelope = (-0.5 * (vertical_delta / width_scale).powi(2)).exp();
    let tangent = Velocity { x: 1.0, y: slope }.normalized();
    let mut velocity = tangent.scale(0.12 + 0.88 * envelope);

    // Localized counter-flow is a real vector-field vortex, not an arbitrary
    // per-particle reversal. Its sign alternates across the two S bends.
    let vortex_radius = (height * 0.13).max(1.0);
    for (fraction, sign) in [(0.25, 1.0), (0.75, -1.0)] {
        let cx = width * fraction;
        let cy = height * 0.5 + height * 0.23 * (std::f64::consts::TAU * fraction).sin();
        let dx = x - cx;
        let dy = (y - cy + height * 0.5).rem_euclid(height) - height * 0.5;
        let radial_sq = dx * dx + dy * dy;
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
    height: f64,
) -> Velocity {
    for obstacle in obstacles {
        if obstacle.source_cell == source_cell && obstacle.source_index == source_index {
            continue;
        }
        if obstacle.solidity <= 1e-6 {
            continue;
        }
        let dx = x - obstacle.x;
        let dy = (y - obstacle.y + height * 0.5).rem_euclid(height) - height * 0.5;
        let distance = dx.hypot(dy).max(1e-9);
        let radius = obstacle.radius.max(0.5);
        // Dense geometry pushes nearby flow around its sides.
        if distance < radius * 2.5 {
            let influence = obstacle.solidity * (1.0 - distance / (radius * 2.5)).powi(2);
            let side = if dy >= 0.0 { 1.0 } else { -1.0 };
            velocity = velocity.add(Velocity {
                x: -dx.signum() * influence * 0.22,
                y: side * influence * 0.65,
            });
        }
        // Downstream wake is sheltered and slower; upstream flow is unchanged.
        if dx > 0.0 && dx < radius * 6.0 && dy.abs() < radius * 2.0 {
            let longitudinal = 1.0 - dx / (radius * 6.0);
            let lateral = 1.0 - dy.abs() / (radius * 2.0);
            let attenuation = (obstacle.solidity * longitudinal * lateral * 0.72).clamp(0.0, 0.85);
            velocity = velocity.scale(1.0 - attenuation);
        }
    }
    velocity
}

fn clamp_horizontal_displacement(
    material: &PhysicalMaterial,
    requested_dx: f64,
    width: f64,
) -> Option<f64> {
    let placements = material.placements.as_ref()?;
    if placements.is_empty() || !width.is_finite() || width <= 0.0 {
        return None;
    }
    let min_x = placements.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = placements.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let world_max = (width - 1e-6).max(0.0);
    let min_dx = -min_x;
    let max_dx = world_max - max_x;
    if min_dx <= max_dx {
        Some(requested_dx.clamp(min_dx, max_dx))
    } else {
        // If an object is wider than the world, preserve it as a unit and keep
        // its anchor in range; never split the object to satisfy the boundary.
        let anchor_x = placements[0].x;
        Some((anchor_x + requested_dx).clamp(0.0, world_max) - anchor_x)
    }
}

fn translated(
    material: &PhysicalMaterial,
    dx: f64,
    dy: f64,
    height: f64,
) -> Option<PhysicalMaterial> {
    let mut moved = material.clone();
    let placements = moved.placements.as_mut()?;
    if placements.is_empty() {
        return None;
    }
    for placement in placements {
        placement.x += dx;
        placement.y = (placement.y + dy).rem_euclid(height);
        if !placement.x.is_finite()
            || !placement.y.is_finite()
            || !placement.rotation_radians.is_finite()
        {
            return None;
        }
    }
    Some(moved)
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
        let min_col = ((obstacle.x - radius * 2.5).max(0.0) / FLOW_BIN_SIZE).floor() as usize;
        let max_col =
            ((obstacle.x + radius * 6.0).min(width - 1e-6) / FLOW_BIN_SIZE).floor() as usize;
        let max_col = max_col.min(flow_cols.saturating_sub(1));
        if min_col > max_col {
            continue;
        }
        let min_row = ((obstacle.y - radius * 2.5) / FLOW_BIN_SIZE).floor() as isize;
        let max_row = ((obstacle.y + radius * 2.5) / FLOW_BIN_SIZE).floor() as isize;
        let row_count = max_row.saturating_sub(min_row).saturating_add(1) as usize;
        let rows = if row_count >= flow_rows {
            (0..flow_rows).collect::<Vec<_>>()
        } else {
            (0..row_count)
                .map(|offset| (min_row + offset as isize).rem_euclid(flow_rows as isize) as usize)
                .collect::<Vec<_>>()
        };
        for row in rows {
            for col in min_col..=max_col {
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
        let flow_col = ((x / FLOW_BIN_SIZE).floor() as usize).min(flow_cols.saturating_sub(1));
        let flow_row = ((y / FLOW_BIN_SIZE).floor() as usize).min(flow_rows.saturating_sub(1));
        let nearby = &obstacle_bins[flow_row * flow_cols + flow_col];
        let velocity = obstacle_adjusted_velocity(
            background_velocity(x, y, width, height),
            x,
            y,
            source_cell,
            source_index,
            nearby,
            height,
        );
        let scale = mobility(mass) * 0.15;
        let mut dx = velocity.x * scale;
        let dy = velocity.y * scale;
        let Some(clamped_dx) = clamp_horizontal_displacement(&material, dx, width) else {
            continue;
        };
        dx = clamped_dx;
        if dx.abs() + dy.abs() <= 1e-12 {
            continue;
        }
        let Some(moved) = translated(&material, dx, dy, height) else {
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
        let counter_flow =
            background_velocity(25.0, 40.0 + 80.0 * 0.23 + 1.5 * (80.0 * 0.13), 100.0, 80.0);
        assert!(
            counter_flow.x < 0.0,
            "the eddy must create a local counter-flow pocket"
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
                x: 24.99,
                y: 200.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("catalog carbon should realize");
        let original = material.clone();
        assert!(field.deposit_physical(24.99, 200.0, material, &catalog));
        let inventory_before = field.total_material();
        let revision_before = field.revision;
        let source_cell = field.index_for_position(24.99, 200.0).unwrap();
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
        let expected = translated(
            &original,
            velocity.x * displacement_scale,
            velocity.y * displacement_scale,
            1000.0,
        )
        .unwrap();
        assert_eq!(
            objects[0], &expected,
            "transport must change only world-space placements, preserving the complete realized material"
        );
    }

    #[test]
    fn horizontal_boundary_keeps_every_constituent_inside_world() {
        let catalog = crate::resources::default_catalog();
        let material = PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 2.0),
            vec![
                crate::structure::Placement {
                    x: 999.8,
                    y: 500.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 999.9,
                    y: 500.0,
                    rotation_radians: 0.25,
                },
            ],
            &catalog,
        )
        .expect("two-atom carbon material should realize");

        let dx = clamp_horizontal_displacement(&material, 0.15, 1000.0).unwrap();
        let moved = translated(&material, dx, 0.0, 1000.0).unwrap();
        let placements = moved.placements.as_ref().unwrap();
        assert!(placements.iter().all(|p| p.x >= 0.0 && p.x < 1000.0));
        assert!(
            ((placements[1].x - placements[0].x) - 0.1).abs() < 1e-9,
            "boundary handling must preserve the object's internal geometry"
        );
    }

    #[test]
    fn vertical_boundary_wraps_without_losing_material() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let material = PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 0.1,
                y: 999.99,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("catalog carbon should realize");
        assert!(field.deposit_physical(0.1, 999.99, material, &catalog));
        let before = field.total_material();

        assert_eq!(transport_field(&mut field, &catalog, 1000.0, 1000.0), 1);
        assert_eq!(field.total_material(), before);
        let objects: Vec<_> = field.cells.iter().flat_map(|c| c.physical_materials.iter()).collect();
        assert_eq!(objects.len(), 1);
        let placement = &objects[0].placements.as_ref().unwrap()[0];
        assert!((0.0..1000.0).contains(&placement.y));
        assert!(placement.y < 1.0, "positive vertical motion should wrap across the seam");
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
