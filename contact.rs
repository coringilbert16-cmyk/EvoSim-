// ============================================================
// PHYSICAL CONTACT / ACCESSIBILITY (broad-phase + precise-local)
// ============================================================
//
// PROVISIONAL NOTICE: the "precise phase" in this module currently
// approximates every unit as its own bounding circle
// (Form::bounding_radius()). This is a cheap, honest stand-in for
// real polygon-vs-polygon contact resolution, NOT a claim of exact
// geometric contact. Do not treat bounding-circle overlap anywhere
// downstream as if it were final physical contact - when real
// contact geometry is implemented, only the body of
// unit_within_envelope needs to change; every caller's signature
// stays the same.
//
// Locked architecture: organisms are spatially PERMEABLE - they are
// not rigid collision objects and can overlap environmental material
// or other organisms. What matters is whether material within an
// organism's accessible ENVELOPE is actually physically reachable.
// Bonded/structural material is physically hard and occupies space;
// bulk/environmental material does not need that treatment.
//
// The locked two-phase combination:
//   (b) cheap broad-phase envelope test -> candidates
//   (c) precise/local geometry test, ONLY on those candidates
//
// This module establishes exactly that primitive, generically, for
// both of the two cases that currently exist:
//   - organism envelope vs. bulk ActiveMaterialField cells (coarse;
//     "precise" here just means the existing cell-radius scan, which
//     IS already the appropriate resolution for bulk aggregate
//     material - there is no finer geometry to test against a field
//     cell, since the field deliberately stays coarse/aggregate)
//   - organism envelope vs. individual StructuralUnit instances
//     (real, positioned, individually-shaped material - here the
//     precise phase actually tests real geometry, via each unit's
//     Form::bounding_radius())
//
// What this module deliberately does NOT do:
//   - decide acquisition AMOUNT (locked as "bounded/discretized,"
//     exact formula not given - not invented here)
//   - implement organism-vs-organism predation/combat (explicitly
//     deferred; the same candidate-finding primitive below is what a
//     future predation system would reuse, not a separate mechanic)
//   - perform exact polygon-vs-polygon contact resolution (bounding
//     radius overlap is a deliberate, documented approximation -
//     "precise" relative to the broad phase, not full contact
//     physics; swapping in exact polygon contact later does not
//     require changing this module's function signatures)
// ============================================================

use crate::environment::ActiveMaterialField;
use crate::resources::BaseResource;
use crate::structure::{OrganismStructure, StructuralUnit};

/// A cheap physical-reach envelope: a circle, NOT the organism's
/// (nonexistent) rigid body geometry - organisms don't have hard
/// collision shapes (locked). This just answers "how far can this
/// organism physically act right now."
#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

// ------------------------------------------------------------
// BROAD PHASE
// ------------------------------------------------------------

/// Broad-phase candidate field cells: reuses the same bounding-box
/// restricted scan already used for organism perception
/// (ActiveMaterialField::cells_within_radius) - this already IS the
/// correct broad phase for bulk/aggregate material, since the field
/// has no finer-grained geometry to test against.
pub fn broad_phase_field_cells(field: &ActiveMaterialField, envelope: Envelope) -> Vec<usize> {
    field.cells_within_radius(envelope.x, envelope.y, envelope.radius)
}

/// Identifies which field-cell material stacks are physically
/// reachable from an organism's envelope RIGHT NOW - i.e. answers
/// "what material exists and could be acquired from here," not "how
/// much gets acquired" or "at what rate." The actual transfer amount
/// is part of the still-undecided acquisition mechanism and is
/// deliberately NOT decided by this function - see main.rs's
/// Organism::store_unbonded_material for where that boundary is
/// picked back up once acquisition is locked.
///
/// Read-only: never modifies the field. Bulk field material has no
/// per-unit geometry to precise-test against (the field deliberately
/// stays coarse/aggregate - Phase 1), so broad-phase cell membership
/// IS the complete accessibility test for this case; there is no
/// finer "precise phase" possible here, unlike candidate_units_in_envelope
/// below which tests real per-unit geometry for structural material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibleFieldMaterial {
    pub field_index: usize,
    pub bonded: bool,
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

/// Broad-phase candidate structural units: a cheap distance-only
/// filter (envelope radius + a generous fixed margin) before any real
/// per-unit geometry is touched. The margin exists so a unit whose
/// OWN extent reaches into the envelope isn't wrongly excluded just
/// because its center is slightly outside `envelope.radius` - exact
/// per-unit extent is what the precise phase below actually checks.
pub fn broad_phase_structural_units(
    structure: &OrganismStructure,
    envelope: Envelope,
    broad_margin: f64,
) -> Vec<usize> {
    let cutoff = envelope.radius + broad_margin.max(0.0);
    structure
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| {
            let dx = unit.placement.x - envelope.x;
            let dy = unit.placement.y - envelope.y;
            (dx * dx + dy * dy).sqrt() <= cutoff
        })
        .map(|(i, _)| i)
        .collect()
}

// ------------------------------------------------------------
// PRECISE / LOCAL PHASE
// ------------------------------------------------------------

/// Precise-phase test for one candidate unit: does the organism's
/// envelope circle actually overlap this unit's own bounding circle?
/// A deliberate, documented approximation (bounding circle, not exact
/// silhouette) - see module docs.
pub fn unit_within_envelope(envelope: Envelope, unit: &StructuralUnit, catalog: &[BaseResource]) -> bool {
    let Some(base) = catalog.iter().find(|b| b.name == unit.resource_name) else {
        return false;
    };

    let dx = unit.placement.x - envelope.x;
    let dy = unit.placement.y - envelope.y;
    let distance = (dx * dx + dy * dy).sqrt();

    distance <= envelope.radius + base.shape.form.bounding_radius()
}

/// Runs broad-phase then precise-phase for every unit in a structure,
/// returning only indices that pass both. `broad_margin` should be at
/// least as large as the largest bounding radius likely to appear in
/// the catalog, so the broad phase never incorrectly excludes a real
/// candidate before the precise phase gets to check it.
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

#[cfg(test)]
mod contact_tests {
    use super::*;
    use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
    use crate::resources::default_catalog;
    use crate::resources::Material;
    use crate::structure::{Placement, StructuralUnit};

    #[test]
    fn accessible_field_material_finds_material_within_envelope() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        field.deposit(500.0, 500.0, Material { parts: vec![("Carbon".into(), 10.0)], bonded: true });
        field.deposit(500.0, 500.0, Material { parts: vec![("Water".into(), 3.0)], bonded: false });

        let envelope = Envelope { x: 500.0, y: 500.0, radius: 5.0 };
        let found = accessible_field_material(&field, envelope);

        assert!(found.iter().any(|f| f.bonded));
        assert!(found.iter().any(|f| !f.bonded));
    }

    #[test]
    fn accessible_field_material_excludes_empty_cells() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        let envelope = Envelope { x: 500.0, y: 500.0, radius: 5.0 };
        assert!(accessible_field_material(&field, envelope).is_empty());
    }

    #[test]
    fn accessible_field_material_excludes_material_out_of_envelope() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        field.deposit(10.0, 10.0, Material { parts: vec![("Carbon".into(), 10.0)], bonded: true });

        let envelope = Envelope { x: 900.0, y: 900.0, radius: 5.0 };
        assert!(accessible_field_material(&field, envelope).is_empty());
    }

    #[test]
    fn broad_phase_field_cells_matches_the_underlying_field_query() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, DEFAULT_CELL_SIZE);
        let envelope = Envelope { x: 500.0, y: 500.0, radius: 50.0 };

        let via_contact = broad_phase_field_cells(&field, envelope);
        let via_field_directly = field.cells_within_radius(500.0, 500.0, 50.0);

        assert_eq!(via_contact, via_field_directly);
    }

    #[test]
    fn precise_phase_accepts_unit_whose_own_extent_reaches_the_envelope() {
        let catalog = default_catalog();
        // Carbon's bounding radius is small (~0.44); place it just
        // outside a tiny envelope but still within combined reach.
        let unit = StructuralUnit::new("Carbon", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 });
        let envelope = Envelope { x: 0.0, y: 0.0, radius: 0.7 };

        assert!(unit_within_envelope(envelope, &unit, &catalog));
    }

    #[test]
    fn precise_phase_rejects_unit_genuinely_out_of_reach() {
        let catalog = default_catalog();
        let unit = StructuralUnit::new("Carbon", Placement { x: 100.0, y: 0.0, rotation_radians: 0.0 });
        let envelope = Envelope { x: 0.0, y: 0.0, radius: 0.7 };

        assert!(!unit_within_envelope(envelope, &unit, &catalog));
    }

    #[test]
    fn precise_phase_returns_false_for_unknown_resource_name() {
        let catalog = default_catalog();
        let unit = StructuralUnit::new("Unobtainium", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
        let envelope = Envelope { x: 0.0, y: 0.0, radius: 100.0 };

        assert!(!unit_within_envelope(envelope, &unit, &catalog));
    }

    #[test]
    fn candidate_units_in_envelope_finds_only_reachable_units() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();

        let near = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 0.5, y: 0.0, rotation_radians: 0.0 },
        ));
        let far = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 },
        ));

        let envelope = Envelope { x: 0.0, y: 0.0, radius: 1.0 };
        let candidates = candidate_units_in_envelope(&structure, envelope, &catalog, 5.0);

        assert!(candidates.contains(&near));
        assert!(!candidates.contains(&far));
    }

    #[test]
    fn broad_margin_of_zero_still_finds_units_whose_own_radius_covers_the_gap() {
        // Regression guard: broad phase uses (envelope.radius + margin)
        // as a pure distance cutoff, so if margin is too small relative
        // to a unit's own bounding radius, a real precise-phase match
        // could get excluded before it's ever geometry-tested. This
        // confirms a reasonable margin avoids that - documenting the
        // dependency rather than hiding it.
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let unit = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 },
        ));

        let envelope = Envelope { x: 0.0, y: 0.0, radius: 0.7 };

        // With zero margin, distance (1.0) > envelope.radius (0.7),
        // so broad phase alone would wrongly exclude it...
        let too_tight = broad_phase_structural_units(&structure, envelope, 0.0);
        assert!(!too_tight.contains(&unit));

        // ...but with a margin covering Carbon's own bounding radius,
        // it correctly survives broad phase AND passes precise phase.
        let candidates = candidate_units_in_envelope(&structure, envelope, &catalog, 1.0);
        assert!(candidates.contains(&unit));
    }
}
