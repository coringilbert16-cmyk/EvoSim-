//! Physical geometry for environmental material instances.
use crate::resources::{BaseResource, Form, Material};
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacedMaterialPart {
    pub part_index: usize,
    pub form: Form,
    pub placement: Placement,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialGeometry {
    pub parts: Vec<PlacedMaterialPart>,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
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

    pub fn placements(&self) -> Vec<Placement> {
        self.parts.iter().map(|part| part.placement).collect()
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
    let center_distance = (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
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
    let center_distance = (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
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
        (Form::Line { .. }, _) | (_, Form::Line { .. }) => {
            false
        }
        (Form::Fluid { .. }, _) | (_, Form::Fluid { .. }) => false,
        _ => polygons_penetrate(a, b, tolerance),
    }
}

fn circle_line_overlap(
    circle: &PlacedMaterialPart,
    radius: f64,
    line: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    let length = match line.form {
        Form::Line { length } => length,
        _ => return false,
    };
    let half = length / 2.0;
    let angle = line.placement.rotation_radians;
    let dx = circle.placement.x - line.placement.x;
    let dy = circle.placement.y - line.placement.y;
    let local_x = dx * angle.cos() + dy * angle.sin();
    let local_y = -dx * angle.sin() + dy * angle.cos();
    let clamped_x = local_x.clamp(-half, half);
    (local_x - clamped_x).hypot(local_y) <= radius + tolerance
}

fn circle_line_penetration(
    circle: &PlacedMaterialPart,
    radius: f64,
    line: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    let length = match line.form {
        Form::Line { length } => length,
        _ => return false,
    };
    let half = length / 2.0;
    let angle = line.placement.rotation_radians;
    let dx = circle.placement.x - line.placement.x;
    let dy = circle.placement.y - line.placement.y;
    let local_x = dx * angle.cos() + dy * angle.sin();
    let local_y = -dx * angle.sin() + dy * angle.cos();
    let clamped_x = local_x.clamp(-half, half);
    (local_x - clamped_x).hypot(local_y) < (radius - tolerance).max(0.0)
}

fn line_line_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    let (al, bl) = match (a.form, b.form) {
        (Form::Line { length: al }, Form::Line { length: bl }) => (al, bl),
        _ => return false,
    };
    let angle_a = a.placement.rotation_radians;
    let angle_b = b.placement.rotation_radians;
    let ax = (al / 2.0) * angle_a.cos();
    let ay = (al / 2.0) * angle_a.sin();
    let bx = (bl / 2.0) * angle_b.cos();
    let by = (bl / 2.0) * angle_b.sin();
    segments_distance(
        (a.placement.x - ax, a.placement.y - ay),
        (a.placement.x + ax, a.placement.y + ay),
        (b.placement.x - bx, b.placement.y - by),
        (b.placement.x + bx, b.placement.y + by),
    ) <= tolerance
}

fn line_polygon_overlap(
    line: &PlacedMaterialPart,
    polygon: &PlacedMaterialPart,
    form: &Form,
    tolerance: f64,
) -> bool {
    let line_form = match line.form {
        Form::Line { length } => length,
        _ => return false,
    };
    let line_angle = line.placement.rotation_radians;
    let dx = line_form / 2.0 * line_angle.cos();
    let dy = line_form / 2.0 * line_angle.sin();
    let a = (line.placement.x - dx, line.placement.y - dy);
    let b = (line.placement.x + dx, line.placement.y + dy);
    let vertices = transformed_vertices(form, polygon.placement);
    if vertices.len() < 2 {
        return false;
    }
    if vertices.iter().any(|v| point_segment_distance(*v, a, b) <= tolerance) {
        return true;
    }
    for i in 0..vertices.len() {
        let j = (i + 1) % vertices.len();
        if segments_distance(a, b, vertices[i], vertices[j]) <= tolerance {
            return true;
        }
    }
    point_in_polygon(a, &vertices) || point_in_polygon(b, &vertices)
}

fn circle_polygon_overlap(
    circle: &PlacedMaterialPart,
    radius: f64,
    polygon: &PlacedMaterialPart,
    form: &Form,
    tolerance: f64,
) -> bool {
    let vertices = transformed_vertices(form, polygon.placement);
    if vertices.len() < 3 {
        return false;
    }
    if point_in_polygon((circle.placement.x, circle.placement.y), &vertices) {
        return true;
    }
    for i in 0..vertices.len() {
        let j = (i + 1) % vertices.len();
        if point_segment_distance(
            (circle.placement.x, circle.placement.y),
            vertices[i],
            vertices[j],
        ) <= radius + tolerance
        {
            return true;
        }
    }
    false
}

fn polygons_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    let av = transformed_vertices(&a.form, a.placement);
    let bv = transformed_vertices(&b.form, b.placement);
    if av.len() < 3 || bv.len() < 3 {
        return false;
    }
    for i in 0..av.len() {
        let j = (i + 1) % av.len();
        for k in 0..bv.len() {
            let l = (k + 1) % bv.len();
            if segments_distance(av[i], av[j], bv[k], bv[l]) <= tolerance {
                return true;
            }
        }
    }
    point_in_polygon(av[0], &bv) || point_in_polygon(bv[0], &av)
}

fn circle_polygon_penetration(
    circle: &PlacedMaterialPart,
    radius: f64,
    polygon: &PlacedMaterialPart,
    form: &Form,
    tolerance: f64,
) -> bool {
    let vertices = transformed_vertices(form, polygon.placement);
    if vertices.len() < 3 {
        return false;
    }
    if point_in_polygon((circle.placement.x, circle.placement.y), &vertices) {
        return true;
    }
    for i in 0..vertices.len() {
        let j = (i + 1) % vertices.len();
        if point_segment_distance(
            (circle.placement.x, circle.placement.y),
            vertices[i],
            vertices[j],
        ) < (radius - tolerance).max(0.0)
        {
            return true;
        }
    }
    false
}

fn polygons_penetrate(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    let av = transformed_vertices(&a.form, a.placement);
    let bv = transformed_vertices(&b.form, b.placement);
    if av.len() < 3 || bv.len() < 3 {
        return false;
    }
    point_in_polygon(av[0], &bv) || point_in_polygon(bv[0], &av) ||
        (0..av.len()).any(|i| {
            let j = (i + 1) % av.len();
            (0..bv.len()).any(|k| {
                let l = (k + 1) % bv.len();
                segments_distance(av[i], av[j], bv[k], bv[l]) < tolerance
            })
        })
}

fn transformed_vertices(form: &Form, placement: Placement) -> Vec<(f64, f64)> {
    let Some(vertices) = form.polygon_vertices() else {
        return Vec::new();
    };
    let (sin, cos) = placement.rotation_radians.sin_cos();
    vertices
        .into_iter()
        .map(|(x, y)| {
            (
                placement.x + x * cos - y * sin,
                placement.y + x * sin + y * cos,
            )
        })
        .collect()
}

fn point_in_polygon(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = vertices.len() - 1;
    for i in 0..vertices.len() {
        let (xi, yi) = vertices[i];
        let (xj, yj) = vertices[j];
        if ((yi > point.1) != (yj > point.1))
            && point.0 < (xj - xi) * (point.1 - yi) / (yj - yi + f64::EPSILON) + xi
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn point_segment_distance(point: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return (point.0 - a.0).hypot(point.1 - a.1);
    }
    let t = ((point.0 - a.0) * dx + (point.1 - a.1) * dy) / length_squared;
    let t = t.clamp(0.0, 1.0);
    let projection = (a.0 + t * dx, a.1 + t * dy);
    (point.0 - projection.0).hypot(point.1 - projection.1)
}

fn segments_distance(
    a1: (f64, f64),
    a2: (f64, f64),
    b1: (f64, f64),
    b2: (f64, f64),
) -> f64 {
    if segments_intersect(a1, a2, b1, b2) {
        return 0.0;
    }
    point_segment_distance(a1, b1, b2)
        .min(point_segment_distance(a2, b1, b2))
        .min(point_segment_distance(b1, a1, a2))
        .min(point_segment_distance(b2, a1, a2))
}

fn segments_intersect(
    a1: (f64, f64),
    a2: (f64, f64),
    b1: (f64, f64),
    b2: (f64, f64),
) -> bool {
    fn orientation(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }
    let o1 = orientation(a1, a2, b1);
    let o2 = orientation(a1, a2, b2);
    let o3 = orientation(b1, b2, a1);
    let o4 = orientation(b1, b2, a2);
    (o1 > 0.0 && o2 < 0.0 || o1 < 0.0 && o2 > 0.0)
        && (o3 > 0.0 && o4 < 0.0 || o3 < 0.0 && o4 > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};

    #[test]
    fn geometry_tracks_each_material_part() {
        let c = default_catalog();
        let m = Material {
            parts: vec![("Carbon".into(), 1.0)],
            internal_bonds: Vec::new(),
        };
        let p = [Placement {
            x: 4.0,
            y: 6.0,
            rotation_radians: 0.0,
        }];
        let g = MaterialGeometry::new(&m, &p, &c).unwrap();
        assert_eq!(g.parts.len(), 1);
        assert_eq!(g.parts[0].placement, p[0]);
    }

    #[test]
    fn geometry_preserves_multiple_parts_and_bounds() {
        let c = default_catalog();
        let m = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let p = [
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            Placement {
                x: 3.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ];
        let g = MaterialGeometry::new(&m, &p, &c).unwrap();
        assert_eq!(g.parts.len(), 2);
        assert!(g.min_x < g.max_x);
        assert!(g.bounding_box_contains(0.0, 0.0));
    }

    #[test]
    fn overlap_detects_contact() {
        let a = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 1.0 },
            placement: Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        };
        let b = PlacedMaterialPart {
            part_index: 1,
            form: Form::Circle { radius: 1.0 },
            placement: Placement {
                x: 2.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        };
        assert!(placed_forms_overlap(&a, &b, 0.0));
        assert!(!placed_forms_penetrate(&a, &b, 0.0));
    }
}
