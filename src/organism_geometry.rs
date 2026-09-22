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

impl OrganismBodyGeometry {
    pub fn from_structure(
        structure: &OrganismStructure,
        _catalog: &[BaseResource],
    ) -> Option<Self> {
        if structure.units.is_empty() {
            return None;
        }

        let mut parts = Vec::with_capacity(structure.units.len());
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );

        for (i, unit) in structure.units.iter().enumerate() {
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
        Form::Fluid { nominal_area } => {
            local_x.hypot(local_y) <= (nominal_area / std::f64::consts::PI).sqrt() + f64::EPSILON
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
    fn non_area_line_does_not_create_containment() {
        let geometry = body(Form::Line { length: 4.0 });
        assert!(!geometry.contains_point(0.0, 0.0));
    }
}
