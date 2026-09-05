//! Physical organism/environment boundary contact.
//!
//! This module bridges the two physical geometry models without changing
//! either material identity or ecological storage. A boundary contact is
//! identified by the actual constituent pair whose *boundaries* touch or
//! intersect. Mere containment is not enough: an environmental constituent
//! entirely inside another rigid shape does not establish an interface.

use crate::material_geometry::{PhysicalMaterialInstance, PlacedMaterialPart};
use crate::organism_geometry::OrganismBodyGeometry;
use crate::resources::Form;
use crate::structure::Placement;

/// A constituent-level physical interface between an organism body and an
/// environmental material instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryContact {
    pub organism_unit_index: usize,
    pub material_part_index: usize,
}

fn world_polygon(part: &PlacedMaterialPart) -> Option<Vec<(f64, f64)>> {
    part.form.polygon_vertices().map(|vertices| {
        vertices
            .into_iter()
            .map(|(x, y)| {
                let angle = part.placement.rotation_radians;
                let cos = angle.cos();
                let sin = angle.sin();
                (
                    part.placement.x + x * cos - y * sin,
                    part.placement.y + x * sin + y * cos,
                )
            })
            .collect()
    })
}

fn point_segment_distance(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f64::EPSILON {
        return (px - ax).hypot(py - ay);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0);
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

fn orientation(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> f64 {
    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
}

fn on_segment(ax: f64, ay: f64, bx: f64, by: f64, px: f64, py: f64, tolerance: f64) -> bool {
    px >= ax.min(bx) - tolerance
        && px <= ax.max(bx) + tolerance
        && py >= ay.min(by) - tolerance
        && py <= ay.max(by) + tolerance
}

fn segments_intersect(
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    d: (f64, f64),
    tolerance: f64,
) -> bool {
    let o1 = orientation(a.0, a.1, b.0, b.1, c.0, c.1);
    let o2 = orientation(a.0, a.1, b.0, b.1, d.0, d.1);
    let o3 = orientation(c.0, c.1, d.0, d.1, a.0, a.1);
    let o4 = orientation(c.0, c.1, d.0, d.1, b.0, b.1);

    let eps = tolerance.max(1e-12);
    if ((o1 > eps && o2 < -eps) || (o1 < -eps && o2 > eps))
        && ((o3 > eps && o4 < -eps) || (o3 < -eps && o4 > eps))
    {
        return true;
    }

    if o1.abs() <= eps && on_segment(a.0, a.1, b.0, b.1, c.0, c.1, tolerance) {
        return true;
    }
    if o2.abs() <= eps && on_segment(a.0, a.1, b.0, b.1, d.0, d.1, tolerance) {
        return true;
    }
    if o3.abs() <= eps && on_segment(c.0, c.1, d.0, d.1, a.0, a.1, tolerance) {
        return true;
    }
    if o4.abs() <= eps && on_segment(c.0, c.1, d.0, d.1, b.0, b.1, tolerance) {
        return true;
    }

    tolerance > 0.0
        && point_segment_distance(a.0, a.1, c.0, c.1, d.0, d.1) <= tolerance
        || tolerance > 0.0
            && point_segment_distance(b.0, b.1, c.0, c.1, d.0, d.1) <= tolerance
        || tolerance > 0.0
            && point_segment_distance(c.0, c.1, a.0, a.1, b.0, b.1) <= tolerance
        || tolerance > 0.0
            && point_segment_distance(d.0, d.1, a.0, a.1, b.0, b.1) <= tolerance
}

fn circle_circle_boundary_contact(
    a: &PlacedMaterialPart,
    ar: f64,
    b: &PlacedMaterialPart,
    br: f64,
    tolerance: f64,
) -> bool {
    let distance = (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y);
    (distance - (ar + br)).abs() <= tolerance
}

fn circle_polygon_boundary_contact(
    circle: &PlacedMaterialPart,
    radius: f64,
    polygon: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    let Some(vertices) = world_polygon(polygon) else {
        return false;
    };
    vertices.iter().enumerate().any(|(i, &a)| {
        let b = vertices[(i + 1) % vertices.len()];
        (point_segment_distance(
            circle.placement.x,
            circle.placement.y,
            a.0,
            a.1,
            b.0,
            b.1,
        ) - radius)
            .abs()
            <= tolerance
    })
}

fn polygon_polygon_boundary_contact(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    let (Some(a_vertices), Some(b_vertices)) = (world_polygon(a), world_polygon(b)) else {
        return false;
    };

    a_vertices.iter().enumerate().any(|(i, &a1)| {
        let a2 = a_vertices[(i + 1) % a_vertices.len()];
        b_vertices.iter().enumerate().any(|(j, &b1)| {
            let b2 = b_vertices[(j + 1) % b_vertices.len()];
            segments_intersect(a1, a2, b1, b2, tolerance)
        })
    })
}

/// Determine whether two rigid constituent boundaries actually meet.
///
/// This is intentionally different from volume/area overlap. It rejects
/// complete containment and therefore identifies the physical interface
/// needed by the later permeability layer. Fluids remain unsupported until
/// they have an authoritative spatial boundary.
fn placed_form_boundaries_intersect(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> bool {
    if tolerance < 0.0 {
        return false;
    }

    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => {
            circle_circle_boundary_contact(a, *ar, b, *br, tolerance)
        }
        (Form::Circle { radius }, _) => circle_polygon_boundary_contact(a, *radius, b, tolerance),
        (_, Form::Circle { radius }) => circle_polygon_boundary_contact(b, *radius, a, tolerance),
        (Form::Fluid { .. }, _) | (_, Form::Fluid { .. }) => false,
        _ => polygon_polygon_boundary_contact(a, b, tolerance),
    }
}

/// Find every organism/material constituent pair whose actual rigid
/// boundaries touch or intersect.
///
/// This is deliberately a contact primitive, not a permeability or transfer
/// calculation. A contact proves only that an interface exists. How much
/// material can cross that interface belongs to the later permeability and
/// interaction-capacity layers.
pub fn boundary_contacts(
    body: &OrganismBodyGeometry,
    material: &PhysicalMaterialInstance,
    tolerance: f64,
) -> Vec<BoundaryContact> {
    let mut contacts = Vec::new();

    for body_part in &body.parts {
        let organism_part = PlacedMaterialPart {
            part_index: body_part.unit_index,
            form: body_part.form.clone(),
            placement: Placement {
                x: body_part.x,
                y: body_part.y,
                rotation_radians: body_part.rotation_radians,
            },
        };

        for material_part in &material.geometry.parts {
            if placed_form_boundaries_intersect(&organism_part, material_part, tolerance) {
                contacts.push(BoundaryContact {
                    organism_unit_index: body_part.unit_index,
                    material_part_index: material_part.part_index,
                });
            }
        }
    }

    contacts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_geometry::PhysicalMaterialInstance;
    use crate::resources::Material;
    use crate::structure::{OrganismStructure, StructuralUnit};

    fn body_at(x: f64, y: f64) -> OrganismBodyGeometry {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        ));
        OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap()
    }

    fn material_at(name: &str, x: f64, y: f64) -> PhysicalMaterialInstance {
        let catalog = crate::resources::default_catalog();
        PhysicalMaterialInstance::new(
            Material::free_base(name, 1.0),
            &[Placement {
                x,
                y,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
    }

    #[test]
    fn touching_rigid_boundaries_create_an_interface() {
        let body = body_at(0.0, 0.0);
        let material = material_at("Hydrogen", 1.5, 0.0);
        assert_eq!(
            boundary_contacts(&body, &material, 0.0),
            vec![BoundaryContact {
                organism_unit_index: 0,
                material_part_index: 0,
            }]
        );
    }

    #[test]
    fn separated_rigid_boundaries_have_no_interface() {
        let body = body_at(0.0, 0.0);
        let material = material_at("Hydrogen", 1000.0, 0.0);
        assert!(boundary_contacts(&body, &material, 0.0).is_empty());
    }

    #[test]
    fn_contained_rigid_material_is_not_mistaken_for_boundary_contact() {
        let body = body_at(0.0, 0.0);
        let material = material_at("Hydrogen", 0.0, 0.0);
        assert!(boundary_contacts(&body, &material, 0.0).is_empty());
    }

    #[test]
    fn fluid_has_no_interface_without_authoritative_boundary_geometry() {
        let catalog = crate::resources::default_catalog();
        let body = body_at(0.0, 0.0);
        let material = PhysicalMaterialInstance::new(
            Material::free_base("Water", 1.0),
            &[Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        assert!(boundary_contacts(&body, &material, 0.0).is_empty());
    }
}
