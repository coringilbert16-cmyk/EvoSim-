//! Persistent geometry reference library.
//!
//! This module is deliberately separate from the live construction runtime.
//! The library is durable knowledge: tests use isolated temporary stores, while
//! the production catalogue lives outside `target/` and survives test runs and
//! process restarts.

use crate::capillary_geometry::{
    solve_water_against_solid, CapillaryContactFamily, ContactTranslationInterval,
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

pub const GEOMETRY_LIBRARY_SCHEMA_VERSION: u32 = 1;
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
    },
    RigidVertex {
        corner_material: String,
        corner_point: usize,
        edge_material: String,
        edge: usize,
        edge_parameter: i64,
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
            })
        }
        _ => None,
    };
    Some(LiveGeometryInterface {
        interface_class: class,
        signature: format!(
            "live-v{}|{}:{}|{}:{}",
            GEOMETRY_LIBRARY_SCHEMA_VERSION, sides[0].0, sides[0].1, sides[1].0, sides[1].1,
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

fn rigid_family_projection(family: &GeometryRigidContactFamily) -> String {
    format!(
        "edge|{}|{}|{}|{}|{}|{}|{}",
        family.candidate_resource,
        family.anchor_constituent,
        family.anchor_edge,
        family.candidate_edge,
        quantize(family.candidate_rotation_radians),
        quantize(family.anchor_parameter_start),
        quantize(family.anchor_parameter_end),
    )
}

fn point_family_projection(family: &GeometryRigidPointContactFamily) -> String {
    format!(
        "point|{}|{}|{}|{}|{}|{}|{}|{}",
        family.candidate_resource,
        family.anchor_constituent,
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
        "vertex|{}|{}|{}|{}|{}|{}|{}|{}",
        family.candidate_resource,
        family.anchor_constituent,
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
            })
        }
        _ => None,
    };
    LiveGeometryInterface {
        interface_class: class,
        signature: format!(
            "live-v{}|{}:{}|{}:{}",
            GEOMETRY_LIBRARY_SCHEMA_VERSION, sides[0].0, sides[0].1, sides[1].0, sides[1].1
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryRigidContactFamily {
    pub schema_version: u32,
    pub formation_signature: String,
    pub candidate_resource: String,
    pub anchor_constituent: usize,
    pub anchor_edge: usize,
    pub candidate_edge: usize,
    pub candidate_rotation_radians: f64,
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

    pub fn canonicalized(mut self, catalog: &[BaseResource]) -> Option<Self> {
        if !validate_formation(&self, catalog) {
            return None;
        }

        let candidates = canonical_pose_candidates(&self, catalog);
        let mut best: Option<Self> = None;
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

            // Reduce each constituent's local rotation by the exact proper
            // rotational symmetry of its physical shape. This removes only
            // rotations that leave the shape itself unchanged; reflections
            // remain distinct formations.
            for constituent in &mut candidate.constituents {
                if let Some(resource) = catalog.iter().find(|r| r.name == constituent.resource) {
                    constituent.placement.rotation_radians = canonical_shape_rotation(
                        &resource.shape.form,
                        constituent.placement.rotation_radians,
                    );
                }
            }

            let mut remap = vec![0usize; indexed.len()];
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

            // Canonicalization changes constituent order and normalizes
            // symmetry-equivalent rotations. Treat the resulting pose as a
            // new physical arrangement and validate it again before exposing
            // or persisting it; the pre-canonical pose's validity is not enough.
            if !validate_formation(&candidate, catalog) {
                continue;
            }
            candidate.signature = candidate.canonical_signature();

            if best
                .as_ref()
                .map(|existing| candidate.signature < existing.signature)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
        best
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
        let rigid_contact_family_path = root.join("rigid_contact_families.jsonl");
        let rigid_point_contact_family_path = root.join("rigid_point_contact_families.jsonl");
        let rigid_vertex_contact_family_path = root.join("rigid_vertex_contact_families.jsonl");

        let formation_storage_version = storage_format_version(&root, &data_path)?;
        let mut entries = if formation_storage_version == 3 {
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
                                    candidate_parameter,
                                    candidate_rotation,
                                    anchor_edge,
                                    anchor_parameter,
                                ) = if candidate_is_a {
                                    (
                                        a_material,
                                        *a_edge,
                                        *a_parameter as f64 / 1e9,
                                        *a_rotation as f64 / 1e9,
                                        *b_edge,
                                        *b_parameter as f64 / 1e9,
                                    )
                                } else {
                                    (
                                        b_material,
                                        *b_edge,
                                        *b_parameter as f64 / 1e9,
                                        *b_rotation as f64 / 1e9,
                                        *a_edge,
                                        *a_parameter as f64 / 1e9,
                                    )
                                };
                                let anchor_material = self
                                    .entries
                                    .get(&family.formation_signature)
                                    .and_then(|formation| {
                                        formation.constituents.get(family.anchor_constituent)
                                    })
                                    .map(|constituent| constituent.resource.as_str());
                                let realized_anchor_material = if candidate_is_a {
                                    b_material
                                } else {
                                    a_material
                                };
                                if candidate_material == &family.candidate_resource
                                    && realized_anchor_material
                                        == anchor_material.unwrap_or_default()
                                    && anchor_edge == family.anchor_edge
                                    && candidate_edge == family.candidate_edge
                                    && anchor_parameter >= family.anchor_parameter_start - QUANTUM
                                    && anchor_parameter <= family.anchor_parameter_end + QUANTUM
                                    && (normalize_angle(
                                        candidate_rotation
                                            - if candidate_is_a {
                                                *b_rotation as f64 / 1e9
                                            } else {
                                                *a_rotation as f64 / 1e9
                                            },
                                    ) - family.candidate_rotation_radians)
                                        .abs()
                                        <= 1e-7
                                {
                                    projections.insert(rigid_family_projection(family), ());
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
                            {
                                projections.insert(vertex_family_projection(family), ());
                            }
                        }
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
                || self.entries.get(&family.formation_signature).is_none()
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
                || self.entries.get(&family.formation_signature).is_none()
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
                || family.anchor_parameter_start > family.anchor_parameter_end
                || self.entries.get(&family.formation_signature).is_none()
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
        let formation_signature = formation_signature.into();
        let candidate_resource = candidate_resource.into();
        let key = format!("{formation_signature}|{candidate_resource}");
        self.frontier.records.insert(
            key,
            GeometryFrontierRecord {
                formation_signature,
                candidate_resource,
                state,
            },
        );
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

fn canonical_pose_candidates(
    formation: &GeometryFormation,
    catalog: &[BaseResource],
) -> Vec<GeometryFormation> {
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

/// A polygon edge expressed as the exact exposed parameter intervals that remain
/// available for a future contact. Parameter 0 is the first vertex and 1 is
/// the second vertex. Coincident boundary portions belonging to another
/// constituent are removed; point contacts do not remove an interval.
#[derive(Clone, Debug, PartialEq)]
pub struct ExposedEdgeInterval {
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
    if fluid_resource.physical_state != crate::resources::PhysicalState::Fluid {
        return Vec::new();
    }
    let area = match &fluid_resource.shape.form {
        Form::Circle { radius } => std::f64::consts::PI * radius * radius,
        Form::Fluid { nominal_area, .. } => *nominal_area,
        _ => return Vec::new(),
    };

    generate_water_contact_families(formation, fluid_resource, catalog)
        .into_iter()
        .map(|family| GeometryFluidBoundaryFamily {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            formation_signature: family.formation_signature,
            fluid_resource: family.candidate_resource,
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

pub fn generate_water_contact_families(
    formation: &GeometryFormation,
    candidate_resource: &BaseResource,
    catalog: &[BaseResource],
) -> Vec<GeometryContactFamily> {
    if candidate_resource.name != "Water"
        || candidate_resource.physical_state != crate::resources::PhysicalState::Fluid
    {
        return Vec::new();
    }
    let Some(water) = catalog.iter().find(|r| r.name == "Water") else {
        return Vec::new();
    };
    let water_area = match water.shape.form {
        Form::Circle { radius } => std::f64::consts::PI * radius * radius,
        Form::Fluid { nominal_area, .. } => nominal_area,
        _ => return Vec::new(),
    };

    let mut out = Vec::new();
    for (anchor_index, constituent) in formation.constituents.iter().enumerate() {
        let Some(resource) = catalog.iter().find(|r| r.name == constituent.resource) else {
            continue;
        };
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            continue;
        }
        let Some(family) = solve_water_against_solid(
            water_area,
            water.properties.cohesion,
            resource.properties.cohesion,
        ) else {
            continue;
        };

        match &resource.shape.form {
            Form::Line { length } => {
                let Some(contact) = ContactTranslationInterval::from_edge_length(*length, family)
                else {
                    continue;
                };
                for interval in exposed_line_intervals(formation, anchor_index, catalog) {
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
                    }
                }
            }
            _ => {
                let Some(vertices) = resource.shape.form.polygon_vertices() else {
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

                for interval in exposed_polygon_edge_intervals(formation, anchor_index, catalog) {
                    let Some(&edge_length) = edge_length_cache.get(&interval.edge) else {
                        continue;
                    };
                    let Some(contact) =
                        ContactTranslationInterval::from_edge_length(edge_length, family)
                    else {
                        continue;
                    };
                    let start = contact.edge_start_parameter.max(interval.start);
                    let end = contact.edge_end_parameter.min(interval.end);
                    if end + 1e-10 < start {
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
                }
            }
        }
    }
    out
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
                    let ratio = candidate_length / anchor_length;
                    let family = GeometryRigidContactFamily {
                        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                        formation_signature: formation.signature.clone(),
                        candidate_resource: candidate_resource.name.clone(),
                        anchor_constituent: anchor_index,
                        anchor_edge: interval.edge,
                        candidate_edge,
                        candidate_rotation_radians: rotation,
                        anchor_parameter_start: interval.start - ratio,
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
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-same-edge-{}",
            std::process::id()
        ));
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
        library.rigid_contact_families.insert(family_signature.clone(), family);
        library.rigid_contact_index
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
        assert_ne!(rigid_family_projection(&edge), rigid_family_projection(&changed_edge));

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
        assert_ne!(point_family_projection(&point), point_family_projection(&changed_point_end));
        let mut changed_point_rotation = point.clone();
        changed_point_rotation.candidate_rotation_end_radians = 1.0;
        assert_ne!(point_family_projection(&point), point_family_projection(&changed_point_rotation));

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
        assert_ne!(vertex_family_projection(&vertex), vertex_family_projection(&changed_vertex_end));
        let mut changed_vertex_rotation = vertex.clone();
        changed_vertex_rotation.candidate_rotation_start_radians = 0.0;
        assert_ne!(vertex_family_projection(&vertex), vertex_family_projection(&changed_vertex_rotation));
    }

    #[test]
    fn same_material_edge_lookup_is_endpoint_order_invariant() {
        let catalog = default_catalog();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-same-material-{}",
            std::process::id()
        ));
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
        library.rigid_contact_families.insert(family_signature.clone(), family);
        library.rigid_contact_index
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
        library.rigid_contact_families.insert(first_signature.clone(), first.clone());
        library.rigid_contact_index
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
        library.rigid_contact_families.insert(second_signature.clone(), second);
        library.rigid_contact_index
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
}
