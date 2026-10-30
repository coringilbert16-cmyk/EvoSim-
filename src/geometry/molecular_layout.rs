#![expect(
    dead_code,
    reason = "Staged API retained for chemistry stability integration"
)]
//! Geometry-only descriptors used to evaluate molecular layout stability.
//!
//! This module deliberately does not assign a stability score or energy. It
//! exposes live geometric inputs so chemistry can later derive physical
//! consequences without inventing a molecule-wide averaged property.
use crate::contact::{connection_pair_candidates, endpoint_world_point};
use crate::resources::BaseResource;
use crate::structure::{Bond, ConnectionEndpoint, OrganismStructure, PhysicalConstituentId};

/// A realized bond's current geometric state. Bond energy and strength are
/// retained as separate stored bond attributes; neither is rewritten here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BondLayout {
    pub bond_index: usize,
    pub endpoint_distance: f64,
    pub facing: f64,
    pub endpoint_load_a: f64,
    pub endpoint_load_b: f64,
    pub bond_energy: f64,
    pub bond_strength: f64,
}

/// Angular relationship between two realized bond endpoints on one
/// constituent. This records actual layout; it does not assume an ideal angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LocalBondAngle {
    pub constituent: PhysicalConstituentId,
    pub bond_index_a: usize,
    pub bond_index_b: usize,
    pub angular_separation_radians: f64,
    pub endpoint_separation: f64,
}

/// The closest currently available endpoint pair between two constituents
/// that do not already share a bond. Distance is a live geometric gap, not a
/// chemical attraction or an energy value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UnbondedApproach {
    pub constituent_a: PhysicalConstituentId,
    pub constituent_b: PhysicalConstituentId,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub distance: f64,
    pub facing: f64,
    pub endpoint_load_a: f64,
    pub endpoint_load_b: f64,
}

/// A read-only snapshot of the geometry surrounding a physical bond graph.
/// No constituent identity, bond, placement, or energy ledger is modified.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct MolecularLayoutSnapshot {
    pub bonds: Vec<BondLayout>,
    pub local_bond_angles: Vec<LocalBondAngle>,
    pub unbonded_approaches: Vec<UnbondedApproach>,
}

fn same_constituent_pair(bond: &Bond, a: PhysicalConstituentId, b: PhysicalConstituentId) -> bool {
    (bond.endpoint_a.constituent_id == a && bond.endpoint_b.constituent_id == b)
        || (bond.endpoint_a.constituent_id == b && bond.endpoint_b.constituent_id == a)
}

impl MolecularLayoutSnapshot {
    /// Resolve current bond endpoint geometry and nearby unbonded endpoint
    /// approaches. Invalid stored bond endpoints or non-finite geometry reject
    /// the snapshot instead of silently treating malformed geometry as stable.
    pub(crate) fn analyze(structure: &OrganismStructure, catalog: &[BaseResource]) -> Option<Self> {
        let mut snapshot = Self::default();

        for (bond_index, bond) in structure.bonds.iter().enumerate() {
            if !bond.bond_energy.is_finite()
                || bond.bond_energy < 0.0
                || !bond.strength.is_finite()
                || !(0.0..=1.0).contains(&bond.strength)
            {
                return None;
            }
            let index_a = structure.unit_index(bond.endpoint_a.constituent_id)?;
            let index_b = structure.unit_index(bond.endpoint_b.constituent_id)?;
            let unit_a = structure.units.get(index_a)?;
            let unit_b = structure.units.get(index_b)?;
            let point_a = endpoint_world_point(bond.endpoint_a.location, unit_a, catalog)?;
            let point_b = endpoint_world_point(bond.endpoint_b.location, unit_b, catalog)?;
            let distance = (point_a.x - point_b.x).hypot(point_a.y - point_b.y);
            let facing = crate::connection_geometry::facing_compatibility(point_a, point_b);
            let load_a = structure.connection_load(index_a, bond.endpoint_a.location, catalog);
            let load_b = structure.connection_load(index_b, bond.endpoint_b.location, catalog);
            if !distance.is_finite()
                || !facing.is_finite()
                || !load_a.is_finite()
                || !load_b.is_finite()
            {
                return None;
            }
            snapshot.bonds.push(BondLayout {
                bond_index,
                endpoint_distance: distance,
                facing,
                endpoint_load_a: load_a,
                endpoint_load_b: load_b,
                bond_energy: bond.bond_energy,
                bond_strength: bond.strength,
            });
        }

        // Record the actual angular spacing of bonds attached to each physical
        // constituent. No preferred angle or penalty is assumed.
        for unit in &structure.units {
            let mut endpoints = Vec::new();
            for (bond_index, bond) in structure.bonds.iter().enumerate() {
                let endpoint = if bond.endpoint_a.constituent_id == unit.physical_id {
                    Some(bond.endpoint_a.location)
                } else if bond.endpoint_b.constituent_id == unit.physical_id {
                    Some(bond.endpoint_b.location)
                } else {
                    None
                };
                let Some(endpoint) = endpoint else {
                    continue;
                };
                let point = endpoint_world_point(endpoint, unit, catalog)?;
                let dx = point.x - unit.placement.x;
                let dy = point.y - unit.placement.y;
                let radius = dx.hypot(dy);
                if !radius.is_finite() || radius <= f64::EPSILON {
                    return None;
                }
                endpoints.push((bond_index, dy.atan2(dx), point.x, point.y));
            }
            for left in 0..endpoints.len() {
                for right in (left + 1)..endpoints.len() {
                    let raw = (endpoints[left].1 - endpoints[right].1).abs();
                    let angular_separation = raw.min(std::f64::consts::TAU - raw);
                    let endpoint_separation = (endpoints[left].2 - endpoints[right].2)
                        .hypot(endpoints[left].3 - endpoints[right].3);
                    if !angular_separation.is_finite() || !endpoint_separation.is_finite() {
                        return None;
                    }
                    snapshot.local_bond_angles.push(LocalBondAngle {
                        constituent: unit.physical_id,
                        bond_index_a: endpoints[left].0,
                        bond_index_b: endpoints[right].0,
                        angular_separation_radians: angular_separation,
                        endpoint_separation,
                    });
                }
            }
        }

        // Chemistry's existing interaction radius defines the range in which
        // a nonbonded interface can be relevant. Bounding radii are only a
        // conservative broad phase; exact candidate endpoint geometry supplies
        // the reported gap.
        let interaction_range = crate::chemistry::CHEMICAL_CONTACT_RADIUS;
        for index_a in 0..structure.units.len() {
            let unit_a = &structure.units[index_a];
            let shape_a = unit_a.shape(catalog)?;
            let radius_a = shape_a.form.bounding_radius();
            if !radius_a.is_finite() || radius_a < 0.0 {
                return None;
            }
            for index_b in (index_a + 1)..structure.units.len() {
                let unit_b = &structure.units[index_b];
                let id_a = unit_a.physical_id;
                let id_b = unit_b.physical_id;
                if structure
                    .bonds
                    .iter()
                    .any(|bond| same_constituent_pair(bond, id_a, id_b))
                {
                    continue;
                }
                let shape_b = unit_b.shape(catalog)?;
                let radius_b = shape_b.form.bounding_radius();
                if !radius_b.is_finite() || radius_b < 0.0 {
                    return None;
                }
                let center_distance = (unit_a.placement.x - unit_b.placement.x)
                    .hypot(unit_a.placement.y - unit_b.placement.y);
                if !center_distance.is_finite() {
                    return None;
                }
                if center_distance > radius_a + radius_b + interaction_range {
                    continue;
                }

                let candidate = connection_pair_candidates(structure, index_a, index_b, catalog)
                    .into_iter()
                    .filter(|candidate| {
                        candidate.distance.is_finite()
                            && candidate.facing.is_finite()
                            && candidate.load_a.is_finite()
                            && candidate.load_b.is_finite()
                    })
                    .min_by(|left, right| left.distance.total_cmp(&right.distance));
                let Some(candidate) = candidate else {
                    continue;
                };
                if candidate.distance > interaction_range {
                    continue;
                }
                snapshot.unbonded_approaches.push(UnbondedApproach {
                    constituent_a: id_a,
                    constituent_b: id_b,
                    endpoint_a: candidate.endpoint_a,
                    endpoint_b: candidate.endpoint_b,
                    distance: candidate.distance,
                    facing: candidate.facing,
                    endpoint_load_a: candidate.load_a,
                    endpoint_load_b: candidate.load_b,
                });
            }
        }

        snapshot.local_bond_angles.sort_by(|left, right| {
            left.constituent
                .0
                .cmp(&right.constituent.0)
                .then_with(|| left.bond_index_a.cmp(&right.bond_index_a))
                .then_with(|| left.bond_index_b.cmp(&right.bond_index_b))
        });
        snapshot.unbonded_approaches.sort_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.constituent_a.0.cmp(&right.constituent_a.0))
                .then_with(|| left.constituent_b.0.cmp(&right.constituent_b.0))
        });
        Some(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::{BondEndpoint, Placement, StructuralUnit};

    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }

    #[test]
    fn bond_layout_tracks_live_endpoint_geometry_without_rewriting_bond_energy() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Carbon", placement(2.0, 0.0)));
        let id_a = structure.physical_id(a).unwrap();
        let id_b = structure.physical_id(b).unwrap();
        structure.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(
                id_a,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                id_b,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::PI,
                },
            ),
            strength: 0.6,
            bond_energy: 1.25,
        });

        let before = MolecularLayoutSnapshot::analyze(&structure, &catalog).unwrap();
        let before_distance = before.bonds[0].endpoint_distance;
        assert_eq!(before.bonds[0].bond_energy, 1.25);
        assert_eq!(before.bonds[0].bond_strength, 0.6);

        structure.units[b].placement.x += 0.25;
        let after = MolecularLayoutSnapshot::analyze(&structure, &catalog).unwrap();
        assert!(after.bonds[0].endpoint_distance > before_distance);
        assert_eq!(after.bonds[0].bond_energy, 1.25);
        assert_eq!(after.bonds[0].bond_strength, 0.6);
    }

    #[test]
    fn local_bond_angles_report_actual_endpoint_spacing_without_an_ideal_angle() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let center = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let first = structure.add_unit(StructuralUnit::new("Carbon", placement(2.0, 0.0)));
        let second = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 2.0)));
        let center_id = structure.physical_id(center).unwrap();
        let first_id = structure.physical_id(first).unwrap();
        let second_id = structure.physical_id(second).unwrap();
        structure.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(
                center_id,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                first_id,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::PI,
                },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });
        structure.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(
                center_id,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::FRAC_PI_2,
                },
            ),
            endpoint_b: BondEndpoint::new(
                second_id,
                ConnectionEndpoint::Boundary {
                    angle_radians: -std::f64::consts::FRAC_PI_2,
                },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });

        let snapshot = MolecularLayoutSnapshot::analyze(&structure, &catalog).unwrap();
        let local = snapshot
            .local_bond_angles
            .iter()
            .find(|angle| angle.constituent == center_id)
            .unwrap();
        assert!((local.angular_separation_radians - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!(local.endpoint_separation > 0.0);
    }

    #[test]
    fn approaching_unbonded_constituents_produce_a_smaller_live_gap() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        structure.add_unit(StructuralUnit::new("Carbon", placement(2.9, 0.0)));

        let separated = MolecularLayoutSnapshot::analyze(&structure, &catalog).unwrap();
        assert_eq!(separated.bonds.len(), 0);
        assert_eq!(separated.unbonded_approaches.len(), 1);
        let separated_gap = separated.unbonded_approaches[0].distance;

        structure.units[1].placement.x = 2.6;
        let approaching = MolecularLayoutSnapshot::analyze(&structure, &catalog).unwrap();
        assert_eq!(approaching.unbonded_approaches.len(), 1);
        assert!(approaching.unbonded_approaches[0].distance < separated_gap);
    }

    #[test]
    fn already_bonded_constituent_pairs_are_not_reported_as_unbonded_approaches() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Carbon", placement(2.0, 0.0)));
        let id_a = structure.physical_id(a).unwrap();
        let id_b = structure.physical_id(b).unwrap();
        structure.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(
                id_a,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                id_b,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::PI,
                },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });

        let snapshot = MolecularLayoutSnapshot::analyze(&structure, &catalog).unwrap();
        assert_eq!(snapshot.bonds.len(), 1);
        assert!(snapshot.unbonded_approaches.is_empty());
    }

    #[test]
    fn invalid_bond_geometry_rejects_the_layout_snapshot() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", placement(0.0, 0.0)));
        let b = structure.add_unit(StructuralUnit::new("Carbon", placement(2.0, 0.0)));
        let id_a = structure.physical_id(a).unwrap();
        let id_b = structure.physical_id(b).unwrap();
        structure.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(
                id_a,
                ConnectionEndpoint::Boundary {
                    angle_radians: f64::NAN,
                },
            ),
            endpoint_b: BondEndpoint::new(
                id_b,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            strength: 0.5,
            bond_energy: 1.0,
        });

        assert!(MolecularLayoutSnapshot::analyze(&structure, &catalog).is_none());
    }
}
