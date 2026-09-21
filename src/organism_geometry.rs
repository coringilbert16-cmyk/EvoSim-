//! Aggregate geometry derived from each structural unit's realized physical geometry.
//!
//! `PhysicalGeometry` is the authority for a constituent's current shape.
//! This module only aggregates those realized shapes into organism-level
//! bounds and placed views.

use crate::resources::{BaseResource, Form};
use crate::structure::{OrganismStructure, Placement};

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

    #[allow(dead_code)]
    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    /// Tests a point against the realized constituent geometry.
    /// Returns the realized organism bounds' width and height.
    pub fn extent(&self) -> (f64, f64) {
        (self.max_x - self.min_x, self.max_y - self.min_y)
    }

    /// Returns the largest realized linear extent of the organism.
    pub fn maximum_extent(&self) -> f64 {
        let (width, height) = self.extent();
        width.max(height)
    }

    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        self.parts.iter().any(|part| {
            form_contains_point(
                &part.form,
                Placement {
                    x: part.x,
                    y: part.y,
                    rotation_radians: part.rotation_radians,
                },
                x,
                y,
            )
        })
    }
}

fn form_contains_point(form: &Form, placement: Placement, x: f64, y: f64) -> bool {
    let dx = x - placement.x;
    let dy = y - placement.y;
    let (sin, cos) = placement.rotation_radians.sin_cos();
    let local_x = dx * cos + dy * sin;
    let local_y = -dx * sin + dy * cos;

    match form {
        Form::Circle { radius } => local_x.hypot(local_y) <= *radius + 1e-12,
        Form::Rectangle { width, height } => {
            local_x.abs() <= width / 2.0 + 1e-12 && local_y.abs() <= height / 2.0 + 1e-12
        }
        Form::RegularPolygon { .. } | Form::Polygon { .. } => {
            let Some(vertices) = form.polygon_vertices() else {
                return false;
            };
            point_in_polygon((local_x, local_y), &vertices)
        }
        Form::Line { length } => local_x.abs() <= length / 2.0 + 1e-12 && local_y.abs() <= 1e-12,
        Form::Fluid { .. } => false,
    }
}

fn point_in_polygon(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    let (px, py) = point;
    let mut inside = false;
    for i in 0..vertices.len() {
        let (x1, y1) = vertices[i];
        let (x2, y2) = vertices[(i + 1) % vertices.len()];
        let cross = (x2 - x1) * (py - y1) - (y2 - y1) * (px - x1);
        if cross.abs() <= 1e-12
            && px >= x1.min(x2) - 1e-12
            && px <= x1.max(x2) + 1e-12
            && py >= y1.min(y2) - 1e-12
            && py <= y1.max(y2) + 1e-12
        {
            return true;
        }
        if (y1 > py) != (y2 > py) && px < (x2 - x1) * (py - y1) / (y2 - y1) + x1 {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_geometry::PhysicalGeometry;
    use crate::resources::{default_catalog, Shape};
    use crate::structure::{Placement, StructuralUnit};

    fn realized_unit(name: &str, x: f64, y: f64) -> StructuralUnit {
        let mut unit = StructuralUnit::new(
            name,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        );
        let shape = default_catalog()
            .into_iter()
            .find(|resource| resource.name == name)
            .unwrap()
            .shape;
        unit.geometry = Some(PhysicalGeometry::from_default(&shape));
        unit
    }

    #[test]
    fn body_geometry_uses_realized_unit_geometry() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(realized_unit("Carbon", 10.0, 20.0));
        structure.add_unit(realized_unit("Hydrogen", 30.0, 20.0));
        let body = OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap();
        assert_eq!(body.parts.len(), 2);
        assert!(body.max_x > body.min_x);
    }

    #[test]
    fn missing_realized_geometry_is_rejected() {
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        assert!(OrganismBodyGeometry::from_structure(&structure, &default_catalog()).is_none());
    }

    #[test]
    fn realized_geometry_can_differ_from_catalog_default() {
        let mut structure = OrganismStructure::new();
        let mut unit = realized_unit("Carbon", 0.0, 0.0);
        unit.geometry = Some(PhysicalGeometry::from_default(&Shape {
            form: Form::Circle { radius: 7.0 },
        }));
        structure.add_unit(unit);
        let body = OrganismBodyGeometry::from_structure(&structure, &default_catalog()).unwrap();
        assert_eq!(body.parts[0].form, Form::Circle { radius: 7.0 });
        assert_eq!(body.max_x, 7.0);
    }

    #[test]
    fn realized_geometry_contains_points_inside_its_actual_shape() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(realized_unit("Carbon", 10.0, 20.0));
        let body = OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap();
        assert!(body.contains_point(10.0, 20.0));
        assert!(!body.contains_point(100.0, 100.0));
    }

    #[test]
    fn maximum_extent_uses_the_larger_realized_axis() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(realized_unit("Carbon", 0.0, 0.0));
        structure.add_unit(realized_unit("Carbon", 10.0, 0.0));
        let body = OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap();
        let (width, height) = body.extent();
        assert!(width > height);
        assert_eq!(body.maximum_extent(), width);
    }

    #[test]
    fn empty_structure_has_no_body() {
        assert!(OrganismBodyGeometry::from_structure(
            &OrganismStructure::new(),
            &default_catalog()
        )
        .is_none());
    }
}
