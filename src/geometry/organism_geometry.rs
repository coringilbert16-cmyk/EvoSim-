//! Aggregate geometry derived from each structural unit's realized physical geometry.
//!
//! `PhysicalGeometry` is the authority for a constituent's current shape.
//! This module only aggregates those realized shapes into organism-level
//! bounds and placed views.

use crate::resources::{BaseResource, Form};
use crate::structure::OrganismStructure;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedForm {
    pub unit_index: usize,
    pub form: Form,
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrganismBodyGeometry {
    pub parts: Vec<PlacedForm>,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

fn point_in_polygon(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
    let (px, py) = point;
    let mut inside = false;
    for i in 0..vertices.len() {
        let (ax, ay) = vertices[i];
        let (bx, by) = vertices[(i + 1) % vertices.len()];
        if (ay > py) != (by > py) {
            let x_at_y = (bx - ax) * (py - ay) / (by - ay) + ax;
            if px < x_at_y {
                inside = !inside;
            }
        }
    }
    inside
}

impl OrganismBodyGeometry {
    pub fn from_structure(structure: &OrganismStructure, catalog: &[BaseResource]) -> Option<Self> {
        if structure.units.is_empty() {
            return None;
        }

        let structural_indices = structure.structural_unit_indices(catalog);
        if structural_indices.is_empty() {
            return None;
        }
        let mut parts = Vec::with_capacity(structural_indices.len());
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );

        for i in structural_indices {
            let unit = &structure.units[i];
            let geometry = unit.geometry.as_ref()?;
            let shape = geometry.shape();
            if !shape.is_valid()
                || !unit.placement.x.is_finite()
                || !unit.placement.y.is_finite()
                || !unit.placement.rotation_radians.is_finite()
            {
                return None;
            }

            let r = shape.form.bounding_radius();
            min_x = min_x.min(unit.placement.x - r);
            max_x = max_x.max(unit.placement.x + r);
            min_y = min_y.min(unit.placement.y - r);
            max_y = max_y.max(unit.placement.y + r);
            parts.push(PlacedForm {
                unit_index: i,
                form: shape.form.clone(),
                x: unit.placement.x,
                y: unit.placement.y,
                rotation_radians: unit.placement.rotation_radians,
            });
        }

        Some(Self {
            parts,
            min_x,
            max_x,
            min_y,
            max_y,
        })
    }

    /// Build aggregate geometry for a physically realized environmental/stored material.
    ///
    /// Unlike a logical material stock, this object has constituent placements and
    /// may cast shadows. Placements are already expressed in world coordinates; the
    /// catalog is used only to validate and resolve each constituent's realized shape.
    #[allow(dead_code)] // Consumed by the forthcoming all-world occluder snapshot.
    pub(crate) fn from_physical_material(
        physical: &crate::physical_material::PhysicalMaterial,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        let placements = physical.placements.as_ref()?;
        if placements.is_empty() || placements.len() != physical.material.parts.len() {
            return None;
        }

        let mut parts = Vec::with_capacity(placements.len());
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for (unit_index, placement) in placements.iter().copied().enumerate() {
            if !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
            {
                return None;
            }
            let shape = physical.realized_shape_for_part(unit_index, catalog)?;
            let radius = shape.form.bounding_radius();
            if !radius.is_finite() || radius < 0.0 {
                return None;
            }
            min_x = min_x.min(placement.x - radius);
            max_x = max_x.max(placement.x + radius);
            min_y = min_y.min(placement.y - radius);
            max_y = max_y.max(placement.y + radius);
            parts.push(PlacedForm {
                unit_index,
                form: shape.form,
                x: placement.x,
                y: placement.y,
                rotation_radians: placement.rotation_radians,
            });
        }

        Some(Self {
            parts,
            min_x,
            max_x,
            min_y,
            max_y,
        })
    }

    /// Test physical containment against the realized constituent geometry.
    ///
    /// The organism boundary is the union of its realized material geometry;
    /// the aggregate bounding box is only a spatial index and is never used as
    /// the physical authority. Boundary points count as contained so that a
    /// constituent touching the organism surface is available at constituent
    /// scale.
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        self.parts.iter().any(|part| {
            form_contains_point(&part.form, part.x, part.y, part.rotation_radians, x, y)
        })
    }

    #[allow(dead_code)]
    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

fn form_contains_point(
    form: &Form,
    origin_x: f64,
    origin_y: f64,
    rotation_radians: f64,
    x: f64,
    y: f64,
) -> bool {
    let dx = x - origin_x;
    let dy = y - origin_y;
    let (sin, cos) = rotation_radians.sin_cos();
    let local_x = dx * cos + dy * sin;
    let local_y = -dx * sin + dy * cos;

    match form {
        Form::Circle { radius } => local_x.hypot(local_y) <= *radius + f64::EPSILON,
        Form::Rectangle { width, height } => {
            local_x.abs() <= *width / 2.0 + f64::EPSILON
                && local_y.abs() <= *height / 2.0 + f64::EPSILON
        }
        Form::RegularPolygon { sides, radius } => polygon_contains_point(
            local_x,
            local_y,
            &Form::RegularPolygon {
                sides: *sides,
                radius: *radius,
            },
        ),
        Form::Polygon { vertices } => polygon_contains_vertices(local_x, local_y, vertices),
        // A line has no interior area. It can form part of the boundary but
        // cannot by itself contain a constituent point.
        Form::Line { .. } => false,
        Form::Fluid {
            nominal_area,
            boundary,
            arc_boundary,
            multi_contact_boundary,
        } => {
            if let Some(multi) = multi_contact_boundary {
                multi.contains_point(local_x, local_y)
            } else if let Some(arc) = arc_boundary {
                let start = arc.start_point();
                let end = arc.end_point();
                let chord_side = (end.0 - start.0) * (local_y - start.1)
                    - (end.1 - start.1) * (local_x - start.0);
                let midpoint_angle = arc.start_angle_radians + arc.sweep_radians * 0.5;
                let midpoint = (
                    arc.center_x + arc.radius * midpoint_angle.cos(),
                    arc.center_y + arc.radius * midpoint_angle.sin(),
                );
                let arc_side = (end.0 - start.0) * (midpoint.1 - start.1)
                    - (end.1 - start.1) * (midpoint.0 - start.0);
                let on_arc_side = chord_side * arc_side >= -f64::EPSILON;
                on_arc_side
                    && (local_x - arc.center_x).hypot(local_y - arc.center_y)
                        <= arc.radius + f64::EPSILON
            } else if let Some(vertices) = boundary {
                point_in_polygon((local_x, local_y), vertices)
            } else {
                local_x.hypot(local_y)
                    <= (nominal_area / std::f64::consts::PI).sqrt() + f64::EPSILON
            }
        }
    }
}

fn polygon_contains_point(x: f64, y: f64, form: &Form) -> bool {
    let Some(vertices) = form.polygon_vertices() else {
        return false;
    };
    polygon_contains_vertices(x, y, &vertices)
}

fn polygon_contains_vertices(x: f64, y: f64, vertices: &[(f64, f64)]) -> bool {
    if vertices.len() < 3 {
        return false;
    }

    let point_on_segment = |ax: f64, ay: f64, bx: f64, by: f64| {
        let cross = (x - ax) * (by - ay) - (y - ay) * (bx - ax);
        if cross.abs() > 1e-10 {
            return false;
        }
        let dot = (x - ax) * (x - bx) + (y - ay) * (y - by);
        dot <= 1e-10
    };

    let mut inside = false;
    for index in 0..vertices.len() {
        let (x1, y1) = vertices[index];
        let (x2, y2) = vertices[(index + 1) % vertices.len()];
        if point_on_segment(x1, y1, x2, y2) {
            return true;
        }
        if (y1 > y) != (y2 > y) && x < (x2 - x1) * (y - y1) / (y2 - y1) + x1 {
            inside = !inside;
        }
    }
    inside
}

/// A visible, source-facing boundary probe on one realized constituent.
///
/// This is a geometric exposure candidate, not an energy allocation: one probe
/// per constituent does not represent the full exposed surface area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SourceFacingSurfacePoint {
    pub body_index: usize,
    pub unit_index: usize,
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}

/// Find one source-facing boundary point per constituent and discard points
/// whose direct path is shadowed by any other realized material. Boundary
/// geometry and normals come from the shared continuous surface query.
#[allow(dead_code)] // Staged exposure query; energy capture is not wired yet.
pub(crate) fn visible_source_facing_points(
    source: (f64, f64),
    bodies: &[OrganismBodyGeometry],
) -> Vec<SourceFacingSurfacePoint> {
    if !source.0.is_finite() || !source.1.is_finite() {
        return Vec::new();
    }

    let mut visible = Vec::new();
    for (body_index, body) in bodies.iter().enumerate() {
        for part in &body.parts {
            let dx = source.0 - part.x;
            let dy = source.1 - part.y;
            let (sin, cos) = part.rotation_radians.sin_cos();
            let local_source_x = dx * cos + dy * sin;
            let local_source_y = -dx * sin + dy * cos;
            let shape = crate::resources::Shape {
                form: part.form.clone(),
            };
            let Some(boundary) = crate::surface_geometry::boundary_point_toward(
                &shape,
                local_source_x,
                local_source_y,
            ) else {
                continue;
            };

            let world_x = part.x + boundary.x * cos - boundary.y * sin;
            let world_y = part.y + boundary.x * sin + boundary.y * cos;
            let normal_x = boundary.normal_x * cos - boundary.normal_y * sin;
            let normal_y = boundary.normal_x * sin + boundary.normal_y * cos;
            let toward_source_x = source.0 - world_x;
            let toward_source_y = source.1 - world_y;

            // A tangent/back-facing surface cannot intercept a directional
            // incoming signal from this source position.
            if normal_x * toward_source_x + normal_y * toward_source_y <= 1e-9 {
                continue;
            }
            if source_path_blocked(
                source,
                (world_x, world_y),
                bodies,
                body_index,
                part.unit_index,
            ) {
                continue;
            }

            visible.push(SourceFacingSurfacePoint {
                body_index,
                unit_index: part.unit_index,
                x: world_x,
                y: world_y,
                normal_x,
                normal_y,
            });
        }
    }
    visible
}

/// Return whether the open segment from the source to a candidate surface point
/// crosses any realized material before reaching that point. Geometry is tested
/// in each constituent's local frame; no raster sampling or cell-size heuristic
/// is used. The candidate constituent itself is excluded because the endpoint
/// lies on its boundary.
#[allow(dead_code)] // Staged geometry primitive; live capture wiring follows surface selection.
pub(crate) fn source_path_blocked(
    source: (f64, f64),
    target: (f64, f64),
    bodies: &[OrganismBodyGeometry],
    target_body_index: usize,
    target_unit_index: usize,
) -> bool {
    if !source.0.is_finite()
        || !source.1.is_finite()
        || !target.0.is_finite()
        || !target.1.is_finite()
        || (target.0 - source.0).hypot(target.1 - source.1) <= f64::EPSILON
    {
        return true;
    }

    for (body_index, body) in bodies.iter().enumerate() {
        for part in &body.parts {
            if body_index == target_body_index && part.unit_index == target_unit_index {
                continue;
            }
            if segment_crosses_part(source, target, part) {
                return true;
            }
        }
    }
    false
}

fn segment_crosses_part(source: (f64, f64), target: (f64, f64), part: &PlacedForm) -> bool {
    let to_local = |point: (f64, f64)| {
        let dx = point.0 - part.x;
        let dy = point.1 - part.y;
        let (sin, cos) = part.rotation_radians.sin_cos();
        (dx * cos + dy * sin, -dx * sin + dy * cos)
    };
    let a = to_local(source);
    let b = to_local(target);
    if form_contains_point(&part.form, 0.0, 0.0, 0.0, a.0, a.1) {
        return true;
    }

    let intersects_segment =
        |c: (f64, f64), d: (f64, f64)| segment_intersection_before_target(a, b, c, d);

    match &part.form {
        Form::Circle { radius } => {
            let dx = b.0 - a.0;
            let dy = b.1 - a.1;
            let quadratic = dx * dx + dy * dy;
            if quadratic <= f64::EPSILON {
                return false;
            }
            let linear = 2.0 * (a.0 * dx + a.1 * dy);
            let constant = a.0 * a.0 + a.1 * a.1 - radius * radius;
            let discriminant = linear * linear - 4.0 * quadratic * constant;
            if !discriminant.is_finite() || discriminant < -1e-9 {
                return false;
            }
            let root = discriminant.max(0.0).sqrt();
            let entry = (-linear - root) / (2.0 * quadratic);
            let exit = (-linear + root) / (2.0 * quadratic);
            [entry, exit]
                .into_iter()
                .any(|t| (-1e-9..(1.0 - 1e-9)).contains(&t))
        }
        Form::Line { length } => intersects_segment((-length * 0.5, 0.0), (length * 0.5, 0.0)),
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
            let Some(vertices) = part.form.polygon_vertices() else {
                return false;
            };
            vertices.iter().enumerate().any(|(index, &start)| {
                intersects_segment(start, vertices[(index + 1) % vertices.len()])
            })
        }
        Form::Fluid {
            nominal_area,
            boundary,
            arc_boundary,
            multi_contact_boundary,
        } => {
            if let Some(vertices) = boundary {
                if vertices.iter().enumerate().any(|(index, &start)| {
                    intersects_segment(start, vertices[(index + 1) % vertices.len()])
                }) {
                    return true;
                }
            }
            if let Some(arc) = arc_boundary {
                if arc
                    .segment_intersections(a.0, a.1, b.0, b.1)
                    .is_some_and(|hits| {
                        hits.into_iter()
                            .any(|point| parameter_before_target(a, b, point))
                    })
                {
                    return true;
                }
            }
            if let Some(multi) = multi_contact_boundary {
                for piece in &multi.pieces {
                    match piece {
                        crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                            start_x,
                            start_y,
                            end_x,
                            end_y,
                            ..
                        } => {
                            if intersects_segment((*start_x, *start_y), (*end_x, *end_y)) {
                                return true;
                            }
                        }
                        crate::capillary_geometry::FluidBoundaryPiece::FreeArc(arc) => {
                            if arc
                                .segment_intersections(a.0, a.1, b.0, b.1)
                                .is_some_and(|hits| {
                                    hits.into_iter()
                                        .any(|point| parameter_before_target(a, b, point))
                                })
                            {
                                return true;
                            }
                        }
                    }
                }
            }
            // Legacy fluid without an explicit boundary uses its nominal-area
            // circular realization, consistent with form containment.
            if boundary.is_none() && arc_boundary.is_none() && multi_contact_boundary.is_none() {
                let radius = (nominal_area / std::f64::consts::PI).sqrt();
                let dx = b.0 - a.0;
                let dy = b.1 - a.1;
                let length_squared = dx * dx + dy * dy;
                if length_squared <= f64::EPSILON {
                    return false;
                }
                let linear = 2.0 * (a.0 * dx + a.1 * dy);
                let constant = a.0 * a.0 + a.1 * a.1 - radius * radius;
                let discriminant = linear * linear - 4.0 * length_squared * constant;
                if !discriminant.is_finite() || discriminant < -1e-9 {
                    return false;
                }
                let root = discriminant.max(0.0).sqrt();
                let entry = (-linear - root) / (2.0 * length_squared);
                let exit = (-linear + root) / (2.0 * length_squared);
                return [entry, exit]
                    .into_iter()
                    .any(|t| (-1e-9..(1.0 - 1e-9)).contains(&t));
            }
            false
        }
    }
}

fn parameter_before_target(a: (f64, f64), b: (f64, f64), point: (f64, f64)) -> bool {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let length_squared = dx * dx + dy * dy;
    length_squared > f64::EPSILON
        && ((point.0 - a.0) * dx + (point.1 - a.1) * dy) / length_squared < 1.0 - 1e-9
}

fn segment_intersection_before_target(
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    d: (f64, f64),
) -> bool {
    let rx = b.0 - a.0;
    let ry = b.1 - a.1;
    let sx = d.0 - c.0;
    let sy = d.1 - c.1;
    let qx = c.0 - a.0;
    let qy = c.1 - a.1;
    let denominator = rx * sy - ry * sx;

    if denominator.abs() <= 1e-12 {
        // Parallel segments only intersect when collinear. Handle that case
        // explicitly: a ray running along a polygon edge is still occluded.
        if (qx * ry - qy * rx).abs() > 1e-9 {
            return false;
        }
        let ray_length_squared = rx * rx + ry * ry;
        if ray_length_squared <= f64::EPSILON {
            return false;
        }
        let tc = (qx * rx + qy * ry) / ray_length_squared;
        let td = ((d.0 - a.0) * rx + (d.1 - a.1) * ry) / ray_length_squared;
        let overlap_start = tc.min(td).max(0.0);
        let overlap_end = tc.max(td).min(1.0);
        return overlap_start <= overlap_end && overlap_start < 1.0 - 1e-9;
    }

    let t = (qx * sy - qy * sx) / denominator;
    let u = (qx * ry - qy * rx) / denominator;
    (-1e-9..=1.0 - 1e-9).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Form;

    fn body(form: Form) -> OrganismBodyGeometry {
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form,
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: -10.0,
            max_x: 10.0,
            min_y: -10.0,
            max_y: 10.0,
        }
    }

    #[test]
    fn physical_material_geometry_preserves_realized_shapes_and_world_placements() {
        let catalog = crate::resources::default_catalog();
        let physical = crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base("Carbon".to_owned(), 1.0),
            vec![crate::structure::Placement {
                x: 12.0,
                y: -4.0,
                rotation_radians: 0.25,
            }],
            &catalog,
        )
        .expect("catalog material should have a valid physical realization");

        let geometry = OrganismBodyGeometry::from_physical_material(&physical, &catalog)
            .expect("realized physical material should expose blocker geometry");
        assert_eq!(geometry.parts.len(), 1);
        assert_eq!(geometry.parts[0].unit_index, 0);
        assert_eq!(geometry.parts[0].x, 12.0);
        assert_eq!(geometry.parts[0].y, -4.0);
        assert_eq!(geometry.parts[0].rotation_radians, 0.25);
        assert_eq!(
            geometry.parts[0].form,
            catalog
                .iter()
                .find(|resource| resource.name == "Carbon")
                .unwrap()
                .shape
                .form
        );
        assert!(geometry.min_x < 12.0 && geometry.max_x > 12.0);
        assert!(geometry.min_y < -4.0 && geometry.max_y > -4.0);
    }

    #[test]
    fn collinear_boundary_blocks_before_target_but_not_at_target_only() {
        assert!(segment_intersection_before_target(
            (-5.0, 0.0),
            (5.0, 0.0),
            (-1.0, 0.0),
            (1.0, 0.0),
        ));
        assert!(!segment_intersection_before_target(
            (-5.0, 0.0),
            (5.0, 0.0),
            (5.0, 0.0),
            (6.0, 0.0),
        ));
    }

    #[test]
    fn visible_source_facing_points_respect_shadowing() {
        let mut blocker = body(Form::Circle { radius: 0.75 });
        blocker.parts[0].x = -3.0;
        let target = body(Form::Circle { radius: 1.0 });

        let visible = visible_source_facing_points((-5.0, 0.0), &[blocker, target]);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].body_index, 0);
        assert!(visible[0].normal_x < -0.99);
        assert!(visible[0].x < -3.0);
    }

    #[test]
    fn visible_source_facing_points_reject_back_facing_geometry() {
        let geometry = body(Form::Rectangle {
            width: 2.0,
            height: 1.0,
        });
        let visible = visible_source_facing_points((5.0, 0.0), &[geometry]);
        assert_eq!(visible.len(), 1);
        assert!(visible[0].normal_x > 0.99);
        assert!(visible[0].x > 0.9);
    }

    #[test]
    fn source_path_detects_an_intervening_circle_without_raster_sampling() {
        let target_body = body(Form::Circle { radius: 1.0 });
        let blocker = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Circle { radius: 1.0 },
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: -1.0,
            max_x: 1.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        assert!(source_path_blocked(
            (-5.0, 0.0),
            (5.0, 0.0),
            &[blocker.clone(), target_body.clone()],
            1,
            0,
        ));
        assert!(!source_path_blocked(
            (-5.0, 4.0),
            (5.0, 4.0),
            &[blocker, target_body],
            1,
            0,
        ));
    }

    #[test]
    fn source_path_ignores_only_the_target_surface_constituent() {
        let target = body(Form::Rectangle {
            width: 2.0,
            height: 1.0,
        });
        assert!(!source_path_blocked(
            (-5.0, 0.0),
            (-1.0, 0.0),
            &[target],
            0,
            0,
        ));
    }

    #[test]
    fn containment_uses_real_circle_geometry_not_bounding_box() {
        let geometry = body(Form::Circle { radius: 2.0 });
        assert!(geometry.contains_point(2.0, 0.0));
        assert!(!geometry.contains_point(1.5, 1.5));
        assert!(geometry.bounding_box_contains(1.5, 1.5));
    }

    #[test]
    fn containment_respects_rotation_for_polygon_geometry() {
        let mut geometry = body(Form::Rectangle {
            width: 4.0,
            height: 1.0,
        });
        geometry.parts[0].rotation_radians = std::f64::consts::FRAC_PI_2;
        assert!(geometry.contains_point(0.0, 1.9));
        assert!(!geometry.contains_point(1.9, 0.0));
    }

    #[test]
    fn containment_is_union_of_realized_parts() {
        let geometry = OrganismBodyGeometry {
            parts: vec![
                PlacedForm {
                    unit_index: 0,
                    form: Form::Circle { radius: 1.0 },
                    x: -1.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                PlacedForm {
                    unit_index: 1,
                    form: Form::Circle { radius: 1.0 },
                    x: 1.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            min_x: -2.0,
            max_x: 2.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        assert!(geometry.contains_point(-1.5, 0.0));
        assert!(geometry.contains_point(1.5, 0.0));
        assert!(!geometry.contains_point(0.0, 1.0));
    }

    #[test]
    fn exact_fluid_arc_contains_only_the_circular_segment() {
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let form = Form::Fluid {
            nominal_area: 0.5,
            boundary: None,
            arc_boundary: Some(arc),
            multi_contact_boundary: None,
        };
        assert!(form_contains_point(
            &form,
            0.0,
            0.0,
            0.0,
            0.0,
            arc.radius * 0.5
        ));
        assert!(!form_contains_point(
            &form,
            0.0,
            0.0,
            0.0,
            0.0,
            -arc.radius * 0.5
        ));
    }

    #[test]
    fn exact_fluid_arc_containment_respects_world_rotation_and_translation() {
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let form = Form::Fluid {
            nominal_area: 0.5,
            boundary: None,
            arc_boundary: Some(arc),
            multi_contact_boundary: None,
        };
        let inside = form_contains_point(
            &form,
            2.0,
            3.0,
            std::f64::consts::FRAC_PI_2,
            2.0 - arc.radius * 0.5,
            3.0,
        );
        let outside = form_contains_point(
            &form,
            2.0,
            3.0,
            std::f64::consts::FRAC_PI_2,
            2.0 + arc.radius * 0.5,
            3.0,
        );
        assert!(inside);
        assert!(!outside);
    }

    #[test]
    fn non_area_line_does_not_create_containment() {
        let geometry = body(Form::Line { length: 4.0 });
        assert!(!geometry.contains_point(0.0, 0.0));
    }
}
