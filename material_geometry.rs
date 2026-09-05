//! Physical geometry for environmental material instances.
//!
//! `Material` remains the authoritative composition + structure identity. This
//! module represents where that material's constituents physically exist in
//! space. It deliberately does not add geometry to `Material` itself, because
//! the same physical material definition may be transferred between systems
//! while its spatial placement is an instance-level fact.

use crate::resources::{BaseResource, Form, Material};
use crate::structure::Placement;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedMaterialPart {
    pub part_index: usize,
    pub form: Form,
    pub placement: Placement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialGeometry {
    pub parts: Vec<PlacedMaterialPart>,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

/// A physical environmental instance: authoritative material identity plus
/// the spatial realization of its constituents.
///
/// This is intentionally distinct from ecological bulk stock. Bulk stock can
/// be aggregated in a field cell without inventing arbitrary constituent
/// positions; a physical instance cannot exist without explicit geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalMaterialInstance {
    pub material: Material,
    pub geometry: MaterialGeometry,
}

impl PhysicalMaterialInstance {
    pub fn new(
        material: Material,
        placements: &[Placement],
        catalog: &[BaseResource],
    ) -> Option<Self> {
        let geometry = MaterialGeometry::new(&material, placements, catalog)?;
        Some(Self { material, geometry })
    }
}

impl MaterialGeometry {
    /// Build physical geometry for a material instance from its constituent
    /// placements and the immutable resource catalog.
    ///
    /// Composition and structure remain owned by `material`; this type only
    /// supplies the spatial realization required for future contact and
    /// boundary calculations.
    pub fn new(
        material: &Material,
        placements: &[Placement],
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if !material.is_valid() || placements.len() != material.parts.len() {
            return None;
        }
        if material.parts.is_empty() {
            return None;
        }

        let mut parts = Vec::with_capacity(material.parts.len());
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for (part_index, ((resource_name, _), placement)) in
            material.parts.iter().zip(placements.iter()).enumerate()
        {
            let resource = catalog.iter().find(|resource| resource.name == *resource_name)?;
            if !resource.shape.is_valid()
                || !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
            {
                return None;
            }

            let radius = resource.shape.form.bounding_radius();
            min_x = min_x.min(placement.x - radius);
            max_x = max_x.max(placement.x + radius);
            min_y = min_y.min(placement.y - radius);
            max_y = max_y.max(placement.y + radius);

            parts.push(PlacedMaterialPart {
                part_index,
                form: resource.shape.form.clone(),
                placement: *placement,
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

    /// Conservative broad-phase test. This is not a shape-level contact test.
    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

/// Whether two placed resource forms physically overlap within a tolerance.
///
/// This is an exact shape-level test for the rigid forms currently represented
/// by the catalog: circles and polygonal forms. `Fluid` deliberately returns
/// `false` because its nominal area is not yet a spatial boundary and must not
/// be turned into an invented circle. The broad-phase bounding radii are only
/// used to reject clearly separated shapes; the final decision uses the actual
/// form geometry.
pub fn placed_forms_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    if !tolerance.is_finite() || !a.placement.x.is_finite() || !a.placement.y.is_finite()
        || !b.placement.x.is_finite() || !b.placement.y.is_finite()
    {
        return false;
    }

    let tolerance = tolerance.max(0.0);
    let center_distance = (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
    if center_distance > a.form.bounding_radius() + b.form.bounding_radius() + tolerance {
        return false;
    }

    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => {
            center_distance <= ar + br + tolerance
        }
        (Form::Circle { radius }, polygon) => circle_polygon_overlap(a, *radius, b, polygon, tolerance),
        (polygon, Form::Circle { radius }) => circle_polygon_overlap(b, *radius, a, polygon, tolerance),
        (Form::Fluid { .. }, _) | (_, Form::Fluid { .. }) => false,
        _ => polygons_overlap(a, b, tolerance),
    }
}

fn circle_polygon_overlap(
    circle: &PlacedMaterialPart,
    radius: f64,
    polygon: &PlacedMaterialPart,
    form: &Form,
    tolerance: f64,
) -> bool {
    let Some(vertices) = world_polygon_vertices(form, polygon.placement) else {
        return false;
    };
    let center = (circle.placement.x, circle.placement.y);
    if point_in_polygon(center, &vertices) {
        return true;
    }

    let expanded_radius = radius + tolerance;
    vertices
        .iter()
        .enumerate()
        .any(|(index, &start)| {
            let end = vertices[(index + 1) % vertices.len()];
            point_segment_distance(center, start, end) <= expanded_radius
        })
}

fn polygons_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    let Some(a_vertices) = world_polygon_vertices(&a.form, a.placement) else {
        return false;
    };
    let Some(b_vertices) = world_polygon_vertices(&b.form, b.placement) else {
        return false;
    };

    let mut axes = polygon_axes(&a_vertices);
    axes.extend(polygon_axes(&b_vertices));
    for (axis_x, axis_y) in axes {
        let (a_min, a_max) = project_polygon(&a_vertices, axis_x, axis_y);
        let (b_min, b_max) = project_polygon(&b_vertices, axis_x, axis_y);
        if a_max + tolerance < b_min || b_max + tolerance < a_min {
            return false;
        }
    }
    true
}

fn world_polygon_vertices(form: &Form, placement: Placement) -> Option<Vec<(f64, f64)>> {
    let vertices = form.polygon_vertices()?;
    let (sin, cos) = placement.rotation_radians.sin_cos();
    Some(
        vertices
            .into_iter()
            .map(|(x, y)| {
                (
                    placement.x + x * cos - y * sin,
                    placement.y + x * sin + y * cos,
                )
            })
            .collect(),
    )
}

fn polygon_axes(vertices: &[(f64, f64)]) -> Vec<(f64, f64)> {
    vertices
        .iter()
        .enumerate()
        .map(|(index, &(x1, y1))| {
            let (x2, y2) = vertices[(index + 1) % vertices.len()];
            let edge_x = x2 - x1;
            let edge_y = y2 - y1;
            let length = edge_x.hypot(edge_y);
            (-edge_y / length, edge_x / length)
        })
        .collect()
}

fn project_polygon(vertices: &[(f64, f64)], axis_x: f64, axis_y: f64) -> (f64, f64) {
    vertices.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min, max), &(x, y)| {
            let projection = x * axis_x + y * axis_y;
            (min.min(projection), max.max(projection))
        },
    )
}

fn point_in_polygon(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
    let (px, py) = point;
    let mut inside = false;
    for index in 0..vertices.len() {
        let (x1, y1) = vertices[index];
        let (x2, y2) = vertices[(index + 1) % vertices.len()];
        let intersects = (y1 > py) != (y2 > py)
            && px < (x2 - x1) * (py - y1) / (y2 - y1) + x1;
        if intersects {
            inside = !inside;
        }
    }
    inside
}

fn point_segment_distance(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let (px, py) = point;
    let (sx, sy) = start;
    let (ex, ey) = end;
    let dx = ex - sx;
    let dy = ey - sy;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return (px - sx).hypot(py - sy);
    }
    let t = (((px - sx) * dx + (py - sy) * dy) / length_squared).clamp(0.0, 1.0);
    (px - (sx + t * dx)).hypot(py - (sy + t * dy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};

    fn part(form: Form, x: f64, y: f64, rotation_radians: f64) -> PlacedMaterialPart {
        PlacedMaterialPart {
            part_index: 0,
            form,
            placement: Placement {
                x,
                y,
                rotation_radians,
            },
        }
    }

    #[test]
    fn geometry_preserves_material_part_identity_and_placement() {
        let catalog = default_catalog();
        let material = Material::free_base("Carbon", 1.0);
        let placements = [Placement {
            x: 12.0,
            y: 8.0,
            rotation_radians: 0.25,
        }];

        let geometry = MaterialGeometry::new(&material, &placements, &catalog).unwrap();
        assert_eq!(geometry.parts.len(), 1);
        assert_eq!(geometry.parts[0].part_index, 0);
        assert_eq!(geometry.parts[0].placement, placements[0]);
        assert!(geometry.bounding_box_contains(12.0, 8.0));
    }

    #[test]
    fn physical_instance_keeps_material_and_geometry_together() {
        let catalog = default_catalog();
        let material = Material::free_base("Carbon", 1.0);
        let placements = [Placement {
            x: 4.0,
            y: 6.0,
            rotation_radians: 0.0,
        }];

        let instance = PhysicalMaterialInstance::new(material.clone(), &placements, &catalog)
            .unwrap();
        assert_eq!(instance.material, material);
        assert_eq!(instance.geometry.parts[0].placement, placements[0]);
    }

    #[test]
    fn structured_material_requires_one_placement_per_constituent() {
        let catalog = default_catalog();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let placements = [Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }];

        assert!(MaterialGeometry::new(&material, &placements, &catalog).is_none());
    }

    #[test]
    fn invalid_geometry_is_rejected() {
        let catalog = default_catalog();
        let material = Material::free_base("Carbon", 1.0);
        let placements = [Placement {
            x: f64::NAN,
            y: 0.0,
            rotation_radians: 0.0,
        }];

        assert!(MaterialGeometry::new(&material, &placements, &catalog).is_none());
    }

    #[test]
    fn overlapping_circles_are_in_contact() {
        let a = part(Form::Circle { radius: 2.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Circle { radius: 2.0 }, 3.0, 0.0, 0.0);
        assert!(placed_forms_overlap(&a, &b, 0.0));
    }

    #[test]
    fn separated_circles_are_not_in_contact() {
        let a = part(Form::Circle { radius: 2.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Circle { radius: 2.0 }, 4.1, 0.0, 0.0);
        assert!(!placed_forms_overlap(&a, &b, 0.0));
    }

    #[test]
    fn rotated_polygons_use_actual_shape_not_bounding_radius() {
        let a = part(
            Form::Rectangle { width: 4.0, height: 1.0 },
            0.0,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );
        let b = part(
            Form::Rectangle { width: 4.0, height: 1.0 },
            3.0,
            3.0,
            0.0,
        );
        assert!(!placed_forms_overlap(&a, &b, 0.0));
    }

    #[test]
    fn polygon_contact_is_detected_when_edges_cross() {
        let a = part(
            Form::Rectangle { width: 4.0, height: 1.0 },
            0.0,
            0.0,
            0.0,
        );
        let b = part(
            Form::Rectangle { width: 1.0, height: 4.0 },
            0.0,
            0.0,
            0.0,
        );
        assert!(placed_forms_overlap(&a, &b, 0.0));
    }

    #[test]
    fn circle_polygon_contact_is_detected_at_the_boundary() {
        let circle = part(Form::Circle { radius: 1.0 }, 2.0, 0.0, 0.0);
        let square = part(
            Form::Rectangle { width: 2.0, height: 2.0 },
            0.0,
            0.0,
            0.0,
        );
        assert!(placed_forms_overlap(&circle, &square, 0.0));
    }

    #[test]
    fn fluid_has_no_invented_spatial_boundary() {
        let fluid = part(Form::Fluid { nominal_area: 100.0 }, 0.0, 0.0, 0.0);
        let circle = part(Form::Circle { radius: 10.0 }, 0.0, 0.0, 0.0);
        assert!(!placed_forms_overlap(&fluid, &circle, 0.0));
    }
}
