#![expect(
    dead_code,
    reason = "Staged construction helper retained for subsystem integration"
)]
use crate::construction_material_selection::{
    rank_available_construction_materials, MIN_CONSTRUCTION_MATERIAL_MATCH,
};
use crate::resources::{BaseResource, Form, PhysicalState};
use crate::state::EnergyLedger;
use crate::structural_blueprint::BlueprintPlacement;
use crate::structure::{ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};

fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> Option<&'a BaseResource> {
    catalog.iter().find(|r| r.name == name)
}

fn placement(p: BlueprintPlacement) -> Placement {
    Placement {
        x: p.x,
        y: p.y,
        rotation_radians: p.rotation_radians,
    }
}
pub(crate) fn candidate_placements(
    structure: &OrganismStructure,
    resource: &BaseResource,
    anchor: Placement,
    targets: &[usize],
    catalog: &[BaseResource],
) -> Vec<Placement> {
    let mut out = vec![anchor];
    for &target in targets {
        let Some(unit) = structure.units.get(target) else {
            continue;
        };
        let Some(target_shape) = unit.shape(catalog) else {
            continue;
        };
        let target_endpoints: Vec<ConnectionEndpoint> = match &target_shape.form {
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
                let count = target_shape.form.polygon_vertices().map_or(0, |v| v.len());
                (0..count)
                    .map(|i| ConnectionEndpoint::Corner { point_index: i })
                    .collect()
            }
            Form::Line { .. } => (0..2)
                .map(|i| ConnectionEndpoint::LineEndpoint { point_index: i })
                .collect(),
            Form::Circle { .. } | Form::Fluid { .. } => Vec::new(),
        };
        for te in target_endpoints {
            let Some(tp) = te.world_point(unit, catalog) else {
                continue;
            };
            match (&resource.shape.form, te) {
                (
                    Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
                    ConnectionEndpoint::Corner {
                        point_index: target_index,
                    },
                ) => {
                    let Some(candidate_count) =
                        resource.shape.form.polygon_vertices().map(|v| v.len())
                    else {
                        continue;
                    };
                    for candidate_index in 0..candidate_count {
                        if let (Some(candidate_normal), Some(target_normal)) = (
                            crate::rigid_boundary::corner_normal(&resource.shape, candidate_index),
                            crate::rigid_boundary::corner_normal(target_shape, target_index),
                        ) {
                            let candidate_angle = candidate_normal.1.atan2(candidate_normal.0);
                            let target_angle = target_normal.1.atan2(target_normal.0);
                            let rotation = target_angle + std::f64::consts::PI - candidate_angle;
                            if let Some(local) = crate::rigid_boundary::world_vertex(
                                &resource.shape,
                                candidate_index,
                                Placement {
                                    x: 0.0,
                                    y: 0.0,
                                    rotation_radians: rotation,
                                },
                            ) {
                                out.push(Placement {
                                    x: tp.x - local.0,
                                    y: tp.y - local.1,
                                    rotation_radians: rotation,
                                });
                            }
                        }
                        for rotation in crate::rigid_boundary::corner_alignment_rotations(
                            &resource.shape,
                            candidate_index,
                            target_shape,
                            target_index,
                            unit.placement.rotation_radians,
                        ) {
                            let Some(local) = crate::rigid_boundary::world_vertex(
                                &resource.shape,
                                candidate_index,
                                Placement {
                                    x: 0.0,
                                    y: 0.0,
                                    rotation_radians: rotation,
                                },
                            ) else {
                                continue;
                            };
                            out.push(Placement {
                                x: tp.x - local.0,
                                y: tp.y - local.1,
                                rotation_radians: rotation,
                            });
                        }
                    }
                }
                (
                    Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
                    ConnectionEndpoint::LineEndpoint {
                        point_index: target_index,
                    },
                ) => {
                    let Some(candidate_count) =
                        resource.shape.form.polygon_vertices().map(|v| v.len())
                    else {
                        continue;
                    };
                    let Some(target_normal) =
                        crate::rigid_boundary::line_endpoint_normal(target_shape, target_index)
                    else {
                        continue;
                    };
                    let target_normal_angle = target_normal.1.atan2(target_normal.0);
                    for candidate_index in 0..candidate_count {
                        let Some(candidate_normal) =
                            crate::rigid_boundary::corner_normal(&resource.shape, candidate_index)
                        else {
                            continue;
                        };
                        let candidate_normal_angle = candidate_normal.1.atan2(candidate_normal.0);
                        let rotation =
                            target_normal_angle + std::f64::consts::PI - candidate_normal_angle;
                        let Some(local) = crate::rigid_boundary::world_vertex(
                            &resource.shape,
                            candidate_index,
                            Placement {
                                x: 0.0,
                                y: 0.0,
                                rotation_radians: rotation,
                            },
                        ) else {
                            continue;
                        };
                        out.push(Placement {
                            x: tp.x - local.0,
                            y: tp.y - local.1,
                            rotation_radians: rotation,
                        });
                    }
                }
                (
                    Form::Line {
                        length: candidate_length,
                    },
                    ConnectionEndpoint::Corner {
                        point_index: target_index,
                    },
                ) => {
                    let Some(target_normal) =
                        crate::rigid_boundary::corner_normal(target_shape, target_index)
                    else {
                        continue;
                    };
                    let target_normal_angle = target_normal.1.atan2(target_normal.0);
                    let half = *candidate_length / 2.0;
                    for candidate_index in 0..2 {
                        let candidate_normal = crate::rigid_boundary::line_endpoint_normal(
                            &resource.shape,
                            candidate_index,
                        )
                        .unwrap();
                        let candidate_normal_angle = candidate_normal.1.atan2(candidate_normal.0);
                        let rotation =
                            target_normal_angle + std::f64::consts::PI - candidate_normal_angle;
                        let local_x = if candidate_index == 0 { -half } else { half };
                        let (s, c) = rotation.sin_cos();
                        let lx = local_x * c;
                        let ly = local_x * s;
                        out.push(Placement {
                            x: tp.x - lx,
                            y: tp.y - ly,
                            rotation_radians: rotation,
                        });
                    }
                }
                (
                    Form::Line {
                        length: candidate_length,
                    },
                    ConnectionEndpoint::LineEndpoint {
                        point_index: target_index,
                    },
                ) => {
                    if !matches!(target_shape.form, Form::Line { .. }) {
                        continue;
                    }
                    let half = *candidate_length / 2.0;
                    for candidate_index in 0..2 {
                        let candidate_endpoint_x = if candidate_index == 0 { -half } else { half };
                        for rotation in crate::rigid_boundary::line_endpoint_alignment_rotations(
                            candidate_index,
                            target_index,
                            unit.placement.rotation_radians,
                        ) {
                            let (s, c) = rotation.sin_cos();
                            let lx = candidate_endpoint_x * c;
                            let ly = candidate_endpoint_x * s;
                            out.push(Placement {
                                x: tp.x - lx,
                                y: tp.y - ly,
                                rotation_radians: rotation,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out.sort_by(|a, b| {
        (a.x - anchor.x)
            .hypot(a.y - anchor.y)
            .partial_cmp(&(b.x - anchor.x).hypot(b.y - anchor.y))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.dedup_by(|a, b| {
        (a.x - b.x).abs() <= 1e-10
            && (a.y - b.y).abs() <= 1e-10
            && (a.rotation_radians - b.rotation_radians).abs() <= 1e-10
    });
    out
}

fn structure_unit_endpoint_options(
    unit: &StructuralUnit,
    catalog: &[BaseResource],
) -> Vec<ConnectionEndpoint> {
    let Some(shape) = unit.shape(catalog) else {
        return Vec::new();
    };
    match &shape.form {
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => shape
            .form
            .polygon_vertices()
            .map(|vertices| {
                (0..vertices.len())
                    .map(|point_index| ConnectionEndpoint::Corner { point_index })
                    .collect()
            })
            .unwrap_or_default(),
        Form::Line { .. } => (0..2)
            .map(|point_index| ConnectionEndpoint::LineEndpoint { point_index })
            .collect(),
        Form::Circle { .. } | Form::Fluid { .. } => Vec::new(),
    }
}

fn placement_for_joint(
    local_point: (f64, f64),
    joint: (f64, f64),
    rotation_radians: f64,
) -> Placement {
    let (s, c) = rotation_radians.sin_cos();
    Placement {
        x: joint.0 - (local_point.0 * c - local_point.1 * s),
        y: joint.1 - (local_point.0 * s + local_point.1 * c),
        rotation_radians,
    }
}

/// Convert a suggested world pose for one constituent into the origin pose
/// required to restore its intact composite. Bob suggests the constituent's
/// pose; restore_material expects the composite origin and transforms every
/// constituent from its stored relative placement.
fn composite_origin_for_constituent_pose(
    suggested_constituent: Placement,
    relative_constituent: Placement,
) -> Placement {
    let rotation_radians =
        suggested_constituent.rotation_radians - relative_constituent.rotation_radians;
    let (s, c) = rotation_radians.sin_cos();
    Placement {
        x: suggested_constituent.x - relative_constituent.x * c + relative_constituent.y * s,
        y: suggested_constituent.y - relative_constituent.x * s - relative_constituent.y * c,
        rotation_radians,
    }
}

/// Canonical Bob identity for the rigid portion of a live organism. The
/// map is keyed by stable physical IDs, never transient vector positions.
#[derive(Clone, Debug)]
struct CanonicalLiveRigidParent {
    formation: crate::geometry_reference_library::GeometryFormation,
    canonical_index_by_id:
        std::collections::HashMap<crate::structure::PhysicalConstituentId, usize>,
    canonical_to_live_rotation: f64,
    canonical_to_live_x: f64,
    canonical_to_live_y: f64,
}

/// Build a connected rigid parent from the live physical graph and preserve
/// stable-ID-to-canonical-slot identity. Disconnected or ambiguous parents are
/// rejected rather than guessed.
fn canonical_live_rigid_parent(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Option<CanonicalLiveRigidParent> {
    use crate::geometry_reference_library::{
        GeometryBond, GeometryConstituent, GeometryFormation, GEOMETRY_LIBRARY_SCHEMA_VERSION,
    };
    use crate::structure::PhysicalConstituentId;
    use std::collections::{BTreeSet, HashMap};

    let mut live_units = Vec::new();
    let mut index_by_id = HashMap::new();
    for unit in &structure.units {
        let [(resource_name, amount)] = unit.material.parts.as_slice() else {
            continue;
        };
        if unit.material.has_internal_structure()
            || !amount.is_finite()
            || (*amount - 1.0).abs() > f64::EPSILON
        {
            continue;
        }
        let resource = catalog
            .iter()
            .find(|resource| resource.name == *resource_name)?;
        if resource.physical_state == PhysicalState::Fluid {
            continue;
        }
        if unit.physical_id.0 == 0 || index_by_id.contains_key(&unit.physical_id) {
            return None;
        }
        let index = live_units.len();
        index_by_id.insert(unit.physical_id, index);
        live_units.push((
            unit.physical_id,
            GeometryConstituent {
                resource: resource.name.clone(),
                placement: unit.placement,
            },
        ));
    }
    if live_units.is_empty() {
        return None;
    }

    let mut bonds = BTreeSet::new();
    for bond in &structure.bonds {
        let Some(&a) = index_by_id.get(&bond.endpoint_a.constituent_id) else {
            continue;
        };
        let Some(&b) = index_by_id.get(&bond.endpoint_b.constituent_id) else {
            continue;
        };
        if a != b {
            bonds.insert((a.min(b), a.max(b)));
        }
    }

    let formation = GeometryFormation {
        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
        constituents: live_units
            .iter()
            .map(|(_, constituent)| constituent.clone())
            .collect(),
        bonds: bonds
            .into_iter()
            .map(|(constituent_a, constituent_b)| GeometryBond {
                constituent_a,
                constituent_b,
            })
            .collect(),
        signature: String::new(),
    };
    let (formation, input_to_canonical) = formation.canonicalized_with_mapping(catalog)?;
    if input_to_canonical.len() != live_units.len() {
        return None;
    }
    let canonical_index_by_id = live_units
        .iter()
        .enumerate()
        .map(|(input_index, (id, _))| (*id, input_to_canonical[input_index]))
        .collect();
    let canonical_index_by_id: HashMap<PhysicalConstituentId, usize> = canonical_index_by_id;

    // Recover the rigid transform from Bob's canonical frame to the live graph.
    // Prefer constituent centers (independent of local shape symmetry); for a
    // singleton parent, use its realized orientation as the frame reference.
    let mut frame_rotation = None;
    'pairs: for first in 0..live_units.len() {
        for second in (first + 1)..live_units.len() {
            let first_canonical = &formation.constituents[input_to_canonical[first]].placement;
            let second_canonical = &formation.constituents[input_to_canonical[second]].placement;
            let canonical_dx = second_canonical.x - first_canonical.x;
            let canonical_dy = second_canonical.y - first_canonical.y;
            let live_dx = live_units[second].1.placement.x - live_units[first].1.placement.x;
            let live_dy = live_units[second].1.placement.y - live_units[first].1.placement.y;
            if canonical_dx.hypot(canonical_dy) > 1e-9 && live_dx.hypot(live_dy) > 1e-9 {
                frame_rotation = Some(live_dy.atan2(live_dx) - canonical_dy.atan2(canonical_dx));
                break 'pairs;
            }
        }
    }
    let canonical_to_live_rotation = frame_rotation.unwrap_or_else(|| {
        let canonical = &formation.constituents[input_to_canonical[0]].placement;
        live_units[0].1.placement.rotation_radians - canonical.rotation_radians
    });
    let (sin, cos) = canonical_to_live_rotation.sin_cos();
    let canonical_anchor = &formation.constituents[input_to_canonical[0]].placement;
    let canonical_to_live_x =
        live_units[0].1.placement.x - (canonical_anchor.x * cos - canonical_anchor.y * sin);
    let canonical_to_live_y =
        live_units[0].1.placement.y - (canonical_anchor.x * sin + canonical_anchor.y * cos);

    for (input_index, (_, live_constituent)) in live_units.iter().enumerate() {
        let canonical = &formation.constituents[input_to_canonical[input_index]].placement;
        let predicted_x = canonical_to_live_x + canonical.x * cos - canonical.y * sin;
        let predicted_y = canonical_to_live_y + canonical.x * sin + canonical.y * cos;
        if (predicted_x - live_constituent.placement.x)
            .hypot(predicted_y - live_constituent.placement.y)
            > 0.1
        {
            return None;
        }
    }

    Some(CanonicalLiveRigidParent {
        formation,
        canonical_index_by_id,
        canonical_to_live_rotation,
        canonical_to_live_x,
        canonical_to_live_y,
    })
}

/// Query Bob for fluid suggestions against the complete connected rigid parent.
/// Legacy records are keyed by one anchor; multi-contact records are eligible
/// from any constituent they explicitly contact. Live placement and transaction
/// validation remain authoritative.
fn bob_fluid_origins_for_live_parent(
    library: &crate::geometry_reference_library::GeometryLibrary,
    parent: &CanonicalLiveRigidParent,
    anchor_id: crate::structure::PhysicalConstituentId,
    fluid_resource: &BaseResource,
    relative_part_placement: Placement,
    catalog: &[BaseResource],
) -> Vec<(Placement, crate::resources::Shape)> {
    if fluid_resource.physical_state != PhysicalState::Fluid {
        return Vec::new();
    }
    let Some(&anchor_canonical_index) = parent.canonical_index_by_id.get(&anchor_id) else {
        return Vec::new();
    };
    let (s, c) = parent.canonical_to_live_rotation.sin_cos();
    let mut out = Vec::new();
    for suggestion in
        library.fluid_formations_for_parent(&parent.formation.signature, &fluid_resource.name)
    {
        // Legacy records are anchored to one constituent. A simultaneous
        // boundary can be discovered from any of its constrained contacts.
        let matches_requested_anchor = if suggestion.contact_constraints.is_empty() {
            suggestion.anchor_constituent == anchor_canonical_index
        } else {
            suggestion
                .contact_constraints
                .iter()
                .any(|constraint| constraint.constituent_index == anchor_canonical_index)
        };
        if !matches_requested_anchor {
            continue;
        }
        for bulge_left in [true, false] {
            let Some((shape, canonical_pose)) =
                suggestion.realize_against_parent(&parent.formation, catalog, bulge_left)
            else {
                continue;
            };
            let live_pose = Placement {
                x: parent.canonical_to_live_x + canonical_pose.x * c - canonical_pose.y * s,
                y: parent.canonical_to_live_y + canonical_pose.x * s + canonical_pose.y * c,
                rotation_radians: parent.canonical_to_live_rotation
                    + canonical_pose.rotation_radians,
            };
            out.push((
                composite_origin_for_constituent_pose(live_pose, relative_part_placement),
                shape,
            ));
        }
    }
    out
}

/// Complete every rigid connection represented by a multi-contact fluid
/// boundary on the isolated trial structure. Any missing/invalid bond rejects
/// the whole candidate; the caller discards this clone, so no partial set leaks.
fn form_remaining_multi_contact_bonds(
    structure: &mut OrganismStructure,
    fluid_unit_index: usize,
    first_target_index: usize,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> bool {
    // Treat this as one transaction: if any segment cannot be realized as a
    // distinct live bond, leave the caller's structure and energy untouched.
    let mut staged_structure = structure.clone();
    let mut staged_ledger = *ledger;
    let mut staged_energy = *energy;
    if !form_remaining_multi_contact_bonds_in_place(
        &mut staged_structure,
        fluid_unit_index,
        first_target_index,
        catalog,
        &mut staged_ledger,
        &mut staged_energy,
    ) {
        return false;
    }
    *structure = staged_structure;
    *ledger = staged_ledger;
    *energy = staged_energy;
    true
}

fn form_remaining_multi_contact_bonds_in_place(
    structure: &mut OrganismStructure,
    fluid_unit_index: usize,
    first_target_index: usize,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> bool {
    let Some(fluid_unit) = structure.units.get(fluid_unit_index) else {
        return false;
    };
    let Some(fluid_shape) = fluid_unit.shape(catalog) else {
        return false;
    };
    let Form::Fluid {
        multi_contact_boundary: Some(boundary),
        ..
    } = &fluid_shape.form
    else {
        return true;
    };
    let boundary = boundary.clone();
    let contact_segments = boundary
        .pieces
        .iter()
        .filter_map(|piece| match piece {
            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                start_x,
                start_y,
                end_x,
                end_y,
                ..
            } => Some(((*start_x, *start_y), (*end_x, *end_y))),
            crate::capillary_geometry::FluidBoundaryPiece::FreeArc(_) => None,
        })
        .collect::<Vec<_>>();
    if contact_segments.len() < 2 || first_target_index == fluid_unit_index {
        return false;
    }

    let fluid_id = structure.units[fluid_unit_index].physical_id;
    let tolerance = crate::combine_runtime::COMBINE_CONTACT_TOLERANCE;
    let point_segment_distance = |point: (f64, f64), segment: ((f64, f64), (f64, f64))| {
        let ((sx, sy), (ex, ey)) = segment;
        let dx = ex - sx;
        let dy = ey - sy;
        let length_squared = dx * dx + dy * dy;
        if length_squared <= f64::EPSILON {
            return f64::INFINITY;
        }
        let t = (((point.0 - sx) * dx + (point.1 - sy) * dy) / length_squared).clamp(0.0, 1.0);
        (point.0 - (sx + t * dx)).hypot(point.1 - (sy + t * dy))
    };
    let fluid_endpoint_on_segment =
        |endpoint: crate::structure::ConnectionEndpoint, segment: ((f64, f64), (f64, f64))| {
            match endpoint {
                crate::structure::ConnectionEndpoint::Fluid { x, y } => {
                    point_segment_distance((x, y), segment) <= tolerance
                }
                _ => false,
            }
        };
    let bond_connects_fluid = |bond: &crate::structure::Bond, target_id| {
        (bond.endpoint_a.constituent_id == fluid_id && bond.endpoint_b.constituent_id == target_id)
            || (bond.endpoint_b.constituent_id == fluid_id
                && bond.endpoint_a.constituent_id == target_id)
    };
    let fluid_location_for_bond = |bond: &crate::structure::Bond| {
        if bond.endpoint_a.constituent_id == fluid_id {
            Some(bond.endpoint_a.location)
        } else if bond.endpoint_b.constituent_id == fluid_id {
            Some(bond.endpoint_b.location)
        } else {
            None
        }
    };

    // The first bond was formed by the caller. It must correspond to one of
    // the persisted solid-contact segments; arbitrary nearby contacts cannot
    // be used as a substitute for the catalogue's boundary contract.
    let Some(first_target) = structure.units.get(first_target_index) else {
        return false;
    };
    if !structure.bonds.iter().any(|bond| {
        bond_connects_fluid(bond, first_target.physical_id)
            && fluid_location_for_bond(bond).is_some_and(|location| {
                contact_segments
                    .iter()
                    .any(|segment| fluid_endpoint_on_segment(location, *segment))
            })
    }) {
        return false;
    }

    // Mark segments already bonded by the initial COMBINE operation. This is
    // based on the fluid endpoint's actual local position, not the number of
    // distinct rigid units: one rigid unit may legitimately receive multiple
    // bonds at different positions.
    let mut covered = vec![false; contact_segments.len()];
    for bond in &structure.bonds {
        if bond.endpoint_a.constituent_id != fluid_id && bond.endpoint_b.constituent_id != fluid_id
        {
            continue;
        }
        let Some(crate::structure::ConnectionEndpoint::Fluid { x, y }) =
            fluid_location_for_bond(bond)
        else {
            continue;
        };
        let point = (x, y);
        if let Some((index, _distance)) = contact_segments
            .iter()
            .enumerate()
            .map(|(index, segment)| (index, point_segment_distance(point, *segment)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .filter(|(_, distance)| *distance <= tolerance)
        {
            covered[index] = true;
        }
    }

    for (segment_index, segment) in contact_segments.iter().copied().enumerate() {
        if covered[segment_index] {
            continue;
        }

        let mut choices = Vec::new();
        for target_index in 0..structure.units.len() {
            if target_index == fluid_unit_index {
                continue;
            }
            let Some(target) = structure.units.get(target_index) else {
                return false;
            };
            let Some(_target_shape) = target.shape(catalog) else {
                return false;
            };
            let Some(material_name) = target.material.parts.first().map(|part| part.0.as_str())
            else {
                return false;
            };
            let Some(material_resource) = resource(catalog, material_name) else {
                return false;
            };
            if material_resource.physical_state == PhysicalState::Fluid {
                continue;
            }

            let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
            let mut candidates = crate::contact::connection_pair_candidates_cached(
                structure,
                target_index,
                fluid_unit_index,
                catalog,
                &mut bond_cache,
            )
            .into_iter()
            .filter(|candidate| {
                candidate.distance <= tolerance
                    && candidate.available_a
                    && candidate.available_b
                    && fluid_endpoint_on_segment(candidate.endpoint_b, segment)
            })
            .collect::<Vec<_>>();
            candidates.sort_by(|a, b| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        b.facing
                            .partial_cmp(&a.facing)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            for candidate in candidates {
                let Some((_, _, investment, _required_energy)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        structure,
                        target_index,
                        fluid_unit_index,
                        candidate,
                        catalog,
                    )
                else {
                    continue;
                };
                choices.push((
                    candidate.distance,
                    -candidate.facing,
                    target_index,
                    candidate,
                    investment,
                ));
            }
        }

        choices.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.2.cmp(&b.2))
        });

        let mut bonded = false;
        for (_, _, target_index, candidate, investment) in choices {
            let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
            if crate::combine_runtime::form_selected_bond(
                structure,
                target_index,
                fluid_unit_index,
                candidate,
                investment,
                catalog,
                &mut bond_cache,
                ledger,
                energy,
            )
            .is_some()
            {
                bonded = true;
                break;
            }
        }
        if !bonded {
            return false;
        }
        covered[segment_index] = true;
    }

    // Each persisted solid-contact segment must be matched to a distinct bond.
    covered.into_iter().all(|is_covered| is_covered)
}

fn overlapping_unit_indices(
    structure: &OrganismStructure,
    candidate: &StructuralUnit,
    ignored_units: &[usize],
    catalog: &[BaseResource],
) -> Vec<usize> {
    let Some(candidate_shape) = candidate.shape(catalog) else {
        return structure
            .units
            .iter()
            .enumerate()
            .filter(|(index, _)| !ignored_units.contains(index))
            .map(|(index, _)| index)
            .collect();
    };
    let candidate_part = crate::material_geometry::PlacedMaterialPart {
        part_index: 0,
        form: candidate_shape.form.clone(),
        placement: candidate.placement,
    };
    structure
        .units
        .iter()
        .enumerate()
        .filter_map(|(index, unit)| {
            if ignored_units.contains(&index) {
                return None;
            }
            let Some(shape) = unit.shape(catalog) else {
                return Some(index);
            };
            let existing_part = crate::material_geometry::PlacedMaterialPart {
                part_index: index + 1,
                form: shape.form.clone(),
                placement: unit.placement,
            };
            crate::material_geometry::placed_forms_penetrate(
                &candidate_part,
                &existing_part,
                crate::material_geometry::GEOMETRY_EPSILON,
            )
            .then_some(index)
        })
        .collect()
}

pub(crate) fn placed_unit_overlaps(
    structure: &OrganismStructure,
    candidate: &StructuralUnit,
    ignored_units: &[usize],
    catalog: &[BaseResource],
) -> bool {
    !overlapping_unit_indices(structure, candidate, ignored_units, catalog).is_empty()
}

fn point_in_triangle(point: (f64, f64), triangle: &[(f64, f64); 3]) -> bool {
    fn cross(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
        (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
    }
    let a = cross(triangle[0], triangle[1], point);
    let b = cross(triangle[1], triangle[2], point);
    let c = cross(triangle[2], triangle[0], point);
    (a >= -1e-10 && b >= -1e-10 && c >= -1e-10) || (a <= 1e-10 && b <= 1e-10 && c <= 1e-10)
}

fn form_vertices_world(form: &Form, placement: Placement) -> Vec<(f64, f64)> {
    form.polygon_vertices()
        .unwrap_or_default()
        .into_iter()
        .map(|(x, y)| {
            let (s, c) = placement.rotation_radians.sin_cos();
            (placement.x + x * c - y * s, placement.y + x * s + y * c)
        })
        .collect()
}

fn point_inside_form(form: &Form, placement: Placement, x: f64, y: f64) -> bool {
    match form {
        Form::Circle { radius } => (x - placement.x).hypot(y - placement.y) <= *radius,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
            let Some(vertices) = form.polygon_vertices() else {
                return false;
            };
            let (s, c) = placement.rotation_radians.sin_cos();
            let local_x = (x - placement.x) * c + (y - placement.y) * s;
            let local_y = -(x - placement.x) * s + (y - placement.y) * c;
            let mut inside = false;
            for i in 0..vertices.len() {
                let a = vertices[i];
                let b = vertices[(i + 1) % vertices.len()];
                if (a.1 > local_y) != (b.1 > local_y)
                    && local_x < (b.0 - a.0) * (local_y - a.1) / (b.1 - a.1) + a.0
                {
                    inside = !inside;
                }
            }
            inside
        }
        Form::Line { .. } | Form::Fluid { .. } => false,
    }
}

fn already_realized_neighbors(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    index: usize,
    realized: &[bool],
) -> Vec<usize> {
    blueprint
        .connections
        .iter()
        .filter_map(|connection| {
            let neighbor = if connection.element_a == index {
                connection.element_b
            } else if connection.element_b == index {
                connection.element_a
            } else {
                return None;
            };
            realized
                .get(neighbor)
                .copied()
                .filter(|v| *v)
                .map(|_| neighbor)
        })
        .collect()
}

/// Sequential developmental constructor: choose A.x and B.y, rotate B about
/// that exact joint, and keep searching all available endpoint pairs and
/// orientations until a physically valid joint is found. In particular, when
/// a later connection would overshoot a previously established anchor, the
/// constructor tries the other connection points on that anchor rather than
/// abandoning the cavity.
fn physical_material_endpoint_options(
    instance: &crate::physical_material::PhysicalMaterial,
    catalog: &[BaseResource],
) -> Vec<(usize, ConnectionEndpoint)> {
    let Some(placements) = instance.placements.as_ref() else {
        return Vec::new();
    };
    instance
        .material
        .parts
        .iter()
        .zip(placements.iter())
        .enumerate()
        .flat_map(|(part_index, ((name, amount), placement))| {
            if (*amount - 1.0).abs() > 1e-9 {
                return Vec::new();
            }
            let Some(unit) = StructuralUnit::from_material(
                crate::resources::Material::free_base(name.clone(), *amount),
                *placement,
            ) else {
                return Vec::new();
            };
            let mut endpoints = crate::contact::endpoint_indices(&unit, catalog)
                .into_iter()
                .map(move |endpoint| (part_index, endpoint))
                .collect::<Vec<_>>();
            if endpoints.is_empty()
                && resource(catalog, name.as_str())
                    .is_some_and(|resource| resource.physical_state == PhysicalState::Fluid)
            {
                if let Some(endpoint) = crate::contact::continuous_endpoint(
                    &unit,
                    crate::contact::world_center(&unit),
                    catalog,
                ) {
                    endpoints.push((part_index, endpoint));
                }
            }
            endpoints
        })
        .collect()
}

fn physical_material_endpoint_local_point(
    instance: &crate::physical_material::PhysicalMaterial,
    part_index: usize,
    endpoint: ConnectionEndpoint,
    catalog: &[BaseResource],
) -> Option<crate::connection_geometry::WorldConnectionPoint> {
    let placements = instance.placements.as_ref()?;
    let (name, amount) = instance.material.parts.get(part_index)?;
    let placement = *placements.get(part_index)?;
    let unit = StructuralUnit::from_material(
        crate::resources::Material::free_base(name.clone(), *amount),
        placement,
    )?;
    endpoint.world_point(&unit, catalog)
}

fn normalize_construction_angle(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

fn construction_angle_candidates(
    existing_shape: &crate::resources::Shape,
    existing_endpoint: ConnectionEndpoint,
    existing_rotation: f64,
    candidate_shape: &crate::resources::Shape,
    candidate_endpoint: ConnectionEndpoint,
    candidate_relative_rotation: f64,
    ideal_angle: f64,
) -> Vec<f64> {
    let mut angles = Vec::new();
    let mut push_unique = |angle: f64| {
        let normalized = normalize_construction_angle(angle);
        if !angles.iter().any(|current: &f64| {
            (normalize_construction_angle(*current - normalized)).abs() <= 1e-10
        }) {
            angles.push(normalized);
        }
    };

    // Developmental geometry has priority over the declared pose.
    // Face/edge alignments are exact consequences of the actual boundaries;
    // the blueprint angle is only the final preference fallback.
    match (existing_endpoint, candidate_endpoint) {
        (
            ConnectionEndpoint::Corner {
                point_index: existing_index,
            },
            ConnectionEndpoint::Corner {
                point_index: candidate_index,
            },
        ) => {
            for angle in crate::rigid_boundary::corner_alignment_rotations(
                candidate_shape,
                candidate_index,
                existing_shape,
                existing_index,
                existing_rotation,
            ) {
                push_unique(angle - candidate_relative_rotation);
            }
        }
        (
            ConnectionEndpoint::LineEndpoint {
                point_index: existing_index,
            },
            ConnectionEndpoint::LineEndpoint {
                point_index: candidate_index,
            },
        ) => {
            for angle in crate::rigid_boundary::line_endpoint_alignment_rotations(
                candidate_index,
                existing_index,
                existing_rotation,
            ) {
                push_unique(angle - candidate_relative_rotation);
            }
        }
        (
            ConnectionEndpoint::Corner {
                point_index: existing_index,
            },
            ConnectionEndpoint::LineEndpoint {
                point_index: candidate_index,
            },
        ) => {
            if let (Some(a), Some(b)) = (
                crate::rigid_boundary::corner_normal(existing_shape, existing_index),
                crate::rigid_boundary::line_endpoint_normal(candidate_shape, candidate_index),
            ) {
                push_unique(
                    a.1.atan2(a.0) + std::f64::consts::PI
                        - b.1.atan2(b.0)
                        - candidate_relative_rotation,
                );
            }
        }
        (
            ConnectionEndpoint::LineEndpoint {
                point_index: existing_index,
            },
            ConnectionEndpoint::Corner {
                point_index: candidate_index,
            },
        ) => {
            if let (Some(a), Some(b)) = (
                crate::rigid_boundary::line_endpoint_normal(existing_shape, existing_index),
                crate::rigid_boundary::corner_normal(candidate_shape, candidate_index),
            ) {
                push_unique(
                    a.1.atan2(a.0) + std::f64::consts::PI
                        - b.1.atan2(b.0)
                        - candidate_relative_rotation,
                );
            }
        }
        _ => {}
    }

    // If no exact boundary alignment was available, the declared pose is
    // still a preference. Do not manufacture arbitrary angular samples.
    push_unique(ideal_angle);
    angles
}

pub(crate) fn try_attach_physical_material_bond_driven(
    structure: &OrganismStructure,
    existing_index: usize,
    new_material: &crate::physical_material::PhysicalMaterial,
    catalog: &[BaseResource],
    nodes: &mut usize,
    ledger: &EnergyLedger,
    available_energy: f64,
) -> Option<(
    OrganismStructure,
    Vec<usize>,
    usize,
    crate::combine_runtime::CombineAttempt,
    EnergyLedger,
    f64,
)> {
    let existing_unit = structure.units.get(existing_index)?;
    let existing_endpoints = structure_unit_endpoint_options(existing_unit, catalog);
    let new_endpoints = physical_material_endpoint_options(new_material, catalog);
    if existing_endpoints.is_empty() || new_endpoints.is_empty() {
        return None;
    }

    for endpoint_a in existing_endpoints {
        let joint = endpoint_a.world_point(existing_unit, catalog)?;
        for (part_index, endpoint_b) in new_endpoints.iter().copied() {
            let local_b = physical_material_endpoint_local_point(
                new_material,
                part_index,
                endpoint_b,
                catalog,
            )?;

            let Some(existing_shape) = existing_unit.shape(catalog) else {
                continue;
            };
            let Some(candidate_shape) = new_material
                .material
                .parts
                .get(part_index)
                .and_then(|(name, _)| resource(catalog, name))
                .map(|resource| &resource.shape)
            else {
                continue;
            };
            let angles = construction_angle_candidates(
                existing_shape,
                endpoint_a,
                existing_unit.placement.rotation_radians,
                candidate_shape,
                endpoint_b,
                new_material
                    .placements
                    .as_ref()
                    .and_then(|placements| placements.get(part_index))
                    .map(|placement| placement.rotation_radians)
                    .unwrap_or(0.0),
                0.0,
            );
            for angle in angles {
                let candidate_origin =
                    placement_for_joint((local_b.x, local_b.y), (joint.x, joint.y), angle);
                *nodes += 1;

                let mut trial = structure.clone();
                let Some(indices) = crate::material_restoration::restore_material(
                    &mut trial,
                    new_material,
                    candidate_origin,
                    catalog,
                ) else {
                    continue;
                };
                let new_unit_index = *indices.get(part_index)?;

                let ignored_units = indices.clone();
                if indices.iter().any(|index| {
                    placed_unit_overlaps(&trial, &trial.units[*index], &ignored_units, catalog)
                }) {
                    continue;
                }

                let mut trial_ledger = *ledger;
                let mut trial_energy = available_energy;
                let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
                let Some(candidate) = crate::contact::connection_pair_candidates_cached(
                    &trial,
                    existing_index,
                    new_unit_index,
                    catalog,
                    &mut bond_cache,
                )
                .into_iter()
                .find(|candidate| {
                    candidate.endpoint_a == endpoint_a
                        && candidate.endpoint_b == endpoint_b
                        && candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                }) else {
                    continue;
                };

                let Some((_, _, investment, _required_energy)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        &trial,
                        existing_index,
                        new_unit_index,
                        candidate,
                        catalog,
                    )
                else {
                    continue;
                };

                let Some(attempt) = crate::combine_runtime::form_selected_bond(
                    &mut trial,
                    existing_index,
                    new_unit_index,
                    candidate,
                    investment,
                    catalog,
                    &mut bond_cache,
                    &mut trial_ledger,
                    &mut trial_energy,
                ) else {
                    continue;
                };

                return Some((
                    trial,
                    indices,
                    part_index,
                    attempt,
                    trial_ledger,
                    trial_energy,
                ));
            }
        }
    }
    None
}

/// A new blueprint element may have several already-realized neighbors.
/// Do not commit its first bond at a pose that makes a declared, already-ready
/// connection geometrically impossible. This is a physical contact constraint,
/// not a demand to match the blueprint's suggested coordinates.
fn missing_realized_neighbor_contacts(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    element_index: usize,
    realized_units: &[Option<Vec<usize>>],
    structure: &OrganismStructure,
    candidate_indices: &[usize],
    catalog: &[BaseResource],
) -> Vec<usize> {
    let mut required_neighbors = Vec::new();
    for connection in &blueprint.connections {
        let neighbor = if connection.element_a == element_index {
            connection.element_b
        } else if connection.element_b == element_index {
            connection.element_a
        } else {
            continue;
        };
        if realized_units.get(neighbor).is_some_and(Option::is_some)
            && !required_neighbors.contains(&neighbor)
        {
            required_neighbors.push(neighbor);
        }
    }

    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    let mut missing_neighbors = Vec::new();
    for neighbor in required_neighbors {
        let has_contact = realized_units
            .get(neighbor)
            .and_then(Option::as_ref)
            .is_some_and(|existing_indices| {
                existing_indices.iter().any(|&existing_index| {
                    candidate_indices.iter().any(|&candidate_index| {
                        crate::contact::connection_pair_candidates_cached(
                            structure,
                            existing_index,
                            candidate_index,
                            catalog,
                            &mut cache,
                        )
                        .into_iter()
                        .any(|candidate| {
                            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                                && candidate.available_a
                                && candidate.available_b
                        })
                    })
                })
            });
        if !has_contact {
            missing_neighbors.push(neighbor);
        }
    }
    missing_neighbors
}

#[derive(Default, Debug)]
struct PlacementRejectionCounts {
    generated_origins: usize,
    deduplicated_origins: usize,
    attempted_origins: usize,
    restore_failures: usize,
    restore_failure_reasons: std::collections::BTreeMap<&'static str, usize>,
    overlap_rejections: usize,
    overlap_rejections_by_unit: std::collections::BTreeMap<usize, usize>,
    neighbor_contact_rejections: usize,
    no_bond_contact: usize,
    candidate_evaluation_rejections: usize,
    combine_rejections: usize,
    bob_coverage_misses: Vec<String>,
    neighbor_rejections_by_index: std::collections::BTreeMap<usize, usize>,
}

fn realize_next_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    structure: &OrganismStructure,
    realized_units: &[Option<Vec<usize>>],
    _index: usize,
    neighbor: usize,
    new_material: &crate::physical_material::PhysicalMaterial,
    nodes: &mut usize,
    rejection_counts: &mut PlacementRejectionCounts,
    ledger: &EnergyLedger,
    available_energy: f64,
    bob_library: Option<&crate::geometry_reference_library::GeometryLibrary>,
) -> Option<(
    OrganismStructure,
    Vec<usize>,
    usize,
    crate::combine_runtime::CombineAttempt,
    EnergyLedger,
    f64,
)> {
    let existing_indices = realized_units[neighbor].as_ref()?.clone();
    if existing_indices.is_empty() {
        return None;
    }

    let needs_fluid_suggestions = new_material.material.parts.iter().any(|(name, _)| {
        resource(catalog, name)
            .is_some_and(|candidate| candidate.physical_state == PhysicalState::Fluid)
    });
    let bob_parent = needs_fluid_suggestions
        .then(|| bob_library.and_then(|_| canonical_live_rigid_parent(structure, catalog)))
        .flatten();

    // Query once per anchor/physical constituent pair. The previous endpoint
    // cross-product repeatedly generated the same pose for every endpoint pair,
    // multiplying work before the candidate even reached collision validation.
    // The persisted formation proposes a pose; live contact discovery below
    // determines the actual bond endpoint.
    for existing_index in existing_indices {
        let existing_unit = structure.units.get(existing_index)?;
        let anchor_material = existing_unit.material.parts.first()?.0.as_str();
        for (part_index, (candidate_name, _)) in new_material.material.parts.iter().enumerate() {
            let Some(candidate_resource) = resource(catalog, candidate_name) else {
                continue;
            };
            let Some(relative_part_placement) = new_material
                .placements
                .as_ref()
                .and_then(|placements| placements.get(part_index))
                .copied()
            else {
                continue;
            };

            let mut candidate_origins = Vec::<(Placement, Option<crate::resources::Shape>)>::new();
            if let Some(library) = bob_library {
                if candidate_resource.physical_state == PhysicalState::Fluid {
                    // Exact-parent fluid records remain proposals only; the
                    // restored shape must pass live geometry and bond checks.
                    if let Some(parent) = bob_parent.as_ref() {
                        candidate_origins.extend(
                            bob_fluid_origins_for_live_parent(
                                library,
                                parent,
                                existing_unit.physical_id,
                                candidate_resource,
                                relative_part_placement,
                                catalog,
                            )
                            .into_iter()
                            .map(|(origin, shape)| (origin, Some(shape))),
                        );
                    }
                } else {
                    let suggestions = library.suggest_persisted_rigid_edge_placements(
                        anchor_material,
                        existing_unit.placement,
                        candidate_resource,
                        catalog,
                    );
                    if suggestions.is_empty() {
                        let miss = format!(
                            "anchor={} candidate={}",
                            anchor_material, candidate_resource.name
                        );
                        if !rejection_counts.bob_coverage_misses.contains(&miss) {
                            rejection_counts.bob_coverage_misses.push(miss);
                        }
                    }
                    candidate_origins.extend(suggestions.into_iter().map(|suggestion| {
                        (
                            composite_origin_for_constituent_pose(
                                suggestion.placement,
                                relative_part_placement,
                            ),
                            None,
                        )
                    }));
                }
            }
            let generated_origins = candidate_origins.len();
            rejection_counts.generated_origins += generated_origins;
            let mut seen_origins = Vec::<((i64, i64, i64), Option<crate::resources::Shape>)>::new();
            candidate_origins.retain(|(placement, shape)| {
                let key = (
                    (placement.x * 1e8).round() as i64,
                    (placement.y * 1e8).round() as i64,
                    (placement.rotation_radians * 1e8).round() as i64,
                );
                if seen_origins
                    .iter()
                    .any(|(seen_key, seen_shape)| *seen_key == key && seen_shape == shape)
                {
                    false
                } else {
                    seen_origins.push((key, shape.clone()));
                    true
                }
            });
            rejection_counts.deduplicated_origins +=
                generated_origins.saturating_sub(candidate_origins.len());
            for (candidate_origin, fluid_shape) in candidate_origins {
                *nodes += 1;
                rejection_counts.attempted_origins += 1;

                let mut trial = structure.clone();
                let candidate_material = if let Some(shape) = fluid_shape {
                    new_material.clone().with_single_fluid_shape(shape, catalog)
                } else {
                    Some(new_material.clone())
                };
                let Some(candidate_material) = candidate_material else {
                    rejection_counts.restore_failures += 1;
                    *rejection_counts
                        .restore_failure_reasons
                        .entry("stored fluid shape candidate rejected")
                        .or_default() += 1;
                    continue;
                };
                let indices = match crate::material_restoration::restore_material_diagnostic(
                    &mut trial,
                    &candidate_material,
                    candidate_origin,
                    catalog,
                ) {
                    Ok(indices) => indices,
                    Err(reason) => {
                        rejection_counts.restore_failures += 1;
                        *rejection_counts
                            .restore_failure_reasons
                            .entry(reason)
                            .or_default() += 1;
                        continue;
                    }
                };

                let new_unit_index = *indices.get(part_index)?;
                let ignored_units = indices.clone();
                let overlapping_units = indices
                    .iter()
                    .flat_map(|index| {
                        overlapping_unit_indices(
                            &trial,
                            &trial.units[*index],
                            &ignored_units,
                            catalog,
                        )
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                if !overlapping_units.is_empty() {
                    rejection_counts.overlap_rejections += 1;
                    for overlap_index in overlapping_units {
                        *rejection_counts
                            .overlap_rejections_by_unit
                            .entry(overlap_index)
                            .or_default() += 1;
                    }
                    continue;
                }
                let missing_neighbors = missing_realized_neighbor_contacts(
                    blueprint,
                    _index,
                    realized_units,
                    &trial,
                    &indices,
                    catalog,
                );
                if !missing_neighbors.is_empty() {
                    rejection_counts.neighbor_contact_rejections += 1;
                    for neighbor in missing_neighbors {
                        *rejection_counts
                            .neighbor_rejections_by_index
                            .entry(neighbor)
                            .or_default() += 1;
                    }
                    continue;
                }

                let mut trial_ledger = *ledger;
                let mut trial_energy = available_energy;
                let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();

                // The official connection points determine where the
                // constructor works from and how the new material is placed.
                // Once the material is physically placed, the actual bond may
                // land anywhere on the touching boundaries.
                let Some(candidate) = crate::contact::connection_pair_candidates_cached(
                    &trial,
                    existing_index,
                    new_unit_index,
                    catalog,
                    &mut bond_cache,
                )
                .into_iter()
                .filter(|candidate| {
                    candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        && candidate.available_a
                        && candidate.available_b
                })
                .min_by(|a, b| {
                    a.distance
                        .partial_cmp(&b.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| {
                            b.facing
                                .partial_cmp(&a.facing)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                }) else {
                    rejection_counts.no_bond_contact += 1;
                    continue;
                };

                let Some((_, _, investment, _required_energy)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        &trial,
                        existing_index,
                        new_unit_index,
                        candidate,
                        catalog,
                    )
                else {
                    rejection_counts.candidate_evaluation_rejections += 1;
                    continue;
                };

                let Some(attempt) = crate::combine_runtime::form_selected_bond(
                    &mut trial,
                    existing_index,
                    new_unit_index,
                    candidate,
                    investment,
                    catalog,
                    &mut bond_cache,
                    &mut trial_ledger,
                    &mut trial_energy,
                ) else {
                    rejection_counts.combine_rejections += 1;
                    continue;
                };

                if !form_remaining_multi_contact_bonds(
                    &mut trial,
                    new_unit_index,
                    existing_index,
                    catalog,
                    &mut trial_ledger,
                    &mut trial_energy,
                ) {
                    rejection_counts.combine_rejections += 1;
                    continue;
                }

                // The first exact, physically admissible continuation wins.
                // Construction is forward-only: do not continue searching for a
                // "better" pose after a permanent bond can already be formed.
                return Some((
                    trial,
                    indices,
                    part_index,
                    attempt,
                    trial_ledger,
                    trial_energy,
                ));
            }
        }
    }

    None
}

/// Bond-driven construction is forward-only. Once a bond is formed it is
/// never undone. When a requested joint cannot be realized, the constructor
/// keeps trying available material variants and orientations until it finds a
/// physically valid attachment. The resulting physical graph, not the declared
/// poses, is authoritative.
pub(crate) fn construct_blueprint_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(OrganismStructure, f64), String> {
    construct_blueprint_bond_driven_internal(blueprint, catalog, ledger, energy, None, false)
}

/// Bond-driven construction using actual physical inventory. The structural
/// blueprint supplies the material preference; storage supplies the material
/// that can actually be used. No candidate below the structural match
/// threshold is consumed.
/// Construct only the genome-forming phase. The constructor stops immediately
/// after a committed physical bond makes the realized cavity qualify. The
/// blueprint is still temporary calibration input; it is not consulted after
/// the milestone is reached.
pub(crate) fn construct_bond_driven_until_genome(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(OrganismStructure, f64), String> {
    construct_blueprint_bond_driven_internal(blueprint, catalog, ledger, energy, None, true)
}

pub(crate) fn construct_blueprint_bond_driven_with_materials(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    available_materials: &mut crate::material_storage::MaterialStorage,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<(OrganismStructure, f64), String> {
    construct_blueprint_bond_driven_internal(
        blueprint,
        catalog,
        ledger,
        energy,
        Some(available_materials),
        false,
    )
}

fn construct_blueprint_bond_driven_internal(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
    mut available_materials: Option<&mut crate::material_storage::MaterialStorage>,
    stop_at_genome: bool,
) -> Result<(OrganismStructure, f64), String> {
    if blueprint.elements.is_empty() {
        return Err("blueprint must contain at least one element".into());
    }
    if blueprint
        .elements
        .iter()
        .any(|element| element.material.parts.len() != 1)
    {
        return Err(
            "bond-driven developmental construction currently requires single-constituent elements"
                .into(),
        );
    }

    let anchor_index = *blueprint
        .anchor_elements
        .first()
        .ok_or_else(|| "blueprint has no construction anchor".to_string())?;
    let anchor_element = blueprint
        .elements
        .get(anchor_index)
        .ok_or_else(|| "construction anchor references a missing element".to_string())?;
    let anchor_preferred = anchor_element.material.parts[0].0.clone();

    // Strict live construction requires a readable Bob store. A missing or
    // invalid store is an infrastructure/coverage error, not permission to
    // silently switch to analytic pose generation.
    let bob_library = crate::geometry_reference_library::open_default_library()
        .map_err(|error| format!("failed to open persistent Bob geometry library: {error}"))?;

    let mut structure = OrganismStructure::new();
    let mut realized = vec![false; blueprint.elements.len()];
    // Each blueprint element maps to every physical unit restored for that
    // element. A stored composite remains one developmental element, but any
    // of its physical constituents may legitimately provide a later bond
    // endpoint.
    let mut realized_units = vec![None::<Vec<usize>>; blueprint.elements.len()];
    let mut construction_ledger = *ledger;
    let mut remaining_energy = *energy;
    let mut total_heat = 0.0;
    let mut nodes = 0usize;
    let mut reserved_storage_indices = Vec::<usize>::new();
    let mut closed_connections = vec![false; blueprint.connections.len()];

    let mut anchor_storage_index = None;
    let anchor_instance = if let Some(storage) = available_materials.as_deref_mut() {
        let candidates = rank_available_construction_materials(storage, &anchor_preferred, catalog)
            .map_err(|e| e.to_string())?;
        let (storage_index, _, _) = candidates
            .into_iter()
            .find(|(storage_index, _, score)| {
                !reserved_storage_indices.contains(storage_index)
                    && *score >= MIN_CONSTRUCTION_MATERIAL_MATCH
            })
            .ok_or_else(|| format!(
                "construction material need: preferred={anchor_preferred}, threshold={MIN_CONSTRUCTION_MATERIAL_MATCH:.6}"
            ))?;
        let crate::material_storage::StoredMaterial::Physical(instance) =
            storage.entries.get(storage_index).cloned().ok_or_else(|| {
                "selected construction anchor material disappeared from storage".to_string()
            })?;
        anchor_storage_index = Some(storage_index);
        instance
    } else {
        let anchor_resource = resource(catalog, &anchor_preferred)
            .ok_or_else(|| "construction anchor references an unknown resource".to_string())?;
        crate::physical_material::PhysicalMaterial::realized(
            crate::resources::Material::free_base(anchor_resource.name.clone(), 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            catalog,
        )
        .ok_or_else(|| "construction anchor has invalid geometry".to_string())?
    };

    let anchor_origin = placement(anchor_element.placement);
    let anchor_indices = crate::material_restoration::restore_material(
        &mut structure,
        &anchor_instance,
        anchor_origin,
        catalog,
    )
    .ok_or_else(|| "construction anchor has invalid physical realization".to_string())?;
    if let Some(storage_index) = anchor_storage_index {
        if available_materials
            .as_deref()
            .and_then(|storage| storage.entries.get(storage_index))
            .is_none()
        {
            return Err(
                "selected construction anchor material disappeared before commit".to_string(),
            );
        }
        reserved_storage_indices.push(storage_index);
    }
    if anchor_indices.is_empty() {
        return Err("construction anchor restored no physical units".to_string());
    }
    realized[anchor_index] = true;
    realized_units[anchor_index] = Some(anchor_indices.clone());
    while !realized.iter().all(|value| *value) {
        // Select the next element only from its currently realized neighbors.
        // Total blueprint degree is deliberately not a tie-breaker: that would
        // use knowledge of future connections to choose which bond gets formed
        // first. If several elements are equally ready, blueprint index provides
        // a deterministic order.
        let mut next: Option<(usize, Vec<usize>)> = None;
        for index in 0..blueprint.elements.len() {
            if realized[index] {
                continue;
            }
            let neighbors = already_realized_neighbors(blueprint, index, &realized);
            if neighbors.is_empty() {
                continue;
            }
            if next
                .as_ref()
                .is_none_or(|(current_index, current_neighbors)| {
                    neighbors.len() > current_neighbors.len()
                        || (neighbors.len() == current_neighbors.len() && index < *current_index)
                })
            {
                next = Some((index, neighbors));
            }
        }

        let Some((index, neighbors)) = next else {
            return Err(
                "bond-driven constructor reached an unrealized disconnected element".into(),
            );
        };

        // One bond, one committed construction step. The constructor does
        // not look ahead and reject a material because some later connection
        // might be difficult. The next construction step gets to solve that
        // next joint using whatever material is actually available then.
        let preferred = blueprint.elements[index].material.parts[0].0.clone();
        let candidate_resources = if let Some(storage) = available_materials.as_deref() {
            let ranked = rank_available_construction_materials(storage, &preferred, catalog)
                .map_err(|e| e.to_string())?;
            let candidates = ranked
                .iter()
                .filter(|(storage_index, _, score)| {
                    !reserved_storage_indices.contains(storage_index)
                        && *score >= MIN_CONSTRUCTION_MATERIAL_MATCH
                })
                .cloned()
                .collect::<Vec<_>>();

            candidates
        } else {
            // Developmental construction may substitute material when the
            // preferred material cannot make the requested physical bond.
            // Preference still controls the search order; physical geometry
            // and the shared bond authority decide what is actually admitted.
            let mut candidates = Vec::new();
            if resource(catalog, &preferred)
                .is_some_and(|candidate| candidate.physical_state == PhysicalState::Rigid)
            {
                candidates.push((usize::MAX, preferred.clone(), 1.0));
            }
            for candidate in catalog {
                if candidate.name == preferred || candidate.physical_state != PhysicalState::Rigid {
                    continue;
                }
                candidates.push((usize::MAX, candidate.name.clone(), 0.0));
            }
            // Water is the final developmental construction fallback after
            // all rigid material alternatives have failed.
            if resource(catalog, "Water").is_some() {
                candidates.push((usize::MAX, "Water".to_string(), 0.0));
            }
            candidates
        };

        if candidate_resources.is_empty() && available_materials.is_some() {
            let best = rank_available_construction_materials(
                available_materials.as_deref().expect("checked above"),
                &preferred,
                catalog,
            )
            .map_err(|e| e.to_string())?
            .first()
            .map(|candidate| candidate.2)
            .unwrap_or(0.0);
            return Err(format!(
                "construction material need: preferred={preferred}, best_available_structural_match={best:.6}, threshold={MIN_CONSTRUCTION_MATERIAL_MATCH:.6}"
            ));
        }

        let mut attached = false;
        let mut rejection_counts = PlacementRejectionCounts::default();
        // A not-yet-realized element may have more than one realized blueprint
        // neighbor. Each neighbor is an independently valid forward anchor;
        // failure against one must not strand the element when another neighbor
        // can admit the same physical material through the shared bond authority.
        'neighbors: for neighbor in neighbors.iter().copied() {
            for (storage_index, candidate_name, _) in candidate_resources.iter().cloned() {
                let candidate_instance = if let Some(storage) = available_materials.as_deref() {
                    let Some(crate::material_storage::StoredMaterial::Physical(instance)) =
                        storage.entries.get(storage_index)
                    else {
                        continue;
                    };
                    instance.clone()
                } else {
                    let candidate_resource =
                        resource(catalog, &candidate_name).ok_or_else(|| {
                            format!("unknown preferred construction resource {candidate_name}")
                        })?;
                    crate::physical_material::PhysicalMaterial::realized(
                        crate::resources::Material::free_base(candidate_resource.name.clone(), 1.0),
                        vec![Placement {
                            x: 0.0,
                            y: 0.0,
                            rotation_radians: 0.0,
                        }],
                        catalog,
                    )
                    .ok_or_else(|| {
                        "preferred construction material could not be realized".to_string()
                    })?
                };

                if let Some((
                    trial_structure,
                    new_indices,
                    _part_index,
                    trial_attempt,
                    trial_ledger,
                    trial_energy,
                )) = realize_next_bond_driven(
                    blueprint,
                    catalog,
                    &structure,
                    &realized_units,
                    index,
                    neighbor,
                    &candidate_instance,
                    &mut nodes,
                    &mut rejection_counts,
                    &construction_ledger,
                    remaining_energy,
                    Some(&bob_library),
                ) {
                    construction_ledger = trial_ledger;
                    remaining_energy = trial_energy;
                    total_heat += trial_attempt.work_cost;
                    structure = trial_structure;
                    realized[index] = true;
                    realized_units[index] = Some(new_indices.clone());
                    if storage_index != usize::MAX {
                        reserved_storage_indices.push(storage_index);
                    }
                    // This successful construction step created the physical
                    // bond for the prescribed edge that selected this neighbor.
                    // Record that edge now; later closure work only handles
                    // edges that were not already realized by a forward bond.
                    for (connection_index, connection) in blueprint.connections.iter().enumerate() {
                        if (connection.element_a == index && connection.element_b == neighbor)
                            || (connection.element_a == neighbor && connection.element_b == index)
                        {
                            closed_connections[connection_index] = true;
                            break;
                        }
                    }
                    attached = true;

                    // The cavity is a construction milestone, not merely a
                    // post-construction validation. As soon as the newly
                    // committed physical graph qualifies, return the realized
                    // structure and leave the next construction phase to the
                    // caller. No uncommitted future topology is inspected.
                    if stop_at_genome
                        && crate::cavity::analyze_genome_cavity(&structure, catalog)?.is_some()
                    {
                        *ledger = construction_ledger;
                        *energy = remaining_energy;
                        return Ok((structure, total_heat));
                    }

                    break 'neighbors;
                }
            }
        }

        if !attached {
            eprintln!(
                "[construction-placement-diagnostics] element={index} cumulative_attempts={nodes} element_attempts={} generated={} deduplicated={} restore_failures={} restore_failure_reasons={:?} overlap_rejections={} overlap_rejections_by_unit={:?} neighbor_contact_rejections={} neighbor_rejections_by_index={:?} no_bond_contact={} candidate_evaluation_rejections={} combine_rejections={} bob_coverage_misses={:?}",
                rejection_counts.attempted_origins,
                rejection_counts.generated_origins,
                rejection_counts.deduplicated_origins,
                rejection_counts.restore_failures,
                rejection_counts.restore_failure_reasons,
                rejection_counts.overlap_rejections,
                rejection_counts.overlap_rejections_by_unit,
                rejection_counts.neighbor_contact_rejections,
                rejection_counts.neighbor_rejections_by_index,
                rejection_counts.no_bond_contact,
                rejection_counts.candidate_evaluation_rejections,
                rejection_counts.combine_rejections,
                rejection_counts.bob_coverage_misses,
            );
            if rejection_counts.generated_origins == 0
                && !rejection_counts.bob_coverage_misses.is_empty()
            {
                return Err(format!(
                    "missing Bob library coverage for blueprint element {index}: {:?}",
                    rejection_counts.bob_coverage_misses
                ));
            }
            return Err(format!(
                "no forward bond-driven placement found for blueprint element {index} after {nodes} placement attempts"
            ));
        }
    }

    // All elements now have permanent physical poses. Any blueprint bonds
    // between already-realized elements are completed as ordinary, single-bond
    // construction steps. This is not future lookahead: the endpoints and
    // geometry already exist, and a failed closure never moves or undoes a
    // committed bond.
    while closed_connections.iter().any(|closed| !closed) {
        let mut progressed = false;
        let mut failed_diagnostic = None;
        for (connection_index, connection) in blueprint.connections.iter().enumerate() {
            if closed_connections[connection_index] {
                continue;
            }
            let Some(units_a) = realized_units[connection.element_a].as_ref() else {
                continue;
            };
            let Some(units_b) = realized_units[connection.element_b].as_ref() else {
                continue;
            };
            'unit_pairs: for &unit_a in units_a {
                for &unit_b in units_b {
                    if unit_a == unit_b {
                        continue;
                    }
                    let candidates = crate::contact::connection_pair_candidates_cached(
                        &structure,
                        unit_a,
                        unit_b,
                        catalog,
                        &mut crate::contact::ConnectionCompatibilityCache::new(),
                    );
                    let total_candidates = candidates.len();
                    let mut contact_candidates = 0usize;
                    let mut evaluated_candidates = 0usize;
                    let mut rejected_by_bond_admission = 0usize;
                    for candidate in candidates.into_iter().filter(|candidate| {
                        candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                            && candidate.available_a
                            && candidate.available_b
                    }) {
                        contact_candidates += 1;
                        let Some((_, _, investment, _)) =
                            crate::combine_runtime::selected_candidate_evaluation(
                                &structure, unit_a, unit_b, candidate, catalog,
                            )
                        else {
                            continue;
                        };
                        evaluated_candidates += 1;

                        let mut trial_structure = structure.clone();
                        let mut trial_ledger = construction_ledger;
                        let mut trial_energy = remaining_energy;
                        let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();
                        let Some(attempt) = crate::combine_runtime::form_selected_bond(
                            &mut trial_structure,
                            unit_a,
                            unit_b,
                            candidate,
                            investment,
                            catalog,
                            &mut bond_cache,
                            &mut trial_ledger,
                            &mut trial_energy,
                        ) else {
                            rejected_by_bond_admission += 1;
                            continue;
                        };

                        structure = trial_structure;
                        construction_ledger = trial_ledger;
                        remaining_energy = trial_energy;
                        total_heat += attempt.work_cost;
                        closed_connections[connection_index] = true;
                        progressed = true;

                        // Closure bonds are evaluated only after both
                        // endpoints already exist. If this committed bond
                        // creates the qualifying cavity, that is the exact
                        // end of the genome-construction phase.
                        if stop_at_genome
                            && crate::cavity::analyze_genome_cavity(&structure, catalog)?.is_some()
                        {
                            *ledger = construction_ledger;
                            *energy = remaining_energy;
                            return Ok((structure, total_heat));
                        }

                        break 'unit_pairs;
                    }

                    if total_candidates > 0
                        || contact_candidates > 0
                        || evaluated_candidates > 0
                        || rejected_by_bond_admission > 0
                    {
                        failed_diagnostic = Some((
                            connection_index,
                            connection.element_a,
                            connection.element_b,
                            total_candidates,
                            contact_candidates,
                            evaluated_candidates,
                            rejected_by_bond_admission,
                        ));
                    }
                }
            }
            if progressed {
                break;
            }
            if let Some((
                connection_index,
                element_a,
                element_b,
                total_candidates,
                contact_candidates,
                evaluated_candidates,
                rejected_by_bond_admission,
            )) = failed_diagnostic
            {
                return Err(format!(
                    "construction closure failed: connection={connection_index} elements=({element_a},{element_b}) candidates={total_candidates} contacts={contact_candidates} evaluated={evaluated_candidates} rejected={rejected_by_bond_admission}"
                ));
            }
            return Err("construction closure made no progress".to_string());
        }
    }

    *ledger = construction_ledger;
    *energy = remaining_energy;
    Ok((structure, total_heat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bond_driven_triangle_commits_all_realized_neighbor_bonds() {
        use crate::resources::Material;
        use crate::structural_blueprint::{
            BlueprintConnection, BlueprintElement, BlueprintPlacement,
        };

        let catalog = crate::resources::default_catalog();
        let radius = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .and_then(|resource| match resource.shape.form {
                crate::resources::Form::RegularPolygon { radius, .. } => Some(radius),
                _ => None,
            })
            .unwrap();

        let spacing = (3.0_f64).sqrt() * radius;
        let blueprint = crate::structural_blueprint::StructuralBlueprint::with_anchor_elements(
            vec![
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: spacing,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: spacing / 2.0,
                        y: spacing * 0.8660254037844386,
                        rotation_radians: 0.0,
                    },
                },
            ],
            vec![
                BlueprintConnection {
                    element_a: 0,
                    element_b: 1,
                },
                BlueprintConnection {
                    element_a: 0,
                    element_b: 2,
                },
                BlueprintConnection {
                    element_a: 1,
                    element_b: 2,
                },
            ],
            vec![0],
        );

        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e6;
        let (structure, _) =
            construct_blueprint_bond_driven(&blueprint, &catalog, &mut ledger, &mut energy)
                .unwrap();

        assert_eq!(structure.units.len(), 3);
        assert_eq!(structure.bonds.len(), 3);
    }

    #[test]
    fn canonical_live_rigid_parent_maps_stable_ids_to_bob_slots() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let b = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                // Carbon's radius is 1.0. At a two-unit center offset,
                // corner 0 of A meets the opposite corner 3 of B.
                x: 2.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        let id_a = structure.physical_id(a).unwrap();
        let id_b = structure.physical_id(b).unwrap();
        structure.bonds.push(crate::structure::Bond {
            endpoint_a: crate::structure::BondEndpoint::new(
                id_a,
                ConnectionEndpoint::Corner { point_index: 0 },
            ),
            endpoint_b: crate::structure::BondEndpoint::new(
                id_b,
                ConnectionEndpoint::Corner { point_index: 3 },
            ),
            strength: 1.0,
            bond_energy: 0.0,
        });

        let parent = canonical_live_rigid_parent(&structure, &catalog)
            .expect("connected rigid parent should canonicalize");
        assert_eq!(parent.formation.constituents.len(), 2);
        assert_eq!(parent.canonical_index_by_id.len(), 2);
        let slot_a = parent.canonical_index_by_id[&id_a];
        let slot_b = parent.canonical_index_by_id[&id_b];
        assert_ne!(slot_a, slot_b);
        assert!(slot_a < 2 && slot_b < 2);
    }

    #[test]
    fn bob_fluid_suggestion_transforms_and_restores_as_exact_fluid_geometry() {
        use crate::geometry_reference_library::{
            fluid_formations_from_boundaries,
            generate_fluid_boundary_families_from_contact_families,
            generate_water_contact_families_with_diagnostics, GeometryFormation, GeometryLibrary,
        };
        use crate::resources::Material;

        let catalog = crate::resources::default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-runtime-bob-fluid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let parent = GeometryFormation::single("Carbon");
        library.insert(parent.clone(), &catalog).unwrap();
        let water = resource(&catalog, "Water").unwrap();
        let (contacts, _) =
            generate_water_contact_families_with_diagnostics(&parent, water, &catalog);
        let boundaries = generate_fluid_boundary_families_from_contact_families(water, &contacts);
        let fluid_formations = fluid_formations_from_boundaries(&boundaries);
        assert!(!fluid_formations.is_empty());
        library.insert_contact_families(contacts).unwrap();
        library.insert_fluid_boundary_families(boundaries).unwrap();
        library
            .insert_fluid_formations(fluid_formations, &catalog)
            .unwrap();

        let anchor_pose = Placement {
            x: 3.0,
            y: -2.0,
            rotation_radians: 0.37,
        };
        let relative_fluid_pose = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut live_structure = OrganismStructure::new();
        let anchor_index = live_structure.add_unit(StructuralUnit::new("Carbon", anchor_pose));
        let anchor_id = live_structure.physical_id(anchor_index).unwrap();
        let live_parent = canonical_live_rigid_parent(&live_structure, &catalog)
            .expect("single rigid parent should map into Bob's canonical frame");
        let suggestions = bob_fluid_origins_for_live_parent(
            &library,
            &live_parent,
            anchor_id,
            water,
            relative_fluid_pose,
            &catalog,
        );
        assert!(!suggestions.is_empty());
        for (origin, shape) in suggestions {
            assert!(origin.x.is_finite());
            assert!(origin.y.is_finite());
            assert!(origin.rotation_radians.is_finite());
            assert!(matches!(
                &shape.form,
                crate::resources::Form::Fluid {
                    nominal_area,
                    boundary: None,
                    arc_boundary: Some(_), ..} if (*nominal_area - crate::resources::NOMINAL_UNIT_AREA).abs() < 1e-9
            ));

            let material = crate::physical_material::PhysicalMaterial::realized(
                Material::free_base("Water", 1.0),
                vec![relative_fluid_pose],
                &catalog,
            )
            .expect("default Water material should have a valid physical realization")
            .with_single_fluid_shape(shape.clone(), &catalog)
            .expect("Bob's conserved-area arc should fit the physical Water material");
            let mut restored = OrganismStructure::new();
            let restored_indices = crate::material_restoration::restore_material(
                &mut restored,
                &material,
                origin,
                &catalog,
            )
            .expect("the Bob fluid shape should survive physical material restoration");
            assert_eq!(
                restored.units[restored_indices[0]].shape(&catalog),
                Some(&shape)
            );
            assert_eq!(material.realized_shape_for_part(0, &catalog), Some(shape));
        }
        drop(library);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reopened_two_piece_bob_record_maps_into_transformed_live_parent() {
        use crate::geometry_reference_library::{
            generate_collinear_multi_contact_formations, GeometryLibrary,
        };
        use crate::resources::{Form, Material};
        use crate::structure::{Bond, BondEndpoint, StructuralUnit};

        let mut catalog = crate::resources::default_catalog();
        let water = resource(&catalog, "Water").unwrap().clone();
        let carbon = resource(&catalog, "Carbon").unwrap().clone();
        let area = match &water.shape.form {
            Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            Form::Fluid { nominal_area, .. } => *nominal_area,
            _ => panic!("Water must expose a conserved fluid area"),
        };
        let family = crate::capillary_geometry::solve_water_against_solid(
            area,
            water.properties.cohesion,
            carbon.properties.cohesion,
        )
        .expect("default Water and Carbon should have an equilibrium contact family");
        let wall_length = family.contact_length * 0.5;
        for name in ["TestWallA", "TestWallB"] {
            let mut wall = carbon.clone();
            wall.name = name.to_string();
            wall.shape.form = Form::Rectangle {
                width: wall_length,
                height: 0.1,
            };
            catalog.push(wall);
        }

        let root = std::env::temp_dir().join(format!(
            "evosim-runtime-bob-two-piece-reopen-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);

        let rotation: f64 = 0.37;
        let (sin, cos) = rotation.sin_cos();
        let live_center = |canonical_x: f64| Placement {
            x: 3.0 + canonical_x * cos,
            y: -2.0 + canonical_x * sin,
            rotation_radians: rotation,
        };
        let mut live = OrganismStructure::new();
        let left = live.add_unit(StructuralUnit::new(
            "TestWallA",
            live_center(-wall_length * 0.5),
        ));
        let right = live.add_unit(StructuralUnit::new(
            "TestWallB",
            live_center(wall_length * 0.5),
        ));
        let left_id = live.physical_id(left).unwrap();
        let right_id = live.physical_id(right).unwrap();
        live.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(
                left_id,
                ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            ),
            endpoint_b: BondEndpoint::new(
                right_id,
                ConnectionEndpoint::Boundary {
                    angle_radians: std::f64::consts::PI,
                },
            ),
            strength: 1.0,
            bond_energy: 0.0,
        });

        let live_parent = canonical_live_rigid_parent(&live, &catalog)
            .expect("bonded two-piece line parent should map into Bob's canonical frame");
        assert_eq!(live_parent.formation.constituents.len(), 2);
        let generated =
            generate_collinear_multi_contact_formations(&live_parent.formation, &water, &catalog);
        let record = generated
            .into_iter()
            .find(|candidate| candidate.contact_constraints.len() == 2)
            .expect("two-piece parent should yield a two-contact Water record");
        // The fluid's origin is offset from the rigid parent's center by its
        // solved contact geometry. Compute both valid bulge orientations in
        // canonical coordinates, then transform them into the live frame.
        let expected_live_poses: Vec<Placement> = [true, false]
            .into_iter()
            .filter_map(|bulge_left| {
                record
                    .realize_against_parent(&live_parent.formation, &catalog, bulge_left)
                    .map(|(_, pose)| {
                        let (s, c) = live_parent.canonical_to_live_rotation.sin_cos();
                        Placement {
                            x: live_parent.canonical_to_live_x + pose.x * c - pose.y * s,
                            y: live_parent.canonical_to_live_y + pose.x * s + pose.y * c,
                            rotation_radians: live_parent.canonical_to_live_rotation
                                + pose.rotation_radians,
                        }
                    })
            })
            .collect();
        assert!(!expected_live_poses.is_empty());

        {
            let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
            library
                .insert(live_parent.formation.clone(), &catalog)
                .unwrap();
            assert_eq!(
                library
                    .insert_fluid_formations_with_multi_contact_counts(
                        Vec::new(),
                        vec![record.clone()],
                        &catalog,
                    )
                    .unwrap(),
                (1, 1)
            );
        }
        let library = GeometryLibrary::open(&root, &catalog).unwrap();
        let suggestions = bob_fluid_origins_for_live_parent(
            &library,
            &live_parent,
            left_id,
            &water,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        );
        let (origin, shape) = suggestions
            .into_iter()
            .find(|(_, shape)| {
                matches!(
                    &shape.form,
                    Form::Fluid {
                        multi_contact_boundary: Some(boundary),
                        ..
                    } if boundary.contact_count() == 2
                )
            })
            .expect("runtime query should retrieve the reopened two-contact record");
        assert!(
            expected_live_poses.iter().any(|expected| {
                (origin.x - expected.x).abs() < 1e-7
                    && (origin.y - expected.y).abs() < 1e-7
                    && (origin.rotation_radians - expected.rotation_radians).abs() < 1e-7
            }),
            "runtime origin must match one of the exact solved poses transformed into the live frame"
        );
        assert!(shape.is_valid());
        match &shape.form {
            Form::Fluid {
                nominal_area,
                multi_contact_boundary: Some(boundary),
                ..
            } => {
                assert!((*nominal_area - area).abs() <= 1e-9 * area.max(1.0));
                assert_eq!(boundary.contact_count(), 2);
            }
            _ => panic!("runtime must preserve the exact multi-contact fluid boundary"),
        }

        // Continue the exact reopened-record path into the same staged bond
        // transaction used by construction. The pre-existing rigid bond is
        // retained, while both fluid-to-rigid contacts must commit together.
        let water_material = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(shape.clone(), &catalog)
        .unwrap();
        let mut trial = live.clone();
        let fluid_index = crate::material_restoration::restore_material(
            &mut trial,
            &water_material,
            origin,
            &catalog,
        )
        .expect("reopened Bob fluid geometry should restore into the live structure")[0];
        let mut first_bond = None;
        for target_index in [left, right] {
            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let candidates = crate::contact::connection_pair_candidates_cached(
                &trial,
                target_index,
                fluid_index,
                &catalog,
                &mut cache,
            );
            for candidate in candidates.into_iter().filter(|candidate| {
                candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                    && candidate.available_a
                    && candidate.available_b
            }) {
                let Some((_, _, investment, _)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        &trial,
                        target_index,
                        fluid_index,
                        candidate,
                        &catalog,
                    )
                else {
                    continue;
                };
                let mut ledger = EnergyLedger::default();
                let mut energy = 100.0;
                if crate::combine_runtime::form_selected_bond(
                    &mut trial,
                    target_index,
                    fluid_index,
                    candidate,
                    investment,
                    &catalog,
                    &mut cache,
                    &mut ledger,
                    &mut energy,
                )
                .is_some()
                {
                    first_bond = Some((target_index, ledger, energy));
                    break;
                }
            }
            if first_bond.is_some() {
                break;
            }
        }
        let (first_target, mut ledger, mut energy) =
            first_bond.expect("the first persisted fluid contact should be bondable");
        assert!(
            form_remaining_multi_contact_bonds(
                &mut trial,
                fluid_index,
                first_target,
                &catalog,
                &mut ledger,
                &mut energy,
            ),
            "all remaining contacts from the reopened Bob record must form in the same trial"
        );
        assert_eq!(trial.bonds.len(), live.bonds.len() + 2);
        assert!(energy.is_finite());
        assert!(trial.units[fluid_index].shape(&catalog).unwrap().is_valid());
        assert_eq!(
            live.bonds.len(),
            1,
            "the source structure must remain unchanged"
        );
        drop(library);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn multi_contact_fluid_forms_multiple_bonds_to_one_rigid_edge() {
        use crate::capillary_geometry::{
            solve_water_against_collinear_contacts, CapillaryContactFamily,
        };
        use crate::resources::{Form, Material, NOMINAL_UNIT_AREA};
        use crate::structure::StructuralUnit;

        let mut catalog = crate::resources::default_catalog();
        let carbon_index = catalog
            .iter()
            .position(|resource| resource.name == "Carbon")
            .unwrap();
        let area = NOMINAL_UNIT_AREA;
        let angle = std::f64::consts::FRAC_PI_2;
        let family = CapillaryContactFamily::solve(area, angle).unwrap();
        let third = family.contact_length / 3.0;
        let segments = [
            ((-family.contact_length * 0.5, 0.0), (-third, 0.0)),
            ((-third, 0.0), (third, 0.0)),
            ((third, 0.0), (family.contact_length * 0.5, 0.0)),
        ];
        let boundary = solve_water_against_collinear_contacts(area, angle, &segments)
            .expect("three collinear segments should form one conserved boundary");
        let shape = crate::resources::Shape {
            form: Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(boundary),
            },
        };
        let material = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(shape, &catalog)
        .unwrap();
        let mut structure = OrganismStructure::new();
        let fluid_index = crate::material_restoration::restore_material(
            &mut structure,
            &material,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .unwrap()[0];
        let half_height = 0.05;
        catalog[carbon_index].shape.form = Form::Polygon {
            vertices: vec![
                (-family.contact_length * 0.5, -half_height),
                (family.contact_length * 0.5, -half_height),
                (family.contact_length * 0.5, half_height),
                (-family.contact_length * 0.5, half_height),
            ],
        };
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 0.0,
                y: -half_height,
                rotation_radians: 0.0,
            },
        ));
        let on_segment = |endpoint: crate::structure::ConnectionEndpoint,
                          segment: ((f64, f64), (f64, f64))| {
            let crate::structure::ConnectionEndpoint::Fluid { x, y } = endpoint else {
                return false;
            };
            let ((sx, sy), (ex, ey)) = segment;
            let dx = ex - sx;
            let dy = ey - sy;
            let length_squared = dx * dx + dy * dy;
            let t = (((x - sx) * dx + (y - sy) * dy) / length_squared).clamp(0.0, 1.0);
            (x - (sx + t * dx)).hypot(y - (sy + t * dy))
                <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
        };
        let first_target = 1;
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let candidate = crate::contact::connection_pair_candidates_cached(
            &structure,
            first_target,
            fluid_index,
            &catalog,
            &mut cache,
        )
        .into_iter()
        .filter(|candidate| {
            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && candidate.available_a
                && candidate.available_b
                && segments
                    .iter()
                    .any(|segment| on_segment(candidate.endpoint_b, *segment))
        })
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("the first edge contact should have a live candidate");
        let (_, _, investment, _) = crate::combine_runtime::selected_candidate_evaluation(
            &structure,
            first_target,
            fluid_index,
            candidate,
            &catalog,
        )
        .expect("the first contact should be energetically admissible");
        let mut ledger = EnergyLedger::default();
        let mut energy = 100.0;
        assert!(crate::combine_runtime::form_selected_bond(
            &mut structure,
            first_target,
            fluid_index,
            candidate,
            investment,
            &catalog,
            &mut cache,
            &mut ledger,
            &mut energy,
        )
        .is_some());
        assert!(
            form_remaining_multi_contact_bonds(
                &mut structure,
                fluid_index,
                first_target,
                &catalog,
                &mut ledger,
                &mut energy,
            ),
            "all three contact segments on the same rigid unit must bond"
        );
        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.bonds.len(), 3);
        assert!(energy.is_finite());

        let fluid_id = structure.units[fluid_index].physical_id;
        let fluid_locations = structure
            .bonds
            .iter()
            .filter_map(|bond| {
                if bond.endpoint_a.constituent_id == fluid_id {
                    Some(bond.endpoint_a.location)
                } else if bond.endpoint_b.constituent_id == fluid_id {
                    Some(bond.endpoint_b.location)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(fluid_locations.len(), segments.len());
        for (index, location) in fluid_locations.iter().enumerate() {
            assert!(
                fluid_locations[index + 1..]
                    .iter()
                    .all(|other| !location.same_location(*other)),
                "each segment needs a distinct fluid bond endpoint"
            );
            assert!(
                segments
                    .iter()
                    .any(|segment| on_segment(*location, *segment)),
                "each bond endpoint must lie on a persisted contact segment"
            );
        }
        let realized_shape = structure.units[fluid_index].shape(&catalog).unwrap();
        assert!(realized_shape.is_valid());
        match &realized_shape.form {
            Form::Fluid { nominal_area, .. } => {
                assert!((*nominal_area - area).abs() <= 1e-9);
            }
            _ => panic!("the bonded fluid must retain its fluid shape"),
        }
    }

    #[test]
    fn multi_contact_fluid_commits_all_three_rigid_bonds_together() {
        use crate::capillary_geometry::{
            solve_water_against_collinear_contacts, CapillaryContactFamily,
        };
        use crate::resources::{Form, Material, NOMINAL_UNIT_AREA};
        use crate::structure::StructuralUnit;

        let mut catalog = crate::resources::default_catalog();
        let carbon_index = catalog
            .iter()
            .position(|resource| resource.name == "Carbon")
            .unwrap();
        let area = NOMINAL_UNIT_AREA;
        let angle = std::f64::consts::FRAC_PI_2;
        let family = CapillaryContactFamily::solve(area, angle).unwrap();
        let segment_length = family.contact_length / 3.0;
        catalog[carbon_index].shape.form = Form::Rectangle {
            width: segment_length,
            height: 0.1,
        };

        let third = family.contact_length / 3.0;
        let segments = [
            ((-family.contact_length * 0.5, 0.0), (-third, 0.0)),
            ((-third, 0.0), (third, 0.0)),
            ((third, 0.0), (family.contact_length * 0.5, 0.0)),
        ];
        let boundary = solve_water_against_collinear_contacts(area, angle, &segments)
            .expect("the three collinear contacts should form one conserved boundary");
        let shape = crate::resources::Shape {
            form: Form::Fluid {
                nominal_area: area,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(boundary),
            },
        };
        let material = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(shape, &catalog)
        .unwrap();

        let mut structure = OrganismStructure::new();
        let fluid_index = crate::material_restoration::restore_material(
            &mut structure,
            &material,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .unwrap()[0];
        for x in [-third, 0.0, third] {
            structure.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x,
                    y: -0.05,
                    rotation_radians: 0.0,
                },
            ));
        }

        let first_target = 1;
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let candidate = crate::contact::connection_pair_candidates_cached(
            &structure,
            first_target,
            fluid_index,
            &catalog,
            &mut cache,
        )
        .into_iter()
        .filter(|candidate| {
            candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                && candidate.available_a
                && candidate.available_b
        })
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("first rigid surface must have a valid fluid connection");
        let (_, _, investment, _) = crate::combine_runtime::selected_candidate_evaluation(
            &structure,
            first_target,
            fluid_index,
            candidate,
            &catalog,
        )
        .expect("first fluid bond should be energetically admissible");
        let mut ledger = EnergyLedger::default();
        let mut energy = 100.0;
        let mut incomplete_trial = structure.clone();
        let mut failed_cache = crate::contact::ConnectionCompatibilityCache::new();
        let mut failed_ledger = EnergyLedger::default();
        let mut failed_energy = 100.0;
        assert!(crate::combine_runtime::form_selected_bond(
            &mut incomplete_trial,
            first_target,
            fluid_index,
            candidate,
            investment,
            &catalog,
            &mut failed_cache,
            &mut failed_ledger,
            &mut failed_energy,
        )
        .is_some());
        let _ = incomplete_trial.units.pop();
        // Snapshot every transaction-visible effect after the deliberately
        // incomplete setup. A failed completion must preserve the already
        // staged trial exactly, including unit identities, bonds, ledger and
        // spendable energy; the caller can then discard the whole trial.
        let units_before_failed_trial = incomplete_trial
            .units
            .iter()
            .map(|unit| (unit.physical_id, unit.placement))
            .collect::<Vec<_>>();
        let bonds_before_failed_trial = incomplete_trial.bonds.len();
        let energy_before_failed_trial = failed_energy;
        let ledger_before_failed_trial = failed_ledger;
        assert!(!form_remaining_multi_contact_bonds(
            &mut incomplete_trial,
            fluid_index,
            first_target,
            &catalog,
            &mut failed_ledger,
            &mut failed_energy,
        ));
        assert_eq!(incomplete_trial.bonds.len(), bonds_before_failed_trial);
        assert_eq!(
            incomplete_trial
                .units
                .iter()
                .map(|unit| (unit.physical_id, unit.placement))
                .collect::<Vec<_>>(),
            units_before_failed_trial,
            "failed completion must not mutate trial unit identities or poses"
        );
        assert_eq!(
            failed_ledger.total_potential_energy_released,
            ledger_before_failed_trial.total_potential_energy_released
        );
        assert_eq!(
            failed_ledger.total_usable_energy_gained,
            ledger_before_failed_trial.total_usable_energy_gained
        );
        assert_eq!(
            failed_ledger.total_heat_dissipated,
            ledger_before_failed_trial.total_heat_dissipated
        );
        assert_eq!(
            failed_ledger.total_usable_energy_held,
            ledger_before_failed_trial.total_usable_energy_held
        );
        assert_eq!(failed_energy, energy_before_failed_trial);
        assert!(
            structure.bonds.is_empty(),
            "the untouched source stays unchanged"
        );

        assert!(crate::combine_runtime::form_selected_bond(
            &mut structure,
            first_target,
            fluid_index,
            candidate,
            investment,
            &catalog,
            &mut cache,
            &mut ledger,
            &mut energy,
        )
        .is_some());

        let fluid_unit = &structure.units[fluid_index];
        let fluid_shape = fluid_unit.shape(&catalog).unwrap();
        let fluid_part = crate::material_geometry::PlacedMaterialPart {
            part_index: fluid_index,
            form: fluid_shape.form.clone(),
            placement: fluid_unit.placement,
        };
        let contact_targets = structure
            .units
            .iter()
            .enumerate()
            .filter_map(|(index, unit)| {
                if index == fluid_index {
                    return None;
                }
                let shape = unit.shape(&catalog)?;
                let rigid_part = crate::material_geometry::PlacedMaterialPart {
                    part_index: index,
                    form: shape.form.clone(),
                    placement: unit.placement,
                };
                crate::material_geometry::placed_forms_boundary_contact(
                    &fluid_part,
                    &rigid_part,
                    crate::combine_runtime::COMBINE_CONTACT_TOLERANCE,
                )
                .then_some(index)
            })
            .collect::<Vec<_>>();
        assert_eq!(contact_targets, vec![1, 2, 3]);
        for target_index in [2, 3] {
            let mut target_cache = crate::contact::ConnectionCompatibilityCache::new();
            let candidates = crate::contact::connection_pair_candidates_cached(
                &structure,
                target_index,
                fluid_index,
                &catalog,
                &mut target_cache,
            )
            .into_iter()
            .filter(|candidate| {
                candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                    && candidate.available_a
                    && candidate.available_b
            })
            .collect::<Vec<_>>();
            assert!(
                !candidates.is_empty(),
                "target {target_index} has no available fluid connection candidate"
            );
            assert!(
                crate::combine_runtime::selected_candidate_evaluation(
                    &structure,
                    target_index,
                    fluid_index,
                    candidates[0],
                    &catalog,
                )
                .is_some(),
                "target {target_index} candidate failed bond evaluation"
            );
        }

        assert!(form_remaining_multi_contact_bonds(
            &mut structure,
            fluid_index,
            first_target,
            &catalog,
            &mut ledger,
            &mut energy,
        ));
        assert_eq!(structure.bonds.len(), 3);
    }

    #[test]
    fn restored_multi_contact_fluid_keeps_three_live_rigid_contacts() {
        use crate::capillary_geometry::{FluidBoundaryPiece, MultiContactFluidBoundary};
        use crate::material_geometry::{
            placed_forms_boundary_contact, placed_forms_rigid_contact, PlacedMaterialPart,
        };
        use crate::resources::{Form, Material, NOMINAL_UNIT_AREA};

        let catalog = crate::resources::default_catalog();
        let half = NOMINAL_UNIT_AREA.sqrt() * 0.5;
        let angle = std::f64::consts::FRAC_PI_2;
        let boundary = MultiContactFluidBoundary {
            nominal_area: NOMINAL_UNIT_AREA,
            pieces: vec![
                FluidBoundaryPiece::SolidContact {
                    start_x: -half,
                    start_y: -half,
                    end_x: half,
                    end_y: -half,
                    contact_angle_radians: angle,
                },
                FluidBoundaryPiece::SolidContact {
                    start_x: half,
                    start_y: -half,
                    end_x: half,
                    end_y: half,
                    contact_angle_radians: angle,
                },
                FluidBoundaryPiece::SolidContact {
                    start_x: half,
                    start_y: half,
                    end_x: -half,
                    end_y: half,
                    contact_angle_radians: angle,
                },
                FluidBoundaryPiece::SolidContact {
                    start_x: -half,
                    start_y: half,
                    end_x: -half,
                    end_y: -half,
                    contact_angle_radians: angle,
                },
            ],
        };
        assert!(boundary.is_valid());
        let shape = crate::resources::Shape {
            form: Form::Fluid {
                nominal_area: NOMINAL_UNIT_AREA,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(boundary),
            },
        };
        let material = crate::physical_material::PhysicalMaterial::realized(
            Material::free_base("Water", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
        .with_single_fluid_shape(shape.clone(), &catalog)
        .unwrap();

        let origin = Placement {
            x: 4.0,
            y: -3.0,
            rotation_radians: 0.41,
        };
        let mut restored = OrganismStructure::new();
        let indices = crate::material_restoration::restore_material(
            &mut restored,
            &material,
            origin,
            &catalog,
        )
        .expect("multi-contact fluid should restore");
        let fluid_unit = &restored.units[indices[0]];
        assert_eq!(fluid_unit.shape(&catalog), Some(&shape));

        let world = |x: f64, y: f64| {
            let (sin, cos) = origin.rotation_radians.sin_cos();
            Placement {
                x: origin.x + x * cos - y * sin,
                y: origin.y + x * sin + y * cos,
                rotation_radians: origin.rotation_radians,
            }
        };
        let fluid = PlacedMaterialPart {
            part_index: indices[0],
            form: fluid_unit.shape(&catalog).unwrap().form.clone(),
            placement: fluid_unit.placement,
        };
        let contacts = [
            (world(0.0, -half), 0.0),
            (world(half, 0.0), std::f64::consts::FRAC_PI_2),
            (world(0.0, half), 0.0),
        ];
        for (pose, rotation) in contacts {
            let rigid = PlacedMaterialPart {
                part_index: 99,
                form: Form::Line { length: half * 1.5 },
                placement: Placement {
                    x: pose.x,
                    y: pose.y,
                    rotation_radians: rotation + origin.rotation_radians,
                },
            };
            assert!(placed_forms_boundary_contact(&fluid, &rigid, 1e-8));
            assert!(placed_forms_rigid_contact(&fluid, &rigid, 1e-8));
        }
        assert!(
            (fluid_unit.shape(&catalog).unwrap().form.bounding_radius()
                - shape.form.bounding_radius())
            .abs()
                < 1e-9
        );
    }

    #[test]
    fn restored_composite_overlap_is_rejected_against_existing_structure() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();

        let existing = StructuralUnit::from_material(
            crate::resources::Material::free_base("Carbon", 1.0),
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        let existing_index = structure.add_unit(existing);

        let overlapping = StructuralUnit::from_material(
            crate::resources::Material::free_base("Carbon", 1.0),
            Placement {
                x: 0.5,
                y: 0.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();

        assert!(placed_unit_overlaps(
            &structure,
            &overlapping,
            &[existing_index + 1],
            &catalog,
        ));
        assert!(!placed_unit_overlaps(
            &structure,
            &overlapping,
            &[existing_index],
            &catalog,
        ));
    }
}
