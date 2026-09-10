//! Physical geometry for environmental material instances.
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
        Some(Self {
            material: material.clone(),
            geometry: MaterialGeometry::new(&material, placements, catalog)?,
        })
    }
}

impl MaterialGeometry {
    pub fn new(
        material: &Material,
        placements: &[Placement],
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if !material.is_valid()
            || placements.len() != material.parts.len()
            || material.parts.is_empty()
        {
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
            let resource = catalog
                .iter()
                .find(|resource| resource.name == *resource_name)?;
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

    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

/// Returns true when two forms touch or overlap. Contact at exactly the
/// boundary is intentionally included because this predicate is used for
/// physical contact detection.
pub fn placed_forms_overlap(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    if !tolerance.is_finite()
        || !a.placement.x.is_finite()
        || !a.placement.y.is_finite()
        || !b.placement.x.is_finite()
        || !b.placement.y.is_finite()
    {
        return false;
    }
    let tolerance = tolerance.max(0.0);
    let center_distance =
        (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
    if center_distance > a.form.bounding_radius() + b.form.bounding_radius() + tolerance {
        return false;
    }
    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => {
            center_distance <= ar + br + tolerance
        }
        (Form::Circle { radius }, Form::Line { .. }) => {
            circle_line_overlap(a, *radius, b, tolerance)
        }
        (Form::Line { .. }, Form::Circle { radius }) => {
            circle_line_overlap(b, *radius, a, tolerance)
        }
        (Form::Line { .. }, Form::Line { .. }) => line_line_overlap(a, b, tolerance),
        (Form::Circle { radius }, polygon) => {
            circle_polygon_overlap(a, *radius, b, polygon, tolerance)
        }
        (polygon, Form::Circle { radius }) => {
            circle_polygon_overlap(b, *radius, a, polygon, tolerance)
        }
        (Form::Line { .. }, polygon) => line_polygon_overlap(a, b, polygon, tolerance),
        (polygon, Form::Line { .. }) => line_polygon_overlap(b, a, polygon, tolerance),
        (Form::Fluid { .. }, _) | (_, Form::Fluid { .. }) => false,
        _ => polygons_overlap(a, b, tolerance),
    }
}

/// Returns true only when two finite-area forms penetrate one another.
/// Merely touching at a boundary is not penetration. This is deliberately
/// separate from `placed_forms_overlap`, whose inclusive semantics represent
/// physical contact.
pub fn placed_forms_penetrate(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    if !tolerance.is_finite()
        || !a.placement.x.is_finite()
        || !a.placement.y.is_finite()
        || !b.placement.x.is_finite()
        || !b.placement.y.is_finite()
    {
        return false;
    }
    let tolerance = tolerance.max(0.0);
    let center_distance =
        (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
    if center_distance >= a.form.bounding_radius() + b.form.bounding_radius() + tolerance {
        return false;
    }
    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => {
            center_distance < ar + br - tolerance
        }
        (Form::Circle { radius }, Form::Line { .. }) => {
            circle_line_penetration(a, *radius, b, tolerance)
        }
        (Form::Line { .. }, Form::Circle { radius }) => {
            circle_line_penetration(b, *radius, a, tolerance)
        }
        (Form::Line { .. }, Form::Line { .. }) => false,
        (Form::Circle { radius }, polygon) => {
            circle_polygon_penetration(a, *radius, b, polygon, tolerance)
        }
        (polygon, Form::Circle { radius }) => {
            circle_polygon_penetration(b, *radius, a, polygon, tolerance)
        }
        (Form::Line { .. }, polygon) | (polygon, Form::Line { .. }) => {
            // Lines have no area, so crossing/contact is not a finite-area
            // penetration. Their existing contact predicate handles them.
            false
        }
        (Form::Fluid { .. }, _) | (_, Form::Fluid { .. }) => false,
        _ => polygons_penetrate(a, b, tolerance),
    }
}

fn line_segment(
    form: &Form,
    placement: Placement,
) -> Option<((f64, f64), (f64, f64))> {
    let Form::Line { length } = form else {
        return None;
    };
    let h = *length / 2.0;
    let (s, c) = placement.rotation_radians.sin_cos();
    Some((
        (placement.x - h * c, placement.y - h * s),
        (placement.x + h * c, placement.y + h * s),
    ))
}

fn circle_line_overlap(
    circle: &PlacedMaterialPart,
    radius: f64,
    line: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    let Some((a, b)) = line_segment(&line.form, line.placement) else {
        return false;
    };
    point_segment_distance((circle.placement.x, circle.placement.y), a, b) <= radius + tolerance
}

fn circle_line_penetration(
    circle: &PlacedMaterialPart,
    radius: f64,
    line: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    let Some((a, b)) = line_segment(&line.form, line.placement) else {
        return false;
    };
    point_segment_distance((circle.placement.x, circle.placement.y), a, b)
        < (radius - tolerance).max(0.0)
}

fn line_line_overlap(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    let Some((a0, a1)) = line_segment(&a.form, a.placement) else {
        return false;
    };
    let Some((b0, b1)) = line_segment(&b.form, b.placement) else {
        return false;
    };
    segments_distance(a0, a1, b0, b1) <= tolerance
}

fn line_polygon_overlap(
    line: &PlacedMaterialPart,
    polygon: &PlacedMaterialPart,
    form: &Form,
    tolerance: f64,
) -> bool {
    let Some((start, end)) = line_segment(&line.form, line.placement) else {
        return false;
    };
    let Some(vertices) = world_polygon_vertices(form, polygon.placement) else {
        return false;
    };
    if point_in_polygon(start, &vertices) || point_in_polygon(end, &vertices) {
        return true;
    }
    vertices.iter().enumerate().any(|(i, &edge_start)| {
        let edge_end = vertices[(i + 1) % vertices.len()];
        segments_distance(start, end, edge_start, edge_end) <= tolerance
    })
}

fn segments_distance(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> f64 {
    if segments_intersect(a0, a1, b0, b1) {
        return 0.0;
    }
    point_segment_distance(a0, b0, b1)
        .min(point_segment_distance(a1, b0, b1))
        .min(point_segment_distance(b0, a0, a1))
        .min(point_segment_distance(b1, a0, a1))
}

fn orientation(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn segments_intersect(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> bool {
    let eps = 1e-12;
    let o1 = orientation(a0, a1, b0);
    let o2 = orientation(a0, a1, b1);
    let o3 = orientation(b0, b1, a0);
    let o4 = orientation(b0, b1, a1);
    if o1.abs() <= eps && point_segment_distance(b0, a0, a1) <= eps {
        return true;
    }
    if o2.abs() <= eps && point_segment_distance(b1, a0, a1) <= eps {
        return true;
    }
    if o3.abs() <= eps && point_segment_distance(a0, b0, b1) <= eps {
        return true;
    }
    if o4.abs() <= eps && point_segment_distance(a1, b0, b1) <= eps {
        return true;
    }
    (o1 > 0.0 && o2 < 0.0 || o1 < 0.0 && o2 > 0.0)
        && (o3 > 0.0 && o4 < 0.0 || o3 < 0.0 && o4 > 0.0)
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
    vertices.iter().enumerate().any(|(index, &start)| {
        let end = vertices[(index + 1) % vertices.len()];
        point_segment_distance(center, start, end) <= expanded_radius
    })
}

fn circle_polygon_penetration(
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
    let effective_radius = (radius - tolerance).max(0.0);
    vertices.iter().enumerate().any(|(index, &start)| {
        let end = vertices[(index + 1) % vertices.len()];
        point_segment_distance(center, start, end) < effective_radius
    })
}

fn polygons_overlap(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
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

fn polygons_penetrate(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
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
        if a_max - tolerance <= b_min || b_max - tolerance <= a_min {
            return false;
        }
    }
    true
}

fn world_polygon_vertices(
    form: &Form,
    placement: Placement,
) -> Option<Vec<(f64, f64)>> {
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

fn project_polygon(
    vertices: &[(f64, f64)],
    axis_x: f64,
    axis_y: f64,
) -> (f64, f64) {
    vertices
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &(x, y)| {
            let p = x * axis_x + y * axis_y;
            (min.min(p), max.max(p))
        })
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

fn point_segment_distance(
    point: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> f64 {
    let (px, py) = point;
    let (sx, sy) = start;
    let (ex, ey) = end;
    let dx = ex - sx;
    let dy = ey - sy;
    let ls = dx * dx + dy * dy;
    if ls <= f64::EPSILON {
        return (px - sx).hypot(py - sy);
    }
    let t = (((px - sx) * dx + (py - sy) * dy) / ls).clamp(0.0, 1.0);
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
        let c = default_catalog();
        let m = Material::free_base("Carbon", 1.0);
        let p = [Placement {
            x: 12.0,
            y: 8.0,
            rotation_radians: 0.25,
        }];
        let g = MaterialGeometry::new(&m, &p, &c).unwrap();
        assert_eq!(g.parts.len(), 1);
        assert_eq!(g.parts[0].part_index, 0);
        assert_eq!(g.parts[0].placement, p[0]);
        assert!(g.bounding_box_contains(12.0, 8.0));
    }

    #[test]
    fn physical_instance_keeps_material_and_geometry_together() {
        let c = default_catalog();
        let m = Material::free_base("Carbon", 1.0);
        let p = [Placement {
            x: 4.0,
            y: 6.0,
            rotation_radians: 0.0,
        }];
        let i = PhysicalMaterialInstance::new(m.clone(), &p, &c).unwrap();
        assert_eq!(i.material, m);
        assert_eq!(i.geometry.parts[0].placement, p[0]);
    }

    #[test]
    fn structured_material_requires_one_placement_per_constituent() {
        let c = default_catalog();
        let m = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let p = [Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        }];
        assert!(MaterialGeometry::new(&m, &p, &c).is_none());
    }

    #[test]
    fn invalid_geometry_is_rejected() {
        let c = default_catalog();
        let m = Material::free_base("Carbon", 1.0);
        let p = [Placement {
            x: f64::NAN,
            y: 0.0,
            rotation_radians: 0.0,
        }];
        assert!(MaterialGeometry::new(&m, &p, &c).is_none());
    }

    #[test]
    fn touching_circles_are_contact_but_not_penetration() {
        let a = part(Form::Circle { radius: 2.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Circle { radius: 2.0 }, 4.0, 0.0, 0.0);
        assert!(placed_forms_overlap(&a, &b, 0.0));
        assert!(!placed_forms_penetrate(&a, &b, 0.0));
    }

    #[test]
    fn overlapping_circles_are_in_contact_and_penetration() {
        let a = part(Form::Circle { radius: 2.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Circle { radius: 2.0 }, 3.0, 0.0, 0.0);
        assert!(placed_forms_overlap(&a, &b, 0.0));
        assert!(placed_forms_penetrate(&a, &b, 0.0));
    }

    #[test]
    fn separated_circles_are_neither_contact_nor_penetration() {
        let a = part(Form::Circle { radius: 2.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Circle { radius: 2.0 }, 4.1, 0.0, 0.0);
        assert!(!placed_forms_overlap(&a, &b, 0.0));
        assert!(!placed_forms_penetrate(&a, &b, 0.0));
    }

    #[test]
    fn rotated_polygons_use_actual_shape_not_bounding_radius() {
        let a = part(
            Form::Rectangle {
                width: 4.0,
                height: 1.0,
            },
            0.0,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );
        let b = part(
            Form::Rectangle {
                width: 4.0,
                height: 1.0,
            },
            3.0,
            3.0,
            0.0,
        );
        assert!(!placed_forms_overlap(&a, &b, 0.0));
    }

    #[test]
    fn polygon_contact_is_detected_when_edges_cross() {
        let a = part(
            Form::Rectangle {
                width: 4.0,
                height: 1.0,
            },
            0.0,
            0.0,
            0.0,
        );
        let b = part(
            Form::Rectangle {
                width: 1.0,
                height: 4.0,
            },
            0.0,
            0.0,
            0.0,
        );
        assert!(placed_forms_overlap(&a, &b, 0.0));
        assert!(placed_forms_penetrate(&a, &b, 0.0));
    }

    #[test]
    fn circle_polygon_contact_is_detected_at_the_boundary() {
        let circle = part(Form::Circle { radius: 1.0 }, 2.0, 0.0, 0.0);
        let square = part(
            Form::Rectangle {
                width: 2.0,
                height: 2.0,
            },
            0.0,
            0.0,
            0.0,
        );
        assert!(placed_forms_overlap(&circle, &square, 0.0));
        assert!(!placed_forms_penetrate(&circle, &square, 0.0));
    }

    #[test]
    fn line_has_physical_endpoints() {
        let line = part(Form::Line { length: 4.0 }, 0.0, 0.0, 0.0);
        let circle = part(Form::Circle { radius: 0.5 }, 2.6, 0.0, 0.0);
        assert!(!placed_forms_overlap(&line, &circle, 0.0));
        assert!(placed_forms_overlap(
            &line,
            &part(Form::Circle { radius: 0.5 }, 2.1, 0.0, 0.0),
            0.0
        ));
    }

    #[test]
    fn fluid_has_no_invented_spatial_boundary() {
        let fluid = part(Form::Fluid { nominal_area: 100.0 }, 0.0, 0.0, 0.0);
        let circle = part(Form::Circle { radius: 10.0 }, 0.0, 0.0, 0.0);
        assert!(!placed_forms_overlap(&fluid, &circle, 0.0));
        assert!(!placed_forms_penetrate(&fluid, &circle, 0.0));
    }
}
