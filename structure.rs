// ============================================================
// STRUCTURAL REPRESENTATION
// ============================================================
//
// The organism-side structural layer: discrete, individually
// positioned material units connected by bonds. This sits ABOVE the
// coarse bulk Material representation used everywhere else (the
// environment's ActiveMaterialField/DeepReservoir, and an organism's
// own un-instantiated stored_unbonded stock) - nothing in this module
// changes how the environment stores or moves material. It only
// describes what a resource unit looks like once an organism has
// actually built it into something.
//
// Composition/mass/potential_energy/reactivity/cohesion/geometry are
// NEVER duplicated here - a StructuralUnit only ever stores which
// catalog resource type it is; everything else is looked up from the
// immutable catalog on demand (§48 minimum-information principle).
// ============================================================

use serde::{Deserialize, Serialize};

use crate::resources::{BaseResource, ConnectionSites, ResourceProperties};

// ------------------------------------------------------------
// WORLD-SPACE PLACEMENT
// ------------------------------------------------------------
//
// Position/rotation belong to an INSTANCE of a unit, never to the
// immutable Shape/Form itself (locked). Fluid, continuous 2D rotation
// only - no torque/angular velocity/inertia/friction/momentum
// (explicitly out of scope until requested).
// ------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

// ------------------------------------------------------------
// STRUCTURAL UNIT
// ------------------------------------------------------------

/// One discrete, physically instantiated occurrence of a resource
/// type, placed somewhere in organism-local (or eventually world)
/// space.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StructuralUnit {
    pub resource_name: String,
    pub placement: Placement,
}

impl StructuralUnit {
    pub fn new(resource_name: impl Into<String>, placement: Placement) -> Self {
        Self {
            resource_name: resource_name.into(),
            placement,
        }
    }

    pub fn properties<'a>(&self, catalog: &'a [BaseResource]) -> Option<&'a ResourceProperties> {
        catalog
            .iter()
            .find(|b| b.name == self.resource_name)
            .map(|b| &b.properties)
    }

    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> {
        catalog
            .iter()
            .find(|b| b.name == self.resource_name)
            .map(|b| b.shape.connection_sites())
    }
}

// ------------------------------------------------------------
// BOND
// ------------------------------------------------------------
//
// References two connection points by (unit index, point index
// within that unit's derived Corners list). Bond strength belongs
// here, and ONLY here (locked) - never on ConnectionPoint.
//
// NOTE: this only supports bonding between two Corners-derived
// points (rigid polygonal units) for now. Bonding to/through a
// Circle's continuous circumference or a Fluid unit requires a
// resolved contact location on that surface, which is genuinely
// unresolved future contact physics - not invented here.
// ------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Bond {
    pub unit_a: usize,
    pub point_a: usize,
    pub unit_b: usize,
    pub point_b: usize,
    /// 0.0-1.0, fixed at formation. Whether strength can later change
    /// (decay/reinforcement) is unresolved - not assumed either way.
    pub strength: f64,
}

impl Bond {
    /// True if this bond touches the given (unit, point) pair on
    /// either end.
    pub fn touches(&self, unit: usize, point: usize) -> bool {
        (self.unit_a == unit && self.point_a == point) || (self.unit_b == unit && self.point_b == point)
    }
}

// ------------------------------------------------------------
// ORGANISM STRUCTURE
// ------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OrganismStructure {
    pub units: Vec<StructuralUnit>,
    pub bonds: Vec<Bond>,
}

impl OrganismStructure {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a unit, returning its index for use in bond formation.
    pub fn add_unit(&mut self, unit: StructuralUnit) -> usize {
        self.units.push(unit);
        self.units.len() - 1
    }

    /// Adds a bond directly. Does not itself decide whether the bond
    /// SHOULD form (that's the formation-threshold/COMBINE process,
    /// not this data structure's job) - this just records it,
    /// leaving every other existing bond completely untouched.
    pub fn add_bond(&mut self, bond: Bond) -> usize {
        self.bonds.push(bond);
        self.bonds.len() - 1
    }

    /// Sum of the strengths of every bond currently attached to a
    /// specific (unit, point) pair - this is L_A / L_B in the locked
    /// formation-threshold formula below. Derived on demand, never
    /// stored redundantly (§48).
    pub fn connection_load(&self, unit: usize, point: usize) -> f64 {
        self.bonds.iter().filter(|b| b.touches(unit, point)).map(|b| b.strength).sum()
    }

    pub fn connection_count(&self, unit: usize, point: usize) -> usize {
        self.bonds.iter().filter(|b| b.touches(unit, point)).count()
    }

    /// Removes exactly ONE bond, by its position in `bonds`. Every
    /// other bond - including every other bond on either of this
    /// bond's own endpoints - is left completely untouched. This is
    /// the locked "break one bond" semantic: breaking A-B out of
    /// {A-B, A-C, A-D, A-E} leaves A-C, A-D, A-E fully intact.
    ///
    /// Breaking a bond does NOT destroy either unit, and does NOT
    /// automatically convert a now-zero-bond unit back to bulk
    /// material - both explicitly locked. The unit simply remains in
    /// `units`, now with fewer (possibly zero) bonds.
    pub fn break_bond(&mut self, bond_index: usize) -> Option<Bond> {
        if bond_index < self.bonds.len() {
            Some(self.bonds.remove(bond_index))
        } else {
            None
        }
    }

    /// Removes EVERY bond currently attached to one specific (unit,
    /// point) pair in a single pass - the "collective disconnection"
    /// operation, distinct from breaking a single bond above. Bonds
    /// on other points (even other points on the same unit) are
    /// untouched.
    pub fn disconnect_point(&mut self, unit: usize, point: usize) -> Vec<Bond> {
        let mut removed = Vec::new();
        let mut i = 0;
        while i < self.bonds.len() {
            if self.bonds[i].touches(unit, point) {
                removed.push(self.bonds.remove(i));
            } else {
                i += 1;
            }
        }
        removed
    }

    /// Every (unit, point) pair currently involved in at least one
    /// bond. Not stored - derived by scanning `bonds` once. Useful
    /// for connection-count-dependent cost calculations that need to
    /// enumerate "everything already loaded" rather than query one
    /// point at a time.
    pub fn loaded_points(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for bond in &self.bonds {
            for pair in [(bond.unit_a, bond.point_a), (bond.unit_b, bond.point_b)] {
                if !pairs.contains(&pair) {
                    pairs.push(pair);
                }
            }
        }
        pairs
    }
}

// ------------------------------------------------------------
// FORMATION THRESHOLD  (locked equation - do not replace)
// ------------------------------------------------------------
//
//     T = ((C_A + C_B) / 2) * (1 + sqrt(L_A) + sqrt(L_B))
//
// C_A / C_B: cohesion of the resource type owning point A / B.
// L_A / L_B: sum of existing bond strengths already attached to
//            point A / B (OrganismStructure::connection_load).
//
// The square-root terms give diminishing returns: unused points are
// easy to connect, increasingly loaded points get progressively
// harder. This is the ONE equation in the whole bonding design that
// has an exact locked formula - everything else (the -1..+1
// interaction value, surplus-investment -> bond-strength mapping,
// BREAK energy) remains genuinely undecided; see the TODOs below.
// This function does not decide what counts as "surpassing" T, or
// what happens to any energy involved - it only computes T itself.
// ------------------------------------------------------------

pub fn formation_threshold(cohesion_a: f64, cohesion_b: f64, load_a: f64, load_b: f64) -> f64 {
    let load_a = load_a.max(0.0);
    let load_b = load_b.max(0.0);
    ((cohesion_a + cohesion_b) / 2.0) * (1.0 + load_a.sqrt() + load_b.sqrt())
}

// ------------------------------------------------------------
// GENUINELY UNRESOLVED - NOT IMPLEMENTED HERE
// ------------------------------------------------------------
//
// TODO(interaction-equation): the -1..+1 bounded interaction value
// (whether a new contact requires or releases energy) is not locked.
// Must derive from potential_energy, reactivity, and geometry of the
// two newly-contacting surfaces, deterministically, WITHOUT existing
// bond load amplifying it (existing load only affects the threshold
// above, never the interaction itself - locked). Do not invent this.
//
// TODO(surplus-investment): once formation_threshold() is exceeded,
// the mapping from surplus organism investment to the resulting
// Bond.strength is not locked. Do not invent.
//
// TODO(break-energy): the energetic consequence of breaking a bond
// (can require OR release energy, depending on state) is not locked.
// Cohesion and "structural state" both contribute to resistance, but
// the exact equation is undecided. Do not invent. COMBINE and BREAK
// are explicitly NOT required to be numerically symmetric.
// ------------------------------------------------------------

#[cfg(test)]
mod structure_tests {
    use super::*;

    fn placement(x: f64, y: f64) -> Placement {
        Placement { x, y, rotation_radians: 0.0 }
    }

    #[test]
    fn add_unit_returns_a_usable_index() {
        let mut structure = OrganismStructure::new();
        let i = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        assert_eq!(i, 0);
        assert_eq!(structure.units.len(), 1);
        assert_eq!(structure.units[0].resource_name, "Carbon");
    }

    #[test]
    fn unit_properties_and_connection_sites_are_looked_up_from_catalog_not_duplicated() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let i = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));

        let unit = &structure.units[i];
        let props = unit.properties(&catalog).unwrap();
        assert_eq!(props.cohesion, 0.95);

        match unit.connection_sites(&catalog).unwrap() {
            ConnectionSites::Corners(points) => assert_eq!(points.len(), 6),
            other => panic!("expected Corners, got {other:?}"),
        }
    }

    #[test]
    fn unknown_resource_name_returns_none_rather_than_panicking() {
        let catalog = crate::resources::default_catalog();
        let unit = StructuralUnit::new("Unobtainium", placement(0.0, 0.0));
        assert!(unit.properties(&catalog).is_none());
        assert!(unit.connection_sites(&catalog).is_none());
    }

    #[test]
    fn adding_a_bond_does_not_touch_units() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.5 });

        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.bonds.len(), 1);
    }

    #[test]
    fn connection_load_and_count_sum_only_bonds_touching_that_point() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        let c = structure.add_unit(StructuralUnit::new("Sulfur", placement(2.0, 0.0)));

        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.3 });
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: c, point_b: 0, strength: 0.4 });
        // A bond on a DIFFERENT point of unit a - must not count.
        structure.add_bond(Bond { unit_a: a, point_a: 1, unit_b: c, point_b: 1, strength: 0.9 });

        assert!((structure.connection_load(a, 0) - 0.7).abs() < 1e-12);
        assert_eq!(structure.connection_count(a, 0), 2);
        assert_eq!(structure.connection_count(a, 1), 1);
        assert_eq!(structure.connection_count(b, 0), 1);
    }

    #[test]
    fn break_bond_removes_only_that_one_bond() {
        // Locked example: A-B, A-C, A-D, A-E; breaking A-B leaves the
        // other three fully intact.
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let units: Vec<usize> = ["Methane", "Sulfur", "Nitrogen", "Phosphorus"]
            .iter()
            .enumerate()
            .map(|(k, name)| structure.add_unit(StructuralUnit::new(*name, placement(k as f64 + 1.0, 0.0))))
            .collect();

        let bond_ab = structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: units[0], point_b: 0, strength: 0.5 });
        structure.add_bond(Bond { unit_a: a, point_a: 1, unit_b: units[1], point_b: 0, strength: 0.5 });
        structure.add_bond(Bond { unit_a: a, point_a: 2, unit_b: units[2], point_b: 0, strength: 0.5 });
        structure.add_bond(Bond { unit_a: a, point_a: 3, unit_b: units[3], point_b: 0, strength: 0.5 });

        assert_eq!(structure.bonds.len(), 4);
        structure.break_bond(bond_ab);
        assert_eq!(structure.bonds.len(), 3);

        // A-C, A-D, A-E (points 1,2,3) all remain.
        assert_eq!(structure.connection_count(a, 0), 0);
        assert_eq!(structure.connection_count(a, 1), 1);
        assert_eq!(structure.connection_count(a, 2), 1);
        assert_eq!(structure.connection_count(a, 3), 1);

        // Both units of the broken bond still exist (breaking a bond
        // does not destroy either unit - locked).
        assert_eq!(structure.units.len(), 5);
    }

    #[test]
    fn zero_bond_unit_remains_in_structure_not_auto_reverted() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        let bond = structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.5 });

        structure.break_bond(bond);

        assert_eq!(structure.connection_count(a, 0), 0);
        assert_eq!(structure.units.len(), 2, "units must remain even with zero bonds - locked");
    }

    #[test]
    fn disconnect_point_removes_every_bond_at_that_point_only() {
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Methane", placement(1.0, 0.0)));
        let c = structure.add_unit(StructuralUnit::new("Sulfur", placement(2.0, 0.0)));
        let d = structure.add_unit(StructuralUnit::new("Nitrogen", placement(3.0, 0.0)));

        // Two separate bonds both attached to (a, point 0) - multiple
        // bonds per point is explicitly allowed.
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.2 });
        structure.add_bond(Bond { unit_a: a, point_a: 0, unit_b: c, point_b: 0, strength: 0.3 });
        // A bond on a different point of a - must survive.
        let surviving = structure.add_bond(Bond { unit_a: a, point_a: 1, unit_b: d, point_b: 0, strength: 0.4 });

        let removed = structure.disconnect_point(a, 0);
        assert_eq!(removed.len(), 2);
        assert_eq!(structure.bonds.len(), 1);
        assert_eq!(structure.connection_count(a, 0), 0);
        assert_eq!(structure.connection_count(a, 1), 1);
        assert!(!structure.bonds.is_empty());
        let _ = surviving;
    }

    #[test]
    fn formation_threshold_increases_with_existing_load_but_not_below_base() {
        let base = formation_threshold(0.5, 0.5, 0.0, 0.0);
        assert!((base - 0.5).abs() < 1e-12, "with zero load, T should equal the average cohesion");

        let loaded = formation_threshold(0.5, 0.5, 1.0, 0.0);
        assert!(loaded > base, "existing load on either point must raise the threshold");

        let more_loaded = formation_threshold(0.5, 0.5, 4.0, 0.0);
        assert!(more_loaded > loaded, "more load must raise it further");

        // Diminishing returns: going from load 4->9 (both +5) adds
        // less than going from load 0->4 (+4), because of sqrt.
        let step1 = formation_threshold(0.5, 0.5, 4.0, 0.0) - formation_threshold(0.5, 0.5, 0.0, 0.0);
        let step2 = formation_threshold(0.5, 0.5, 9.0, 0.0) - formation_threshold(0.5, 0.5, 4.0, 0.0);
        assert!(step2 < step1, "sqrt term must give diminishing returns as load grows");
    }

    #[test]
    fn formation_threshold_is_symmetric_in_its_two_points() {
        let a = formation_threshold(0.9, 0.1, 3.0, 1.0);
        let b = formation_threshold(0.1, 0.9, 1.0, 3.0);
        assert!((a - b).abs() < 1e-12, "swapping which point is 'A' vs 'B' must not change T");
    }

    #[test]
    fn negative_load_is_treated_as_zero_defensively() {
        // connection_load can never actually be negative in practice
        // (bond strengths are meant to be 0..1), but the formula
        // shouldn't produce NaN/complex results if it somehow were.
        let t = formation_threshold(0.5, 0.5, -1.0, -1.0);
        assert!(t.is_finite());
        assert!((t - formation_threshold(0.5, 0.5, 0.0, 0.0)).abs() < 1e-12);
    }
}
