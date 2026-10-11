//! Persistent geometry reference library.
//!
//! This module is deliberately separate from the live construction runtime.
//! The library is durable knowledge: tests use isolated temporary stores, while
//! the production catalogue lives outside `target/` and survives test runs and
//! process restarts.

use crate::capillary_geometry::{
    solve_water_against_collinear_contacts, solve_water_against_solid, CapillaryContactFamily,
    CircularArcBoundary, ContactSegment, ContactTranslationInterval, FluidBoundaryPiece,
    MultiContactFluidBoundary,
};
use crate::material_geometry::{
    placed_forms_penetrate, placed_forms_rigid_contact, PlacedMaterialPart,
};
use crate::resources::{default_catalog, BaseResource, Form};
use crate::structure::{ConnectionEndpoint, Placement};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const GEOMETRY_LIBRARY_SCHEMA_VERSION: u32 = 2;
/// Version for the live-interface cache-key format, independent of persisted Bob records.
pub const LIVE_GEOMETRY_INTERFACE_SCHEMA_VERSION: u32 = 1;
/// Positional equivalence used by Bob when deciding whether two otherwise
/// identical geometric records describe the same meaningful contact.
/// Differences at or below this distance do not create a new record.
pub const GEOMETRY_EQUIVALENCE_TOLERANCE: f64 = 0.5;
const QUANTUM: f64 = 1e-9;

/// Stable 128-bit prefix of SHA-256. Unlike `DefaultHasher`, this is a persistence contract.
fn formation_id(signature: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let digest = Sha256::digest(signature.as_bytes());
    let bytes = &digest[..16];
    let mut encoded = String::with_capacity(22);
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let a = bytes[index] as usize;
        let b = bytes[index + 1] as usize;
        let c = bytes[index + 2] as usize;
        encoded.push(ALPHABET[a >> 2] as char);
        encoded.push(ALPHABET[((a & 0x03) << 4) | (b >> 4)] as char);
        encoded.push(ALPHABET[((b & 0x0f) << 2) | (c >> 6)] as char);
        encoded.push(ALPHABET[c & 0x3f] as char);
        index += 3;
    }
    if index < bytes.len() {
        let a = bytes[index] as usize;
        encoded.push(ALPHABET[a >> 2] as char);
        if index + 1 < bytes.len() {
            let b = bytes[index + 1] as usize;
            encoded.push(ALPHABET[((a & 0x03) << 4) | (b >> 4)] as char);
            encoded.push(ALPHABET[(b & 0x0f) << 2] as char);
        } else {
            encoded.push(ALPHABET[(a & 0x03) << 4] as char);
        }
    }
    encoded
}

fn compact_family_value<T: Serialize>(family: &T) -> std::io::Result<serde_json::Value> {
    let mut value = serde_json::to_value(family)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let object = value.as_object_mut().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "family record must serialize as an object",
        )
    })?;
    let signature = object
        .remove("formation_signature")
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "family record lacks formation_signature",
            )
        })?;
    object.insert(
        "formation_id".to_owned(),
        serde_json::Value::String(formation_id(&signature)),
    );
    Ok(value)
}

fn write_compact_family<W: Write, T: Serialize>(writer: &mut W, family: &T) -> std::io::Result<()> {
    let value = compact_family_value(family)?;
    serde_json::to_writer(writer, &value)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn resolve_family_formation_id(
    object: &mut serde_json::Map<String, serde_json::Value>,
    formation_ids: &BTreeMap<String, String>,
) -> Result<(), serde_json::Error> {
    if object.contains_key("formation_signature") {
        return Err(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "compact family record contains both formation_signature and formation_id",
        )));
    }
    let id = object
        .remove("formation_id")
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| {
            serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "compact family record lacks formation_id",
            ))
        })?;
    let signature = formation_ids.get(&id).ok_or_else(|| {
        serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unresolved formation_id: {id}"),
        ))
    })?;
    object.insert(
        "formation_signature".to_owned(),
        serde_json::Value::String(signature.clone()),
    );
    Ok(())
}

fn deserialize_family_record<T: DeserializeOwned>(
    line: &str,
    formation_ids: &BTreeMap<String, String>,
) -> Result<T, serde_json::Error> {
    let mut value: serde_json::Value = serde_json::from_str(line)?;
    let object = value.as_object_mut().ok_or_else(|| {
        serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "family record must be an object",
        ))
    })?;
    if let Some(version) = object.remove("storage_version") {
        if version.as_u64() != Some(2) {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unsupported family row storage version",
            )));
        }
        if object.contains_key("formation_signature") {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "compact family record contains both formation_signature and formation_id",
            )));
        }
        resolve_family_formation_id(object, formation_ids)?;
    } else if object
        .get("formation_signature")
        .and_then(serde_json::Value::as_str)
        .is_none()
    {
        resolve_family_formation_id(object, formation_ids)?;
    }
    serde_json::from_value(value)
}

/// Canonical identity of a live physical interface.
///
/// This is deliberately derived from already-realized endpoint topology. It
/// does not search geometry space, sample orientations, or invent a persistent
/// family. A later family lookup may use this identity as its lookup boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveGeometryQuery {
    RigidEdge {
        a_material: String,
        a_edge: usize,
        a_parameter: i64,
        a_rotation: i64,
        b_material: String,
        b_edge: usize,
        b_parameter: i64,
        b_rotation: i64,
    },
    RigidPoint {
        line_material: String,
        line_point: usize,
        edge_material: String,
        edge: usize,
        edge_parameter: i64,
        candidate_rotation: Option<i64>,
    },
    RigidVertex {
        corner_material: String,
        corner_point: usize,
        edge_material: String,
        edge: usize,
        edge_parameter: i64,
        candidate_rotation: Option<i64>,
    },
    FluidBoundary {
        fluid_material: String,
        solid_material: String,
        solid_edge: usize,
        edge_parameter: i64,
        fluid_area: i64,
        contact_angle: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveGeometryInterface {
    pub interface_class: &'static str,
    pub signature: String,
    pub query: Option<LiveGeometryQuery>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveFamilyResolution {
    Unresolved,
    Ambiguous,
    Unique,
}

/// Classifies whether a live interface identity is sufficient by itself to
/// select one persisted Bob family. Most live contacts are not: a family also
/// carries continuous edge/rotation information that endpoint topology alone
/// cannot recover.
pub fn classify_live_family_resolution(interface: &LiveGeometryInterface) -> LiveFamilyResolution {
    match interface.interface_class {
        "rigid_edge" | "rigid_point" | "rigid_vertex" | "fluid_boundary" | "rigid_surface" => {
            LiveFamilyResolution::Unresolved
        }
        _ => LiveFamilyResolution::Unresolved,
    }
}

/// Resolve a runtime contact using the geometry that was actually realized.
///
/// Unlike the topology-only resolver above, this includes the local contact
/// point and the realized boundary parameter when one exists. It still does
/// not search Bob's formation space or sample orientations. The result is a
/// canonical live identity; persistent-family resolution remains a separate
/// step until the library has enough metadata to match it uniquely.
pub fn resolve_live_contact_candidate(
    material_a: &str,
    unit_a: &crate::structure::StructuralUnit,
    material_b: &str,
    unit_b: &crate::structure::StructuralUnit,
    candidate: crate::contact::ConnectionPairCandidate,
    catalog: &[BaseResource],
) -> Option<LiveGeometryInterface> {
    // Distance and facing are runtime state, not interface identity. The
    // canonical key must survive movement while retaining the realized local
    // geometry of the contact itself.
    let local_a = local_contact_descriptor(candidate.endpoint_a, unit_a, catalog)?;
    let local_b = local_contact_descriptor(candidate.endpoint_b, unit_b, catalog)?;

    let class = match (
        endpoint_class(candidate.endpoint_a),
        endpoint_class(candidate.endpoint_b),
    ) {
        ("fluid", _) | (_, "fluid") => "fluid_boundary",
        ("boundary", "boundary") => "rigid_edge",
        ("corner", "boundary") | ("boundary", "corner") | ("corner", "corner") => "rigid_vertex",
        ("line", _) | (_, "line") => "rigid_point",
        _ => "rigid_surface",
    };

    let candidate_rotation = if class == "rigid_vertex"
        && local_a.starts_with("corner:")
        && local_b.starts_with("corner:")
    {
        // A corner-corner contact does not identify which endpoint is the
        // persisted family candidate. Do not attach one side's rotation to
        // an arbitrary candidate role.
        None
    } else {
        let rotation = if local_a.starts_with("line:") || local_a.starts_with("corner:") {
            unit_a.placement.rotation_radians
        } else {
            unit_b.placement.rotation_radians
        };
        Some(quantize(normalized_angle(rotation)))
    };

    let mut sides = [(material_a, local_a), (material_b, local_b)];
    sides.sort_by(|a, b| a.cmp(b));

    let query = match class {
        "rigid_edge" => match (
            parse_edge_descriptor(sides[0].0, &sides[0].1),
            parse_edge_descriptor(sides[1].0, &sides[1].1),
        ) {
            (Some(a), Some(b)) => Some(LiveGeometryQuery::RigidEdge {
                a_material: a.material,
                a_edge: a.edge,
                a_parameter: quantize(a.parameter),
                a_rotation: quantize(a.rotation),
                b_material: b.material,
                b_edge: b.edge,
                b_parameter: quantize(b.parameter),
                b_rotation: quantize(b.rotation),
            }),
            _ => None,
        },
        "rigid_point" => {
            let parsed = if sides[0].1.starts_with("line:") {
                parse_point_descriptor(sides[0].0, &sides[0].1, "line:").and_then(|line| {
                    parse_edge_descriptor(sides[1].0, &sides[1].1).map(|edge| (line, edge))
                })
            } else {
                parse_point_descriptor(sides[1].0, &sides[1].1, "line:").and_then(|line| {
                    parse_edge_descriptor(sides[0].0, &sides[0].1).map(|edge| (line, edge))
                })
            };
            parsed.map(|(line, edge)| LiveGeometryQuery::RigidPoint {
                line_material: line.material,
                line_point: line.point_index,
                edge_material: edge.material,
                edge: edge.edge,
                edge_parameter: quantize(edge.parameter),
                candidate_rotation,
            })
        }
        "rigid_vertex" => {
            let parsed = if sides[0].1.starts_with("corner:") {
                parse_point_descriptor(sides[0].0, &sides[0].1, "corner:").and_then(|corner| {
                    parse_edge_descriptor(sides[1].0, &sides[1].1).map(|edge| (corner, edge))
                })
            } else if sides[1].1.starts_with("corner:") {
                parse_point_descriptor(sides[1].0, &sides[1].1, "corner:").and_then(|corner| {
                    parse_edge_descriptor(sides[0].0, &sides[0].1).map(|edge| (corner, edge))
                })
            } else {
                None
            };
            parsed.map(|(corner, edge)| LiveGeometryQuery::RigidVertex {
                corner_material: corner.material,
                corner_point: corner.point_index,
                edge_material: edge.material,
                edge: edge.edge,
                edge_parameter: quantize(edge.parameter),
                candidate_rotation,
            })
        }
        "fluid_boundary" => {
            let parsed = if sides[0].1.starts_with("fluid:") {
                parse_edge_descriptor(sides[1].0, &sides[1].1).map(|edge| (sides[0].0, edge))
            } else if sides[1].1.starts_with("fluid:") {
                parse_edge_descriptor(sides[0].0, &sides[0].1).map(|edge| (sides[1].0, edge))
            } else {
                None
            };
            parsed.and_then(|(fluid_material, edge)| {
                let fluid = catalog
                    .iter()
                    .find(|resource| resource.name == fluid_material)?;
                let solid = catalog
                    .iter()
                    .find(|resource| resource.name == edge.material)?;
                let area = match &fluid.shape.form {
                    Form::Circle { radius } => std::f64::consts::PI * *radius * *radius,
                    Form::Fluid { nominal_area, .. } => *nominal_area,
                    _ => return None,
                };
                let equilibrium = solve_water_against_solid(
                    area,
                    fluid.properties.cohesion,
                    solid.properties.cohesion,
                )?;
                Some(LiveGeometryQuery::FluidBoundary {
                    fluid_material: fluid_material.to_string(),
                    solid_material: edge.material,
                    solid_edge: edge.edge,
                    edge_parameter: quantize(edge.parameter),
                    fluid_area: quantize(area),
                    contact_angle: quantize(equilibrium.contact_angle_radians),
                })
            })
        }
        _ => None,
    };
    Some(LiveGeometryInterface {
        interface_class: class,
        signature: format!(
            "live-v{}|{}:{}|{}:{}",
            LIVE_GEOMETRY_INTERFACE_SCHEMA_VERSION, sides[0].0, sides[0].1, sides[1].0, sides[1].1,
        ),
        query,
    })
}

fn local_contact_descriptor(
    endpoint: ConnectionEndpoint,
    unit: &crate::structure::StructuralUnit,
    catalog: &[BaseResource],
) -> Option<String> {
    match endpoint {
        ConnectionEndpoint::Corner { point_index } => Some(format!("corner:{}", point_index)),
        ConnectionEndpoint::LineEndpoint { point_index } => Some(format!("line:{}", point_index)),
        ConnectionEndpoint::Fluid { x, y } => {
            Some(format!("fluid:{},{}", quantize(x), quantize(y)))
        }
        ConnectionEndpoint::Boundary { angle_radians } => {
            let shape = unit.shape(catalog)?;
            let (s, c) = angle_radians.sin_cos();
            let point = crate::surface_geometry::boundary_point_toward(shape, c, s)?;
            let edge = shape.form.polygon_vertices().and_then(|vertices| {
                (0..vertices.len()).find_map(|i| {
                    let a = vertices[i];
                    let b = vertices[(i + 1) % vertices.len()];
                    let dx = b.0 - a.0;
                    let dy = b.1 - a.1;
                    let len2 = dx * dx + dy * dy;
                    if len2 <= 1e-24 {
                        return None;
                    }
                    let t = ((point.x - a.0) * dx + (point.y - a.1) * dy) / len2;
                    if !(-1e-9..=1.000000001).contains(&t) {
                        return None;
                    }
                    let px = a.0 + dx * t.clamp(0.0, 1.0);
                    let py = a.1 + dy * t.clamp(0.0, 1.0);
                    if (px - point.x).hypot(py - point.y) <= 1e-8 {
                        Some(format!(
                            "edge:{}@{}@{}",
                            i,
                            quantize(t.clamp(0.0, 1.0)),
                            quantize(normalized_angle(unit.placement.rotation_radians))
                        ))
                    } else {
                        None
                    }
                })
            });
            edge.or_else(|| {
                Some(format!(
                    "boundary:{}",
                    quantize(normalized_angle(angle_radians))
                ))
            })
        }
    }
}

fn endpoint_descriptor(endpoint: ConnectionEndpoint) -> String {
    match endpoint {
        ConnectionEndpoint::Corner { point_index } => format!("corner:{point_index}"),
        ConnectionEndpoint::LineEndpoint { point_index } => format!("line:{point_index}"),
        ConnectionEndpoint::Boundary { angle_radians } => {
            format!("boundary:{}", quantize(normalized_angle(angle_radians)))
        }
        ConnectionEndpoint::Fluid { x, y } => {
            format!("fluid:{},{}", quantize(x), quantize(y))
        }
    }
}

fn endpoint_class(endpoint: ConnectionEndpoint) -> &'static str {
    match endpoint {
        ConnectionEndpoint::Corner { .. } => "corner",
        ConnectionEndpoint::LineEndpoint { .. } => "line",
        ConnectionEndpoint::Boundary { .. } => "boundary",
        ConnectionEndpoint::Fluid { .. } => "fluid",
    }
}

/* Restored Bob generation/lookup helpers. These remain pure library operations:
live construction stays authoritative for physical validity. */

#[derive(Clone, Debug)]
struct ParsedEdgeDescriptor {
    material: String,
    edge: usize,
    parameter: f64,
    rotation: f64,
}

#[derive(Clone, Debug)]
struct ParsedPointDescriptor {
    material: String,
    point_index: usize,
}

fn parse_edge_descriptor(material: &str, descriptor: &str) -> Option<ParsedEdgeDescriptor> {
    let rest = descriptor.strip_prefix("edge:")?;
    let mut fields = rest.split('@');
    let edge = fields.next()?.parse().ok()?;
    let parameter = fields.next()?.parse::<i64>().ok()? as f64 / 1e9;
    let rotation = fields.next()?.parse::<i64>().ok()? as f64 / 1e9;
    fields.next().is_none().then_some(ParsedEdgeDescriptor {
        material: material.to_owned(),
        edge,
        parameter,
        rotation,
    })
}

fn parse_point_descriptor(
    material: &str,
    descriptor: &str,
    prefix: &str,
) -> Option<ParsedPointDescriptor> {
    let value = descriptor.strip_prefix(prefix)?.parse().ok()?;
    Some(ParsedPointDescriptor {
        material: material.to_owned(),
        point_index: value,
    })
}

fn contact_bucket_hash(material: &str, anchor_feature: usize, candidate_feature: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    material.hash(&mut hasher);
    anchor_feature.hash(&mut hasher);
    candidate_feature.hash(&mut hasher);
    hasher.finish()
}

fn rigid_family_projection(
    family: &GeometryRigidContactFamily,
    relative_rotation_radians: f64,
) -> String {
    format!(
        "edge|{}|{}|{}|{}|{}|{}",
        family.candidate_resource,
        family.anchor_edge,
        family.candidate_edge,
        quantize(relative_rotation_radians),
        quantize(family.anchor_parameter_start),
        quantize(family.anchor_parameter_end),
    )
}

fn point_family_projection(family: &GeometryRigidPointContactFamily) -> String {
    format!(
        "point|{}|{}|{}|{}|{}|{}|{}",
        family.candidate_resource,
        family.anchor_edge,
        family.candidate_endpoint,
        quantize(family.anchor_parameter_start),
        quantize(family.anchor_parameter_end),
        quantize(family.candidate_rotation_start_radians),
        quantize(family.candidate_rotation_end_radians),
    )
}

fn vertex_family_projection(family: &GeometryRigidVertexContactFamily) -> String {
    format!(
        "vertex|{}|{}|{}|{}|{}|{}|{}",
        family.candidate_resource,
        family.anchor_edge,
        family.candidate_vertex,
        quantize(family.anchor_parameter_start),
        quantize(family.anchor_parameter_end),
        quantize(family.candidate_rotation_start_radians),
        quantize(family.candidate_rotation_end_radians),
    )
}

/// Resolve a live contact into a canonical Bob-side interface identity.
///
/// This function is intentionally a resolver, not a geometry generator:
/// callers provide the endpoints that contact detection has already realized.
/// If the same physical interface is presented in the opposite endpoint order,
/// the returned identity is unchanged.
pub fn resolve_live_contact_interface(
    material_a: &str,
    endpoint_a: ConnectionEndpoint,
    material_b: &str,
    endpoint_b: ConnectionEndpoint,
) -> LiveGeometryInterface {
    let class = match (endpoint_class(endpoint_a), endpoint_class(endpoint_b)) {
        ("fluid", _) | (_, "fluid") => "fluid_boundary",
        ("boundary", "boundary") => "rigid_edge",
        ("corner", "boundary") | ("boundary", "corner") | ("corner", "corner") => "rigid_vertex",
        ("line", _) | (_, "line") => "rigid_point",
        _ => "rigid_surface",
    };

    let mut sides = [
        (material_a, endpoint_descriptor(endpoint_a)),
        (material_b, endpoint_descriptor(endpoint_b)),
    ];
    sides.sort_by(|a, b| a.cmp(b));

    let query = match class {
        "rigid_edge" => match (
            parse_edge_descriptor(sides[0].0, &sides[0].1),
            parse_edge_descriptor(sides[1].0, &sides[1].1),
        ) {
            (Some(a), Some(b)) => Some(LiveGeometryQuery::RigidEdge {
                a_material: a.material,
                a_edge: a.edge,
                a_parameter: quantize(a.parameter),
                a_rotation: quantize(a.rotation),
                b_material: b.material,
                b_edge: b.edge,
                b_parameter: quantize(b.parameter),
                b_rotation: quantize(b.rotation),
            }),
            _ => None,
        },
        "rigid_point" => {
            let parsed = if sides[0].1.starts_with("line:") {
                parse_point_descriptor(sides[0].0, &sides[0].1, "line:").and_then(|line| {
                    parse_edge_descriptor(sides[1].0, &sides[1].1).map(|edge| (line, edge))
                })
            } else {
                parse_point_descriptor(sides[1].0, &sides[1].1, "line:").and_then(|line| {
                    parse_edge_descriptor(sides[0].0, &sides[0].1).map(|edge| (line, edge))
                })
            };
            parsed.map(|(line, edge)| LiveGeometryQuery::RigidPoint {
                line_material: line.material,
                line_point: line.point_index,
                edge_material: edge.material,
                edge: edge.edge,
                edge_parameter: quantize(edge.parameter),
                candidate_rotation: None,
            })
        }
        "rigid_vertex" => {
            let parsed = if sides[0].1.starts_with("corner:") {
                parse_point_descriptor(sides[0].0, &sides[0].1, "corner:").and_then(|corner| {
                    parse_edge_descriptor(sides[1].0, &sides[1].1).map(|edge| (corner, edge))
                })
            } else {
                parse_point_descriptor(sides[1].0, &sides[1].1, "corner:").and_then(|corner| {
                    parse_edge_descriptor(sides[0].0, &sides[0].1).map(|edge| (corner, edge))
                })
            };
            parsed.map(|(corner, edge)| LiveGeometryQuery::RigidVertex {
                corner_material: corner.material,
                corner_point: corner.point_index,
                edge_material: edge.material,
                edge: edge.edge,
                edge_parameter: quantize(edge.parameter),
                candidate_rotation: None,
            })
        }
        _ => None,
    };
    LiveGeometryInterface {
        interface_class: class,
        signature: format!(
            "live-v{}|{}:{}|{}:{}",
            LIVE_GEOMETRY_INTERFACE_SCHEMA_VERSION, sides[0].0, sides[0].1, sides[1].0, sides[1].1
        ),
        query,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryContactFamily {
    pub schema_version: u32,
    pub formation_signature: String,
    pub candidate_resource: String,
    pub anchor_constituent: usize,
    pub anchor_edge: usize,
    pub contact_angle_radians: f64,
    pub curvature_radius: f64,
    pub contact_length: f64,
    pub edge_parameter_start: f64,
    pub edge_parameter_end: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryFluidBoundaryFamily {
    pub schema_version: u32,
    pub formation_signature: String,
    pub fluid_resource: String,
    pub anchor_constituent: usize,
    pub anchor_edge: usize,
    pub area: f64,
    pub contact_angle_radians: f64,
    pub curvature_radius: f64,
    pub free_arc_angle_radians: f64,
    pub contact_length: f64,
    pub edge_parameter_start: f64,
    pub edge_parameter_end: f64,
}

/// One rigid-surface constraint for a fluid-inclusive formation. Parameters
/// describe the contact interval on an edge of the canonical rigid parent.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryFluidContactConstraint {
    pub constituent_index: usize,
    pub edge_index: usize,
    pub contact_length: f64,
    pub edge_parameter_start: f64,
    pub edge_parameter_end: f64,
}

/// A persistent fluid-inclusive arrangement: a rigid parent formation plus
/// one deformable fluid constituent. The legacy anchor/scalar fields remain
/// readable; contact_constraints is the forward-compatible representation for
/// simultaneous contact with multiple rigid surfaces.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryFluidFormation {
    pub schema_version: u32,
    pub parent_formation_signature: String,
    pub fluid_resource: String,
    pub anchor_constituent: usize,
    pub anchor_edge: usize,
    pub area: f64,
    pub contact_angle_radians: f64,
    pub curvature_radius: f64,
    pub free_arc_angle_radians: f64,
    pub contact_length: f64,
    pub edge_parameter_start: f64,
    pub edge_parameter_end: f64,
    #[serde(default)]
    pub contact_constraints: Vec<GeometryFluidContactConstraint>,
    /// Complete boundary in the canonical rigid-parent coordinate frame.
    /// Present only when the simultaneous multi-contact solver has solved it.
    #[serde(default)]
    pub solved_boundary: Option<MultiContactFluidBoundary>,
}

impl GeometryFluidFormation {
    /// Realize this suggestion in its parent's coordinate frame.
    ///
    /// The stored edge parameters bound the allowed *contact-center* interval,
    /// not the chord endpoints. The actual chord is derived from the conserved
    /// area and must fit on the selected anchor edge. The caller must still
    /// transform this pose to the live structure and revalidate it there.
    pub fn realize_against_parent(
        &self,
        parent: &GeometryFormation,
        catalog: &[BaseResource],
        bulge_left: bool,
    ) -> Option<(crate::resources::Shape, Placement)> {
        if parent.signature != self.parent_formation_signature
            || !validate_formation(parent, catalog)
            || self.anchor_constituent >= parent.constituents.len()
            || !self.edge_parameter_start.is_finite()
            || !self.edge_parameter_end.is_finite()
            || self.edge_parameter_start < 0.0
            || self.edge_parameter_end > 1.0
            || self.edge_parameter_end < self.edge_parameter_start
        {
            return None;
        }
        // A multi-contact record is only realizable when Bob has persisted the
        // complete solved boundary. Never degrade it to the legacy single arc.
        if !self.contact_constraints.is_empty() || self.solved_boundary.is_some() {
            let boundary = self.solved_boundary.as_ref()?;
            if self.contact_constraints.len() < 2
                || boundary.contact_count() != self.contact_constraints.len()
                || !boundary.is_valid()
                || !self.area.is_finite()
                || (boundary.nominal_area - self.area).abs() > 1e-9 * self.area.max(1.0)
                || !multi_contact_boundary_matches_constraints(
                    boundary,
                    parent,
                    &self.contact_constraints,
                    catalog,
                )
                || self.contact_constraints.iter().any(|constraint| {
                    constraint.constituent_index >= parent.constituents.len()
                        || !parent
                            .constituents
                            .get(constraint.constituent_index)
                            .and_then(|constituent| {
                                catalog
                                    .iter()
                                    .find(|item| item.name == constituent.resource)
                            })
                            .is_some_and(|resource| {
                                constraint.edge_index
                                    < rigid_boundary_segments(&resource.shape.form).len()
                            })
                        || !constraint.contact_length.is_finite()
                        || constraint.contact_length <= 0.0
                        || !constraint.edge_parameter_start.is_finite()
                        || !constraint.edge_parameter_end.is_finite()
                        || constraint.edge_parameter_start < 0.0
                        || constraint.edge_parameter_end > 1.0
                        || constraint.edge_parameter_end <= constraint.edge_parameter_start
                })
            {
                return None;
            }
            let shape = crate::resources::Shape {
                form: Form::Fluid {
                    nominal_area: self.area,
                    boundary: None,
                    arc_boundary: None,
                    multi_contact_boundary: Some(boundary.clone()),
                },
            };
            if !shape.is_valid() {
                return None;
            }
            // The stored boundary is already in the parent's canonical frame.
            // Keep its placement at the frame origin; the caller maps the whole
            // realized arrangement into the live organism frame and revalidates.
            return Some((
                shape,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        let anchor = parent.constituents.get(self.anchor_constituent)?;
        let resource = catalog.iter().find(|item| item.name == anchor.resource)?;
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            return None;
        }
        let (edge_start, edge_end) =
            *rigid_boundary_segments(&resource.shape.form).get(self.anchor_edge)?;
        let edge_start = world_point(edge_start, anchor.placement);
        let edge_end = world_point(edge_end, anchor.placement);
        let dx = edge_end.0 - edge_start.0;
        let dy = edge_end.1 - edge_start.1;
        let edge_length = dx.hypot(dy);
        if !edge_length.is_finite() || edge_length <= f64::EPSILON {
            return None;
        }
        let arc = self.exact_arc_boundary()?;
        let half_fraction = (self.contact_length * 0.5) / edge_length;
        let center_parameter = (self.edge_parameter_start + self.edge_parameter_end) * 0.5;
        let interval_tolerance = 1e-9;
        if half_fraction > 0.5 + interval_tolerance
            || center_parameter < half_fraction - interval_tolerance
            || center_parameter > 1.0 - half_fraction + interval_tolerance
        {
            return None;
        }
        let shape = crate::resources::Shape {
            form: Form::Fluid {
                nominal_area: self.area,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        if !shape.is_valid() {
            return None;
        }
        Some((
            shape,
            Placement {
                x: edge_start.0 + dx * center_parameter,
                y: edge_start.1 + dy * center_parameter,
                rotation_radians: dy.atan2(dx)
                    + if bulge_left {
                        0.0
                    } else {
                        std::f64::consts::PI
                    },
            },
        ))
    }

    /// Reconstruct the exact free interface represented by this persisted record.
    /// Invalid or internally inconsistent stored scalars produce a coverage miss.
    pub fn exact_arc_boundary(&self) -> Option<CircularArcBoundary> {
        CircularArcBoundary::from_persisted_solution(
            self.area,
            self.contact_angle_radians,
            self.curvature_radius,
            self.free_arc_angle_radians,
            self.contact_length,
        )
    }

    pub fn signature(&self) -> String {
        let mut signature = format!(
            "fluid-v{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.parent_formation_signature,
            self.fluid_resource,
            self.anchor_constituent,
            self.anchor_edge,
            quantize(self.area),
            quantize(self.contact_angle_radians),
            quantize(self.curvature_radius),
            quantize(self.free_arc_angle_radians),
            quantize(self.contact_length),
            quantize(self.edge_parameter_start),
            quantize(self.edge_parameter_end),
        );
        for constraint in &self.contact_constraints {
            signature.push_str(&format!(
                "|contact:{}:{}:{}:{}:{}",
                constraint.constituent_index,
                constraint.edge_index,
                quantize(constraint.contact_length),
                quantize(constraint.edge_parameter_start),
                quantize(constraint.edge_parameter_end),
            ));
        }
        if let Some(boundary) = &self.solved_boundary {
            signature.push_str(&format!(
                "|solved-boundary:{}:{}",
                quantize(boundary.nominal_area),
                multi_contact_boundary_signature(boundary),
            ));
        }
        signature
    }
}

fn multi_contact_boundary_signature(boundary: &MultiContactFluidBoundary) -> String {
    boundary
        .pieces
        .iter()
        .map(|piece| match piece {
            FluidBoundaryPiece::SolidContact {
                start_x,
                start_y,
                end_x,
                end_y,
                contact_angle_radians,
            } => format!(
                "S:{},{},{},{},{}",
                quantize(*start_x),
                quantize(*start_y),
                quantize(*end_x),
                quantize(*end_y),
                quantize(*contact_angle_radians),
            ),
            FluidBoundaryPiece::FreeArc(arc) => format!(
                "A:{},{},{},{},{}",
                quantize(arc.center_x),
                quantize(arc.center_y),
                quantize(arc.radius),
                quantize(arc.start_angle_radians),
                quantize(arc.sweep_radians),
            ),
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn multi_contact_boundary_matches_constraints(
    boundary: &MultiContactFluidBoundary,
    parent: &GeometryFormation,
    constraints: &[GeometryFluidContactConstraint],
    catalog: &[BaseResource],
) -> bool {
    const TOLERANCE: f64 = 1e-7;
    if constraints.len() < 2 || boundary.contact_count() != constraints.len() {
        return false;
    }
    let mut matched_pieces = std::collections::BTreeSet::new();

    for constraint in constraints {
        let Some(constituent) = parent.constituents.get(constraint.constituent_index) else {
            return false;
        };
        let Some(resource) = catalog
            .iter()
            .find(|item| item.name == constituent.resource)
        else {
            return false;
        };
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            return false;
        }
        let Some((local_start, local_end)) = rigid_boundary_segments(&resource.shape.form)
            .get(constraint.edge_index)
            .copied()
        else {
            return false;
        };
        let edge_start = world_point(local_start, constituent.placement);
        let edge_end = world_point(local_end, constituent.placement);
        let edge_dx = edge_end.0 - edge_start.0;
        let edge_dy = edge_end.1 - edge_start.1;
        let edge_length_squared = edge_dx * edge_dx + edge_dy * edge_dy;
        let edge_length = edge_length_squared.sqrt();
        if !edge_length.is_finite() || edge_length <= TOLERANCE {
            return false;
        }

        let mut found = None;
        for (piece_index, piece) in boundary.pieces.iter().enumerate() {
            if matched_pieces.contains(&piece_index) {
                continue;
            }
            let FluidBoundaryPiece::SolidContact {
                start_x,
                start_y,
                end_x,
                end_y,
                ..
            } = piece
            else {
                continue;
            };
            let project = |x: f64, y: f64| {
                let dx = x - edge_start.0;
                let dy = y - edge_start.1;
                let parameter = (dx * edge_dx + dy * edge_dy) / edge_length_squared;
                let nearest_x = edge_start.0 + parameter * edge_dx;
                let nearest_y = edge_start.1 + parameter * edge_dy;
                let distance = (x - nearest_x).hypot(y - nearest_y);
                (parameter, distance)
            };
            let (start_parameter, start_distance) = project(*start_x, *start_y);
            let (end_parameter, end_distance) = project(*end_x, *end_y);
            let segment_length = (*end_x - *start_x).hypot(*end_y - *start_y);
            let center_parameter = (start_parameter + end_parameter) * 0.5;
            if start_distance <= TOLERANCE
                && end_distance <= TOLERANCE
                && start_parameter >= -TOLERANCE
                && start_parameter <= 1.0 + TOLERANCE
                && end_parameter >= -TOLERANCE
                && end_parameter <= 1.0 + TOLERANCE
                && (segment_length - constraint.contact_length).abs() <= TOLERANCE
                && center_parameter >= constraint.edge_parameter_start - TOLERANCE
                && center_parameter <= constraint.edge_parameter_end + TOLERANCE
            {
                found = Some(piece_index);
                break;
            }
        }
        let Some(piece_index) = found else {
            return false;
        };
        matched_pieces.insert(piece_index);
    }

    matched_pieces.len() == boundary.contact_count()
}

/// Build one persisted Bob record from a simultaneous collinear capillary
/// solution and the exact parent-edge constraints corresponding to its ordered
/// contact pieces. Candidate edge selection remains the caller's job; this
/// adapter verifies the entire solution before returning it.
#[allow(dead_code)] // Called by the forthcoming exposed-edge candidate enumerator.
pub fn solve_collinear_multi_contact_formation(
    parent: &GeometryFormation,
    fluid_resource: &BaseResource,
    constraints: Vec<GeometryFluidContactConstraint>,
    ordered_segments: &[ContactSegment],
    catalog: &[BaseResource],
) -> Option<GeometryFluidFormation> {
    if !validate_formation(parent, catalog)
        || fluid_resource.name != "Water"
        || fluid_resource.physical_state != crate::resources::PhysicalState::Fluid
        || constraints.len() < 2
        || constraints.len() != ordered_segments.len()
    {
        return None;
    }
    let area = match &fluid_resource.shape.form {
        Form::Circle { radius } => std::f64::consts::PI * radius * radius,
        Form::Fluid { nominal_area, .. } => *nominal_area,
        _ => return None,
    };
    let first_constraint = constraints.first()?;
    let first_constituent = parent
        .constituents
        .get(first_constraint.constituent_index)?;
    let first_solid = catalog
        .iter()
        .find(|resource| resource.name == first_constituent.resource)?;
    if first_solid.physical_state == crate::resources::PhysicalState::Fluid {
        return None;
    }
    let family = solve_water_against_solid(
        area,
        fluid_resource.properties.cohesion,
        first_solid.properties.cohesion,
    )?;
    let mut distinct_constituents = std::collections::BTreeSet::new();
    for constraint in &constraints {
        let constituent = parent.constituents.get(constraint.constituent_index)?;
        let solid = catalog
            .iter()
            .find(|resource| resource.name == constituent.resource)?;
        if solid.physical_state == crate::resources::PhysicalState::Fluid {
            return None;
        }
        let compatible = solve_water_against_solid(
            area,
            fluid_resource.properties.cohesion,
            solid.properties.cohesion,
        )?;
        if (compatible.contact_angle_radians - family.contact_angle_radians).abs() > 1e-9 {
            return None;
        }
        distinct_constituents.insert(constraint.constituent_index);
    }
    if distinct_constituents.len() < 2 {
        return None;
    }
    let boundary = solve_water_against_collinear_contacts(
        area,
        family.contact_angle_radians,
        ordered_segments,
    )?;
    if boundary.contact_count() != constraints.len()
        || !multi_contact_boundary_matches_constraints(&boundary, parent, &constraints, catalog)
    {
        return None;
    }
    let anchor = first_constraint;
    let formation = GeometryFluidFormation {
        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
        parent_formation_signature: parent.signature.clone(),
        fluid_resource: fluid_resource.name.clone(),
        anchor_constituent: anchor.constituent_index,
        anchor_edge: anchor.edge_index,
        area,
        contact_angle_radians: family.contact_angle_radians,
        curvature_radius: family.curvature_radius,
        free_arc_angle_radians: family.free_arc_angle_radians,
        contact_length: family.contact_length,
        edge_parameter_start: anchor.edge_parameter_start,
        edge_parameter_end: anchor.edge_parameter_end,
        contact_constraints: constraints,
        solved_boundary: Some(boundary),
    };
    formation
        .realize_against_parent(parent, catalog, true)
        .map(|_| formation)
}

fn clip_aligned_contact_window(
    aligned: &[AlignedExposedInterval],
    start_projection: f64,
    contact_length: f64,
    direction: (f64, f64),
) -> Option<Vec<AlignedExposedInterval>> {
    let end_projection = start_projection + contact_length;
    let tolerance = 1e-8 * contact_length.max(1.0);
    let mut cursor = start_projection;
    let mut chosen = Vec::new();

    for item in aligned {
        if item.1 <= cursor + tolerance || item.0 >= end_projection - tolerance {
            continue;
        }
        let natural_start = item.0.max(start_projection);
        if natural_start > cursor + tolerance {
            return None;
        }
        let clipped_start = cursor.max(natural_start);
        let clipped_end = item.1.min(end_projection);
        if clipped_end <= clipped_start + tolerance {
            continue;
        }

        let source = item.2;
        let source_projection_start = source.4 .0 * direction.0 + source.4 .1 * direction.1;
        let source_projection_end = source.5 .0 * direction.0 + source.5 .1 * direction.1;
        let projection_span = source_projection_end - source_projection_start;
        if projection_span.abs() <= tolerance {
            return None;
        }
        let start_ratio = (clipped_start - source_projection_start) / projection_span;
        let end_ratio = (clipped_end - source_projection_start) / projection_span;
        let start_parameter = source.2 + start_ratio * (source.3 - source.2);
        let end_parameter = source.2 + end_ratio * (source.3 - source.2);
        let start_point = (
            source.4 .0 + start_ratio * (source.5 .0 - source.4 .0),
            source.4 .1 + start_ratio * (source.5 .1 - source.4 .1),
        );
        let end_point = (
            source.4 .0 + end_ratio * (source.5 .0 - source.4 .0),
            source.4 .1 + end_ratio * (source.5 .1 - source.4 .1),
        );
        let clipped_source = (
            source.0,
            source.1,
            start_parameter.min(end_parameter),
            start_parameter.max(end_parameter),
            source.4,
            source.5,
        );
        chosen.push((
            clipped_start,
            clipped_end,
            clipped_source,
            start_point,
            end_point,
        ));
        cursor = clipped_end;
        if cursor >= end_projection - tolerance {
            break;
        }
    }

    (cursor >= end_projection - tolerance && !chosen.is_empty()).then_some(chosen)
}

/// Discover complete multi-contact Water formations from exposed rigid-edge
/// intervals in a canonical parent. This first enumerator accepts exact
/// contiguous intervals whose combined length equals the analytic contact
/// chord; it never clips edges or combines independent pairwise boundaries.
pub fn generate_collinear_multi_contact_formations(
    parent: &GeometryFormation,
    fluid_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryFluidFormation> {
    if fluid_resource.name != "Water"
        || fluid_resource.physical_state != crate::resources::PhysicalState::Fluid
        || !validate_formation(parent, catalog)
    {
        return Vec::new();
    }
    let area = match &fluid_resource.shape.form {
        Form::Circle { radius } => std::f64::consts::PI * *radius * *radius,
        Form::Fluid { nominal_area, .. } => *nominal_area,
        _ => return Vec::new(),
    };
    let mut exposed = Vec::<(usize, usize, f64, f64, (f64, f64), (f64, f64))>::new();
    for (constituent_index, constituent) in parent.constituents.iter().enumerate() {
        let Some(resource) = catalog.iter().find(|r| r.name == constituent.resource) else {
            continue;
        };
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            continue;
        }
        let intervals = match &resource.shape.form {
            Form::Line { .. } => exposed_line_intervals(parent, constituent_index, catalog),
            form if form.polygon_vertices().is_some() => {
                exposed_polygon_edge_intervals(parent, constituent_index, catalog)
            }
            _ => Vec::new(),
        };
        let segments = rigid_boundary_segments(&resource.shape.form);
        for interval in intervals {
            let Some((local_start, local_end)) = segments.get(interval.edge).copied() else {
                continue;
            };
            let start = (
                local_start.0 + (local_end.0 - local_start.0) * interval.start,
                local_start.1 + (local_end.1 - local_start.1) * interval.start,
            );
            let end = (
                local_start.0 + (local_end.0 - local_start.0) * interval.end,
                local_start.1 + (local_end.1 - local_start.1) * interval.end,
            );
            let world_start = world_point(start, constituent.placement);
            let world_end = world_point(end, constituent.placement);
            if (world_end.0 - world_start.0).hypot(world_end.1 - world_start.1) > 1e-9 {
                exposed.push((
                    constituent_index,
                    interval.edge,
                    interval.start,
                    interval.end,
                    world_start,
                    world_end,
                ));
            }
        }
    }

    let mut generated = BTreeMap::<String, GeometryFluidFormation>::new();
    let mut processed_line_families = std::collections::BTreeSet::new();
    for seed in &exposed {
        let seed_dx = seed.5 .0 - seed.4 .0;
        let seed_dy = seed.5 .1 - seed.4 .1;
        let seed_length = seed_dx.hypot(seed_dy);
        if seed_length <= 1e-9 {
            continue;
        }
        let mut direction = (seed_dx / seed_length, seed_dy / seed_length);
        if direction.0 < -1e-9 || (direction.0.abs() <= 1e-9 && direction.1 < 0.0) {
            direction = (-direction.0, -direction.1);
        }
        let normal = (-direction.1, direction.0);
        let line_offset = seed.4 .0 * normal.0 + seed.4 .1 * normal.1;
        let seed_constituent = seed.0;
        let Some(seed_resource) = catalog
            .iter()
            .find(|r| r.name == parent.constituents[seed_constituent].resource)
        else {
            continue;
        };
        let Some(seed_family) = solve_water_against_solid(
            area,
            fluid_resource.properties.cohesion,
            seed_resource.properties.cohesion,
        ) else {
            continue;
        };
        let line_family_key = (
            quantize(direction.0),
            quantize(direction.1),
            quantize(line_offset),
            quantize(seed_family.contact_angle_radians),
        );
        if !processed_line_families.insert(line_family_key) {
            continue;
        }
        let mut aligned = exposed
            .iter()
            .filter_map(|edge| {
                let dx = edge.5 .0 - edge.4 .0;
                let dy = edge.5 .1 - edge.4 .1;
                let length = dx.hypot(dy);
                if length <= 1e-9 {
                    return None;
                }
                let parallel_error = (dx * direction.1 - dy * direction.0).abs() / length;
                let offset_a = edge.4 .0 * normal.0 + edge.4 .1 * normal.1;
                let offset_b = edge.5 .0 * normal.0 + edge.5 .1 * normal.1;
                if parallel_error > 1e-8
                    || (offset_a - line_offset).abs() > 1e-7
                    || (offset_b - line_offset).abs() > 1e-7
                {
                    return None;
                }
                let projection_a = edge.4 .0 * direction.0 + edge.4 .1 * direction.1;
                let projection_b = edge.5 .0 * direction.0 + edge.5 .1 * direction.1;
                let (start, end) = if projection_a <= projection_b {
                    (edge.4, edge.5)
                } else {
                    (edge.5, edge.4)
                };
                Some((
                    projection_a.min(projection_b),
                    projection_a.max(projection_b),
                    *edge,
                    start,
                    end,
                ))
            })
            .collect::<Vec<_>>();
        aligned.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let contact_length = seed_family.contact_length;
        let mut start_positions = Vec::new();
        for item in &aligned {
            start_positions.push(item.0);
            start_positions.push((item.1 - contact_length * 0.5).clamp(item.0, item.1));
            start_positions.push((item.1 - contact_length).clamp(item.0, item.1));
        }
        start_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        start_positions.dedup_by(|a, b| (*a - *b).abs() <= 1e-8);

        for start_projection in start_positions {
            let Some(chosen) =
                clip_aligned_contact_window(&aligned, start_projection, contact_length, direction)
            else {
                continue;
            };
            let distinct_constituents = chosen
                .iter()
                .map(|item| item.2 .0)
                .collect::<std::collections::BTreeSet<_>>();
            if distinct_constituents.len() < 2 || distinct_constituents.len() != chosen.len() {
                continue;
            }
            let Some(first_resource) = catalog
                .iter()
                .find(|r| r.name == parent.constituents[chosen[0].2 .0].resource)
            else {
                continue;
            };
            let Some(family) = solve_water_against_solid(
                area,
                fluid_resource.properties.cohesion,
                first_resource.properties.cohesion,
            ) else {
                continue;
            };
            let length_tolerance = 1e-8 * family.contact_length.max(1.0);
            if (contact_length - family.contact_length).abs() > length_tolerance {
                continue;
            }

            let mut constraints = Vec::with_capacity(chosen.len());
            let mut compatible = true;
            for item in &chosen {
                let constituent_index = item.2 .0;
                let Some(resource) = catalog
                    .iter()
                    .find(|r| r.name == parent.constituents[constituent_index].resource)
                else {
                    compatible = false;
                    break;
                };
                let Some(other_family) = solve_water_against_solid(
                    area,
                    fluid_resource.properties.cohesion,
                    resource.properties.cohesion,
                ) else {
                    compatible = false;
                    break;
                };
                if (other_family.contact_angle_radians - family.contact_angle_radians).abs() > 1e-9
                {
                    compatible = false;
                    break;
                }
                constraints.push(GeometryFluidContactConstraint {
                    constituent_index,
                    edge_index: item.2 .1,
                    contact_length: item.1 - item.0,
                    edge_parameter_start: item.2 .2,
                    edge_parameter_end: item.2 .3,
                });
            }
            if !compatible {
                continue;
            }
            let segments = chosen
                .iter()
                .map(|item| (item.3, item.4))
                .collect::<Vec<_>>();
            if let Some(formation) = solve_collinear_multi_contact_formation(
                parent,
                fluid_resource,
                constraints,
                &segments,
                catalog,
            ) {
                generated.entry(formation.signature()).or_insert(formation);
            }
        }
    }
    generated.into_values().collect()
}

pub fn fluid_formations_from_boundaries(
    boundaries: &[GeometryFluidBoundaryFamily],
) -> Vec<GeometryFluidFormation> {
    boundaries
        .iter()
        .map(|family| GeometryFluidFormation {
            schema_version: family.schema_version,
            parent_formation_signature: family.formation_signature.clone(),
            fluid_resource: family.fluid_resource.clone(),
            anchor_constituent: family.anchor_constituent,
            anchor_edge: family.anchor_edge,
            area: family.area,
            contact_angle_radians: family.contact_angle_radians,
            curvature_radius: family.curvature_radius,
            free_arc_angle_radians: family.free_arc_angle_radians,
            contact_length: family.contact_length,
            edge_parameter_start: family.edge_parameter_start,
            edge_parameter_end: family.edge_parameter_end,
            contact_constraints: Vec::new(),
            solved_boundary: None,
        })
        .collect()
}

fn fluid_boundary_state_is_self_consistent(family: &GeometryFluidBoundaryFamily) -> bool {
    if !(0.0 < family.contact_angle_radians && family.contact_angle_radians < std::f64::consts::PI)
    {
        return false;
    }
    if family.edge_parameter_start < 0.0
        || family.edge_parameter_end > 1.0
        || family.edge_parameter_end < family.edge_parameter_start
        || family.area <= 0.0
        || family.curvature_radius <= 0.0
        || family.contact_length <= 0.0
        || family.free_arc_angle_radians <= 0.0
    {
        return false;
    }

    let Some(expected) = CapillaryContactFamily::solve(family.area, family.contact_angle_radians)
    else {
        return false;
    };

    (expected.curvature_radius - family.curvature_radius).abs()
        <= QUANTUM * expected.curvature_radius.max(1.0)
        && (expected.contact_length - family.contact_length).abs()
            <= QUANTUM * expected.contact_length.max(1.0)
        && (expected.free_arc_angle_radians - family.free_arc_angle_radians).abs()
            <= QUANTUM * expected.free_arc_angle_radians.max(1.0)
        && family.edge_parameter_end - family.edge_parameter_start >= 0.0
}

impl GeometryFluidBoundaryFamily {
    pub fn signature(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.formation_signature,
            self.fluid_resource,
            self.anchor_constituent,
            self.anchor_edge,
            quantize(self.area),
            quantize(self.contact_angle_radians),
            quantize(self.curvature_radius),
            quantize(self.free_arc_angle_radians),
            quantize(self.edge_parameter_start),
            quantize(self.edge_parameter_end),
            quantize(self.contact_length),
        )
    }
}

impl GeometryContactFamily {
    pub fn signature(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.formation_signature,
            self.candidate_resource,
            self.anchor_constituent,
            self.anchor_edge,
            quantize(self.contact_angle_radians),
            quantize(self.curvature_radius),
            quantize(self.edge_parameter_start),
            quantize(self.edge_parameter_end)
        )
    }
}

/// A geometry-library proposal for placing one rigid candidate edge against an
/// already-realized anchor edge. This is advisory: callers must still run the
/// live collision, contact, bond, and energy checks before committing it.
#[derive(Clone, Debug, PartialEq)]
pub struct RigidEdgePlacementSuggestion {
    pub candidate_resource: String,
    pub candidate_edge: usize,
    pub anchor_edge: usize,
    pub anchor_contact_parameter: f64,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryRigidContactFamily {
    pub schema_version: u32,
    pub formation_signature: String,
    pub candidate_resource: String,
    pub anchor_constituent: usize,
    pub anchor_edge: usize,
    pub candidate_edge: usize,
    pub candidate_rotation_radians: f64,
    /// Exposed interval on the anchor edge where a contact point may lie.
    /// These are normalized edge parameters in [0, 1], not candidate-edge
    /// translation coordinates.
    pub anchor_parameter_start: f64,
    pub anchor_parameter_end: f64,
}

impl GeometryRigidContactFamily {
    pub fn signature(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.formation_signature,
            self.candidate_resource,
            self.anchor_constituent,
            self.anchor_edge,
            self.candidate_edge,
            quantize(self.candidate_rotation_radians),
            quantize(self.anchor_parameter_start),
            quantize(self.anchor_parameter_end),
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryRigidPointContactFamily {
    pub schema_version: u32,
    pub formation_signature: String,
    pub candidate_resource: String,
    pub anchor_constituent: usize,
    pub anchor_edge: usize,
    pub candidate_endpoint: usize,
    pub anchor_parameter_start: f64,
    pub anchor_parameter_end: f64,
    pub candidate_rotation_start_radians: f64,
    pub candidate_rotation_end_radians: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryRigidVertexContactFamily {
    pub schema_version: u32,
    pub formation_signature: String,
    pub candidate_resource: String,
    pub anchor_constituent: usize,
    pub anchor_edge: usize,
    pub candidate_vertex: usize,
    pub anchor_parameter_start: f64,
    pub anchor_parameter_end: f64,
    pub candidate_rotation_start_radians: f64,
    pub candidate_rotation_end_radians: f64,
}

impl GeometryRigidVertexContactFamily {
    pub fn signature(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.formation_signature,
            self.candidate_resource,
            self.anchor_constituent,
            self.anchor_edge,
            self.candidate_vertex,
            quantize(self.anchor_parameter_start),
            quantize(self.anchor_parameter_end),
            quantize(self.candidate_rotation_start_radians),
            quantize(self.candidate_rotation_end_radians),
            "vertex",
        )
    }
}

impl GeometryRigidPointContactFamily {
    pub fn signature(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.formation_signature,
            self.candidate_resource,
            self.anchor_constituent,
            self.anchor_edge,
            self.candidate_endpoint,
            quantize(self.anchor_parameter_start),
            quantize(self.anchor_parameter_end),
            quantize(self.candidate_rotation_start_radians),
            quantize(self.candidate_rotation_end_radians),
            "point",
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryConstituent {
    pub resource: String,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometryBond {
    pub constituent_a: usize,
    pub constituent_b: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryFormation {
    pub schema_version: u32,
    pub constituents: Vec<GeometryConstituent>,
    pub bonds: Vec<GeometryBond>,
    pub signature: String,
}

impl GeometryFormation {
    pub fn single(resource: impl Into<String>) -> Self {
        let mut formation = Self {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![GeometryConstituent {
                resource: resource.into(),
                placement: Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            }],
            bonds: Vec::new(),
            signature: String::new(),
        };
        formation.signature = formation.canonical_signature();
        formation
    }

    pub fn constituent_count(&self) -> usize {
        self.constituents.len()
    }

    pub fn canonicalized(self, catalog: &[BaseResource]) -> Option<Self> {
        self.canonicalized_with_mapping(catalog)
            .map(|(formation, _)| formation)
    }

    /// Canonicalize a formation while preserving the mapping from each input
    /// constituent index to its canonical index. If equally canonical poses
    /// assign the same canonical slots to different input constituents, the
    /// mapping is ambiguous and this method rejects it rather than guessing.
    pub fn canonicalized_with_mapping(
        self,
        catalog: &[BaseResource],
    ) -> Option<(Self, Vec<usize>)> {
        if !validate_formation(&self, catalog) {
            return None;
        }

        let input_count = self.constituents.len();
        let candidates = canonical_pose_candidates(&self);
        let mut best: Option<(Self, Vec<usize>)> = None;
        let mut ambiguous_best = false;
        for mut candidate in candidates {
            let mut indexed: Vec<(usize, GeometryConstituent)> =
                candidate.constituents.into_iter().enumerate().collect();
            indexed.sort_by(|(_, a), (_, b)| {
                a.resource
                    .cmp(&b.resource)
                    .then_with(|| quantize(a.placement.x).cmp(&quantize(b.placement.x)))
                    .then_with(|| quantize(a.placement.y).cmp(&quantize(b.placement.y)))
                    .then_with(|| {
                        quantize(normalized_angle(a.placement.rotation_radians))
                            .cmp(&quantize(normalized_angle(b.placement.rotation_radians)))
                    })
            });
            candidate.constituents = indexed.iter().map(|(_, c)| c.clone()).collect();

            for constituent in &mut candidate.constituents {
                if let Some(resource) = catalog.iter().find(|r| r.name == constituent.resource) {
                    constituent.placement.rotation_radians = canonical_shape_rotation(
                        &resource.shape.form,
                        constituent.placement.rotation_radians,
                    );
                }
            }

            let mut remap = vec![0usize; input_count];
            for (new_index, (old_index, _)) in indexed.iter().enumerate() {
                remap[*old_index] = new_index;
            }
            for bond in &mut candidate.bonds {
                bond.constituent_a = remap[bond.constituent_a];
                bond.constituent_b = remap[bond.constituent_b];
                if bond.constituent_a > bond.constituent_b {
                    std::mem::swap(&mut bond.constituent_a, &mut bond.constituent_b);
                }
            }
            candidate
                .bonds
                .sort_by_key(|b| (b.constituent_a, b.constituent_b));

            if !validate_formation(&candidate, catalog) {
                continue;
            }
            candidate.signature = candidate.canonical_signature();

            match best.as_ref() {
                None => {
                    best = Some((candidate, remap));
                    ambiguous_best = false;
                }
                Some((existing, _existing_mapping)) if candidate.signature < existing.signature => {
                    best = Some((candidate, remap));
                    ambiguous_best = false;
                }
                Some((existing, existing_mapping)) if candidate.signature == existing.signature => {
                    if existing_mapping != &remap {
                        ambiguous_best = true;
                    }
                }
                _ => {}
            }
        }
        if ambiguous_best {
            None
        } else {
            best
        }
    }

    pub fn canonical_signature(&self) -> String {
        let mut out = format!("v{}|", self.schema_version);
        for c in &self.constituents {
            out.push_str(&c.resource);
            out.push('@');
            out.push_str(&quantize(c.placement.x).to_string());
            out.push(',');
            out.push_str(&quantize(c.placement.y).to_string());
            out.push(',');
            out.push_str(&quantize(normalized_angle(c.placement.rotation_radians)).to_string());
            out.push(';');
        }
        out.push('|');
        for b in &self.bonds {
            out.push_str(&format!("{}-{};", b.constituent_a, b.constituent_b));
        }
        out
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GeometryLibraryManifest {
    pub schema_version: u32,
    pub resource_catalog_version: String,
    pub entries: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum GeometryFrontierState {
    Unexplored,
    InProgress,
    Exhausted,
    ContinuousFamilyPending,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GeometryFrontierRecord {
    pub formation_signature: String,
    pub candidate_resource: String,
    pub state: GeometryFrontierState,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct GeometryFrontier {
    pub records: BTreeMap<String, GeometryFrontierRecord>,
}

#[derive(Debug)]
pub struct GeometryLibrary {
    root: PathBuf,
    entries: BTreeMap<String, GeometryFormation>,
    equivalence_index: BTreeMap<String, Vec<String>>,
    manifest: GeometryLibraryManifest,
    frontier: GeometryFrontier,
    contact_families: BTreeMap<String, GeometryContactFamily>,
    fluid_boundary_families: BTreeMap<String, GeometryFluidBoundaryFamily>,
    fluid_formations: BTreeMap<String, GeometryFluidFormation>,
    rigid_contact_families: BTreeMap<String, GeometryRigidContactFamily>,
    rigid_contact_index: HashMap<u64, Vec<String>>,
    rigid_point_contact_families: BTreeMap<String, GeometryRigidPointContactFamily>,
    rigid_point_contact_index: HashMap<u64, Vec<String>>,
    rigid_vertex_contact_families: BTreeMap<String, GeometryRigidVertexContactFamily>,
    rigid_vertex_contact_index: HashMap<u64, Vec<String>>,
    formation_ids: BTreeMap<String, String>,
    formation_storage_version: u32,
}

fn compact_formation_value(
    formation: &GeometryFormation,
    existing: &BTreeMap<String, GeometryFormation>,
    batch: &BTreeMap<String, GeometryFormation>,
) -> std::io::Result<serde_json::Value> {
    let mut full =
        serde_json::to_value(formation).map_err(|error| invalid_storage_data(error.to_string()))?;
    let full_object = full
        .as_object_mut()
        .ok_or_else(|| invalid_storage_data("formation record must serialize as an object"))?;
    full_object.remove("signature");
    full_object.insert(
        "formation_id".to_owned(),
        serde_json::Value::String(formation_id(&formation.signature)),
    );
    let mut best = full;
    let mut best_size = serde_json::to_vec(&best)
        .map_err(|error| invalid_storage_data(error.to_string()))?
        .len();

    if formation.constituents.len() <= 1 {
        return Ok(best);
    }
    for removed_index in 0..formation.constituents.len() {
        let removed_constituent = formation.constituents[removed_index].clone();
        let kept_constituents: Vec<_> = formation
            .constituents
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != removed_index)
            .map(|(_, constituent)| constituent.clone())
            .collect();
        let mut kept_bonds = Vec::new();
        let mut incident_bonds = Vec::new();
        for (bond_index, bond) in formation.bonds.iter().enumerate() {
            if bond.constituent_a == removed_index || bond.constituent_b == removed_index {
                incident_bonds.push(serde_json::json!({
                    "at": bond_index,
                    "bond": bond,
                }));
            } else {
                let mut bond = *bond;
                if bond.constituent_a > removed_index {
                    bond.constituent_a -= 1;
                }
                if bond.constituent_b > removed_index {
                    bond.constituent_b -= 1;
                }
                kept_bonds.push(bond);
            }
        }

        let base_probe = GeometryFormation {
            schema_version: formation.schema_version,
            constituents: kept_constituents.clone(),
            bonds: kept_bonds.clone(),
            signature: String::new(),
        };
        let base_signature = base_probe.canonical_signature();
        let Some(base) = existing
            .get(&base_signature)
            .or_else(|| batch.get(&base_signature))
        else {
            continue;
        };
        if base.schema_version != formation.schema_version
            || base.constituents != kept_constituents
            || base.bonds != kept_bonds
        {
            continue;
        }

        let candidate = serde_json::json!({
            "formation_id": formation_id(&formation.signature),
            "schema_version": formation.schema_version,
            "base_id": formation_id(&base.signature),
            "insert_at": removed_index,
            "constituent": removed_constituent,
            "incident_bonds": incident_bonds,
        });
        let candidate_size = serde_json::to_vec(&candidate)
            .map_err(|error| invalid_storage_data(error.to_string()))?
            .len();
        if candidate_size < best_size {
            best = candidate;
            best_size = candidate_size;
        }
    }
    Ok(best)
}

fn write_formation_record<W: Write>(
    writer: &mut W,
    formation: &GeometryFormation,
    existing: &BTreeMap<String, GeometryFormation>,
    batch: &BTreeMap<String, GeometryFormation>,
    storage_version: u32,
) -> std::io::Result<()> {
    if storage_version == 3 {
        let value = compact_formation_value(formation, existing, batch)?;
        serde_json::to_writer(writer, &value)
            .map_err(|error| invalid_storage_data(error.to_string()))
    } else {
        serde_json::to_writer(writer, formation)
            .map_err(|error| invalid_storage_data(error.to_string()))
    }
}

fn build_formation_id_index(
    entries: &BTreeMap<String, GeometryFormation>,
) -> std::io::Result<BTreeMap<String, String>> {
    let mut ids = BTreeMap::new();
    for signature in entries.keys() {
        let id = formation_id(signature);
        if let Some(previous) = ids.insert(id.clone(), signature.clone()) {
            if previous != *signature {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("formation ID collision for {id}"),
                ));
            }
        }
    }
    Ok(ids)
}

fn invalid_storage_data(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}

fn storage_format_version(root: &Path, formations_path: &Path) -> std::io::Result<u32> {
    let path = root.join("storage_manifest.json");
    if path.exists() {
        let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)
            .map_err(|error| invalid_storage_data(error.to_string()))?;
        let version = value
            .get("storage_format_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| invalid_storage_data("storage manifest lacks storage_format_version"))?;
        match version {
            1 => return Ok(1),
            3 => {
                if value
                    .get("formation_encoding")
                    .and_then(serde_json::Value::as_str)
                    != Some("exact-one-constituent-delta-v1")
                    || value
                        .get("formation_id_algorithm")
                        .and_then(serde_json::Value::as_str)
                        != Some("sha256-128-base64url")
                {
                    return Err(invalid_storage_data(
                        "unsupported compositional storage encoding",
                    ));
                }
                return Ok(3);
            }
            _ => {
                return Err(invalid_storage_data(format!(
                    "unsupported geometry storage format version: {version}"
                )));
            }
        }
    }
    if formations_path
        .metadata()
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
    {
        Ok(1)
    } else {
        Ok(3)
    }
}

fn write_storage_manifest(root: &Path, version: u32) -> std::io::Result<()> {
    let value = if version == 3 {
        serde_json::json!({
            "storage_format_version": 3,
            "formation_encoding": "exact-one-constituent-delta-v1",
            "formation_id_algorithm": "sha256-128-base64url",
            "family_encoding": "formation-id-reference-v2"
        })
    } else {
        serde_json::json!({
            "storage_format_version": 1,
            "formation_encoding": "legacy-full-signature",
            "formation_id_algorithm": "sha256-128-base64url",
            "family_encoding": "formation-id-reference-v2"
        })
    };
    let path = root.join("storage_manifest.json");
    let temp = root.join("storage_manifest.json.tmp");
    {
        let mut file = File::create(&temp)?;
        serde_json::to_writer(&mut file, &value)
            .map_err(|error| invalid_storage_data(error.to_string()))?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    fs::rename(temp, path)?;
    Ok(())
}

fn decode_compositional_formation(
    id: &str,
    rows: &BTreeMap<String, serde_json::Value>,
    cache: &mut BTreeMap<String, GeometryFormation>,
    active: &mut std::collections::BTreeSet<String>,
    depth: usize,
    catalog: &[BaseResource],
) -> std::io::Result<GeometryFormation> {
    const MAX_COMPOSITION_DEPTH: usize = 64;
    if let Some(formation) = cache.get(id) {
        return Ok(formation.clone());
    }
    if depth > MAX_COMPOSITION_DEPTH {
        return Err(invalid_storage_data(
            "compositional formation reference depth exceeds 64",
        ));
    }
    if !active.insert(id.to_owned()) {
        return Err(invalid_storage_data(format!(
            "cyclic compositional formation reference: {id}"
        )));
    }
    let row = rows
        .get(id)
        .cloned()
        .ok_or_else(|| invalid_storage_data(format!("unresolved compositional base ID: {id}")))?;
    let schema_version = row
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| invalid_storage_data(format!("formation row {id} lacks schema_version")))?
        as u32;

    let (constituents, bonds) = if let Some(base_value) = row.get("base_id") {
        let base_id = base_value.as_str().ok_or_else(|| {
            invalid_storage_data(format!("invalid base_id in formation row {id}"))
        })?;
        let base =
            decode_compositional_formation(base_id, rows, cache, active, depth + 1, catalog)?;
        if base.schema_version != schema_version {
            return Err(invalid_storage_data(format!(
                "schema version differs across formation delta {id}"
            )));
        }
        let insert_at = row
            .get("insert_at")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| invalid_storage_data(format!("formation delta {id} lacks insert_at")))?
            as usize;
        if insert_at > base.constituents.len() {
            return Err(invalid_storage_data(format!(
                "invalid constituent insertion index in {id}"
            )));
        }
        let constituent: GeometryConstituent =
            serde_json::from_value(row.get("constituent").cloned().ok_or_else(|| {
                invalid_storage_data(format!("formation delta {id} lacks constituent"))
            })?)
            .map_err(|error| invalid_storage_data(error.to_string()))?;
        let mut constituents = base.constituents;
        constituents.insert(insert_at, constituent);

        let mut shifted_base_bonds = base.bonds;
        for bond in &mut shifted_base_bonds {
            if bond.constituent_a >= insert_at {
                bond.constituent_a += 1;
            }
            if bond.constituent_b >= insert_at {
                bond.constituent_b += 1;
            }
        }
        let incident_rows = row
            .get("incident_bonds")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                invalid_storage_data(format!("formation delta {id} lacks incident_bonds"))
            })?;
        let mut incident = BTreeMap::<usize, GeometryBond>::new();
        for item in incident_rows {
            let position = item
                .get("at")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    invalid_storage_data(format!("invalid incident-bond position in {id}"))
                })? as usize;
            let bond: GeometryBond =
                serde_json::from_value(item.get("bond").cloned().ok_or_else(|| {
                    invalid_storage_data(format!("incident bond missing in {id}"))
                })?)
                .map_err(|error| invalid_storage_data(error.to_string()))?;
            if bond.constituent_a != insert_at && bond.constituent_b != insert_at {
                return Err(invalid_storage_data(format!(
                    "incident bond does not touch inserted constituent in {id}"
                )));
            }
            if bond.constituent_a >= constituents.len()
                || bond.constituent_b >= constituents.len()
                || incident.insert(position, bond).is_some()
            {
                return Err(invalid_storage_data(format!(
                    "invalid or duplicate incident bond in {id}"
                )));
            }
        }
        let total_bonds = shifted_base_bonds.len() + incident.len();
        if incident.keys().any(|position| *position >= total_bonds) {
            return Err(invalid_storage_data(format!(
                "incident-bond position outside reconstructed list in {id}"
            )));
        }
        let mut base_iter = shifted_base_bonds.into_iter();
        let mut bonds = Vec::with_capacity(total_bonds);
        for position in 0..total_bonds {
            if let Some(bond) = incident.remove(&position) {
                bonds.push(bond);
            } else {
                bonds.push(base_iter.next().ok_or_else(|| {
                    invalid_storage_data(format!(
                        "bond reconstruction underflow in formation delta {id}"
                    ))
                })?);
            }
        }
        if base_iter.next().is_some() || !incident.is_empty() {
            return Err(invalid_storage_data(format!(
                "bond reconstruction mismatch in formation delta {id}"
            )));
        }
        (constituents, bonds)
    } else {
        let allowed = ["formation_id", "schema_version", "constituents", "bonds"];
        if let Some(object) = row.as_object() {
            if object.keys().any(|key| !allowed.contains(&key.as_str())) {
                return Err(invalid_storage_data(format!(
                    "unexpected field in full compositional formation row {id}"
                )));
            }
        }
        let constituents: Vec<GeometryConstituent> =
            serde_json::from_value(row.get("constituents").cloned().ok_or_else(|| {
                invalid_storage_data(format!("full formation row {id} lacks constituents"))
            })?)
            .map_err(|error| invalid_storage_data(error.to_string()))?;
        let bonds: Vec<GeometryBond> =
            serde_json::from_value(row.get("bonds").cloned().ok_or_else(|| {
                invalid_storage_data(format!("full formation row {id} lacks bonds"))
            })?)
            .map_err(|error| invalid_storage_data(error.to_string()))?;
        (constituents, bonds)
    };

    let mut formation = GeometryFormation {
        schema_version,
        constituents,
        bonds,
        signature: String::new(),
    };
    let signature = formation.canonical_signature();
    if formation_id(&signature) != id {
        return Err(invalid_storage_data(format!(
            "canonical signature does not match formation ID {id}"
        )));
    }
    if formation.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
        || !validate_formation(&formation, catalog)
    {
        return Err(invalid_storage_data(format!(
            "invalid reconstructed formation {id}"
        )));
    }
    formation.signature = signature;
    active.remove(id);
    cache.insert(id.to_owned(), formation.clone());
    Ok(formation)
}

fn load_compositional_formations(
    path: &Path,
    catalog: &[BaseResource],
) -> std::io::Result<BTreeMap<String, GeometryFormation>> {
    let mut rows = BTreeMap::<String, serde_json::Value>::new();
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let file = File::open(path)?;
    let mut lines = BufReader::new(file).lines().peekable();
    while let Some(line_result) = lines.next() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        let is_last = lines.peek().is_none();
        let row: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) if is_last => {
                let _ = error;
                continue;
            }
            Err(error) => return Err(invalid_storage_data(error.to_string())),
        };
        let id = row
            .get("formation_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_storage_data("compositional formation row lacks formation_id"))?
            .to_owned();
        if rows.insert(id.clone(), row).is_some() {
            return Err(invalid_storage_data(format!(
                "duplicate formation ID in storage: {id}"
            )));
        }
    }

    let mut cache = BTreeMap::new();
    let mut active = std::collections::BTreeSet::new();
    let mut entries = BTreeMap::new();
    for id in rows.keys() {
        let formation =
            decode_compositional_formation(id, &rows, &mut cache, &mut active, 0, catalog)?;
        if entries
            .insert(formation.signature.clone(), formation)
            .is_some()
        {
            return Err(invalid_storage_data(
                "duplicate canonical formation signature in compositional storage",
            ));
        }
    }
    Ok(entries)
}

impl GeometryLibrary {
    pub fn open(root: impl AsRef<Path>, catalog: &[BaseResource]) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let data_path = root.join("formations.jsonl");
        let manifest_path = root.join("manifest.json");
        let frontier_path = root.join("frontier.json");
        let contact_family_path = root.join("contact_families.jsonl");
        let fluid_boundary_family_path = root.join("fluid_boundary_families.jsonl");
        let fluid_formation_path = root.join("fluid_formations.jsonl");
        let rigid_contact_family_path = root.join("rigid_contact_families.jsonl");
        let rigid_point_contact_family_path = root.join("rigid_point_contact_families.jsonl");
        let rigid_vertex_contact_family_path = root.join("rigid_vertex_contact_families.jsonl");

        let formation_storage_version = storage_format_version(&root, &data_path)?;
        let entries = if formation_storage_version == 3 {
            load_compositional_formations(&data_path, catalog)?
        } else {
            let mut entries = BTreeMap::new();
            if data_path.exists() {
                let file = File::open(&data_path)?;
                let mut lines = BufReader::new(file).lines().peekable();
                while let Some(line_result) = lines.next() {
                    let line = line_result?;
                    let is_last = lines.peek().is_none();
                    if line.trim().is_empty() {
                        continue;
                    }
                    let formation: GeometryFormation = match serde_json::from_str(&line) {
                        Ok(value) => value,
                        Err(error) if is_last => {
                            // An interrupted final append can leave a truncated
                            // JSON record. Earlier durable records remain valid;
                            // ignore only the incomplete tail so restart can resume.
                            let _ = error;
                            continue;
                        }
                        Err(error) => {
                            return Err(invalid_storage_data(error.to_string()));
                        }
                    };
                    if formation.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                        || !validate_formation(&formation, catalog)
                        || formation.signature != formation.canonical_signature()
                    {
                        continue;
                    }
                    entries.insert(formation.signature.clone(), formation);
                }
            }
            entries
        };

        let catalog_version = resource_catalog_signature(catalog);
        let manifest = if manifest_path.exists() {
            let bytes = fs::read(&manifest_path)?;
            serde_json::from_slice(&bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        } else {
            GeometryLibraryManifest {
                schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                resource_catalog_version: catalog_version.clone(),
                entries: 0,
            }
        };

        if manifest.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
            || manifest.resource_catalog_version != catalog_version
        {
            // A geometry/schema change must not silently reuse stale knowledge.
            // Keep the old artifact intact; the caller can migrate or create a
            // new versioned root explicitly.
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "geometry library version/catalog mismatch",
            ));
        }

        let formation_ids = build_formation_id_index(&entries)?;

        let mut contact_families = BTreeMap::new();
        if contact_family_path.exists() {
            let file = File::open(&contact_family_path)?;
            let mut lines = BufReader::new(file).lines().peekable();
            while let Some(line_result) = lines.next() {
                let line = line_result?;
                let is_last = lines.peek().is_none();
                if line.trim().is_empty() {
                    continue;
                }
                let family: GeometryContactFamily =
                    match deserialize_family_record(&line, &formation_ids) {
                        Ok(value) => value,
                        Err(error) if is_last => {
                            let _ = error;
                            continue;
                        }
                        Err(error) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                error,
                            ));
                        }
                    };
                if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                    || !family.contact_angle_radians.is_finite()
                    || !family.curvature_radius.is_finite()
                    || !family.contact_length.is_finite()
                    || !family.edge_parameter_start.is_finite()
                    || !family.edge_parameter_end.is_finite()
                    || family.edge_parameter_start < 0.0
                    || family.edge_parameter_end > 1.0
                    || family.edge_parameter_end < family.edge_parameter_start
                    || !entries.contains_key(&family.formation_signature)
                    || catalog
                        .iter()
                        .all(|resource| resource.name != family.candidate_resource)
                    || family.anchor_constituent
                        >= entries
                            .get(&family.formation_signature)
                            .map(|formation| formation.constituents.len())
                            .unwrap_or(0)
                {
                    continue;
                }
                contact_families.insert(family.signature(), family);
            }
        }

        let mut rigid_contact_families = BTreeMap::new();
        if rigid_contact_family_path.exists() {
            let file = File::open(&rigid_contact_family_path)?;
            let mut lines = BufReader::new(file).lines().peekable();
            while let Some(line_result) = lines.next() {
                let line = line_result?;
                let is_last = lines.peek().is_none();
                if line.trim().is_empty() {
                    continue;
                }
                let family: GeometryRigidContactFamily =
                    match deserialize_family_record(&line, &formation_ids) {
                        Ok(value) => value,
                        Err(error) if is_last => {
                            let _ = error;
                            continue;
                        }
                        Err(error) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                error,
                            ));
                        }
                    };
                if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                    || !family.candidate_rotation_radians.is_finite()
                    || !family.anchor_parameter_start.is_finite()
                    || !family.anchor_parameter_end.is_finite()
                    || family.anchor_parameter_start < 0.0
                    || family.anchor_parameter_end > 1.0
                    || family.anchor_parameter_start > family.anchor_parameter_end
                    || !entries.contains_key(&family.formation_signature)
                    || family.anchor_constituent
                        >= entries
                            .get(&family.formation_signature)
                            .map(|formation| formation.constituents.len())
                            .unwrap_or(0)
                    || catalog
                        .iter()
                        .all(|resource| resource.name != family.candidate_resource)
                {
                    continue;
                }
                let formation = &entries[&family.formation_signature];
                let Some(anchor_resource) = catalog.iter().find(|resource| {
                    resource.name == formation.constituents[family.anchor_constituent].resource
                }) else {
                    continue;
                };
                let Some(candidate_resource) = catalog
                    .iter()
                    .find(|resource| resource.name == family.candidate_resource)
                else {
                    continue;
                };
                if family.anchor_edge >= rigid_boundary_segments(&anchor_resource.shape.form).len()
                    || family.candidate_edge
                        >= rigid_boundary_segments(&candidate_resource.shape.form).len()
                {
                    continue;
                }
                rigid_contact_families.insert(family.signature(), family);
            }
        }

        let frontier = if frontier_path.exists() {
            let bytes = fs::read(&frontier_path)?;
            serde_json::from_slice(&bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        } else {
            GeometryFrontier::default()
        };

        let fluid_boundary_families = load_fluid_boundary_families(
            &fluid_boundary_family_path,
            &entries,
            catalog,
            &formation_ids,
        );
        let fluid_formations = load_fluid_formations(&fluid_formation_path, &entries, catalog)?;
        let rigid_point_contact_families = load_rigid_point_contact_families(
            &rigid_point_contact_family_path,
            &entries,
            catalog,
            &formation_ids,
        );
        let rigid_vertex_contact_families = load_rigid_vertex_contact_families(
            &rigid_vertex_contact_family_path,
            &entries,
            catalog,
            &formation_ids,
        );

        let mut rigid_contact_index = HashMap::<u64, Vec<String>>::new();
        for (signature, family) in &rigid_contact_families {
            rigid_contact_index
                .entry(contact_bucket_hash(
                    &family.candidate_resource,
                    family.anchor_edge,
                    family.candidate_edge,
                ))
                .or_default()
                .push(signature.clone());
        }

        let mut rigid_point_contact_index = HashMap::<u64, Vec<String>>::new();
        for (signature, family) in &rigid_point_contact_families {
            rigid_point_contact_index
                .entry(contact_bucket_hash(
                    &family.candidate_resource,
                    family.anchor_edge,
                    family.candidate_endpoint,
                ))
                .or_default()
                .push(signature.clone());
        }
        let mut rigid_vertex_contact_index = HashMap::<u64, Vec<String>>::new();
        for (signature, family) in &rigid_vertex_contact_families {
            rigid_vertex_contact_index
                .entry(contact_bucket_hash(
                    &family.candidate_resource,
                    family.anchor_edge,
                    family.candidate_vertex,
                ))
                .or_default()
                .push(signature.clone());
        }

        let mut equivalence_index = BTreeMap::<String, Vec<String>>::new();
        for (signature, formation) in &entries {
            equivalence_index
                .entry(formation_equivalence_key(formation))
                .or_default()
                .push(signature.clone());
        }

        let mut library = Self {
            root,
            entries,
            equivalence_index,
            manifest,
            frontier,
            contact_families,
            fluid_boundary_families,
            fluid_formations,
            rigid_contact_families,
            rigid_contact_index,
            rigid_point_contact_families,
            rigid_point_contact_index,
            rigid_vertex_contact_families,
            rigid_vertex_contact_index,
            formation_ids,
            formation_storage_version,
        };
        library.manifest.entries = library.entries.len() as u64;
        library.write_manifest()?;
        if !library.root.join("storage_manifest.json").exists() {
            write_storage_manifest(&library.root, library.formation_storage_version)?;
        }
        Ok(library)
    }

    pub fn load_formations_only(
        root: impl AsRef<Path>,
        catalog: &[BaseResource],
    ) -> std::io::Result<Vec<GeometryFormation>> {
        let root = root.as_ref();
        let path = root.join("formations.jsonl");
        if storage_format_version(root, &path)? == 3 {
            return Ok(load_compositional_formations(&path, catalog)?
                .into_values()
                .collect());
        }
        let mut entries = BTreeMap::new();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(path)?;
        for (line_index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let formation: GeometryFormation = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(_) => {
                    let _ = line_index;
                    continue;
                }
            };
            if formation.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                || !validate_formation(&formation, catalog)
                || formation.signature != formation.canonical_signature()
            {
                continue;
            }
            entries.insert(formation.signature.clone(), formation);
        }
        Ok(entries.into_values().collect())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn contains(&self, signature: &str) -> bool {
        self.entries.contains_key(signature)
    }

    pub fn get(&self, signature: &str) -> Option<&GeometryFormation> {
        self.entries.get(signature)
    }

    pub fn formations(&self) -> impl Iterator<Item = &GeometryFormation> {
        self.entries.values()
    }

    pub fn frontier(&self) -> &GeometryFrontier {
        &self.frontier
    }

    pub fn contact_families(&self) -> impl Iterator<Item = &GeometryContactFamily> {
        self.contact_families.values()
    }

    pub fn fluid_formations(&self) -> impl Iterator<Item = &GeometryFluidFormation> {
        self.fluid_formations.values()
    }

    /// Read-only construction query for fluid formations recorded against an
    /// exact canonical rigid-parent arrangement. This does not realize or
    /// authorize a placement; callers must still derive a live fluid shape
    /// and pass it through the physical geometry/transaction authorities.
    pub fn fluid_formations_for_parent<'a>(
        &'a self,
        parent_formation_signature: &'a str,
        fluid_resource: &'a str,
    ) -> impl Iterator<Item = &'a GeometryFluidFormation> + 'a {
        self.fluid_formations.values().filter(move |formation| {
            formation.parent_formation_signature == parent_formation_signature
                && formation.fluid_resource == fluid_resource
        })
    }

    pub fn fluid_boundary_families(&self) -> impl Iterator<Item = &GeometryFluidBoundaryFamily> {
        self.fluid_boundary_families.values()
    }

    pub fn rigid_contact_families(&self) -> impl Iterator<Item = &GeometryRigidContactFamily> {
        self.rigid_contact_families.values()
    }

    pub fn rigid_point_contact_families(
        &self,
    ) -> impl Iterator<Item = &GeometryRigidPointContactFamily> {
        self.rigid_point_contact_families.values()
    }

    pub fn rigid_vertex_contact_families(
        &self,
    ) -> impl Iterator<Item = &GeometryRigidVertexContactFamily> {
        self.rigid_vertex_contact_families.values()
    }

    /// Match a realized interface against persisted family geometry without
    /// generating formations or sampling orientations. Multiple formation
    /// records that describe the same local interface collapse to one
    /// canonical projection; genuinely different projections remain ambiguous.
    pub fn persistent_interface_projection(
        &self,
        interface: &LiveGeometryInterface,
    ) -> Option<String> {
        let projections = self.indexed_interface_projections(interface);
        (projections.len() == 1).then(|| projections.into_keys().next().unwrap())
    }

    /// Propose candidate placements from persisted rigid-edge families.
    ///
    /// The family contributes a candidate edge, an anchor-relative orientation,
    /// and a normalized contact interval. The interval midpoint is used as a
    /// deterministic seed point; callers may explore other points and must
    /// validate every returned pose against the live physical structure.
    pub fn suggest_rigid_edge_placements(
        &self,
        anchor_material: &str,
        anchor_placement: Placement,
        candidate_resource: &BaseResource,
        catalog: &[BaseResource],
    ) -> Vec<RigidEdgePlacementSuggestion> {
        if candidate_resource.physical_state == crate::resources::PhysicalState::Fluid {
            return Vec::new();
        }
        let Some(anchor_resource) = catalog.iter().find(|r| r.name == anchor_material) else {
            return Vec::new();
        };
        let anchor_segments = rigid_boundary_segments(&anchor_resource.shape.form);
        let candidate_segments = rigid_boundary_segments(&candidate_resource.shape.form);
        let mut indexed_families = BTreeMap::<String, ()>::new();
        for anchor_edge in 0..anchor_segments.len() {
            for candidate_edge in 0..candidate_segments.len() {
                let key =
                    contact_bucket_hash(&candidate_resource.name, anchor_edge, candidate_edge);
                if let Some(signatures) = self.rigid_contact_index.get(&key) {
                    for signature in signatures {
                        indexed_families.insert(signature.clone(), ());
                    }
                }
            }
        }
        let mut families = Vec::<(GeometryRigidContactFamily, f64)>::new();
        for signature in indexed_families.keys() {
            let Some(family) = self.rigid_contact_families.get(signature) else {
                continue;
            };
            let Some(formation) = self.entries.get(&family.formation_signature) else {
                continue;
            };
            let Some(stored_anchor) = formation.constituents.get(family.anchor_constituent) else {
                continue;
            };
            if stored_anchor.resource == anchor_material {
                families.push((family.clone(), stored_anchor.placement.rotation_radians));
            }
        }
        // An intentionally empty catalogue is a supported bootstrap state.
        // Derive local single-anchor families on demand without persisting
        // unverified records or starting a bulk geometry worker.
        if families.is_empty() {
            let seed = GeometryFormation::single(anchor_material);
            families.extend(
                generate_rigid_contact_families(&seed, candidate_resource, catalog)
                    .into_iter()
                    .map(|family| (family, 0.0)),
            );
        }

        let mut suggestions = BTreeMap::<String, RigidEdgePlacementSuggestion>::new();
        for (family, stored_anchor_rotation) in families {
            if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                || family.candidate_resource != candidate_resource.name
            {
                continue;
            }
            let Some(&(a0, a1)) = anchor_segments.get(family.anchor_edge) else {
                continue;
            };
            let Some(&(c0, c1)) = candidate_segments.get(family.candidate_edge) else {
                continue;
            };
            let anchor_length = (a1.0 - a0.0).hypot(a1.1 - a0.1);
            let candidate_length = (c1.0 - c0.0).hypot(c1.1 - c0.1);
            if anchor_length <= QUANTUM || candidate_length <= QUANTUM {
                continue;
            }
            let start = family.anchor_parameter_start.clamp(0.0, 1.0);
            let end = family.anchor_parameter_end.clamp(0.0, 1.0);
            if start > end {
                continue;
            }
            let parameter = (start + end) * 0.5;
            let anchor_local = (
                a0.0 + (a1.0 - a0.0) * parameter,
                a0.1 + (a1.1 - a0.1) * parameter,
            );
            let anchor_world = world_point(anchor_local, anchor_placement);
            let anchor_edge_angle =
                (a1.1 - a0.1).atan2(a1.0 - a0.0) + anchor_placement.rotation_radians;
            let relative_rotation =
                normalize_angle(family.candidate_rotation_radians - stored_anchor_rotation);
            let rotation = normalize_angle(anchor_placement.rotation_radians + relative_rotation);
            let candidate_edge_angle = (c1.1 - c0.1).atan2(c1.0 - c0.0) + rotation;
            if normalize_angle(candidate_edge_angle - anchor_edge_angle).abs() > 1e-7
                && normalize_angle(candidate_edge_angle - anchor_edge_angle - std::f64::consts::PI)
                    .abs()
                    > 1e-7
            {
                continue;
            }
            let candidate_local_midpoint = ((c0.0 + c1.0) * 0.5, (c0.1 + c1.1) * 0.5);
            let candidate_rotated_midpoint = rotated_point(candidate_local_midpoint, rotation);
            let placement = Placement {
                x: anchor_world.0 - candidate_rotated_midpoint.0,
                y: anchor_world.1 - candidate_rotated_midpoint.1,
                rotation_radians: rotation,
            };
            let suggestion = RigidEdgePlacementSuggestion {
                candidate_resource: candidate_resource.name.clone(),
                candidate_edge: family.candidate_edge,
                anchor_edge: family.anchor_edge,
                anchor_contact_parameter: parameter,
                placement,
            };
            let key = format!(
                "{}|{}|{}|{}|{}|{}",
                suggestion.candidate_edge,
                suggestion.anchor_edge,
                quantize(suggestion.anchor_contact_parameter),
                quantize(suggestion.placement.x),
                quantize(suggestion.placement.y),
                quantize(suggestion.placement.rotation_radians)
            );
            suggestions.entry(key).or_insert(suggestion);
        }
        suggestions.into_values().collect()
    }

    pub fn resolve_persistent_interface(
        &self,
        interface: &LiveGeometryInterface,
    ) -> LiveFamilyResolution {
        match self.indexed_interface_projections(interface).len() {
            0 => LiveFamilyResolution::Unresolved,
            1 => LiveFamilyResolution::Unique,
            _ => LiveFamilyResolution::Ambiguous,
        }
    }

    fn indexed_interface_projections(
        &self,
        interface: &LiveGeometryInterface,
    ) -> BTreeMap<String, ()> {
        let mut projections = BTreeMap::<String, ()>::new();
        match interface.query.as_ref() {
            Some(LiveGeometryQuery::RigidEdge {
                a_material,
                a_edge,
                a_parameter,
                a_rotation,
                b_material,
                b_edge,
                b_parameter,
                b_rotation,
            }) => {
                for key in [
                    contact_bucket_hash(a_material, *b_edge, *a_edge),
                    contact_bucket_hash(b_material, *a_edge, *b_edge),
                ] {
                    if let Some(signatures) = self.rigid_contact_index.get(&key) {
                        for signature in signatures {
                            if let Some(family) = self.rigid_contact_families.get(signature) {
                                let candidate_is_a = if a_material == b_material
                                    && a_material == &family.candidate_resource
                                {
                                    // If the feature indices are also identical, the query
                                    // contains no fact that distinguishes candidate from anchor.
                                    if family.candidate_edge == family.anchor_edge {
                                        continue;
                                    }
                                    if *a_edge == family.candidate_edge
                                        && *b_edge == family.anchor_edge
                                    {
                                        true
                                    } else if *b_edge == family.candidate_edge
                                        && *a_edge == family.anchor_edge
                                    {
                                        false
                                    } else {
                                        continue;
                                    }
                                } else {
                                    a_material == &family.candidate_resource
                                };
                                let (
                                    candidate_material,
                                    candidate_edge,
                                    candidate_rotation,
                                    anchor_edge,
                                    anchor_parameter,
                                ) = if candidate_is_a {
                                    (
                                        a_material,
                                        *a_edge,
                                        *a_rotation as f64 / 1e9,
                                        *b_edge,
                                        *b_parameter as f64 / 1e9,
                                    )
                                } else {
                                    (
                                        b_material,
                                        *b_edge,
                                        *b_rotation as f64 / 1e9,
                                        *a_edge,
                                        *a_parameter as f64 / 1e9,
                                    )
                                };
                                let anchor = self
                                    .entries
                                    .get(&family.formation_signature)
                                    .and_then(|formation| {
                                        formation.constituents.get(family.anchor_constituent)
                                    });
                                let anchor_material =
                                    anchor.map(|constituent| constituent.resource.as_str());
                                let stored_anchor_rotation = anchor
                                    .map(|constituent| constituent.placement.rotation_radians)
                                    .unwrap_or_default();
                                let expected_relative_rotation = normalize_angle(
                                    family.candidate_rotation_radians - stored_anchor_rotation,
                                );
                                let realized_anchor_material = if candidate_is_a {
                                    b_material
                                } else {
                                    a_material
                                };
                                let realized_anchor_rotation = if candidate_is_a {
                                    *b_rotation as f64 / 1e9
                                } else {
                                    *a_rotation as f64 / 1e9
                                };
                                let realized_relative_rotation =
                                    normalize_angle(candidate_rotation - realized_anchor_rotation);
                                if candidate_material == &family.candidate_resource
                                    && realized_anchor_material
                                        == anchor_material.unwrap_or_default()
                                    && anchor_edge == family.anchor_edge
                                    && candidate_edge == family.candidate_edge
                                    && anchor_parameter >= family.anchor_parameter_start - QUANTUM
                                    && anchor_parameter <= family.anchor_parameter_end + QUANTUM
                                    && normalize_angle(
                                        realized_relative_rotation - expected_relative_rotation,
                                    )
                                    .abs()
                                        <= 1e-7
                                {
                                    projections.insert(
                                        rigid_family_projection(family, expected_relative_rotation),
                                        (),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Some(LiveGeometryQuery::RigidPoint {
                line_material,
                line_point,
                edge_material,
                edge,
                edge_parameter,
                candidate_rotation,
            }) => {
                let key = contact_bucket_hash(line_material, *edge, *line_point);
                if let Some(signatures) = self.rigid_point_contact_index.get(&key) {
                    for signature in signatures {
                        if let Some(family) = self.rigid_point_contact_families.get(signature) {
                            let parameter = *edge_parameter as f64 / 1e9;
                            let anchor_material = self
                                .entries
                                .get(&family.formation_signature)
                                .and_then(|formation| {
                                    formation.constituents.get(family.anchor_constituent)
                                })
                                .map(|constituent| constituent.resource.as_str());
                            if line_material == &family.candidate_resource
                                && edge_material == anchor_material.unwrap_or_default()
                                && *line_point == family.candidate_endpoint
                                && *edge == family.anchor_edge
                                && parameter >= family.anchor_parameter_start - QUANTUM
                                && parameter <= family.anchor_parameter_end + QUANTUM
                                && candidate_rotation.is_some_and(|rotation| {
                                    angle_in_periodic_interval(
                                        rotation as f64 / 1e9,
                                        family.candidate_rotation_start_radians,
                                        family.candidate_rotation_end_radians,
                                    )
                                })
                            {
                                projections.insert(point_family_projection(family), ());
                            }
                        }
                    }
                }
            }
            Some(LiveGeometryQuery::RigidVertex {
                corner_material,
                corner_point,
                edge_material,
                edge,
                edge_parameter,
                candidate_rotation,
            }) => {
                let key = contact_bucket_hash(corner_material, *edge, *corner_point);
                if let Some(signatures) = self.rigid_vertex_contact_index.get(&key) {
                    for signature in signatures {
                        if let Some(family) = self.rigid_vertex_contact_families.get(signature) {
                            let parameter = *edge_parameter as f64 / 1e9;
                            let anchor_material = self
                                .entries
                                .get(&family.formation_signature)
                                .and_then(|formation| {
                                    formation.constituents.get(family.anchor_constituent)
                                })
                                .map(|constituent| constituent.resource.as_str());
                            if corner_material == &family.candidate_resource
                                && edge_material == anchor_material.unwrap_or_default()
                                && *corner_point == family.candidate_vertex
                                && *edge == family.anchor_edge
                                && parameter >= family.anchor_parameter_start - QUANTUM
                                && parameter <= family.anchor_parameter_end + QUANTUM
                                && candidate_rotation.is_some_and(|rotation| {
                                    angle_in_periodic_interval(
                                        rotation as f64 / 1e9,
                                        family.candidate_rotation_start_radians,
                                        family.candidate_rotation_end_radians,
                                    )
                                })
                            {
                                projections.insert(vertex_family_projection(family), ());
                            }
                        }
                    }
                }
            }
            Some(LiveGeometryQuery::FluidBoundary {
                fluid_material,
                solid_material,
                solid_edge,
                edge_parameter,
                fluid_area,
                contact_angle,
            }) => {
                let parameter = *edge_parameter as f64 / 1e9;
                let area = *fluid_area as f64 / 1e9;
                let angle = *contact_angle as f64 / 1e9;
                for family in self.fluid_boundary_families.values() {
                    let anchor_material = self
                        .entries
                        .get(&family.formation_signature)
                        .and_then(|formation| formation.constituents.get(family.anchor_constituent))
                        .map(|constituent| constituent.resource.as_str());
                    if family.fluid_resource.as_str() == fluid_material.as_str()
                        && anchor_material == Some(solid_material.as_str())
                        && family.anchor_edge == *solid_edge
                        && (family.area - area).abs() <= QUANTUM * area.max(1.0)
                        && (family.contact_angle_radians - angle).abs() <= QUANTUM * angle.max(1.0)
                        && parameter >= family.edge_parameter_start - QUANTUM
                        && parameter <= family.edge_parameter_end + QUANTUM
                    {
                        let projection = format!(
                            "fluid|{}|{}|{}|{}|{}|{}|{}",
                            family.fluid_resource,
                            anchor_material.unwrap_or_default(),
                            family.anchor_edge,
                            quantize(family.area),
                            quantize(family.contact_angle_radians),
                            quantize(family.edge_parameter_start),
                            quantize(family.edge_parameter_end),
                        );
                        projections.insert(projection, ());
                    }
                }
            }
            None => {}
        }
        projections
    }
    pub fn insert_rigid_vertex_contact_families(
        &mut self,
        families: Vec<GeometryRigidVertexContactFamily>,
    ) -> std::io::Result<usize> {
        let mut unique = BTreeMap::new();
        for family in families {
            if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                || !family.anchor_parameter_start.is_finite()
                || !family.anchor_parameter_end.is_finite()
                || !family.candidate_rotation_start_radians.is_finite()
                || !family.candidate_rotation_end_radians.is_finite()
                || family.anchor_parameter_start > family.anchor_parameter_end
                || !self.entries.contains_key(&family.formation_signature)
                || family.anchor_constituent
                    >= self
                        .entries
                        .get(&family.formation_signature)
                        .map(|f| f.constituents.len())
                        .unwrap_or(0)
            {
                continue;
            }
            let signature = family.signature();
            if !self.rigid_vertex_contact_families.contains_key(&signature) {
                unique.insert(signature, family);
            }
        }
        if unique.is_empty() {
            return Ok(0);
        }
        let path = self.root.join("rigid_vertex_contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for family in unique.values() {
            write_compact_family(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
        for (signature, family) in &unique {
            self.rigid_vertex_contact_index
                .entry(contact_bucket_hash(
                    &family.candidate_resource,
                    family.anchor_edge,
                    family.candidate_vertex,
                ))
                .or_default()
                .push(signature.clone());
        }
        self.rigid_vertex_contact_families.extend(unique);
        Ok(added)
    }

    pub fn insert_rigid_point_contact_families(
        &mut self,
        families: Vec<GeometryRigidPointContactFamily>,
    ) -> std::io::Result<usize> {
        let mut unique = BTreeMap::new();
        for family in families {
            if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                || family.candidate_endpoint > 1
                || !family.anchor_parameter_start.is_finite()
                || !family.anchor_parameter_end.is_finite()
                || !family.candidate_rotation_start_radians.is_finite()
                || !family.candidate_rotation_end_radians.is_finite()
                || family.anchor_parameter_start > family.anchor_parameter_end
                || !self.entries.contains_key(&family.formation_signature)
                || family.anchor_constituent
                    >= self
                        .entries
                        .get(&family.formation_signature)
                        .map(|formation| formation.constituents.len())
                        .unwrap_or(0)
            {
                continue;
            }
            let signature = family.signature();
            if !self.rigid_point_contact_families.contains_key(&signature) {
                unique.insert(signature, family);
            }
        }
        if unique.is_empty() {
            return Ok(0);
        }
        let path = self.root.join("rigid_point_contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for family in unique.values() {
            write_compact_family(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
        for (signature, family) in &unique {
            self.rigid_point_contact_index
                .entry(contact_bucket_hash(
                    &family.candidate_resource,
                    family.anchor_edge,
                    family.candidate_endpoint,
                ))
                .or_default()
                .push(signature.clone());
        }
        self.rigid_point_contact_families.extend(unique);
        Ok(added)
    }

    pub fn insert_rigid_contact_families(
        &mut self,
        families: Vec<GeometryRigidContactFamily>,
    ) -> std::io::Result<usize> {
        let mut unique = BTreeMap::new();
        for family in families {
            if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                || !family.candidate_rotation_radians.is_finite()
                || !family.anchor_parameter_start.is_finite()
                || !family.anchor_parameter_end.is_finite()
                || family.anchor_parameter_start < 0.0
                || family.anchor_parameter_end > 1.0
                || family.anchor_parameter_start > family.anchor_parameter_end
                || !self.entries.contains_key(&family.formation_signature)
                || family.anchor_constituent
                    >= self
                        .entries
                        .get(&family.formation_signature)
                        .map(|formation| formation.constituents.len())
                        .unwrap_or(0)
            {
                continue;
            }
            let signature = family.signature();
            if !self.rigid_contact_families.contains_key(&signature) {
                unique.insert(signature, family);
            }
        }
        if unique.is_empty() {
            return Ok(0);
        }
        let path = self.root.join("rigid_contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for family in unique.values() {
            write_compact_family(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
        for (signature, family) in &unique {
            self.rigid_contact_index
                .entry(contact_bucket_hash(
                    &family.candidate_resource,
                    family.anchor_edge,
                    family.candidate_edge,
                ))
                .or_default()
                .push(signature.clone());
        }
        self.rigid_contact_families.extend(unique);
        Ok(added)
    }

    pub fn insert_fluid_formations(
        &mut self,
        formations: Vec<GeometryFluidFormation>,
        catalog: &[BaseResource],
    ) -> std::io::Result<usize> {
        self.insert_fluid_formations_with_multi_contact_counts(formations, Vec::new(), catalog)
            .map(|(added, _)| added)
    }

    pub fn insert_fluid_formations_with_multi_contact_counts(
        &mut self,
        legacy_formations: Vec<GeometryFluidFormation>,
        multi_contact_formations: Vec<GeometryFluidFormation>,
        catalog: &[BaseResource],
    ) -> std::io::Result<(usize, usize)> {
        let multi_contact_signatures = multi_contact_formations
            .iter()
            .map(GeometryFluidFormation::signature)
            .collect::<std::collections::BTreeSet<_>>();
        let mut unique = BTreeMap::new();
        for formation in legacy_formations
            .into_iter()
            .chain(multi_contact_formations)
        {
            if fluid_formation_is_valid(&formation, &self.entries, catalog) {
                unique.entry(formation.signature()).or_insert(formation);
            }
        }
        unique.retain(|signature, _| !self.fluid_formations.contains_key(signature));
        if unique.is_empty() {
            return Ok((0, 0));
        }
        let multi_contact_added = unique
            .keys()
            .filter(|signature| multi_contact_signatures.contains(signature.as_str()))
            .count();
        let path = self.root.join("fluid_formations.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for formation in unique.values() {
            serde_json::to_writer(&mut file, formation)
                .map_err(|error| invalid_storage_data(error.to_string()))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
        self.fluid_formations.extend(unique);
        Ok((added, multi_contact_added))
    }

    pub fn insert_fluid_boundary_families(
        &mut self,
        families: Vec<GeometryFluidBoundaryFamily>,
    ) -> std::io::Result<usize> {
        let mut unique = BTreeMap::new();
        for family in families {
            if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                || !family.area.is_finite()
                || family.area <= 0.0
                || !family.contact_angle_radians.is_finite()
                || !family.curvature_radius.is_finite()
                || !family.free_arc_angle_radians.is_finite()
                || !family.contact_length.is_finite()
                || !family.edge_parameter_start.is_finite()
                || !family.edge_parameter_end.is_finite()
                || family.edge_parameter_start < 0.0
                || family.edge_parameter_end > 1.0
                || family.edge_parameter_end < family.edge_parameter_start
                || !fluid_boundary_state_is_self_consistent(&family)
                || !self.entries.contains_key(&family.formation_signature)
            {
                continue;
            }
            unique.insert(family.signature(), family);
        }
        unique.retain(|key, _| !self.fluid_boundary_families.contains_key(key));
        if unique.is_empty() {
            return Ok(0);
        }
        let path = self.root.join("fluid_boundary_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for family in unique.values() {
            write_compact_family(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
        self.fluid_boundary_families.extend(unique);
        Ok(added)
    }

    pub fn insert_contact_family(
        &mut self,
        family: GeometryContactFamily,
    ) -> std::io::Result<bool> {
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
            || !family.contact_angle_radians.is_finite()
            || !family.curvature_radius.is_finite()
            || !family.contact_length.is_finite()
            || !family.edge_parameter_start.is_finite()
            || !family.edge_parameter_end.is_finite()
            || family.edge_parameter_end < family.edge_parameter_start
        {
            return Ok(false);
        }
        let signature = family.signature();
        if self.contact_families.contains_key(&signature) {
            return Ok(false);
        }
        let path = self.root.join("contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        write_compact_family(&mut file, &family)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        self.contact_families.insert(signature, family);
        Ok(true)
    }

    pub fn insert_contact_families(
        &mut self,
        families: Vec<GeometryContactFamily>,
    ) -> std::io::Result<usize> {
        let mut unique = BTreeMap::new();
        for family in families {
            if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                || !family.contact_angle_radians.is_finite()
                || !family.curvature_radius.is_finite()
                || !family.contact_length.is_finite()
                || !family.edge_parameter_start.is_finite()
                || !family.edge_parameter_end.is_finite()
                || family.edge_parameter_end < family.edge_parameter_start
            {
                continue;
            }
            let signature = family.signature();
            if !self.contact_families.contains_key(&signature) {
                unique.insert(signature, family);
            }
        }
        if unique.is_empty() {
            return Ok(0);
        }

        let path = self.root.join("contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for family in unique.values() {
            write_compact_family(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;

        let added = unique.len();
        self.contact_families.extend(unique);
        Ok(added)
    }

    pub fn set_frontier_state(
        &mut self,
        formation_signature: impl Into<String>,
        candidate_resource: impl Into<String>,
        state: GeometryFrontierState,
    ) -> std::io::Result<()> {
        self.set_frontier_states(vec![(
            formation_signature.into(),
            candidate_resource.into(),
            state,
        )])
    }

    /// Update a complete group of frontier records with one durable file write.
    ///
    /// The worker processes all resource frontiers for a formation as a batch.
    /// Persisting the whole frontier JSON after every individual state transition
    /// multiplies I/O as the catalogue grows. If a pass is interrupted before this
    /// write, its old unfinished records remain and idempotent generation can retry.
    pub fn set_frontier_states(
        &mut self,
        updates: Vec<(String, String, GeometryFrontierState)>,
    ) -> std::io::Result<()> {
        if updates.is_empty() {
            return Ok(());
        }
        for (formation_signature, candidate_resource, state) in updates {
            let key = format!("{formation_signature}|{candidate_resource}");
            self.frontier.records.insert(
                key,
                GeometryFrontierRecord {
                    formation_signature,
                    candidate_resource,
                    state,
                },
            );
        }
        self.write_frontier()
    }

    pub fn insert(
        &mut self,
        formation: GeometryFormation,
        catalog: &[BaseResource],
    ) -> std::io::Result<bool> {
        let mut candidates = Vec::new();
        candidates.push(formation);
        Ok(self.insert_many(candidates, catalog)? > 0)
    }

    /// Persist a batch of formations with one append/sync and one manifest write.
    ///
    /// The worker discovers many continuations at once. Syncing every candidate
    /// individually turns durable storage into the geometry-search bottleneck,
    /// so durability is retained at the batch boundary instead.
    pub fn insert_many(
        &mut self,
        formations: Vec<GeometryFormation>,
        catalog: &[BaseResource],
    ) -> std::io::Result<usize> {
        let mut unique = BTreeMap::new();
        let mut batch_equivalence_index = BTreeMap::<String, Vec<String>>::new();
        let mut batch_formation_ids = BTreeMap::<String, String>::new();
        for formation in formations {
            if let Some(canonical) = formation.canonicalized(catalog) {
                let signature = canonical.signature.clone();
                let id = formation_id(&signature);
                if let Some(previous) = self
                    .formation_ids
                    .get(&id)
                    .or_else(|| batch_formation_ids.get(&id))
                {
                    if previous != &signature {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("formation ID collision for {id}"),
                        ));
                    }
                }
                let key = formation_equivalence_key(&canonical);
                let existing_match = self.equivalence_index.get(&key).into_iter().flatten().any(
                    |existing_signature| {
                        self.entries
                            .get(existing_signature)
                            .is_some_and(|existing| {
                                formations_equivalent_within_tolerance(existing, &canonical)
                            })
                    },
                );
                let batch_match = batch_equivalence_index
                    .get(&key)
                    .into_iter()
                    .flatten()
                    .filter_map(|existing_signature| unique.get(existing_signature))
                    .any(|existing| formations_equivalent_within_tolerance(existing, &canonical));
                if existing_match || batch_match {
                    continue;
                }
                batch_equivalence_index
                    .entry(key)
                    .or_default()
                    .push(signature.clone());
                batch_formation_ids.insert(id, signature.clone());
                unique.insert(signature, canonical);
            }
        }
        if unique.is_empty() {
            return Ok(0);
        }

        let data_path = self.root.join("formations.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(data_path)?;
        for formation in unique.values() {
            write_formation_record(
                &mut file,
                formation,
                &self.entries,
                &unique,
                self.formation_storage_version,
            )?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;

        let added = unique.len();
        for signature in unique.keys() {
            self.formation_ids
                .insert(formation_id(signature), signature.clone());
        }
        for (signature, formation) in &unique {
            self.equivalence_index
                .entry(formation_equivalence_key(formation))
                .or_default()
                .push(signature.clone());
        }
        self.entries.extend(unique);
        self.manifest.entries = self.entries.len() as u64;
        self.write_manifest()?;
        Ok(added)
    }

    fn write_frontier(&self) -> std::io::Result<()> {
        let path = self.root.join("frontier.json");
        let temp = self.root.join("frontier.json.tmp");
        let bytes = serde_json::to_vec_pretty(&self.frontier)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&temp, bytes)?;
        fs::rename(temp, path)?;
        Ok(())
    }

    fn write_manifest(&self) -> std::io::Result<()> {
        let path = self.root.join("manifest.json");
        let temp = self.root.join("manifest.json.tmp");
        let bytes = serde_json::to_vec_pretty(&self.manifest)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&temp, bytes)?;
        fs::rename(temp, path)?;
        Ok(())
    }
}

fn canonical_pose_candidates(formation: &GeometryFormation) -> Vec<GeometryFormation> {
    let mut candidates = Vec::with_capacity(formation.constituents.len().max(1));
    for anchor_index in 0..formation.constituents.len() {
        let anchor = &formation.constituents[anchor_index];
        let anchor_rotation = normalized_angle(anchor.placement.rotation_radians);
        let (s, c) = anchor_rotation.sin_cos();
        let mut candidate = formation.clone();

        for constituent in &mut candidate.constituents {
            let dx = constituent.placement.x - anchor.placement.x;
            let dy = constituent.placement.y - anchor.placement.y;
            constituent.placement.x = dx * c + dy * s;
            constituent.placement.y = -dx * s + dy * c;
            constituent.placement.rotation_radians =
                normalized_angle(constituent.placement.rotation_radians - anchor_rotation);
        }

        candidates.push(candidate);
    }
    candidates
}

fn formation_equivalence_key(formation: &GeometryFormation) -> String {
    let mut out = String::new();
    for constituent in &formation.constituents {
        out.push_str(&constituent.resource);
        out.push('@');
        out.push_str(
            &quantize(normalized_angle(constituent.placement.rotation_radians)).to_string(),
        );
        out.push(';');
    }
    out.push('|');
    for bond in &formation.bonds {
        out.push_str(&format!("{}-{};", bond.constituent_a, bond.constituent_b));
    }
    out
}

fn formations_equivalent_within_tolerance(a: &GeometryFormation, b: &GeometryFormation) -> bool {
    if a.schema_version != b.schema_version
        || a.constituents.len() != b.constituents.len()
        || a.bonds != b.bonds
    {
        return false;
    }

    a.constituents
        .iter()
        .zip(&b.constituents)
        .all(|(left, right)| {
            left.resource == right.resource
                && angular_difference(
                    left.placement.rotation_radians,
                    right.placement.rotation_radians,
                ) <= QUANTUM
                && (left.placement.x - right.placement.x)
                    .hypot(left.placement.y - right.placement.y)
                    <= GEOMETRY_EQUIVALENCE_TOLERANCE
        })
}

fn angular_difference(a: f64, b: f64) -> f64 {
    normalized_angle(a - b).abs()
}

fn resource_catalog_signature(catalog: &[BaseResource]) -> String {
    let mut out = String::new();
    for resource in catalog {
        out.push_str(&resource.name);
        out.push(':');
        out.push_str(&serde_json::to_string(&resource.shape).unwrap_or_default());
        out.push(';');
    }
    out
}

fn quantize(value: f64) -> i64 {
    (value / QUANTUM).round() as i64
}

/// Tests an absolute angle against a possibly seam-crossing interval.
/// Family intervals are generated in unwrapped radians; the live angle is
/// normalized, so equivalent turns must be considered without changing the
/// persisted interval convention.
fn angle_in_periodic_interval(angle: f64, start: f64, end: f64) -> bool {
    if !angle.is_finite() || !start.is_finite() || !end.is_finite() || start > end {
        return false;
    }
    let width = end - start;
    if width >= std::f64::consts::TAU {
        return true;
    }
    let angle = normalized_angle(angle);
    let delta = (angle - start).rem_euclid(std::f64::consts::TAU);
    delta <= width + QUANTUM || std::f64::consts::TAU - delta <= QUANTUM
}

fn normalized_angle(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

fn canonical_shape_rotation(form: &Form, angle: f64) -> f64 {
    let angle = normalized_angle(angle);
    let period = match form {
        Form::Circle { .. } => return 0.0,
        Form::Rectangle { .. } => std::f64::consts::PI,
        Form::RegularPolygon { sides, .. } => std::f64::consts::TAU / (*sides as f64),
        Form::Polygon { vertices } => rotational_symmetry_period(vertices),
        Form::Line { .. } => std::f64::consts::PI,
        Form::Fluid { boundary, .. } => boundary
            .as_deref()
            .map(rotational_symmetry_period)
            .unwrap_or(std::f64::consts::TAU),
    };
    if period >= std::f64::consts::TAU - 1e-12 {
        return angle;
    }
    normalized_angle((angle / period).round() * period)
}

fn rotational_symmetry_period(vertices: &[(f64, f64)]) -> f64 {
    if vertices.len() < 3 {
        return std::f64::consts::TAU;
    }
    let n = vertices.len() as f64;
    let center = vertices
        .iter()
        .fold((0.0, 0.0), |(x, y), (vx, vy)| (x + vx, y + vy));
    let center = (center.0 / n, center.1 / n);
    let points: Vec<(f64, f64)> = vertices
        .iter()
        .map(|(x, y)| (x - center.0, y - center.1))
        .collect();
    let first = points[0];
    if first.0.hypot(first.1) <= 1e-12 {
        return std::f64::consts::TAU;
    }
    let first_angle = first.1.atan2(first.0);
    let tol = 1e-9;
    for shift in 1..vertices.len() {
        let target = points[shift];
        let rotation = normalized_angle(target.1.atan2(target.0) - first_angle);
        let (sin, cos) = rotation.sin_cos();
        if points.iter().all(|p| {
            let rotated = (p.0 * cos - p.1 * sin, p.0 * sin + p.1 * cos);
            points
                .iter()
                .any(|q| (rotated.0 - q.0).hypot(rotated.1 - q.1) <= tol)
        }) {
            return rotation.abs();
        }
    }
    std::f64::consts::TAU
}

fn normalize_global_pose(formation: &mut GeometryFormation, catalog: &[BaseResource]) {
    if formation.constituents.is_empty() {
        return;
    }

    for constituent in &mut formation.constituents {
        if catalog
            .iter()
            .find(|r| r.name == constituent.resource)
            .map(|r| matches!(r.shape.form, Form::Circle { .. }))
            .unwrap_or(false)
        {
            constituent.placement.rotation_radians = 0.0;
        }
    }

    let anchor_rotation = formation
        .constituents
        .iter()
        .map(|c| normalized_angle(c.placement.rotation_radians))
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    let (s, c) = anchor_rotation.sin_cos();
    for constituent in &mut formation.constituents {
        let x = constituent.placement.x;
        let y = constituent.placement.y;
        constituent.placement.x = x * c + y * s;
        constituent.placement.y = -x * s + y * c;
        constituent.placement.rotation_radians =
            normalized_angle(constituent.placement.rotation_radians - anchor_rotation);
    }

    let (ox, oy) = formation
        .constituents
        .iter()
        .find(|c| (c.placement.rotation_radians - 0.0).abs() <= 1e-12)
        .map(|c| (c.placement.x, c.placement.y))
        .unwrap_or((0.0, 0.0));

    for constituent in &mut formation.constituents {
        constituent.placement.x -= ox;
        constituent.placement.y -= oy;
    }
}

pub fn validate_formation(formation: &GeometryFormation, catalog: &[BaseResource]) -> bool {
    if formation.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
        || formation.constituents.is_empty()
    {
        return false;
    }

    for constituent in &formation.constituents {
        if !constituent.placement.x.is_finite()
            || !constituent.placement.y.is_finite()
            || !constituent.placement.rotation_radians.is_finite()
        {
            return false;
        }
        let Some(resource) = catalog.iter().find(|r| r.name == constituent.resource) else {
            return false;
        };
        if !resource.shape.is_valid() {
            return false;
        }
    }

    for bond in &formation.bonds {
        if bond.constituent_a >= formation.constituents.len()
            || bond.constituent_b >= formation.constituents.len()
            || bond.constituent_a == bond.constituent_b
        {
            return false;
        }

        // Fluid-to-fluid combination is not a rigid geometric connection.
        // It produces the same fluid shape with greater volume instead of a
        // second bonded constituent, so the reference library must never
        // encode two fluid constituents as a rigidly bonded formation.
        let resource_a = catalog
            .iter()
            .find(|resource| resource.name == formation.constituents[bond.constituent_a].resource)
            .unwrap();
        let resource_b = catalog
            .iter()
            .find(|resource| resource.name == formation.constituents[bond.constituent_b].resource)
            .unwrap();
        if resource_a.physical_state == crate::resources::PhysicalState::Fluid
            && resource_b.physical_state == crate::resources::PhysicalState::Fluid
        {
            return false;
        }
    }

    let mut seen = BTreeMap::new();
    for bond in &formation.bonds {
        let key = (
            bond.constituent_a.min(bond.constituent_b),
            bond.constituent_a.max(bond.constituent_b),
        );
        if seen.insert(key, ()).is_some() {
            return false;
        }
    }

    if formation.constituents.len() > 1 {
        if formation.bonds.len() < formation.constituents.len() - 1 {
            return false;
        }

        let mut connected = vec![false; formation.constituents.len()];
        connected[0] = true;
        loop {
            let mut changed = false;
            for bond in &formation.bonds {
                if connected[bond.constituent_a] && !connected[bond.constituent_b] {
                    connected[bond.constituent_b] = true;
                    changed = true;
                }
                if connected[bond.constituent_b] && !connected[bond.constituent_a] {
                    connected[bond.constituent_a] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        if connected.iter().any(|v| !v) {
            return false;
        }
    }

    let mut parts = Vec::with_capacity(formation.constituents.len());
    for (index, constituent) in formation.constituents.iter().enumerate() {
        let resource = catalog
            .iter()
            .find(|r| r.name == constituent.resource)
            .unwrap();
        parts.push(PlacedMaterialPart {
            part_index: index,
            form: resource.shape.form.clone(),
            placement: constituent.placement,
        });
    }

    // A declared rigid bond must correspond to actual realized boundary
    // contact. The graph is not allowed to invent a connection between
    // separated constituents.
    for bond in &formation.bonds {
        let a = &parts[bond.constituent_a];
        let b = &parts[bond.constituent_b];
        if matches!(a.form, Form::Fluid { boundary: None, .. })
            || matches!(b.form, Form::Fluid { boundary: None, .. })
        {
            continue;
        }
        if !placed_forms_rigid_contact(a, b, 1e-9) {
            return false;
        }
    }

    for i in 0..parts.len() {
        for j in (i + 1)..parts.len() {
            // Fluids with no explicit boundary have no finite collision surface;
            // their geometric realization is handled by the fluid-field layer.
            if matches!(parts[i].form, Form::Fluid { boundary: None, .. })
                || matches!(parts[j].form, Form::Fluid { boundary: None, .. })
            {
                continue;
            }
            if placed_forms_penetrate(&parts[i], &parts[j], 1e-9) {
                return false;
            }
        }
    }

    true
}

type AlignedExposedInterval = (
    f64,
    f64,
    (usize, usize, f64, f64, (f64, f64), (f64, f64)),
    (f64, f64),
    (f64, f64),
);

/// A polygon edge expressed as the exact exposed parameter intervals that remain
/// available for a future contact. Parameter 0 is the first vertex and 1 is
/// the second vertex. Coincident boundary portions belonging to another
/// constituent are removed; point contacts do not remove an interval.
#[derive(Clone, Debug, PartialEq)]
struct ExposedEdgeInterval {
    pub edge: usize,
    pub start: f64,
    pub end: f64,
}

/// Return the exposed portions of one rigid polygon's boundary.
///
/// This is deliberately interval-based rather than sampled. Because formation
/// validation already rejects positive-area penetration, another rigid polygon
/// can only occlude a boundary edge by lying on that same boundary. We therefore
/// project exact collinear edge overlaps onto the source edge and subtract them.
pub fn exposed_polygon_edge_intervals(
    formation: &GeometryFormation,
    anchor_index: usize,
    catalog: &[BaseResource],
) -> Vec<ExposedEdgeInterval> {
    if anchor_index >= formation.constituents.len() {
        return Vec::new();
    }
    let Some(anchor_resource) = catalog
        .iter()
        .find(|r| r.name == formation.constituents[anchor_index].resource)
    else {
        return Vec::new();
    };
    let Some(anchor_vertices) = anchor_resource.shape.form.polygon_vertices() else {
        return Vec::new();
    };
    let anchor_placement = formation.constituents[anchor_index].placement;
    let mut out = Vec::new();

    for edge in 0..anchor_vertices.len() {
        let a = world_point(anchor_vertices[edge], anchor_placement);
        let b = world_point(
            anchor_vertices[(edge + 1) % anchor_vertices.len()],
            anchor_placement,
        );
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let length_sq = dx * dx + dy * dy;
        if length_sq <= f64::EPSILON {
            continue;
        }

        let mut covered = Vec::<(f64, f64)>::new();
        for (other_index, other) in formation.constituents.iter().enumerate() {
            if other_index == anchor_index {
                continue;
            }
            let Some(resource) = catalog.iter().find(|r| r.name == other.resource) else {
                continue;
            };
            if resource.physical_state == crate::resources::PhysicalState::Fluid {
                continue;
            }
            let Some(vertices) = resource.shape.form.polygon_vertices() else {
                continue;
            };
            let p = other.placement;
            for other_edge in 0..vertices.len() {
                let c = world_point(vertices[other_edge], p);
                let d = world_point(vertices[(other_edge + 1) % vertices.len()], p);
                let ex = d.0 - c.0;
                let ey = d.1 - c.1;
                let cross = dx * (c.1 - a.1) - dy * (c.0 - a.0);
                let cross_end = dx * (d.1 - a.1) - dy * (d.0 - a.0);
                if cross.abs() > 1e-9 * length_sq.sqrt()
                    || cross_end.abs() > 1e-9 * length_sq.sqrt()
                {
                    continue;
                }
                let t0 = ((c.0 - a.0) * dx + (c.1 - a.1) * dy) / length_sq;
                let t1 = ((d.0 - a.0) * dx + (d.1 - a.1) * dy) / length_sq;
                let lo = t0.min(t1).max(0.0);
                let hi = t0.max(t1).min(1.0);
                if hi - lo > 1e-10 {
                    covered.push((lo, hi));
                }
                let _ = (ex, ey);
            }
        }

        covered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut cursor = 0.0;
        for (start, end) in covered {
            if start > cursor + 1e-10 {
                out.push(ExposedEdgeInterval {
                    edge,
                    start: cursor,
                    end: start.min(1.0),
                });
            }
            cursor = cursor.max(end);
            if cursor >= 1.0 - 1e-10 {
                break;
            }
        }
        if cursor < 1.0 - 1e-10 {
            out.push(ExposedEdgeInterval {
                edge,
                start: cursor,
                end: 1.0,
            });
        }
    }
    out
}

/// Derive exact Water/rigid contact families from exposed rigid boundary features.
///
/// Polygon edges use exact collinear interval subtraction. A rigid line is
/// represented as one boundary segment. Water therefore does not need a
/// special sampled placement for either supported rigid boundary primitive.
fn fluid_formation_is_valid(
    formation: &GeometryFluidFormation,
    parents: &BTreeMap<String, GeometryFormation>,
    catalog: &[BaseResource],
) -> bool {
    let Some(parent) = parents.get(&formation.parent_formation_signature) else {
        return false;
    };
    let Some(fluid) = catalog
        .iter()
        .find(|resource| resource.name == formation.fluid_resource)
    else {
        return false;
    };
    if formation.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
        || fluid.physical_state != crate::resources::PhysicalState::Fluid
        || formation.anchor_constituent >= parent.constituents.len()
        || !formation.area.is_finite()
        || formation.area <= 0.0
    {
        return false;
    }

    if !formation.contact_constraints.is_empty() || formation.solved_boundary.is_some() {
        let Some(boundary) = formation.solved_boundary.as_ref() else {
            return false;
        };
        if formation.contact_constraints.len() < 2
            || boundary.contact_count() != formation.contact_constraints.len()
            || !boundary.is_valid()
            || (boundary.nominal_area - formation.area).abs() > 1e-9 * formation.area.max(1.0)
            || formation.contact_constraints.iter().any(|constraint| {
                let Some(constituent) = parent.constituents.get(constraint.constituent_index)
                else {
                    return true;
                };
                let Some(resource) = catalog
                    .iter()
                    .find(|item| item.name == constituent.resource)
                else {
                    return true;
                };
                resource.physical_state == crate::resources::PhysicalState::Fluid
                    || constraint.edge_index >= rigid_boundary_segments(&resource.shape.form).len()
                    || !constraint.contact_length.is_finite()
                    || constraint.contact_length <= 0.0
                    || !constraint.edge_parameter_start.is_finite()
                    || !constraint.edge_parameter_end.is_finite()
                    || constraint.edge_parameter_start < 0.0
                    || constraint.edge_parameter_end > 1.0
                    || constraint.edge_parameter_end <= constraint.edge_parameter_start
            })
        {
            return false;
        }
        return formation
            .realize_against_parent(parent, catalog, true)
            .is_some();
    }

    let Some(anchor) = catalog.iter().find(|resource| {
        resource.name == parent.constituents[formation.anchor_constituent].resource
    }) else {
        return false;
    };
    if anchor.physical_state == crate::resources::PhysicalState::Fluid
        || formation.anchor_edge >= rigid_boundary_segments(&anchor.shape.form).len()
    {
        return false;
    }
    let family = GeometryFluidBoundaryFamily {
        schema_version: formation.schema_version,
        formation_signature: formation.parent_formation_signature.clone(),
        fluid_resource: formation.fluid_resource.clone(),
        anchor_constituent: formation.anchor_constituent,
        anchor_edge: formation.anchor_edge,
        area: formation.area,
        contact_angle_radians: formation.contact_angle_radians,
        curvature_radius: formation.curvature_radius,
        free_arc_angle_radians: formation.free_arc_angle_radians,
        contact_length: formation.contact_length,
        edge_parameter_start: formation.edge_parameter_start,
        edge_parameter_end: formation.edge_parameter_end,
    };
    formation.contact_angle_radians.is_finite()
        && formation.curvature_radius.is_finite()
        && formation.free_arc_angle_radians.is_finite()
        && formation.contact_length.is_finite()
        && formation.edge_parameter_start.is_finite()
        && formation.edge_parameter_end.is_finite()
        && formation.edge_parameter_start >= 0.0
        && formation.edge_parameter_end <= 1.0
        && formation.edge_parameter_end >= formation.edge_parameter_start
        && fluid_boundary_state_is_self_consistent(&family)
        && formation
            .realize_against_parent(parent, catalog, true)
            .is_some()
}

fn load_fluid_formations(
    path: &Path,
    parents: &BTreeMap<String, GeometryFormation>,
    catalog: &[BaseResource],
) -> std::io::Result<BTreeMap<String, GeometryFluidFormation>> {
    let mut records = BTreeMap::new();
    if !path.exists() {
        return Ok(records);
    }
    let file = File::open(path)?;
    let mut lines = BufReader::new(file).lines().peekable();
    while let Some(line_result) = lines.next() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        let is_last = lines.peek().is_none();
        let record: GeometryFluidFormation = match serde_json::from_str(&line) {
            Ok(record) => record,
            Err(_) if is_last => continue,
            Err(error) => return Err(invalid_storage_data(error.to_string())),
        };
        if !fluid_formation_is_valid(&record, parents, catalog) {
            continue;
        }
        records.entry(record.signature()).or_insert(record);
    }
    Ok(records)
}

fn load_fluid_boundary_families(
    path: &Path,
    entries: &BTreeMap<String, GeometryFormation>,
    catalog: &[BaseResource],
    formation_ids: &BTreeMap<String, String>,
) -> BTreeMap<String, GeometryFluidBoundaryFamily> {
    let mut out = BTreeMap::new();
    let Ok(file) = File::open(path) else {
        return out;
    };
    let mut lines = BufReader::new(file).lines();
    while let Some(line_result) = lines.next() {
        // Preserve the previous all-or-empty behavior if the file itself
        // cannot be read, without buffering the entire JSONL catalogue.
        let Ok(line) = line_result else {
            return BTreeMap::new();
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(family) =
            deserialize_family_record::<GeometryFluidBoundaryFamily>(&line, formation_ids)
        else {
            continue;
        };
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
            || !family.area.is_finite()
            || family.area <= 0.0
            || !family.contact_angle_radians.is_finite()
            || !family.curvature_radius.is_finite()
            || !family.free_arc_angle_radians.is_finite()
            || !family.contact_length.is_finite()
            || !family.edge_parameter_start.is_finite()
            || !family.edge_parameter_end.is_finite()
            || family.edge_parameter_start < 0.0
            || family.edge_parameter_end > 1.0
            || family.edge_parameter_end < family.edge_parameter_start
            || !fluid_boundary_state_is_self_consistent(&family)
            || !entries.contains_key(&family.formation_signature)
            || family.anchor_constituent
                >= entries
                    .get(&family.formation_signature)
                    .map(|f| f.constituents.len())
                    .unwrap_or(0)
            || catalog.iter().all(|r| r.name != family.fluid_resource)
        {
            continue;
        }
        out.insert(family.signature(), family);
    }
    out
}

pub fn generate_fluid_boundary_families(
    formation: &GeometryFormation,
    fluid_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryFluidBoundaryFamily> {
    let families = generate_water_contact_families(formation, fluid_resource, catalog);
    generate_fluid_boundary_families_from_contact_families(fluid_resource, &families)
}

/// Derive durable fluid-boundary records from contact families already generated
/// for the same formation. This avoids solving the same capillary geometry twice.
pub fn generate_fluid_boundary_families_from_contact_families(
    fluid_resource: &BaseResource,
    families: &[GeometryContactFamily],
) -> Vec<GeometryFluidBoundaryFamily> {
    if fluid_resource.physical_state != crate::resources::PhysicalState::Fluid {
        return Vec::new();
    }
    let area = match &fluid_resource.shape.form {
        Form::Circle { radius } => std::f64::consts::PI * radius * radius,
        Form::Fluid { nominal_area, .. } => *nominal_area,
        _ => return Vec::new(),
    };

    families
        .iter()
        .map(|family| GeometryFluidBoundaryFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: family.formation_signature.clone(),
            fluid_resource: family.candidate_resource.clone(),
            anchor_constituent: family.anchor_constituent,
            anchor_edge: family.anchor_edge,
            area,
            contact_angle_radians: family.contact_angle_radians,
            curvature_radius: family.curvature_radius,
            free_arc_angle_radians: 2.0 * (std::f64::consts::PI - family.contact_angle_radians),
            contact_length: family.contact_length,
            edge_parameter_start: family.edge_parameter_start,
            edge_parameter_end: family.edge_parameter_end,
        })
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaterContactGenerationDiagnostics {
    pub eligible_anchors: usize,
    pub unresolved_anchor_resources: usize,
    pub unsupported_anchor_geometry: usize,
    pub capillary_solver_failures: usize,
    pub anchors_without_exposed_intervals: usize,
    pub invalid_contact_intervals: usize,
    pub anchors_without_valid_contact_overlap: usize,
}

/// Preserve the original generator API while exposing reason-level outcomes
/// to Bob's worker and focused diagnostics tests.
pub fn generate_water_contact_families(
    formation: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryContactFamily> {
    generate_water_contact_families_with_diagnostics(formation, candidate_resource, catalog).0
}

pub fn generate_water_contact_families_with_diagnostics(
    formation: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> (
    Vec<GeometryContactFamily>,
    WaterContactGenerationDiagnostics,
) {
    let mut diagnostics = WaterContactGenerationDiagnostics::default();
    if candidate_resource.name != "Water"
        || candidate_resource.physical_state != crate::resources::PhysicalState::Fluid
    {
        return (Vec::new(), diagnostics);
    }
    let Some(water) = catalog.iter().find(|r| r.name == "Water") else {
        return (Vec::new(), diagnostics);
    };
    let water_area = match water.shape.form {
        Form::Circle { radius } => std::f64::consts::PI * radius * radius,
        Form::Fluid { nominal_area, .. } => nominal_area,
        _ => return (Vec::new(), diagnostics),
    };

    let mut out = Vec::new();
    for (anchor_index, constituent) in formation.constituents.iter().enumerate() {
        let Some(resource) = catalog.iter().find(|r| r.name == constituent.resource) else {
            diagnostics.unresolved_anchor_resources += 1;
            continue;
        };
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            continue;
        }

        let intervals = match &resource.shape.form {
            Form::Line { .. } => exposed_line_intervals(formation, anchor_index, catalog),
            form if form.polygon_vertices().is_some() => {
                exposed_polygon_edge_intervals(formation, anchor_index, catalog)
            }
            _ => {
                diagnostics.unsupported_anchor_geometry += 1;
                continue;
            }
        };
        diagnostics.eligible_anchors += 1;
        if intervals.is_empty() {
            diagnostics.anchors_without_exposed_intervals += 1;
            continue;
        }

        let Some(family) = solve_water_against_solid(
            water_area,
            water.properties.cohesion,
            resource.properties.cohesion,
        ) else {
            diagnostics.capillary_solver_failures += 1;
            continue;
        };

        let mut emitted_for_anchor = 0usize;
        let mut invalid_overlap_for_anchor = false;
        match &resource.shape.form {
            Form::Line { length } => {
                let Some(contact) = ContactTranslationInterval::from_edge_length(*length, family)
                else {
                    diagnostics.invalid_contact_intervals += 1;
                    continue;
                };
                for interval in intervals {
                    let start = contact.edge_start_parameter.max(interval.start);
                    let end = contact.edge_end_parameter.min(interval.end);
                    if end + 1e-10 >= start {
                        out.push(GeometryContactFamily {
                            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                            formation_signature: formation.signature.clone(),
                            candidate_resource: candidate_resource.name.clone(),
                            anchor_constituent: anchor_index,
                            anchor_edge: 0,
                            contact_angle_radians: family.contact_angle_radians,
                            curvature_radius: family.curvature_radius,
                            contact_length: family.contact_length,
                            edge_parameter_start: start,
                            edge_parameter_end: end,
                        });
                        emitted_for_anchor += 1;
                    } else {
                        invalid_overlap_for_anchor = true;
                    }
                }
            }
            _ => {
                let Some(vertices) = resource.shape.form.polygon_vertices() else {
                    diagnostics.unsupported_anchor_geometry += 1;
                    continue;
                };
                let edge_length_cache = vertices
                    .iter()
                    .enumerate()
                    .map(|(edge, &a)| {
                        let b = vertices[(edge + 1) % vertices.len()];
                        (edge, (b.0 - a.0).hypot(b.1 - a.1))
                    })
                    .collect::<BTreeMap<_, _>>();

                for interval in intervals {
                    let Some(&edge_length) = edge_length_cache.get(&interval.edge) else {
                        diagnostics.invalid_contact_intervals += 1;
                        continue;
                    };
                    let Some(contact) =
                        ContactTranslationInterval::from_edge_length(edge_length, family)
                    else {
                        diagnostics.invalid_contact_intervals += 1;
                        continue;
                    };
                    let start = contact.edge_start_parameter.max(interval.start);
                    let end = contact.edge_end_parameter.min(interval.end);
                    if end + 1e-10 < start {
                        invalid_overlap_for_anchor = true;
                        continue;
                    }
                    out.push(GeometryContactFamily {
                        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                        formation_signature: formation.signature.clone(),
                        candidate_resource: candidate_resource.name.clone(),
                        anchor_constituent: anchor_index,
                        anchor_edge: interval.edge,
                        contact_angle_radians: family.contact_angle_radians,
                        curvature_radius: family.curvature_radius,
                        contact_length: family.contact_length,
                        edge_parameter_start: start,
                        edge_parameter_end: end,
                    });
                    emitted_for_anchor += 1;
                }
            }
        }
        if emitted_for_anchor == 0 && invalid_overlap_for_anchor {
            diagnostics.anchors_without_valid_contact_overlap += 1;
        }
    }
    (out, diagnostics)
}

fn exposed_line_intervals(
    formation: &GeometryFormation,
    anchor_index: usize,
    catalog: &[BaseResource],
) -> Vec<ExposedEdgeInterval> {
    let Some(anchor) = formation.constituents.get(anchor_index) else {
        return Vec::new();
    };
    let Some(resource) = catalog.iter().find(|r| r.name == anchor.resource) else {
        return Vec::new();
    };
    let Form::Line { length } = resource.shape.form else {
        return Vec::new();
    };
    let half = length * 0.5;
    let anchor_start = (
        anchor.placement.x - half * anchor.placement.rotation_radians.cos(),
        anchor.placement.y - half * anchor.placement.rotation_radians.sin(),
    );
    let anchor_end = (
        anchor.placement.x + half * anchor.placement.rotation_radians.cos(),
        anchor.placement.y + half * anchor.placement.rotation_radians.sin(),
    );
    let dx = anchor_end.0 - anchor_start.0;
    let dy = anchor_end.1 - anchor_start.1;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return Vec::new();
    }

    let mut covered = Vec::new();
    for (other_index, other) in formation.constituents.iter().enumerate() {
        if other_index == anchor_index {
            continue;
        }
        let Some(other_resource) = catalog.iter().find(|r| r.name == other.resource) else {
            continue;
        };
        if other_resource.physical_state == crate::resources::PhysicalState::Fluid {
            continue;
        }

        match &other_resource.shape.form {
            Form::Line {
                length: other_length,
            } => {
                let half_other = *other_length * 0.5;
                let c = (
                    other.placement.x - half_other * other.placement.rotation_radians.cos(),
                    other.placement.y - half_other * other.placement.rotation_radians.sin(),
                );
                let d = (
                    other.placement.x + half_other * other.placement.rotation_radians.cos(),
                    other.placement.y + half_other * other.placement.rotation_radians.sin(),
                );
                let cross_c = dx * (c.1 - anchor_start.1) - dy * (c.0 - anchor_start.0);
                let cross_d = dx * (d.1 - anchor_start.1) - dy * (d.0 - anchor_start.0);
                if cross_c.abs() > 1e-9 * length_sq.sqrt()
                    || cross_d.abs() > 1e-9 * length_sq.sqrt()
                {
                    continue;
                }
                let t0 = ((c.0 - anchor_start.0) * dx + (c.1 - anchor_start.1) * dy) / length_sq;
                let t1 = ((d.0 - anchor_start.0) * dx + (d.1 - anchor_start.1) * dy) / length_sq;
                let lo = t0.min(t1).max(0.0);
                let hi = t0.max(t1).min(1.0);
                if hi - lo > 1e-10 {
                    covered.push((lo, hi));
                }
            }
            _ => {
                let Some(vertices) = other_resource.shape.form.polygon_vertices() else {
                    continue;
                };
                for edge in 0..vertices.len() {
                    let c = world_point(vertices[edge], other.placement);
                    let d = world_point(vertices[(edge + 1) % vertices.len()], other.placement);
                    let cross_c = dx * (c.1 - anchor_start.1) - dy * (c.0 - anchor_start.0);
                    let cross_d = dx * (d.1 - anchor_start.1) - dy * (d.0 - anchor_start.0);
                    if cross_c.abs() > 1e-9 * length_sq.sqrt()
                        || cross_d.abs() > 1e-9 * length_sq.sqrt()
                    {
                        continue;
                    }
                    let t0 =
                        ((c.0 - anchor_start.0) * dx + (c.1 - anchor_start.1) * dy) / length_sq;
                    let t1 =
                        ((d.0 - anchor_start.0) * dx + (d.1 - anchor_start.1) * dy) / length_sq;
                    let lo = t0.min(t1).max(0.0);
                    let hi = t0.max(t1).min(1.0);
                    if hi - lo > 1e-10 {
                        covered.push((lo, hi));
                    }
                }
            }
        }
    }

    covered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = Vec::new();
    let mut cursor = 0.0;
    for (start, end) in covered {
        if start > cursor + 1e-10 {
            out.push(ExposedEdgeInterval {
                edge: 0,
                start: cursor,
                end: start.min(1.0),
            });
        }
        cursor = cursor.max(end);
        if cursor >= 1.0 - 1e-10 {
            break;
        }
    }
    if cursor < 1.0 - 1e-10 {
        out.push(ExposedEdgeInterval {
            edge: 0,
            start: cursor,
            end: 1.0,
        });
    }
    out
}

fn rigid_boundary_segments(form: &Form) -> Vec<((f64, f64), (f64, f64))> {
    match form {
        Form::Line { length } => vec![((-length * 0.5, 0.0), (length * 0.5, 0.0))],
        _ => form
            .polygon_vertices()
            .map(|vertices| {
                (0..vertices.len())
                    .map(|i| (vertices[i], vertices[(i + 1) % vertices.len()]))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

pub fn instantiate_rigid_contact_family(
    formation: &GeometryFormation,
    family: &GeometryRigidContactFamily,
    anchor_parameter: f64,
    catalog: &[BaseResource],
) -> Option<GeometryFormation> {
    if anchor_parameter < family.anchor_parameter_start - QUANTUM
        || anchor_parameter > family.anchor_parameter_end + QUANTUM
        || family.anchor_constituent >= formation.constituents.len()
    {
        return None;
    }
    let anchor_resource = catalog.iter().find(|resource| {
        resource.name == formation.constituents[family.anchor_constituent].resource
    })?;
    let anchor_segments = rigid_boundary_segments(&anchor_resource.shape.form);
    let (a0, a1) = *anchor_segments.get(family.anchor_edge)?;
    let candidate_resource = catalog
        .iter()
        .find(|resource| resource.name == family.candidate_resource)?;
    let candidate_segments = rigid_boundary_segments(&candidate_resource.shape.form);
    let (c0, _) = *candidate_segments.get(family.candidate_edge)?;
    let anchor = formation.constituents[family.anchor_constituent].placement;
    let world_a0 = world_point(a0, anchor);
    let world_a1 = world_point(a1, anchor);
    let contact_point = (
        world_a0.0 + (world_a1.0 - world_a0.0) * anchor_parameter,
        world_a0.1 + (world_a1.1 - world_a0.1) * anchor_parameter,
    );
    let rotated_c0 = rotated_point(c0, family.candidate_rotation_radians);
    let placement = Placement {
        x: contact_point.0 - rotated_c0.0,
        y: contact_point.1 - rotated_c0.1,
        rotation_radians: family.candidate_rotation_radians,
    };

    let mut result = formation.clone();
    let candidate_index = result.constituents.len();
    result.constituents.push(GeometryConstituent {
        resource: family.candidate_resource.clone(),
        placement,
    });
    result.bonds.push(GeometryBond {
        constituent_a: family.anchor_constituent,
        constituent_b: candidate_index,
    });
    result.canonicalized(catalog)
}

fn load_rigid_point_contact_families(
    path: &Path,
    entries: &BTreeMap<String, GeometryFormation>,
    catalog: &[BaseResource],
    formation_ids: &BTreeMap<String, String>,
) -> BTreeMap<String, GeometryRigidPointContactFamily> {
    let mut out = BTreeMap::new();
    let Ok(file) = File::open(path) else {
        return out;
    };
    // Keep peak memory bounded: these family files can be large. A read error
    // preserves the previous all-or-empty behavior of the whole-file loader.
    for line_result in BufReader::new(file).lines() {
        let Ok(line) = line_result else {
            return BTreeMap::new();
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(family) =
            deserialize_family_record::<GeometryRigidPointContactFamily>(&line, formation_ids)
        else {
            continue;
        };
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
            || !family.anchor_parameter_start.is_finite()
            || !family.anchor_parameter_end.is_finite()
            || !family.candidate_rotation_start_radians.is_finite()
            || !family.candidate_rotation_end_radians.is_finite()
            || family.anchor_parameter_start > family.anchor_parameter_end
            || !entries.contains_key(&family.formation_signature)
            || family.anchor_constituent
                >= entries
                    .get(&family.formation_signature)
                    .map(|f| f.constituents.len())
                    .unwrap_or(0)
            || family.candidate_endpoint > 1
            || catalog.iter().all(|r| r.name != family.candidate_resource)
        {
            continue;
        }
        let formation = &entries[&family.formation_signature];
        let Some(anchor_resource) = catalog.iter().find(|resource| {
            resource.name == formation.constituents[family.anchor_constituent].resource
        }) else {
            continue;
        };
        let Some(anchor_vertices) = anchor_resource.shape.form.polygon_vertices() else {
            continue;
        };
        let Some(candidate_resource) = catalog
            .iter()
            .find(|resource| resource.name == family.candidate_resource)
        else {
            continue;
        };
        if !matches!(&candidate_resource.shape.form, Form::Line { .. })
            || family.anchor_edge >= anchor_vertices.len()
        {
            continue;
        }
        out.insert(family.signature(), family);
    }
    out
}

fn load_rigid_vertex_contact_families(
    path: &Path,
    entries: &BTreeMap<String, GeometryFormation>,
    catalog: &[BaseResource],
    formation_ids: &BTreeMap<String, String>,
) -> BTreeMap<String, GeometryRigidVertexContactFamily> {
    let mut out = BTreeMap::new();
    let Ok(file) = File::open(path) else {
        return out;
    };
    // Keep peak memory bounded: these family files can be large. A read error
    // preserves the previous all-or-empty behavior of the whole-file loader.
    for line_result in BufReader::new(file).lines() {
        let Ok(line) = line_result else {
            return BTreeMap::new();
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(family) =
            deserialize_family_record::<GeometryRigidVertexContactFamily>(&line, formation_ids)
        else {
            continue;
        };
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
            || !family.anchor_parameter_start.is_finite()
            || !family.anchor_parameter_end.is_finite()
            || !family.candidate_rotation_start_radians.is_finite()
            || !family.candidate_rotation_end_radians.is_finite()
            || family.anchor_parameter_start > family.anchor_parameter_end
            || !entries.contains_key(&family.formation_signature)
            || family.anchor_constituent
                >= entries
                    .get(&family.formation_signature)
                    .map(|f| f.constituents.len())
                    .unwrap_or(0)
            || catalog.iter().all(|r| r.name != family.candidate_resource)
        {
            continue;
        }
        let Some(anchor_resource) = catalog.iter().find(|r| {
            r.name
                == entries[&family.formation_signature].constituents[family.anchor_constituent]
                    .resource
        }) else {
            continue;
        };
        let Some(anchor_vertices) = anchor_resource.shape.form.polygon_vertices() else {
            continue;
        };
        let Some(candidate_resource) = catalog.iter().find(|r| r.name == family.candidate_resource)
        else {
            continue;
        };
        let Some(candidate_vertices) = candidate_resource.shape.form.polygon_vertices() else {
            continue;
        };
        if family.anchor_edge >= anchor_vertices.len()
            || family.candidate_vertex >= candidate_vertices.len()
        {
            continue;
        }
        out.insert(family.signature(), family);
    }
    out
}

pub fn generate_rigid_point_contact_families(
    formation: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryRigidPointContactFamily> {
    let Form::Line { .. } = candidate_resource.shape.form else {
        return Vec::new();
    };
    let mut unique = BTreeMap::new();

    for anchor_index in 0..formation.constituents.len() {
        let Some(anchor_resource) = catalog
            .iter()
            .find(|r| r.name == formation.constituents[anchor_index].resource)
        else {
            continue;
        };
        let Some(vertices) = anchor_resource.shape.form.polygon_vertices() else {
            continue;
        };
        let exposed = exposed_polygon_edge_intervals(formation, anchor_index, catalog);
        let placement = formation.constituents[anchor_index].placement;
        let signed_area = vertices
            .iter()
            .enumerate()
            .map(|(i, &(x0, y0))| {
                let (x1, y1) = vertices[(i + 1) % vertices.len()];
                x0 * y1 - y0 * x1
            })
            .sum::<f64>();

        for interval in exposed {
            let Some(&a0) = vertices.get(interval.edge) else {
                continue;
            };
            let a1 = vertices[(interval.edge + 1) % vertices.len()];
            let dx = a1.0 - a0.0;
            let dy = a1.1 - a0.1;
            let length = dx.hypot(dy);
            if length <= QUANTUM {
                continue;
            }

            // For a CCW polygon the outward normal is the right-hand normal;
            // for CW it is the left-hand normal.
            let (nx, ny) = if signed_area >= 0.0 {
                (dy / length, -dx / length)
            } else {
                (-dy / length, dx / length)
            };
            let world_n = (
                nx * placement.rotation_radians.cos() - ny * placement.rotation_radians.sin(),
                nx * placement.rotation_radians.sin() + ny * placement.rotation_radians.cos(),
            );
            let outward_angle = world_n.1.atan2(world_n.0);

            for endpoint in 0..2 {
                let local_interior_angle = if endpoint == 0 {
                    0.0
                } else {
                    std::f64::consts::PI
                };
                let center = outward_angle - local_interior_angle;
                let start = center - std::f64::consts::FRAC_PI_2;
                let end = center + std::f64::consts::FRAC_PI_2;
                let family = GeometryRigidPointContactFamily {
                    schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                    formation_signature: formation.signature.clone(),
                    candidate_resource: candidate_resource.name.clone(),
                    anchor_constituent: anchor_index,
                    anchor_edge: interval.edge,
                    candidate_endpoint: endpoint,
                    anchor_parameter_start: interval.start,
                    anchor_parameter_end: interval.end,
                    candidate_rotation_start_radians: start,
                    candidate_rotation_end_radians: end,
                };
                unique.insert(family.signature(), family);
            }
        }
    }
    unique.into_values().collect()
}

pub fn generate_rigid_vertex_contact_families(
    formation: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryRigidVertexContactFamily> {
    let Some(candidate_vertices) = candidate_resource.shape.form.polygon_vertices() else {
        return Vec::new();
    };
    if !is_convex_polygon(&candidate_vertices) {
        return Vec::new();
    }
    let mut unique = BTreeMap::new();

    for anchor_index in 0..formation.constituents.len() {
        let Some(anchor_resource) = catalog
            .iter()
            .find(|r| r.name == formation.constituents[anchor_index].resource)
        else {
            continue;
        };
        let Some(anchor_vertices) = anchor_resource.shape.form.polygon_vertices() else {
            continue;
        };
        let exposed = exposed_polygon_edge_intervals(formation, anchor_index, catalog);
        let placement = formation.constituents[anchor_index].placement;
        let signed_area = anchor_vertices
            .iter()
            .enumerate()
            .map(|(i, &(x0, y0))| {
                let (x1, y1) = anchor_vertices[(i + 1) % anchor_vertices.len()];
                x0 * y1 - y0 * x1
            })
            .sum::<f64>();

        for interval in exposed {
            let a0 = anchor_vertices[interval.edge];
            let a1 = anchor_vertices[(interval.edge + 1) % anchor_vertices.len()];
            let dx = a1.0 - a0.0;
            let dy = a1.1 - a0.1;
            let length = dx.hypot(dy);
            if length <= QUANTUM {
                continue;
            }

            let (nx, ny) = if signed_area >= 0.0 {
                (dy / length, -dx / length)
            } else {
                (-dy / length, dx / length)
            };
            let world_n = (
                nx * placement.rotation_radians.cos() - ny * placement.rotation_radians.sin(),
                nx * placement.rotation_radians.sin() + ny * placement.rotation_radians.cos(),
            );
            let outward_angle = world_n.1.atan2(world_n.0);
            for candidate_vertex in 0..candidate_vertices.len() {
                // A convex polygon remains outside the anchor while its selected
                // vertex is on the supporting line whenever the polygon interior
                // direction lies in the inward half-plane. The boundary of that
                // admissible set is therefore a +/- pi/2 interval around the
                // outward normal, adjusted by the local vertex radial direction.
                let vertex_angle = candidate_vertices[candidate_vertex]
                    .1
                    .atan2(candidate_vertices[candidate_vertex].0);
                let center = outward_angle - vertex_angle;
                let start = center - std::f64::consts::FRAC_PI_2;
                let end = center + std::f64::consts::FRAC_PI_2;
                let family = GeometryRigidVertexContactFamily {
                    schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                    formation_signature: formation.signature.clone(),
                    candidate_resource: candidate_resource.name.clone(),
                    anchor_constituent: anchor_index,
                    anchor_edge: interval.edge,
                    candidate_vertex,
                    anchor_parameter_start: interval.start,
                    anchor_parameter_end: interval.end,
                    candidate_rotation_start_radians: start,
                    candidate_rotation_end_radians: end,
                };
                unique.insert(family.signature(), family);
            }
        }
    }
    unique.into_values().collect()
}

pub fn generate_rigid_contact_families(
    formation: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryRigidContactFamily> {
    if candidate_resource.physical_state == crate::resources::PhysicalState::Fluid {
        return Vec::new();
    }
    let mut unique = BTreeMap::new();
    for anchor_index in 0..formation.constituents.len() {
        let Some(anchor_resource) = catalog
            .iter()
            .find(|r| r.name == formation.constituents[anchor_index].resource)
        else {
            continue;
        };
        let anchor_segments = rigid_boundary_segments(&anchor_resource.shape.form);
        let exposed = if matches!(anchor_resource.shape.form, Form::Line { .. }) {
            exposed_line_intervals(formation, anchor_index, catalog)
        } else {
            exposed_polygon_edge_intervals(formation, anchor_index, catalog)
        };
        let candidate_segments = rigid_boundary_segments(&candidate_resource.shape.form);
        for interval in exposed {
            let Some(&(a0, a1)) = anchor_segments.get(interval.edge) else {
                continue;
            };
            let anchor_length = (a1.0 - a0.0).hypot(a1.1 - a0.1);
            if anchor_length <= QUANTUM {
                continue;
            }
            let anchor = formation.constituents[anchor_index].placement;
            let world_a0 = world_point(a0, anchor);
            let world_a1 = world_point(a1, anchor);
            let anchor_angle = (world_a1.1 - world_a0.1).atan2(world_a1.0 - world_a0.0);
            for (candidate_edge, &(c0, c1)) in candidate_segments.iter().enumerate() {
                let candidate_length = (c1.0 - c0.0).hypot(c1.1 - c0.1);
                if candidate_length <= QUANTUM {
                    continue;
                }
                let candidate_angle = (c1.1 - c0.1).atan2(c1.0 - c0.0);
                for flip in [0.0, std::f64::consts::PI] {
                    let rotation = normalize_angle(anchor_angle + flip - candidate_angle);
                    let family = GeometryRigidContactFamily {
                        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                        formation_signature: formation.signature.clone(),
                        candidate_resource: candidate_resource.name.clone(),
                        anchor_constituent: anchor_index,
                        anchor_edge: interval.edge,
                        candidate_edge,
                        candidate_rotation_radians: rotation,
                        // The interval describes where the realized contact
                        // point may lie on the exposed anchor edge. Candidate
                        // edge length and orientation affect pose reconstruction,
                        // not this normalized contact-point interval.
                        anchor_parameter_start: interval.start,
                        anchor_parameter_end: interval.end,
                    };
                    if family.anchor_parameter_end >= family.anchor_parameter_start - QUANTUM {
                        unique.insert(family.signature(), family);
                    }
                }
            }
        }
    }
    unique.into_values().collect()
}

pub fn generate_two_constituent_candidates(
    target: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryFormation> {
    if target.constituents.len() != 1 {
        return Vec::new();
    }
    let Some(target_resource) = catalog
        .iter()
        .find(|r| r.name == target.constituents[0].resource)
    else {
        return Vec::new();
    };

    // Fluid + fluid is volume accumulation, not a new rigid formation. There
    // is therefore no second constituent geometry to enumerate here.
    if target_resource.physical_state == crate::resources::PhysicalState::Fluid
        && candidate_resource.physical_state == crate::resources::PhysicalState::Fluid
    {
        return Vec::new();
    }
    let target_placement = target.constituents[0].placement;
    let mut out = Vec::new();
    let target_vertices = target_resource.shape.form.polygon_vertices();
    let candidate_vertices = candidate_resource.shape.form.polygon_vertices();

    if let (Some(tv), Some(cv)) = (&target_vertices, &candidate_vertices) {
        for ti in 0..tv.len() {
            for ci in 0..cv.len() {
                for rotation in crate::rigid_boundary::corner_alignment_rotations(
                    &candidate_resource.shape,
                    ci,
                    &target_resource.shape,
                    ti,
                    target_placement.rotation_radians,
                ) {
                    let (tx, ty) = crate::rigid_boundary::world_vertex(
                        &target_resource.shape,
                        ti,
                        target_placement,
                    )
                    .unwrap();
                    let (cx, cy) = rotated_point(cv[ci], rotation);
                    push_candidate(
                        &mut out,
                        target,
                        candidate_resource,
                        Placement {
                            x: tx - cx,
                            y: ty - cy,
                            rotation_radians: rotation,
                        },
                        catalog,
                    );
                }
            }
        }

        // Exact flat-to-flat contacts: align boundary edges and coincide one
        // endpoint. No angular sampling is used.
        for te in 0..tv.len() {
            let tn = (te + 1) % tv.len();
            for ce in 0..cv.len() {
                let cn = (ce + 1) % cv.len();
                let Some(ta) = edge_angle_world(tv[te], tv[tn], target_placement.rotation_radians)
                else {
                    continue;
                };
                let Some(ca) = edge_angle(cv[ce], cv[cn]) else {
                    continue;
                };
                for flip in [0.0, std::f64::consts::PI] {
                    let rotation = normalize_angle(ta + flip - ca);
                    let (tx, ty) = world_point(tv[te], target_placement);
                    let (cx, cy) = rotated_point(cv[ce], rotation);
                    push_candidate(
                        &mut out,
                        target,
                        candidate_resource,
                        Placement {
                            x: tx - cx,
                            y: ty - cy,
                            rotation_radians: rotation,
                        },
                        catalog,
                    );
                }
            }
        }
    }

    if let Form::Line { length } = &candidate_resource.shape.form {
        let half = *length / 2.0;
        let endpoints = [(-half, 0.0), (half, 0.0)];
        if let Some(tv) = target_vertices {
            for ti in 0..tv.len() {
                for endpoint in 0..2 {
                    for rotation in crate::rigid_boundary::line_endpoint_alignment_rotations(
                        endpoint,
                        0,
                        target_placement.rotation_radians,
                    ) {
                        let (tx, ty) = world_point(tv[ti], target_placement);
                        let (cx, cy) = rotated_point(endpoints[endpoint], rotation);
                        push_candidate(
                            &mut out,
                            target,
                            candidate_resource,
                            Placement {
                                x: tx - cx,
                                y: ty - cy,
                                rotation_radians: rotation,
                            },
                            catalog,
                        );
                    }
                }
            }
        }
    }

    let mut unique = BTreeMap::new();
    for formation in out {
        if let Some(canonical) = formation.canonicalized(catalog) {
            unique.insert(canonical.signature.clone(), canonical);
        }
    }
    unique.into_values().collect()
}

pub fn expand_three_constituent_candidates(
    two_constituent: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryFormation> {
    if two_constituent.constituents.len() != 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for anchor_index in 0..2 {
        let anchor = &two_constituent.constituents[anchor_index];
        let anchor_formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![anchor.clone()],
            bonds: Vec::new(),
            signature: String::new(),
        };
        for pair in
            generate_two_constituent_candidates(&anchor_formation, candidate_resource, catalog)
        {
            let world = compose_placements(anchor.placement, pair.constituents[1].placement);
            let mut formation = two_constituent.clone();
            formation.constituents.push(GeometryConstituent {
                resource: candidate_resource.name.clone(),
                placement: world,
            });
            formation.bonds.push(GeometryBond {
                constituent_a: anchor_index,
                constituent_b: 2,
            });
            if validate_formation(&formation, catalog) {
                out.push(formation);
            }
        }
    }
    let mut unique = BTreeMap::new();
    for formation in out {
        if let Some(canonical) = formation.canonicalized(catalog) {
            unique.insert(canonical.signature.clone(), canonical);
        }
    }
    unique.into_values().collect()
}

fn push_candidate(
    out: &mut Vec<GeometryFormation>,
    target: &GeometryFormation,
    resource: &BaseResource,
    placement: Placement,
    catalog: &[BaseResource],
) {
    let formation = GeometryFormation {
        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
        constituents: vec![
            target.constituents[0].clone(),
            GeometryConstituent {
                resource: resource.name.clone(),
                placement,
            },
        ],
        bonds: vec![GeometryBond {
            constituent_a: 0,
            constituent_b: 1,
        }],
        signature: String::new(),
    };
    if validate_formation(&formation, catalog) {
        out.push(formation);
    }
}

fn rotated_point(point: (f64, f64), rotation: f64) -> (f64, f64) {
    let (s, c) = rotation.sin_cos();
    (point.0 * c - point.1 * s, point.0 * s + point.1 * c)
}

fn world_point(point: (f64, f64), placement: Placement) -> (f64, f64) {
    let (x, y) = rotated_point(point, placement.rotation_radians);
    (placement.x + x, placement.y + y)
}

fn edge_angle(a: (f64, f64), b: (f64, f64)) -> Option<f64> {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    if dx.hypot(dy) <= f64::EPSILON {
        None
    } else {
        Some(dy.atan2(dx))
    }
}

fn edge_angle_world(a: (f64, f64), b: (f64, f64), rotation: f64) -> Option<f64> {
    Some(normalize_angle(edge_angle(a, b)? + rotation))
}

fn is_convex_polygon(vertices: &[(f64, f64)]) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    let mut sign = 0.0;
    for i in 0..vertices.len() {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];
        let c = vertices[(i + 2) % vertices.len()];
        let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
        if cross.abs() <= QUANTUM {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    sign != 0.0
}

fn normalize_angle(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

fn compose_placements(parent: Placement, local: Placement) -> Placement {
    let (x, y) = rotated_point((local.x, local.y), parent.rotation_radians);
    Placement {
        x: parent.x + x,
        y: parent.y + y,
        rotation_radians: normalize_angle(parent.rotation_radians + local.rotation_radians),
    }
}

pub fn expand_formation_candidates(
    formation: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryFormation> {
    if formation.constituents.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for anchor_index in 0..formation.constituents.len() {
        let anchor = &formation.constituents[anchor_index];
        let anchor_formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![anchor.clone()],
            bonds: Vec::new(),
            signature: String::new(),
        };

        for pair in
            generate_two_constituent_candidates(&anchor_formation, candidate_resource, catalog)
        {
            let world = compose_placements(anchor.placement, pair.constituents[1].placement);
            let mut candidate = formation.clone();
            let new_index = candidate.constituents.len();
            candidate.constituents.push(GeometryConstituent {
                resource: candidate_resource.name.clone(),
                placement: world,
            });
            candidate.bonds.push(GeometryBond {
                constituent_a: anchor_index,
                constituent_b: new_index,
            });

            if validate_formation(&candidate, catalog) {
                out.push(candidate);
            }
        }
    }

    let mut unique = BTreeMap::new();
    for candidate in out {
        if let Some(canonical) = candidate.canonicalized(catalog) {
            unique.insert(canonical.signature.clone(), canonical);
        }
    }
    unique.into_values().collect()
}

pub fn seed_two_constituent_catalogue(
    library: &mut GeometryLibrary,
    catalog: &[BaseResource],
) -> std::io::Result<usize> {
    let singles: Vec<_> = catalog
        .iter()
        .map(|resource| GeometryFormation::single(resource.name.clone()))
        .collect();
    let mut candidates = Vec::new();
    for target in singles {
        for resource in catalog {
            candidates.extend(generate_two_constituent_candidates(
                &target, resource, catalog,
            ));
        }
    }
    library.insert_many(candidates, catalog)
}

pub fn seed_three_constituent_catalogue(
    library: &mut GeometryLibrary,
    catalog: &[BaseResource],
) -> std::io::Result<usize> {
    let two: Vec<_> = library
        .formations()
        .filter(|formation| formation.constituents.len() == 2)
        .cloned()
        .collect();
    let mut candidates = Vec::new();
    for formation in two {
        for resource in catalog {
            candidates.extend(expand_three_constituent_candidates(
                &formation, resource, catalog,
            ));
        }
    }
    library.insert_many(candidates, catalog)
}

pub fn seed_base_catalogue(
    library: &mut GeometryLibrary,
    catalog: &[BaseResource],
) -> std::io::Result<usize> {
    let mut added = 0;
    for resource in catalog {
        if library.insert(GeometryFormation::single(resource.name.clone()), catalog)? {
            added += 1;
        }
    }
    Ok(added)
}

/// Resolve Bob's persistent store once, consistently for the worker and viewer.
/// An explicit environment override lets a source checkout use an existing
/// catalogue without copying it into the checkout.
pub fn default_library_root() -> PathBuf {
    std::env::var_os("EVOSIM_GEOMETRY_LIBRARY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("geometry_library/data"))
}

pub fn open_default_library() -> std::io::Result<GeometryLibrary> {
    let root = default_library_root();
    if std::env::var_os("EVOSIM_GEOMETRY_LIBRARY_DIR").is_some() {
        let empty_directory = root.is_dir()
            && fs::read_dir(&root)
                .and_then(|mut entries| entries.next().transpose())
                .map(|entry| entry.is_none())
                .unwrap_or(false);
        let has_existing_store =
            root.join("manifest.json").is_file() && root.join("formations.jsonl").is_file();
        if !root.is_dir() || (!has_existing_store && !empty_directory) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "EVOSIM_GEOMETRY_LIBRARY_DIR must point to an empty directory for a new Bob store or an existing store containing manifest.json and formations.jsonl: {}",
                    root.display()
                ),
            ));
        }
    }
    let catalog = default_catalog();
    GeometryLibrary::open(root, &catalog)
}

#[cfg(test)]
mod bob_lookup_contract_tests {
    use super::*;

    #[test]
    fn fresh_empty_library_opens_and_reopens_without_seeding_geometry() {
        let catalog = crate::resources::default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-bob-empty-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        {
            let library = GeometryLibrary::open(&root, &catalog)
                .expect("an empty isolated geometry root should open");
            assert_eq!(library.len(), 0);
            assert_eq!(library.manifest.entries, 0);
            assert_eq!(library.formation_storage_version, 3);
            assert!(library.frontier.records.is_empty());
            assert_eq!(library.contact_families().count(), 0);
            assert_eq!(library.fluid_boundary_families().count(), 0);
            assert_eq!(library.rigid_contact_families().count(), 0);
            assert_eq!(library.rigid_point_contact_families().count(), 0);
            assert_eq!(library.rigid_vertex_contact_families().count(), 0);

            // Opening initializes schema metadata only; it must not seed or
            // generate formation/family records as a side effect.
            assert!(!root.join("formations.jsonl").exists());
            assert!(!root.join("frontier.json").exists());
            assert!(!root.join("contact_families.jsonl").exists());
            assert!(!root.join("fluid_boundary_families.jsonl").exists());
            assert!(!root.join("rigid_contact_families.jsonl").exists());
            assert!(!root.join("rigid_point_contact_families.jsonl").exists());
            assert!(!root.join("rigid_vertex_contact_families.jsonl").exists());

            let storage: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join("storage_manifest.json"))
                    .expect("open should initialize storage metadata"),
            )
            .expect("storage manifest should be valid JSON");
            assert_eq!(storage["storage_format_version"], 3);
        }

        let reopened = GeometryLibrary::open(&root, &catalog)
            .expect("the empty initialized store should reopen");
        assert_eq!(reopened.len(), 0);
        assert_eq!(reopened.manifest.entries, 0);
        assert_eq!(reopened.formation_storage_version, 3);
        assert!(reopened.frontier.records.is_empty());

        drop(reopened);
        std::fs::remove_dir_all(root).expect("remove isolated test root");
    }

    #[test]
    fn periodic_angle_interval_handles_normal_and_wrapped_ranges() {
        let pi = std::f64::consts::PI;
        let tau = std::f64::consts::TAU;
        assert!(angle_in_periodic_interval(0.5, 0.25, 0.75));
        assert!(angle_in_periodic_interval(0.25, 0.25, 0.75));
        assert!(angle_in_periodic_interval(0.75, 0.25, 0.75));
        assert!(!angle_in_periodic_interval(0.8, 0.25, 0.75));
        assert!(angle_in_periodic_interval(-pi + 0.1, pi - 0.2, pi + 0.2));
        assert!(angle_in_periodic_interval(pi + 0.1, pi - 0.2, pi + 0.2));
        assert!(!angle_in_periodic_interval(0.0, pi - 0.2, pi + 0.2));
        assert!(!angle_in_periodic_interval(0.0, 1.0, -1.0));
        assert!(angle_in_periodic_interval(0.0, -tau, tau));
    }
    use crate::resources::default_catalog;
    use std::hint::black_box;
    use std::time::Instant;

    fn edge_query(library: &GeometryLibrary) -> Option<LiveGeometryInterface> {
        let family = library.rigid_contact_families().next()?;
        let formation = library.get(&family.formation_signature)?;
        let anchor = formation.constituents.get(family.anchor_constituent)?;
        let anchor_parameter = (family.anchor_parameter_start + family.anchor_parameter_end) * 0.5;

        Some(LiveGeometryInterface {
            interface_class: "rigid_edge",
            signature: String::new(),
            query: Some(LiveGeometryQuery::RigidEdge {
                a_material: family.candidate_resource.clone(),
                a_edge: family.candidate_edge,
                a_parameter: 0,
                a_rotation: (family.candidate_rotation_radians * 1_000_000_000.0) as i64,
                b_material: anchor.resource.clone(),
                b_edge: family.anchor_edge,
                b_parameter: (anchor_parameter * 1_000_000_000.0) as i64,
                b_rotation: 0,
            }),
        })
    }

    #[test]
    fn canonicalized_formation_preserves_constituent_index_mapping() {
        let catalog = default_catalog();
        let formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "Carbon".to_string(),
                    placement: Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                GeometryConstituent {
                    resource: "Carbon".to_string(),
                    placement: Placement {
                        x: 3.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
            ],
            bonds: vec![GeometryBond {
                constituent_a: 0,
                constituent_b: 1,
            }],
            signature: String::new(),
        };
        let (canonical, mapping) = formation
            .canonicalized_with_mapping(&catalog)
            .expect("asymmetric two-constituent parent has a unique canonical mapping");
        assert_eq!(mapping.len(), 2);
        assert_ne!(mapping[0], mapping[1]);
        assert!(mapping
            .iter()
            .all(|index| *index < canonical.constituents.len()));
        assert_eq!(
            canonical.bonds,
            vec![GeometryBond {
                constituent_a: 0,
                constituent_b: 1,
            }]
        );
    }

    #[test]
    fn live_interface_is_endpoint_order_invariant() {
        let forward = resolve_live_contact_interface(
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            "Nitrogen",
            ConnectionEndpoint::Boundary {
                angle_radians: std::f64::consts::PI,
            },
        );
        let reverse = resolve_live_contact_interface(
            "Nitrogen",
            ConnectionEndpoint::Boundary {
                angle_radians: std::f64::consts::PI,
            },
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
        );

        assert_eq!(forward.interface_class, reverse.interface_class);
        assert_eq!(forward.signature, reverse.signature);
        assert_eq!(forward.query, reverse.query);
    }

    #[test]
    fn indexed_edge_lookup_rejects_wrong_anchor_material() {
        let catalog = default_catalog();
        let root = std::env::temp_dir().join(format!("evosim-bob-contract-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();

        let formation = GeometryFormation::single("Carbon");
        let signature = formation.signature.clone();
        let family = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: signature,
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };

        library
            .entries
            .insert(formation.signature.clone(), formation);
        library
            .rigid_contact_families
            .insert(family.signature(), family.clone());
        library
            .rigid_contact_index
            .entry(contact_bucket_hash(
                &family.candidate_resource,
                family.anchor_edge,
                family.candidate_edge,
            ))
            .or_default()
            .push(family.signature());

        let interface = LiveGeometryInterface {
            interface_class: "rigid_edge",
            signature: String::new(),
            query: Some(LiveGeometryQuery::RigidEdge {
                a_material: "Carbon".to_string(),
                a_edge: 0,
                a_parameter: 0,
                a_rotation: 0,
                b_material: "Nitrogen".to_string(),
                b_edge: 0,
                b_parameter: 500_000_000,
                b_rotation: 0,
            }),
        };

        assert_eq!(
            library.resolve_persistent_interface(&interface),
            LiveFamilyResolution::Unresolved
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn same_material_same_edge_lookup_fails_closed_when_orientation_is_unknown() {
        let catalog = default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-bob-same-edge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        let signature = formation.signature.clone();
        library.entries.insert(signature.clone(), formation);

        let family = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: signature,
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.25,
            anchor_parameter_end: 0.75,
        };
        let family_signature = family.signature();
        library
            .rigid_contact_families
            .insert(family_signature.clone(), family);
        library
            .rigid_contact_index
            .entry(contact_bucket_hash("Carbon", 0, 0))
            .or_default()
            .push(family_signature);

        let query = LiveGeometryInterface {
            interface_class: "rigid_edge",
            signature: String::new(),
            query: Some(LiveGeometryQuery::RigidEdge {
                a_material: "Carbon".to_string(),
                a_edge: 0,
                a_parameter: 0,
                a_rotation: 0,
                b_material: "Carbon".to_string(),
                b_edge: 0,
                b_parameter: 500_000_000,
                b_rotation: 0,
            }),
        };
        assert_eq!(
            library.resolve_persistent_interface(&query),
            LiveFamilyResolution::Unresolved,
            "same-material, same-edge queries cannot identify candidate vs anchor"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn family_projections_preserve_interval_ends_and_rotation_ranges() {
        let edge = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: "formation".to_string(),
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 1,
            candidate_rotation_radians: 0.5,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 0.5,
        };
        let mut changed_edge = edge.clone();
        changed_edge.anchor_parameter_end = 0.75;
        assert_ne!(
            rigid_family_projection(&edge, 0.5),
            rigid_family_projection(&changed_edge, 0.5)
        );
        let mut context_edge = edge.clone();
        context_edge.anchor_constituent = 7;
        assert_eq!(
            rigid_family_projection(&edge, 0.5),
            rigid_family_projection(&context_edge, 0.5),
            "constituent indices belong to formation context, not local interface identity"
        );

        let point = GeometryRigidPointContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: "formation".to_string(),
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_endpoint: 1,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 0.5,
            candidate_rotation_start_radians: 0.25,
            candidate_rotation_end_radians: 0.75,
        };
        let mut changed_point_end = point.clone();
        changed_point_end.anchor_parameter_end = 0.75;
        assert_ne!(
            point_family_projection(&point),
            point_family_projection(&changed_point_end)
        );
        let mut changed_point_rotation = point.clone();
        changed_point_rotation.candidate_rotation_end_radians = 1.0;
        assert_ne!(
            point_family_projection(&point),
            point_family_projection(&changed_point_rotation)
        );
        let mut context_point = point.clone();
        context_point.anchor_constituent = 7;
        assert_eq!(
            point_family_projection(&point),
            point_family_projection(&context_point)
        );

        let vertex = GeometryRigidVertexContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: "formation".to_string(),
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_vertex: 1,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 0.5,
            candidate_rotation_start_radians: 0.25,
            candidate_rotation_end_radians: 0.75,
        };
        let mut changed_vertex_end = vertex.clone();
        changed_vertex_end.anchor_parameter_end = 0.75;
        assert_ne!(
            vertex_family_projection(&vertex),
            vertex_family_projection(&changed_vertex_end)
        );
        let mut changed_vertex_rotation = vertex.clone();
        changed_vertex_rotation.candidate_rotation_start_radians = 0.0;
        assert_ne!(
            vertex_family_projection(&vertex),
            vertex_family_projection(&changed_vertex_rotation)
        );
        let mut context_vertex = vertex.clone();
        context_vertex.anchor_constituent = 7;
        assert_eq!(
            vertex_family_projection(&vertex),
            vertex_family_projection(&context_vertex)
        );
    }

    #[test]
    fn same_material_edge_lookup_is_endpoint_order_invariant() {
        let catalog = default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-bob-same-material-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        let signature = formation.signature.clone();
        library.entries.insert(signature.clone(), formation);

        let family = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: signature,
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 1,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.25,
            anchor_parameter_end: 0.75,
        };
        let family_signature = family.signature();
        library
            .rigid_contact_families
            .insert(family_signature.clone(), family);
        library
            .rigid_contact_index
            .entry(contact_bucket_hash("Carbon", 0, 1))
            .or_default()
            .push(family_signature);

        let query = |candidate_first: bool| {
            let (a_edge, b_edge, a_parameter, b_parameter) = if candidate_first {
                (1, 0, 0, 500_000_000)
            } else {
                (0, 1, 500_000_000, 0)
            };
            LiveGeometryInterface {
                interface_class: "rigid_edge",
                signature: String::new(),
                query: Some(LiveGeometryQuery::RigidEdge {
                    a_material: "Carbon".to_string(),
                    a_edge,
                    a_parameter,
                    a_rotation: 0,
                    b_material: "Carbon".to_string(),
                    b_edge,
                    b_parameter,
                    b_rotation: 0,
                }),
            }
        };

        assert_eq!(
            library.resolve_persistent_interface(&query(true)),
            LiveFamilyResolution::Unique
        );
        assert_eq!(
            library.resolve_persistent_interface(&query(false)),
            LiveFamilyResolution::Unique,
            "same-material contacts must not depend on endpoint ordering"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn empty_library_proposes_rigid_edge_placements_without_persisting_families() {
        let catalog = default_catalog();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap()
            .clone();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-empty-placement-suggestions-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let library = GeometryLibrary::open(&root, &catalog).unwrap();
        assert!(library.rigid_contact_families.is_empty());
        assert!(library.entries.is_empty());

        let live_anchor = Placement {
            x: 12.0,
            y: -4.0,
            rotation_radians: 0.37,
        };
        let suggestions =
            library.suggest_rigid_edge_placements("Carbon", live_anchor, &carbon, &catalog);
        assert!(!suggestions.is_empty());
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        assert!(
            library
                .suggest_rigid_edge_placements("Carbon", live_anchor, water, &catalog)
                .is_empty(),
            "fluid candidates do not use the rigid-edge placement path"
        );
        let phosphorus = catalog
            .iter()
            .find(|resource| resource.name == "Phosphorus")
            .unwrap();
        let phosphorus_suggestions =
            library.suggest_rigid_edge_placements("Carbon", live_anchor, phosphorus, &catalog);
        assert!(
            !phosphorus_suggestions.is_empty(),
            "empty-library fallback should support a different rigid shape"
        );
        let anchor_segments = rigid_boundary_segments(&carbon.shape.form);
        let phosphorus_segments = rigid_boundary_segments(&phosphorus.shape.form);
        let phosphorus_lengths = phosphorus_segments
            .iter()
            .map(|(start, end)| (end.0 - start.0).hypot(end.1 - start.1))
            .collect::<Vec<_>>();
        assert!(
            phosphorus_lengths
                .iter()
                .any(|length| (*length - 1.5).abs() < 1e-6)
                && phosphorus_lengths
                    .iter()
                    .any(|length| (*length - 0.5).abs() < 1e-6),
            "the candidate shape should exercise unequal edge lengths"
        );
        assert!(phosphorus_suggestions.iter().all(|suggestion| {
            let Some(&(a0, a1)) = anchor_segments.get(suggestion.anchor_edge) else {
                return false;
            };
            let Some(&(c0, c1)) = phosphorus_segments.get(suggestion.candidate_edge) else {
                return false;
            };
            let parameter = suggestion.anchor_contact_parameter;
            let anchor_local = (
                a0.0 + (a1.0 - a0.0) * parameter,
                a0.1 + (a1.1 - a0.1) * parameter,
            );
            let candidate_midpoint = ((c0.0 + c1.0) * 0.5, (c0.1 + c1.1) * 0.5);
            let anchor_world = world_point(anchor_local, live_anchor);
            let candidate_world = world_point(candidate_midpoint, suggestion.placement);
            (anchor_world.0 - candidate_world.0).abs() < 1e-7
                && (anchor_world.1 - candidate_world.1).abs() < 1e-7
        }));
        let candidate_segments = rigid_boundary_segments(&carbon.shape.form);
        assert!(suggestions.iter().all(|suggestion| {
            let Some(&(a0, a1)) = anchor_segments.get(suggestion.anchor_edge) else {
                return false;
            };
            let Some(&(c0, c1)) = candidate_segments.get(suggestion.candidate_edge) else {
                return false;
            };
            let parameter = suggestion.anchor_contact_parameter;
            let anchor_local = (
                a0.0 + (a1.0 - a0.0) * parameter,
                a0.1 + (a1.1 - a0.1) * parameter,
            );
            let candidate_midpoint = ((c0.0 + c1.0) * 0.5, (c0.1 + c1.1) * 0.5);
            let anchor_world = world_point(anchor_local, live_anchor);
            let candidate_world = world_point(candidate_midpoint, suggestion.placement);
            suggestion.candidate_resource == "Carbon"
                && (0.0..=1.0).contains(&parameter)
                && suggestion.placement.x.is_finite()
                && suggestion.placement.y.is_finite()
                && suggestion.placement.rotation_radians.is_finite()
                && (anchor_world.0 - candidate_world.0).abs() < 1e-7
                && (anchor_world.1 - candidate_world.1).abs() < 1e-7
        }));
        assert!(suggestions.iter().any(|suggestion| {
            (suggestion.placement.x - live_anchor.x).abs() > 1e-6
                || (suggestion.placement.y - live_anchor.y).abs() > 1e-6
        }));

        assert!(library.entries.is_empty());
        assert!(library.rigid_contact_families.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn generated_rigid_edge_families_store_exposed_contact_intervals() {
        let catalog = crate::resources::default_catalog();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let formation = GeometryFormation::single("Carbon");
        let families = generate_rigid_contact_families(&formation, carbon, &catalog);
        assert!(!families.is_empty());
        assert!(families.iter().all(|family| {
            family.anchor_parameter_start >= 0.0
                && family.anchor_parameter_end <= 1.0
                && family.anchor_parameter_start <= family.anchor_parameter_end
        }));

        let same_edge = families
            .iter()
            .filter(|family| family.anchor_edge == 0 && family.candidate_edge == 0)
            .collect::<Vec<_>>();
        assert_eq!(
            same_edge.len(),
            2,
            "both edge orientations should be retained"
        );
        assert!(same_edge.iter().all(|family| {
            (family.anchor_parameter_start - 0.0).abs() < 1e-9
                && (family.anchor_parameter_end - 1.0).abs() < 1e-9
        }));
    }

    #[test]
    fn indexed_edge_lookup_compares_rotation_relative_to_anchor_pose() {
        let catalog = crate::resources::default_catalog();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-relative-rotation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();

        let mut formation = GeometryFormation::single("Carbon");
        formation.constituents[0].placement.rotation_radians = 0.5;
        formation.signature = formation.canonical_signature();
        let formation_signature = formation.signature.clone();
        library
            .entries
            .insert(formation_signature.clone(), formation);

        let family = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature,
            candidate_resource: "Hydrogen".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 1.2,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };
        let family_signature = family.signature();
        library
            .rigid_contact_families
            .insert(family_signature.clone(), family);
        library
            .rigid_contact_index
            .entry(contact_bucket_hash("Hydrogen", 0, 0))
            .or_default()
            .push(family_signature);

        let interface = LiveGeometryInterface {
            interface_class: "rigid_edge",
            signature: "relative-rotation-regression".to_string(),
            query: Some(LiveGeometryQuery::RigidEdge {
                a_material: "Hydrogen".to_string(),
                a_edge: 0,
                a_parameter: 0,
                a_rotation: quantize(1.2),
                b_material: "Carbon".to_string(),
                b_edge: 0,
                b_parameter: quantize(0.5),
                b_rotation: quantize(0.5),
            }),
        };
        assert_eq!(
            library.resolve_persistent_interface(&interface),
            LiveFamilyResolution::Unique,
            "family rotation must be compared relative to its anchor's stored pose"
        );
        assert!(library
            .persistent_interface_projection(&interface)
            .is_some());

        drop(library);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_edge_lookup_distinguishes_unique_ambiguous_and_unresolved() {
        let catalog = default_catalog();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-resolution-matrix-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        let signature = formation.signature.clone();
        library.entries.insert(signature.clone(), formation);

        let make_family = |start: f64, end: f64| GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: signature.clone(),
            candidate_resource: "Nitrogen".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: start,
            anchor_parameter_end: end,
        };
        let first = make_family(0.0, 0.75);
        let first_signature = first.signature();
        library
            .rigid_contact_families
            .insert(first_signature.clone(), first.clone());
        library
            .rigid_contact_index
            .entry(contact_bucket_hash("Nitrogen", 0, 0))
            .or_default()
            .push(first_signature);

        let query_at = |parameter: i64| LiveGeometryInterface {
            interface_class: "rigid_edge",
            signature: String::new(),
            query: Some(LiveGeometryQuery::RigidEdge {
                a_material: "Nitrogen".to_string(),
                a_edge: 0,
                a_parameter: 0,
                a_rotation: 0,
                b_material: "Carbon".to_string(),
                b_edge: 0,
                b_parameter: parameter,
                b_rotation: 0,
            }),
        };

        assert_eq!(
            library.resolve_persistent_interface(&query_at(500_000_000)),
            LiveFamilyResolution::Unique,
            "one matching family should resolve uniquely"
        );
        assert_eq!(
            library.resolve_persistent_interface(&query_at(900_000_000)),
            LiveFamilyResolution::Unresolved,
            "a parameter outside every family interval must not match"
        );

        let second = make_family(0.25, 1.0);
        let second_signature = second.signature();
        library
            .rigid_contact_families
            .insert(second_signature.clone(), second);
        library
            .rigid_contact_index
            .entry(contact_bucket_hash("Nitrogen", 0, 0))
            .or_default()
            .push(second_signature);
        assert_eq!(
            library.resolve_persistent_interface(&query_at(500_000_000)),
            LiveFamilyResolution::Ambiguous,
            "two distinct matching projections must be reported as ambiguous"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "requires the persistent Bob catalogue and is intended for local performance measurement"]
    fn benchmark_indexed_lookup_throughput() {
        let catalog = default_catalog();
        let library = GeometryLibrary::open("geometry_library/data", &catalog).unwrap();
        let Some(query) = edge_query(&library) else {
            panic!("Bob has no rigid-edge family to benchmark");
        };

        const WARMUP: u64 = 10_000;
        const ITERATIONS: u64 = 200_000;

        for _ in 0..WARMUP {
            black_box(library.resolve_persistent_interface(&query));
        }

        let start = Instant::now();
        let mut resolved = 0u64;
        for _ in 0..ITERATIONS {
            if black_box(library.resolve_persistent_interface(&query))
                != LiveFamilyResolution::Unresolved
            {
                resolved += 1;
            }
        }

        let elapsed = start.elapsed();
        let qps = ITERATIONS as f64 / elapsed.as_secs_f64();
        println!(
            "Bob rigid-edge lookup: {} queries in {:?}; {:.0} queries/sec; {} resolved",
            ITERATIONS, elapsed, qps, resolved
        );

        assert_eq!(resolved, ITERATIONS);
    }
}

#[cfg(test)]
mod bob_candidate_generation_contract_tests {
    use super::*;
    use crate::resources::default_catalog;
    use std::collections::BTreeSet;

    #[test]
    fn overlapping_rigid_constituents_are_rejected_even_if_declared_bonded() {
        let catalog = default_catalog();
        let mut formation = GeometryFormation::single("Carbon");
        formation.constituents.push(GeometryConstituent {
            resource: "Carbon".to_string(),
            placement: Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        });
        formation.bonds.push(GeometryBond {
            constituent_a: 0,
            constituent_b: 1,
        });

        assert!(!validate_formation(&formation, &catalog));
    }

    #[test]
    fn two_constituent_candidates_are_valid_unique_and_do_not_mutate_the_seed() {
        let catalog = default_catalog();
        let seed = GeometryFormation::single("Carbon");
        let original_seed = seed.clone();
        let nitrogen = catalog
            .iter()
            .find(|resource| resource.name == "Nitrogen")
            .expect("default catalog must include Nitrogen");

        let candidates = generate_two_constituent_candidates(&seed, nitrogen, &catalog);
        assert_eq!(seed, original_seed, "candidate generation mutated its seed");
        assert!(
            !candidates.is_empty(),
            "expected at least one valid Carbon-Nitrogen contact arrangement"
        );
        assert!(candidates
            .iter()
            .all(|candidate| validate_formation(candidate, &catalog)));

        let signatures: BTreeSet<_> = candidates
            .iter()
            .map(|candidate| candidate.signature.clone())
            .collect();
        assert_eq!(
            signatures.len(),
            candidates.len(),
            "candidate generation returned duplicate canonical signatures"
        );
    }

    #[test]
    fn candidate_signatures_do_not_depend_on_catalog_iteration_order() {
        let catalog = default_catalog();
        let seed = GeometryFormation::single("Carbon");
        let nitrogen = catalog
            .iter()
            .find(|resource| resource.name == "Nitrogen")
            .expect("default catalog must include Nitrogen");
        let forward: BTreeSet<_> = generate_two_constituent_candidates(&seed, nitrogen, &catalog)
            .into_iter()
            .map(|candidate| candidate.signature)
            .collect();

        let mut reversed_catalog = catalog.clone();
        reversed_catalog.reverse();
        let reversed_nitrogen = reversed_catalog
            .iter()
            .find(|resource| resource.name == "Nitrogen")
            .expect("reversed catalog must include Nitrogen");
        let reversed: BTreeSet<_> =
            generate_two_constituent_candidates(&seed, reversed_nitrogen, &reversed_catalog)
                .into_iter()
                .map(|candidate| candidate.signature)
                .collect();

        assert_eq!(
            forward, reversed,
            "candidate results changed when catalog iteration order changed"
        );
    }

    #[test]
    fn rigid_family_writers_reject_missing_anchor_formations() {
        let catalog = default_catalog();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-family-validation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();

        let edge = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: "missing-formation".to_string(),
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };
        let point = GeometryRigidPointContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: "missing-formation".to_string(),
            candidate_resource: "Hydrogen".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_endpoint: 0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
            candidate_rotation_start_radians: 0.0,
            candidate_rotation_end_radians: 1.0,
        };

        assert_eq!(
            library.insert_rigid_contact_families(vec![edge]).unwrap(),
            0
        );
        assert_eq!(
            library
                .insert_rigid_point_contact_families(vec![point])
                .unwrap(),
            0
        );
        assert!(
            !root.join("rigid_contact_families.jsonl").exists(),
            "invalid edge family should not be persisted"
        );
        assert!(
            !root.join("rigid_point_contact_families.jsonl").exists(),
            "invalid point family should not be persisted"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rigid_family_loaders_reject_invalid_geometry_references() {
        let catalog = default_catalog();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-family-loader-validation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        assert!(library.insert(formation.clone(), &catalog).unwrap());

        let edge = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: formation.signature.clone(),
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: usize::MAX,
            candidate_edge: usize::MAX,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };
        let point = GeometryRigidPointContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: formation.signature,
            candidate_resource: "Carbon".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_endpoint: 0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
            candidate_rotation_start_radians: 0.0,
            candidate_rotation_end_radians: 1.0,
        };
        assert_eq!(
            library.insert_rigid_contact_families(vec![edge]).unwrap(),
            1
        );
        assert_eq!(
            library
                .insert_rigid_point_contact_families(vec![point])
                .unwrap(),
            1
        );
        drop(library);

        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert_eq!(
            reopened.rigid_contact_families().count(),
            0,
            "edge families with invalid edge indices must be discarded on load"
        );
        assert_eq!(
            reopened.rigid_point_contact_families().count(),
            0,
            "point families whose candidate is not a line must be discarded on load"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod compact_family_storage_tests {
    use super::*;
    use crate::resources::default_catalog;
    use std::collections::BTreeSet;

    #[test]
    fn compact_family_records_round_trip_and_legacy_rows_remain_readable() {
        let formation = GeometryFormation::single("Carbon");
        let mut entries = BTreeMap::new();
        entries.insert(formation.signature.clone(), formation.clone());
        let ids = build_formation_id_index(&entries).unwrap();

        let family = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: formation.signature.clone(),
            candidate_resource: "Nitrogen".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };

        let compact = compact_family_value(&family).unwrap();
        let object = compact.as_object().unwrap();
        assert!(!object.contains_key("formation_signature"));
        assert!(object.get("storage_version").is_none());
        let expected_id = formation_id(&formation.signature);
        assert_eq!(
            object
                .get("formation_id")
                .and_then(serde_json::Value::as_str),
            Some(expected_id.as_str())
        );
        let compact_line = serde_json::to_string(&compact).unwrap();
        let decoded: GeometryRigidContactFamily =
            deserialize_family_record(&compact_line, &ids).unwrap();
        assert_eq!(decoded, family);

        let legacy_line = serde_json::to_string(&family).unwrap();
        let legacy_decoded: GeometryRigidContactFamily =
            deserialize_family_record(&legacy_line, &ids).unwrap();
        assert_eq!(legacy_decoded, family);
    }

    #[test]
    fn compact_family_records_reject_unknown_versions_and_unresolved_ids() {
        let formation = GeometryFormation::single("Carbon");
        let mut entries = BTreeMap::new();
        entries.insert(formation.signature.clone(), formation.clone());
        let ids = build_formation_id_index(&entries).unwrap();
        let family = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: formation.signature,
            candidate_resource: "Nitrogen".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };
        let mut value = compact_family_value(&family).unwrap();
        value["storage_version"] = serde_json::Value::from(3);
        assert!(deserialize_family_record::<GeometryRigidContactFamily>(
            &serde_json::to_string(&value).unwrap(),
            &ids
        )
        .is_err());

        value = compact_family_value(&family).unwrap();
        value["formation_id"] = serde_json::Value::String("unresolved-id".to_string());
        assert!(deserialize_family_record::<GeometryRigidContactFamily>(
            &serde_json::to_string(&value).unwrap(),
            &ids
        )
        .is_err());
    }

    #[test]
    fn profitable_one_constituent_delta_is_written_and_reconstructed() {
        let catalog = default_catalog();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-compositional-family-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let carbon = GeometryFormation::single("Carbon");
        let nitrogen = GeometryFormation::single("Nitrogen");
        library
            .insert_many(vec![carbon.clone(), nitrogen], &catalog)
            .unwrap();

        let nitrogen_resource = catalog
            .iter()
            .find(|resource| resource.name == "Nitrogen")
            .unwrap();
        let candidates = generate_two_constituent_candidates(&carbon, nitrogen_resource, &catalog);
        assert!(!candidates.is_empty());
        let mut found_delta = false;
        for candidate in &candidates {
            let value =
                compact_formation_value(candidate, &library.entries, &BTreeMap::new()).unwrap();
            if value.get("base_id").is_some() {
                found_delta = true;
                break;
            }
        }
        assert!(
            found_delta,
            "expected at least one exact, profitable one-unit delta"
        );

        library.insert_many(candidates, &catalog).unwrap();
        let expected: BTreeSet<_> = library
            .formations()
            .map(|formation| formation.signature.clone())
            .collect();
        drop(library);

        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        let actual: BTreeSet<_> = reopened
            .formations()
            .map(|formation| formation.signature.clone())
            .collect();
        assert_eq!(actual, expected);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn family_writer_persists_compact_rows_and_reopens_them() {
        let catalog = default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-bob-compact-family-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        library.insert(formation.clone(), &catalog).unwrap();

        let family = GeometryRigidContactFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: formation.signature.clone(),
            candidate_resource: "Nitrogen".to_string(),
            anchor_constituent: 0,
            anchor_edge: 0,
            candidate_edge: 0,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };
        assert_eq!(
            library
                .insert_rigid_contact_families(vec![family.clone()])
                .unwrap(),
            1
        );

        let storage_manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("storage_manifest.json")).unwrap())
                .unwrap();
        assert_eq!(
            storage_manifest
                .get("storage_format_version")
                .and_then(serde_json::Value::as_u64),
            Some(3)
        );

        let path = root.join("rigid_contact_families.jsonl");
        let line = std::fs::read_to_string(&path).unwrap();
        let record: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert!(record.get("formation_signature").is_none());
        assert!(record.get("formation_id").is_some());
        assert!(record.get("storage_version").is_none());
        drop(library);

        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        let loaded = reopened.rigid_contact_families().next().unwrap();
        assert_eq!(loaded.signature(), family.signature());
        drop(reopened);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn water_generator_reports_unsupported_anchor_geometry() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("UnknownResource");
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();

        let (families, diagnostics) =
            generate_water_contact_families_with_diagnostics(&formation, water, &catalog);

        assert!(families.is_empty());
        assert_eq!(diagnostics.unresolved_anchor_resources, 1);
        assert_eq!(diagnostics.unsupported_anchor_geometry, 0);
        assert_eq!(diagnostics.eligible_anchors, 0);
    }

    #[test]
    fn water_generator_reports_known_but_unsupported_anchor_shape() {
        let mut catalog = default_catalog();
        let carbon = catalog
            .iter_mut()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        carbon.shape.form = Form::Circle { radius: 1.0 };
        let formation = GeometryFormation::single("Carbon");
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();

        let (families, diagnostics) =
            generate_water_contact_families_with_diagnostics(&formation, water, &catalog);

        assert!(families.is_empty());
        assert_eq!(diagnostics.unresolved_anchor_resources, 0);
        assert_eq!(diagnostics.unsupported_anchor_geometry, 1);
        assert_eq!(diagnostics.eligible_anchors, 0);
    }

    #[test]
    fn fluid_boundary_interface_resolves_persisted_family_after_reopen() {
        let catalog = default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-bob-fluid-query-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        library.insert(formation.clone(), &catalog).unwrap();

        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let area = match &water.shape.form {
            Form::Circle { radius } => std::f64::consts::PI * *radius * *radius,
            Form::Fluid { nominal_area, .. } => *nominal_area,
            _ => panic!("water must have a fluid-compatible shape"),
        };
        let equilibrium =
            solve_water_against_solid(area, water.properties.cohesion, carbon.properties.cohesion)
                .unwrap();
        // Exercise Bob's real fluid-contact calculation rather than manually
        // fabricating a persistence row. The Water/Carbon capillary solution
        // must produce an exposed-edge family before it can be stored.
        let (contact_families, diagnostics) =
            generate_water_contact_families_with_diagnostics(&formation, water, &catalog);
        assert_eq!(diagnostics.eligible_anchors, 1);
        assert_eq!(diagnostics.unsupported_anchor_geometry, 0);
        assert_eq!(diagnostics.capillary_solver_failures, 0);
        assert_eq!(diagnostics.anchors_without_exposed_intervals, 0);
        assert_eq!(diagnostics.invalid_contact_intervals, 0);
        assert_eq!(diagnostics.anchors_without_valid_contact_overlap, 0);
        assert!(
            contact_families.iter().any(|family| {
                family.anchor_constituent == 0
                    && family.anchor_edge == 0
                    && family.edge_parameter_start <= 0.5
                    && family.edge_parameter_end >= 0.5
            }),
            "expected the capillary solver to find Water contact on Carbon edge 0"
        );
        let families =
            generate_fluid_boundary_families_from_contact_families(water, &contact_families);
        assert!(
            !families.is_empty(),
            "expected fluid-boundary records from solved contacts"
        );

        // Persistence must reject malformed fluid boundary constraints even
        // when they originate from otherwise valid solver output.
        let mut invalid_interval = families[0].clone();
        invalid_interval.edge_parameter_end = 1.01;
        assert_eq!(
            library
                .insert_fluid_boundary_families(vec![invalid_interval])
                .unwrap(),
            0,
            "fluid boundary intervals must remain within the normalized edge"
        );
        let mut invalid_contact_angle = families[0].clone();
        invalid_contact_angle.contact_angle_radians = f64::NAN;
        assert_eq!(
            library
                .insert_fluid_boundary_families(vec![invalid_contact_angle])
                .unwrap(),
            0,
            "fluid boundary records must reject non-finite contact angles"
        );

        assert_eq!(
            library.insert_contact_families(contact_families).unwrap(),
            families.len(),
            "each generated Water contact must be persisted once"
        );
        assert_eq!(
            library
                .insert_fluid_boundary_families(families.clone())
                .unwrap(),
            families.len(),
            "each generated fluid boundary must be persisted once"
        );
        let fluid_formations = fluid_formations_from_boundaries(&families);
        assert!(
            !fluid_formations.is_empty(),
            "expected explicit Water-inclusive formations"
        );
        let expected_fluid_formation_signatures: BTreeMap<_, _> = fluid_formations
            .iter()
            .map(|formation| (formation.signature(), formation.clone()))
            .collect();
        assert_eq!(
            library
                .insert_fluid_formations(fluid_formations.clone(), &catalog)
                .unwrap(),
            fluid_formations.len(),
            "each valid Water-inclusive formation must be persisted"
        );
        assert_eq!(library.fluid_formations().count(), fluid_formations.len());
        let parent_signature = GeometryFormation::single("Carbon").signature;
        let parent_suggestions: Vec<_> = library
            .fluid_formations_for_parent(&parent_signature, "Water")
            .collect();
        assert!(!parent_suggestions.is_empty());
        assert!(parent_suggestions.iter().all(|formation| {
            formation.parent_formation_signature == parent_signature
                && formation.fluid_resource == "Water"
        }));
        let realized = parent_suggestions
            .iter()
            .find_map(|suggestion| suggestion.realize_against_parent(&formation, &catalog, true))
            .expect("persisted Bob fluid suggestion should reconstruct an exact shape and pose");
        assert!(realized.0.is_valid());
        assert!(matches!(
            realized.0.form,
            Form::Fluid {
                nominal_area,
                boundary: None,
                arc_boundary: Some(_), ..} if (nominal_area - area).abs() < 1e-10
        ));
        assert_eq!(
            library
                .fluid_formations_for_parent("missing-parent", "Water")
                .count(),
            0,
            "the fluid query must not silently match an unresolved parent"
        );
        assert_eq!(
            library
                .insert_fluid_formations(fluid_formations, &catalog)
                .unwrap(),
            0,
            "re-inserting identical fluid formations must be idempotent"
        );
        let mut unresolved_parent = expected_fluid_formation_signatures
            .values()
            .next()
            .unwrap()
            .clone();
        unresolved_parent.parent_formation_signature = "missing-parent".to_string();
        assert_eq!(
            library
                .insert_fluid_formations(vec![unresolved_parent], &catalog)
                .unwrap(),
            0,
            "fluid formations must reference a stored parent"
        );
        let mut invalid_area = expected_fluid_formation_signatures
            .values()
            .next()
            .unwrap()
            .clone();
        invalid_area.area = f64::NAN;
        assert_eq!(
            library
                .insert_fluid_formations(vec![invalid_area], &catalog)
                .unwrap(),
            0,
            "fluid formations must retain a finite conserved area"
        );

        let mut invalid_anchor_edge = expected_fluid_formation_signatures
            .values()
            .next()
            .unwrap()
            .clone();
        invalid_anchor_edge.anchor_edge = usize::MAX;
        assert_eq!(
            library
                .insert_fluid_formations(vec![invalid_anchor_edge], &catalog)
                .unwrap(),
            0,
            "fluid formations must reference a real rigid boundary edge"
        );
        let mut invalid_contact_angle = expected_fluid_formation_signatures
            .values()
            .next()
            .unwrap()
            .clone();
        invalid_contact_angle.contact_angle_radians = f64::NAN;
        assert_eq!(
            library
                .insert_fluid_formations(vec![invalid_contact_angle], &catalog)
                .unwrap(),
            0,
            "fluid formations must reject non-finite contact angles"
        );
        let interface = LiveGeometryInterface {
            interface_class: "fluid_boundary",
            signature: "test-fluid-boundary".to_string(),
            query: Some(LiveGeometryQuery::FluidBoundary {
                fluid_material: "Water".to_string(),
                solid_material: "Carbon".to_string(),
                solid_edge: 0,
                edge_parameter: quantize(0.5),
                fluid_area: quantize(area),
                contact_angle: quantize(equilibrium.contact_angle_radians),
            }),
        };
        assert_eq!(
            library.resolve_persistent_interface(&interface),
            LiveFamilyResolution::Unique
        );
        drop(library);

        // A corrupted persisted row must not poison valid records on reopen.
        // Exercise the loader's validation independently of the insertion API.
        let mut malformed_persisted = expected_fluid_formation_signatures
            .values()
            .next()
            .unwrap()
            .clone();
        malformed_persisted.anchor_edge = usize::MAX;
        let mut persisted_file = OpenOptions::new()
            .append(true)
            .open(root.join("fluid_formations.jsonl"))
            .unwrap();
        serde_json::to_writer(&mut persisted_file, &malformed_persisted).unwrap();
        persisted_file.write_all(b"\n").unwrap();
        persisted_file.sync_data().unwrap();
        drop(persisted_file);

        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert_eq!(
            reopened.resolve_persistent_interface(&interface),
            LiveFamilyResolution::Unique
        );

        // The persisted suggestion must remain directly realizable after the
        // library is closed and reopened; a successful interface lookup alone
        // is not enough to prove the constructor-facing geometry survived.
        let reopened_realized = reopened
            .fluid_formations_for_parent(&parent_signature, "Water")
            .find_map(|suggestion| suggestion.realize_against_parent(&formation, &catalog, true))
            .expect("reopened Bob fluid formation should reconstruct its exact shape and pose");
        assert!(reopened_realized.0.is_valid());
        assert!(reopened_realized.1.x.is_finite());
        assert!(reopened_realized.1.y.is_finite());
        assert!(reopened_realized.1.rotation_radians.is_finite());
        assert!(matches!(
            reopened_realized.0.form,
            Form::Fluid {
                nominal_area,
                boundary: None,
                arc_boundary: Some(_), ..} if (nominal_area - area).abs() < 1e-10
        ));

        let reopened_fluid_formations: BTreeMap<_, _> = reopened
            .fluid_formations()
            .map(|formation| (formation.signature(), formation.clone()))
            .collect();
        assert_eq!(
            reopened_fluid_formations,
            expected_fluid_formation_signatures
        );
        assert!(reopened_fluid_formations.values().all(|formation| {
            formation.fluid_resource == "Water"
                && formation.parent_formation_signature
                    == GeometryFormation::single("Carbon").signature
                && formation.area.is_finite()
                && formation.area > 0.0
        }));
        drop(reopened);
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod fluid_contact_constraint_schema_tests {
    use super::*;

    fn legacy_fluid_formation() -> GeometryFluidFormation {
        GeometryFluidFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            parent_formation_signature: "parent-signature".to_string(),
            fluid_resource: "Water".to_string(),
            anchor_constituent: 0,
            anchor_edge: 1,
            area: 1.0,
            contact_angle_radians: std::f64::consts::FRAC_PI_2,
            curvature_radius: 1.0,
            free_arc_angle_radians: std::f64::consts::PI,
            contact_length: 1.0,
            edge_parameter_start: 0.25,
            edge_parameter_end: 0.75,
            contact_constraints: Vec::new(),
            solved_boundary: None,
        }
    }

    #[test]
    fn legacy_fluid_records_default_to_no_extra_constraints() {
        let formation = legacy_fluid_formation();
        let mut value = serde_json::to_value(&formation).unwrap();
        let object = value.as_object_mut().unwrap();
        let _ = object.remove("contact_constraints");
        let _ = object.remove("solved_boundary");

        let decoded: GeometryFluidFormation = serde_json::from_value(value).unwrap();
        assert!(decoded.contact_constraints.is_empty());
        assert_eq!(decoded.signature(), formation.signature());
    }

    #[test]
    fn every_contact_constraint_contributes_to_fluid_signature() {
        let legacy = legacy_fluid_formation();
        let mut constrained = legacy.clone();
        constrained
            .contact_constraints
            .push(GeometryFluidContactConstraint {
                constituent_index: 1,
                edge_index: 2,
                contact_length: 0.5,
                edge_parameter_start: 0.2,
                edge_parameter_end: 0.8,
            });

        assert_ne!(legacy.signature(), constrained.signature());
    }

    #[test]
    fn unresolved_multi_contact_records_fail_closed() {
        let parent = GeometryFormation::single("Carbon");
        let mut constrained = legacy_fluid_formation();
        constrained.parent_formation_signature = parent.signature.clone();
        constrained
            .contact_constraints
            .push(GeometryFluidContactConstraint {
                constituent_index: 0,
                edge_index: 0,
                contact_length: 0.5,
                edge_parameter_start: 0.25,
                edge_parameter_end: 0.75,
            });

        assert!(constrained
            .realize_against_parent(&parent, &crate::resources::default_catalog(), true)
            .is_none());
    }

    #[test]
    fn misaligned_multi_contact_boundary_is_rejected_before_persistence() {
        use crate::capillary_geometry::{FluidBoundaryPiece, MultiContactFluidBoundary};
        use crate::resources::NOMINAL_UNIT_AREA;

        let catalog = crate::resources::default_catalog();
        let parent = GeometryFormation::single("Carbon");
        let half = NOMINAL_UNIT_AREA.sqrt() * 0.5;
        let angle = std::f64::consts::FRAC_PI_2;
        let boundary = MultiContactFluidBoundary {
            nominal_area: NOMINAL_UNIT_AREA,
            pieces: vec![
                FluidBoundaryPiece::SolidContact {
                    start_x: 100.0 - half,
                    start_y: 100.0 - half,
                    end_x: 100.0 + half,
                    end_y: 100.0 - half,
                    contact_angle_radians: angle,
                },
                FluidBoundaryPiece::SolidContact {
                    start_x: 100.0 + half,
                    start_y: 100.0 - half,
                    end_x: 100.0 + half,
                    end_y: 100.0 + half,
                    contact_angle_radians: angle,
                },
                FluidBoundaryPiece::SolidContact {
                    start_x: 100.0 + half,
                    start_y: 100.0 + half,
                    end_x: 100.0 - half,
                    end_y: 100.0 + half,
                    contact_angle_radians: angle,
                },
                FluidBoundaryPiece::SolidContact {
                    start_x: 100.0 - half,
                    start_y: 100.0 + half,
                    end_x: 100.0 - half,
                    end_y: 100.0 - half,
                    contact_angle_radians: angle,
                },
            ],
        };
        let carbon = catalog.iter().find(|item| item.name == "Carbon").unwrap();
        let edge_count = rigid_boundary_segments(&carbon.shape.form).len();
        assert!(edge_count >= 1);
        let mut formation = legacy_fluid_formation();
        formation.parent_formation_signature = parent.signature.clone();
        formation.area = NOMINAL_UNIT_AREA;
        formation.contact_length = NOMINAL_UNIT_AREA.sqrt();
        formation.contact_constraints = (0..4)
            .map(|edge| GeometryFluidContactConstraint {
                constituent_index: 0,
                edge_index: edge % edge_count,
                contact_length: NOMINAL_UNIT_AREA.sqrt(),
                edge_parameter_start: 0.0,
                edge_parameter_end: 1.0,
            })
            .collect();
        formation.solved_boundary = Some(boundary.clone());

        assert!(
            formation
                .realize_against_parent(&parent, &catalog, true)
                .is_none(),
            "boundary segments must match their referenced parent edges"
        );
        assert_ne!(formation.signature(), {
            let mut legacy = formation.clone();
            legacy.solved_boundary = None;
            legacy.signature()
        });

        let root = std::env::temp_dir().join(format!(
            "evosim-bob-multicontact-fluid-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
            library.insert(parent.clone(), &catalog).unwrap();
            assert_eq!(
                library
                    .insert_fluid_formations(vec![formation.clone()], &catalog)
                    .unwrap(),
                0,
                "Bob must not persist a boundary that misses its referenced rigid edges"
            );
        }
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert_eq!(
            reopened
                .fluid_formations_for_parent(&parent.signature, "Water")
                .count(),
            0
        );
        drop(reopened);
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn clipped_multi_contact_window_reconstructs_partial_edge_parameters() {
        let aligned: Vec<AlignedExposedInterval> = vec![
            (
                0.0,
                2.0,
                (0, 0, 0.0, 1.0, (0.0, 0.0), (2.0, 0.0)),
                (0.0, 0.0),
                (2.0, 0.0),
            ),
            (
                2.0,
                4.0,
                (1, 0, 0.0, 1.0, (2.0, 0.0), (4.0, 0.0)),
                (2.0, 0.0),
                (4.0, 0.0),
            ),
        ];
        let clipped = clip_aligned_contact_window(&aligned, 1.5, 1.0, (1.0, 0.0)).unwrap();
        assert_eq!(clipped.len(), 2);
        assert!((clipped[0].0 - 1.5).abs() < 1e-12);
        assert!((clipped[0].1 - 2.0).abs() < 1e-12);
        assert!((clipped[0].2 .2 - 0.75).abs() < 1e-12);
        assert!((clipped[0].2 .3 - 1.0).abs() < 1e-12);
        assert!((clipped[1].2 .2 - 0.0).abs() < 1e-12);
        assert!((clipped[1].2 .3 - 0.25).abs() < 1e-12);
        assert!((clipped[1].1 - clipped[0].0 - 1.0).abs() < 1e-12);
        assert!((clipped[0].4 .0 - clipped[1].3 .0).abs() < 1e-12);
    }

    #[test]
    fn simultaneous_collinear_solution_persists_and_reloads_three_parent_contacts() {
        use crate::resources::Form;

        let mut catalog = crate::resources::default_catalog();
        let water = catalog
            .iter()
            .find(|item| item.name == "Water")
            .unwrap()
            .clone();
        let carbon = catalog
            .iter()
            .find(|item| item.name == "Carbon")
            .unwrap()
            .clone();
        let area = match &water.shape.form {
            Form::Circle { radius } => std::f64::consts::PI * radius * radius,
            Form::Fluid { nominal_area, .. } => *nominal_area,
            _ => panic!("Water must expose a conserved fluid area"),
        };
        let family =
            solve_water_against_solid(area, water.properties.cohesion, carbon.properties.cohesion)
                .unwrap();
        let wall_length = family.contact_length * 0.45;
        for name in ["TestWallA", "TestWallB", "TestWallC"] {
            let mut wall = carbon.clone();
            wall.name = name.to_string();
            wall.shape.form = Form::Line {
                length: wall_length,
            };
            catalog.push(wall);
        }

        let mut raw_parent = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "TestWallA".to_string(),
                    placement: Placement {
                        x: -wall_length,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                GeometryConstituent {
                    resource: "TestWallB".to_string(),
                    placement: Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                GeometryConstituent {
                    resource: "TestWallC".to_string(),
                    placement: Placement {
                        x: wall_length,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
            ],
            bonds: vec![
                GeometryBond {
                    constituent_a: 0,
                    constituent_b: 1,
                },
                GeometryBond {
                    constituent_a: 1,
                    constituent_b: 2,
                },
            ],
            signature: String::new(),
        };
        raw_parent.signature = raw_parent.canonical_signature();
        assert!(validate_formation(&raw_parent, &catalog));
        let parent = raw_parent.canonicalized(&catalog).unwrap();
        let automatically_generated =
            generate_collinear_multi_contact_formations(&parent, &water, &catalog);
        assert!(
            !automatically_generated.is_empty(),
            "Bob should discover the three exposed collinear contacts without caller-supplied segments"
        );

        let record = automatically_generated
            .into_iter()
            .find(|candidate| candidate.contact_constraints.len() == 3)
            .expect("clipped exposed intervals should produce a three-surface contact record");
        assert_eq!(record.contact_constraints.len(), 3);
        assert_eq!(record.solved_boundary.as_ref().unwrap().contact_count(), 3);
        let parents = BTreeMap::from([(parent.signature.clone(), parent.clone())]);
        assert!(fluid_formation_is_valid(&record, &parents, &catalog));

        let root =
            std::env::temp_dir().join(format!("evosim-bob-collinear-fluid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
            library.insert(parent.clone(), &catalog).unwrap();
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
            assert_eq!(
                library
                    .insert_fluid_formations_with_multi_contact_counts(
                        Vec::new(),
                        vec![record.clone()],
                        &catalog,
                    )
                    .unwrap(),
                (0, 0)
            );
        }
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        let stored = reopened
            .fluid_formations_for_parent(&parent.signature, "Water")
            .collect::<Vec<_>>();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].signature(), record.signature());
        drop(reopened);
        let _ = std::fs::remove_dir_all(root);
    }
}
