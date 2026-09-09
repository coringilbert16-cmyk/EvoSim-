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
    pub fn new(material: Material, placements: &[Placement], catalog: &[BaseResource]) -> Option<Self> {
        Some(Self { material: material.clone(), geometry: MaterialGeometry::new(&material, placements, catalog)? })
    }
}

impl MaterialGeometry {
    pub fn new(material: &Material, placements: &[Placement], catalog: &[BaseResource]) -> Option<Self> {
        if !material.is_valid() || material.parts.is_empty() || placements.len() != material.parts.len() { return None; }
        let mut parts = Vec::with_capacity(material.parts.len());
        let mut min_x = f64::INFINITY; let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY; let mut max_y = f64::NEG_INFINITY;
        for (part_index, ((resource_name, _), placement)) in material.parts.iter().zip(placements.iter()).enumerate() {
            let resource = catalog.iter().find(|resource| resource.name == *resource_name)?;
            if !resource.shape.is_valid() || !placement.x.is_finite() || !placement.y.is_finite() || !placement.rotation_radians.is_finite() { return None; }
            let radius = resource.shape.form.bounding_radius();
            min_x = min_x.min(placement.x - radius); max_x = max_x.max(placement.x + radius);
            min_y = min_y.min(placement.y - radius); max_y = max_y.max(placement.y + radius);
            parts.push(PlacedMaterialPart { part_index, form: resource.shape.form.clone(), placement: *placement });
        }
        Some(Self { parts, min_x, max_x, min_y, max_y })
    }

    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

/// Exact rigid-shape overlap for the explicit catalog geometry.
pub fn placed_forms_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    if !tolerance.is_finite() || !a.placement.x.is_finite() || !a.placement.y.is_finite() || !b.placement.x.is_finite() || !b.placement.y.is_finite() { return false; }
    let tolerance = tolerance.max(0.0);
    let center_distance = (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
    if center_distance > a.form.bounding_radius() + b.form.bounding_radius() + tolerance { return false; }
    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => center_distance <= ar + br + tolerance,
        (Form::Circle { radius }, polygon) => circle_polygon_overlap(a, *radius, b, polygon, tolerance),
        (polygon, Form::Circle { radius }) => circle_polygon_overlap(b, *radius, a, polygon, tolerance),
        _ => polygons_overlap(a, b, tolerance),
    }
}

fn circle_polygon_overlap(circle: &PlacedMaterialPart, radius: f64, polygon: &PlacedMaterialPart, form: &Form, tolerance: f64) -> bool {
    let Some(vertices) = world_polygon_vertices(form, polygon.placement) else { return false };
    let center = (circle.placement.x, circle.placement.y);
    if point_in_polygon(center, &vertices) { return true; }
    let expanded_radius = radius + tolerance;
    vertices.iter().enumerate().any(|(index, &start)| {
        let end = vertices[(index + 1) % vertices.len()];
        point_segment_distance(center, start, end) <= expanded_radius
    })
}

fn polygons_overlap(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> bool {
    let Some(a_vertices) = world_polygon_vertices(&a.form, a.placement) else { return false };
    let Some(b_vertices) = world_polygon_vertices(&b.form, b.placement) else { return false };
    let mut axes = polygon_axes(&a_vertices); axes.extend(polygon_axes(&b_vertices));
    axes.into_iter().all(|axis| {
        let (a_min, a_max) = project_polygon(&a_vertices, axis.0, axis.1);
        let (b_min, b_max) = project_polygon(&b_vertices, axis.0, axis.1);
        !(a_max + tolerance < b_min || b_max + tolerance < a_min)
    })
}

fn world_polygon_vertices(form: &Form, placement: Placement) -> Option<Vec<(f64, f64)>> {
    let vertices = form.polygon_vertices()?;
    let (sin, cos) = placement.rotation_radians.sin_cos();
    Some(vertices.into_iter().map(|(x, y)| (placement.x + x * cos - y * sin, placement.y + x * sin + y * cos)).collect())
}

fn polygon_axes(vertices: &[(f64, f64)]) -> Vec<(f64, f64)> {
    vertices.iter().enumerate().map(|(index, &(x1, y1))| {
        let (x2, y2) = vertices[(index + 1) % vertices.len()];
        let edge_x = x2 - x1; let edge_y = y2 - y1; let length = edge_x.hypot(edge_y);
        (-edge_y / length, edge_x / length)
    }).collect()
}

fn project_polygon(vertices: &[(f64, f64)], axis_x: f64, axis_y: f64) -> (f64, f64) {
    vertices.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &(x, y)| {
        let projection = x * axis_x + y * axis_y; (min.min(projection), max.max(projection))
    })
}

fn point_in_polygon(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
    let mut inside = false;
    for index in 0..vertices.len() {
        let (x1, y1) = vertices[index]; let (x2, y2) = vertices[(index + 1) % vertices.len()];
        if (y1 > point.1) != (y2 > point.1) && point.0 < (x2 - x1) * (point.1 - y1) / (y2 - y1) + x1 { inside = !inside; }
    }
    inside
}

fn point_segment_distance(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let dx = end.0 - start.0; let dy = end.1 - start.1; let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON { return (point.0 - start.0).hypot(point.1 - start.1); }
    let t = (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length_squared).clamp(0.0, 1.0);
    (point.0 - (start.0 + t * dx)).hypot(point.1 - (start.1 + t * dy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};
    fn part(form: Form, x: f64, y: f64, rotation_radians: f64) -> PlacedMaterialPart { PlacedMaterialPart { part_index: 0, form, placement: Placement { x, y, rotation_radians } } }
    #[test] fn geometry_preserves_material_part_identity_and_placement() { let c=default_catalog(); let m=Material::free_base("Carbon",1.0); let p=[Placement{x:12.0,y:8.0,rotation_radians:0.25}]; let g=MaterialGeometry::new(&m,&p,&c).unwrap(); assert_eq!(g.parts[0].placement,p[0]); }
    #[test] fn physical_instance_keeps_material_and_geometry_together() { let c=default_catalog(); let m=Material::free_base("Carbon",1.0); let p=[Placement{x:4.0,y:6.0,rotation_radians:0.0}]; let i=PhysicalMaterialInstance::new(m.clone(),&p,&c).unwrap(); assert_eq!(i.material,m); }
    #[test] fn structured_material_requires_one_placement_per_constituent() { let c=default_catalog(); let m=Material{parts:vec![("Carbon".into(),1.0),("Hydrogen".into(),1.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1}]}; let p=[Placement{x:0.0,y:0.0,rotation_radians:0.0}]; assert!(MaterialGeometry::new(&m,&p,&c).is_none()); }
    #[test] fn invalid_geometry_is_rejected() { let c=default_catalog(); let m=Material::free_base("Carbon",1.0); let p=[Placement{x:f64::NAN,y:0.0,rotation_radians:0.0}]; assert!(MaterialGeometry::new(&m,&p,&c).is_none()); }
    #[test] fn overlapping_circles_are_in_contact() { let a=part(Form::Circle{radius:2.0},0.0,0.0,0.0); let b=part(Form::Circle{radius:2.0},3.0,0.0,0.0); assert!(placed_forms_overlap(&a,&b,0.0)); }
    #[test] fn separated_circles_are_not_in_contact() { let a=part(Form::Circle{radius:2.0},0.0,0.0,0.0); let b=part(Form::Circle{radius:2.0},4.1,0.0,0.0); assert!(!placed_forms_overlap(&a,&b,0.0)); }
    #[test] fn water_uses_explicit_circle_geometry() { let water=default_catalog().into_iter().find(|r|r.name=="Water").unwrap(); assert!(matches!(water.shape.form,Form::Circle{radius} if radius>0.0)); let a=part(water.shape.form.clone(),0.0,0.0,0.0); let b=part(Form::Circle{radius:0.398_942},0.5,0.0,0.0); assert!(placed_forms_overlap(&a,&b,0.0)); }
}