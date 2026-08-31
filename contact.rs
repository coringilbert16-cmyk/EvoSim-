// ============================================================
// PHYSICAL CONTACT / ACCESSIBILITY
// ============================================================
//
// Two-phase physical accessibility for bulk field material and
// individually positioned structural units. Contact detection is
// deliberately separate from acquisition, bonding, and energetic
// consequences.

use crate::environment::ActiveMaterialField;
use crate::resources::{BaseResource, ConnectionPoint};
use crate::structure::{OrganismStructure, StructuralUnit};
use crate::connection_geometry::{
    facing_compatibility, point_distance, transform_connection_point,
    within_contact_tolerance, WorldConnectionPoint,
};

#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibleFieldMaterial {
    pub field_index: usize,
    pub bonded: bool,
}

pub fn broad_phase_field_cells(field: &ActiveMaterialField, envelope: Envelope) -> Vec<usize> {
    field.cells_within_radius(envelope.x, envelope.y, envelope.radius)
}

pub fn accessible_field_material(
    field: &ActiveMaterialField,
    envelope: Envelope,
) -> Vec<AccessibleFieldMaterial> {
    let mut found = Vec::new();
    for field_index in broad_phase_field_cells(field, envelope) {
        let cell = &field.cells[field_index];
        if cell.bonded.total_amount() > 0.0 {
            found.push(AccessibleFieldMaterial { field_index, bonded: true });
        }
        if cell.unbonded.total_amount() > 0.0 {
            found.push(AccessibleFieldMaterial { field_index, bonded: false });
        }
    }
    found
}

pub fn broad_phase_structural_units(
    structure: &OrganismStructure,
    envelope: Envelope,
    broad_margin: f64,
) -> Vec<usize> {
    let cutoff = envelope.radius + broad_margin.max(0.0);
    let cutoff_sq = cutoff * cutoff;
    structure
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| {
            let dx = unit.placement.x - envelope.x;
            let dy = unit.placement.y - envelope.y;
            dx * dx + dy * dy <= cutoff_sq
        })
        .map(|(i, _)| i)
        .collect()
}

/// Existing coarse precise-phase approximation: the unit is represented by
/// its catalog bounding circle. This remains the fallback accessibility test
/// for callers that only need reachability, while connection-point contact
/// below provides the more informative local geometry for actual attachments.
pub fn unit_within_envelope(
    envelope: Envelope,
    unit: &StructuralUnit,
    catalog: &[BaseResource],
) -> bool {
    let Some(base) = catalog.iter().find(|b| b.name == unit.resource_name) else {
        return false;
    };
    let dx = unit.placement.x - envelope.x;
    let dy = unit.placement.y - envelope.y;
    let distance = (dx * dx + dy * dy).sqrt();
    distance <= envelope.radius + base.shape.form.bounding_radius()
}

pub fn candidate_units_in_envelope(
    structure: &OrganismStructure,
    envelope: Envelope,
    catalog: &[BaseResource],
    broad_margin: f64,
) -> Vec<usize> {
    broad_phase_structural_units(structure, envelope, broad_margin)
        .into_iter()
        .filter(|&i| unit_within_envelope(envelope, &structure.units[i], catalog))
        .collect()
}

/// Transform one catalog-authored connection point using a structural unit's
/// placement. This is the precise-phase geometry entry point used by future
/// bond/contact selection; it does not create or modify a bond.
pub fn world_connection_point(
    point: ConnectionPoint,
    unit: &StructuralUnit,
) -> WorldConnectionPoint {
    transform_connection_point(
        point,
        unit.placement.x,
        unit.placement.y,
        unit.placement.rotation_radians,
    )
}

/// Local geometric contact between two connection points.
///
/// `tolerance` controls spatial contact only. `min_facing` optionally rejects
/// contacts whose outward normals do not face one another sufficiently. No
/// bond-strength or energy decision is made here.
pub fn connection_points_contact(
    a: ConnectionPoint,
    unit_a: &StructuralUnit,
    b: ConnectionPoint,
    unit_b: &StructuralUnit,
    tolerance: f64,
    min_facing: f64,
) -> bool {
    let wa = world_connection_point(a, unit_a);
    let wb = world_connection_point(b, unit_b);
    within_contact_tolerance(wa, wb, tolerance)
        && facing_compatibility(wa, wb) >= min_facing
}

/// Returns the geometric separation of two positioned connection points.
pub fn connection_point_distance(
    a: ConnectionPoint,
    unit_a: &StructuralUnit,
    b: ConnectionPoint,
    unit_b: &StructuralUnit,
) -> f64 {
    point_distance(world_connection_point(a, unit_a), world_connection_point(b, unit_b))
}

#[cfg(test)]
mod contact_tests {
    use super::*;
    use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
    use crate::resources::{default_catalog, Material};
    use crate::structure::Placement;
    use std::f64::consts::PI;

    #[test]
    fn accessible_field_material_finds_bonded_and_unbonded() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        field.deposit(500.0, 500.0, Material { parts: vec![("Carbon".into(), 10.0)], bonded: true });
        field.deposit(500.0, 500.0, Material { parts: vec![("Water".into(), 3.0)], bonded: false });
        let found = accessible_field_material(&field, Envelope { x: 500.0, y: 500.0, radius: 5.0 });
        assert!(found.iter().any(|f| f.bonded));
        assert!(found.iter().any(|f| !f.bonded));
    }

    #[test]
    fn broad_phase_field_cells_delegates_to_field_query() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        let envelope = Envelope { x: 500.0, y: 500.0, radius: 50.0 };
        assert_eq!(broad_phase_field_cells(&field, envelope), field.cells_within_radius(500.0, 500.0, 50.0));
    }

    #[test]
    fn candidate_units_uses_broad_then_precise_phase() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let near = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.5, y: 0.0, rotation_radians: 0.0 }));
        let far = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 }));
        let candidates = candidate_units_in_envelope(
            &structure,
            Envelope { x: 0.0, y: 0.0, radius: 1.0 },
            &catalog,
            1.0,
        );
        assert!(candidates.contains(&near));
        assert!(!candidates.contains(&far));
    }

    #[test]
    fn connection_point_geometry_respects_unit_rotation() {
        let unit = StructuralUnit::new("Carbon", Placement { x: 10.0, y: 20.0, rotation_radians: PI / 2.0 });
        let world = world_connection_point(ConnectionPoint { x: 1.0, y: 0.0, direction_radians: 0.0 }, &unit);
        assert!((world.x - 10.0).abs() < 1e-12);
        assert!((world.y - 21.0).abs() < 1e-12);
        assert!(world.normal_x.abs() < 1e-12);
        assert!((world.normal_y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn directly_facing_connection_points_contact() {
        let a = StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
        let b = StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 });
        let pa = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
        let pb = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: PI };
        assert!(connection_points_contact(pa, &a, pb, &b, 1.0, 0.9));
        assert!((connection_point_distance(pa, &a, pb, &b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn same_facing_connection_points_are_rejected_when_facing_threshold_is_positive() {
        let a = StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
        let b = StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 });
        let pa = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
        let pb = ConnectionPoint { x: 0.0, y: 0.0, direction_radians: 0.0 };
        assert!(!connection_points_contact(pa, &a, pb, &b, 1.0, 0.1));
    }
}
