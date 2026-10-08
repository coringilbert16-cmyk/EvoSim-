//! Persistent geometry reference library.
//!
//! This module is deliberately separate from the live construction runtime.
//! The library is durable knowledge: tests use isolated temporary stores, while
//! the production catalogue lives outside `target/` and survives test runs and
//! process restarts.

use crate::capillary_geometry::{solve_water_against_solid, CapillaryContactFamily, ContactTranslationInterval};
use crate::material_geometry::{placed_forms_penetrate, placed_forms_rigid_contact, PlacedMaterialPart};
use crate::resources::{default_catalog, BaseResource, Form};
use crate::structure::{ConnectionEndpoint, Placement};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const GEOMETRY_LIBRARY_SCHEMA_VERSION: u32 = 1;
/// Positional equivalence used by Bob when deciding whether two otherwise
/// identical geometric records describe the same meaningful contact.
/// Differences at or below this distance do not create a new record.
pub const GEOMETRY_EQUIVALENCE_TOLERANCE: f64 = 0.5;
const QUANTUM: f64 = 1e-9;

/// Canonical identity of a live physical interface.
///
/// This is deliberately derived from already-realized endpoint topology. It
/// does not search geometry space, sample orientations, or invent a persistent
/// family. A later family lookup may use this identity as its lookup boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveGeometryInterface {
    pub interface_class: &'static str,
    pub signature: String,
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

    let class = match (endpoint_class(candidate.endpoint_a), endpoint_class(candidate.endpoint_b)) {
        ("fluid", _) | (_, "fluid") => "fluid_boundary",
        ("boundary", "boundary") => "rigid_edge",
        ("corner", "boundary") | ("boundary", "corner") | ("corner", "corner") => "rigid_vertex",
        ("line", _) | (_, "line") => "rigid_point",
        _ => "rigid_surface",
    };

    let mut sides = [
        (material_a, local_a),
        (material_b, local_b),
    ];
    sides.sort_by(|a, b| a.cmp(b));

    Some(LiveGeometryInterface {
        interface_class: class,
        signature: format!(
            "live-v{}|{}:{}|{}:{}",
            GEOMETRY_LIBRARY_SCHEMA_VERSION,
            sides[0].0,
            sides[0].1,
            sides[1].0,
            sides[1].1,
        ),
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
        ConnectionEndpoint::Fluid { x, y } => Some(format!("fluid:{},{}", quantize(x), quantize(y))),
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
                    if len2 <= 1e-24 { return None; }
                    let t = ((point.x - a.0) * dx + (point.y - a.1) * dy) / len2;
                    if !(-1e-9..=1.000000001).contains(&t) { return None; }
                    let px = a.0 + dx * t.clamp(0.0, 1.0);
                    let py = a.1 + dy * t.clamp(0.0, 1.0);
                    if (px - point.x).hypot(py - point.y) <= 1e-8 {
                        Some(format!("edge:{}@{}@{}", i, quantize(t.clamp(0.0, 1.0)), quantize(normalized_angle(unit.placement.rotation_radians))))
                    } else { None }
                })
            });
            edge.or_else(|| Some(format!("boundary:{}", quantize(normalized_angle(angle_radians)))))
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
        ("corner", "boundary") | ("boundary", "corner")
        | ("corner", "corner") => "rigid_vertex",
        ("line", _) | (_, "line") => "rigid_point",
        _ => "rigid_surface",
    };

    let mut sides = [
        (material_a, endpoint_descriptor(endpoint_a)),
        (material_b, endpoint_descriptor(endpoint_b)),
    ];
    sides.sort_by(|a, b| a.cmp(b));

    LiveGeometryInterface {
        interface_class: class,
        signature: format!(
            "live-v{}|{}:{}|{}:{}",
            GEOMETRY_LIBRARY_SCHEMA_VERSION,
            sides[0].0,
            sides[0].1,
            sides[1].0,
            sides[1].1,
        ),
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
    if !(0.0 < family.contact_angle_radians
        && family.contact_angle_radians < std::f64::consts::PI)
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

    let Some(expected) =
        CapillaryContactFamily::solve(family.area, family.contact_angle_radians)
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
        format!("v{}|{}|{}|{}|{}|{}|{}|{}|{}", self.schema_version, self.formation_signature, self.candidate_resource, self.anchor_constituent, self.anchor_edge, quantize(self.contact_angle_radians), quantize(self.curvature_radius), quantize(self.edge_parameter_start), quantize(self.edge_parameter_end))
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
                    constituent.placement.rotation_radians =
                        canonical_shape_rotation(&resource.shape.form, constituent.placement.rotation_radians);
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
            candidate.bonds.sort_by_key(|b| (b.constituent_a, b.constituent_b));
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
    manifest: GeometryLibraryManifest,
    frontier: GeometryFrontier,
    contact_families: BTreeMap<String, GeometryContactFamily>,
    fluid_boundary_families: BTreeMap<String, GeometryFluidBoundaryFamily>,
    rigid_contact_families: BTreeMap<String, GeometryRigidContactFamily>,
    rigid_point_contact_families: BTreeMap<String, GeometryRigidPointContactFamily>,
    rigid_vertex_contact_families: BTreeMap<String, GeometryRigidVertexContactFamily>,
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

        let mut entries = BTreeMap::new();
        if data_path.exists() {
            let file = File::open(&data_path)?;
            let lines: Vec<String> = BufReader::new(file).lines().collect::<Result<_, _>>()?;
            for (line_index, line) in lines.iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let formation: GeometryFormation = match serde_json::from_str(line) {
                    Ok(value) => value,
                    Err(error) if line_index + 1 == lines.len() => {
                        // An interrupted final append can leave a truncated
                        // JSON record. Earlier durable records remain valid;
                        // ignore only the incomplete tail so restart can resume.
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
                if formation.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                    || !validate_formation(&formation, catalog)
                    || formation.signature != formation.canonical_signature()
                {
                    continue;
                }
                entries.insert(formation.signature.clone(), formation);
            }
        }

        let catalog_version = resource_catalog_signature(catalog);
        let manifest = if manifest_path.exists() {
            let bytes = fs::read(&manifest_path)?;
            serde_json::from_slice(&bytes).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?
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

        let mut contact_families = BTreeMap::new();
        if contact_family_path.exists() {
            let file = File::open(&contact_family_path)?;
            let lines: Vec<String> = BufReader::new(file).lines().collect::<Result<_, _>>()?;
            for (line_index, line) in lines.iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let family: GeometryContactFamily = match serde_json::from_str(line) {
                    Ok(value) => value,
                    Err(error) if line_index + 1 == lines.len() => {
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
                    || catalog.iter().all(|resource| resource.name != family.candidate_resource)
                    || family.anchor_constituent >= entries
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
            let lines: Vec<String> = BufReader::new(file).lines().collect::<Result<_, _>>()?;
            for (line_index, line) in lines.iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let family: GeometryRigidContactFamily = match serde_json::from_str(line) {
                    Ok(value) => value,
                    Err(error) if line_index + 1 == lines.len() => {
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
                    || catalog.iter().all(|resource| resource.name != family.candidate_resource)
                {
                    continue;
                }
                rigid_contact_families.insert(family.signature(), family);
            }
        }

        let frontier = if frontier_path.exists() {
            let bytes = fs::read(&frontier_path)?;
            serde_json::from_slice(&bytes).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?
        } else {
            GeometryFrontier::default()
        };

        let fluid_boundary_families =
            load_fluid_boundary_families(&fluid_boundary_family_path, &entries, catalog);
        let rigid_point_contact_families = load_rigid_point_contact_families(
            &rigid_point_contact_family_path,
            &entries,
            catalog,
        );
        let rigid_vertex_contact_families = load_rigid_vertex_contact_families(
            &rigid_vertex_contact_family_path,
            &entries,
            catalog,
        );

        let mut library = Self {
            root,
            entries,
            manifest,
            frontier,
            contact_families,
            fluid_boundary_families,
            rigid_contact_families,
            rigid_point_contact_families,
            rigid_vertex_contact_families,
        };
        library.manifest.entries = library.entries.len() as u64;
        library.write_manifest()?;
        Ok(library)
    }

    pub fn load_formations_only(root: impl AsRef<Path>, catalog: &[BaseResource]) -> std::io::Result<Vec<GeometryFormation>> {
        let path = root.as_ref().join("formations.jsonl");
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
                Err(_) => continue,
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

    pub fn rigid_point_contact_families(&self) -> impl Iterator<Item = &GeometryRigidPointContactFamily> {
        self.rigid_point_contact_families.values()
    }

    pub fn rigid_vertex_contact_families(&self) -> impl Iterator<Item = &GeometryRigidVertexContactFamily> {
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
        let mut projections = BTreeMap::<String, ()>::new();
        match interface.interface_class {
            "rigid_edge" => {
                if let Some((a, b)) = parse_edge_pair(&interface.signature) {
                    for family in self.rigid_contact_families.values() {
                        if edge_pair_matches_family(&a, &b, family) {
                            projections.insert(rigid_family_projection(family), ());
                        }
                    }
                }
            }
            "rigid_point" => {
                if let Some((a, b)) = parse_edge_pair(&interface.signature) {
                    for family in self.rigid_point_contact_families.values() {
                        if edge_point_pair_matches_family(&a, &b, family) {
                            projections.insert(point_family_projection(family), ());
                        }
                    }
                }
            }
            "rigid_vertex" => {
                if let Some((a, b)) = parse_edge_pair(&interface.signature) {
                    for family in self.rigid_vertex_contact_families.values() {
                        if edge_vertex_pair_matches_family(&a, &b, family) {
                            projections.insert(vertex_family_projection(family), ());
                        }
                    }
                }
            }
            _ => {}
        }
        (projections.len() == 1).then(|| projections.into_keys().next().unwrap())
    }

    pub fn resolve_persistent_interface(
        &self,
        interface: &LiveGeometryInterface,
    ) -> LiveFamilyResolution {
        let mut projections = BTreeMap::<String, ()>::new();
        match interface.interface_class {
            "rigid_edge" => {
                if let Some((a, b)) = parse_edge_pair(&interface.signature) {
                    for family in self.rigid_contact_families.values() {
                        if edge_pair_matches_family(&a, &b, family) {
                            projections.insert(rigid_family_projection(family), ());
                        }
                    }
                }
            }
            "rigid_point" => {
                if let Some((a, b)) = parse_edge_pair(&interface.signature) {
                    for family in self.rigid_point_contact_families.values() {
                        if edge_point_pair_matches_family(&a, &b, family) {
                            projections.insert(point_family_projection(family), ());
                        }
                    }
                }
            }
            "rigid_vertex" => {
                if let Some((a, b)) = parse_edge_pair(&interface.signature) {
                    for family in self.rigid_vertex_contact_families.values() {
                        if edge_vertex_pair_matches_family(&a, &b, family) {
                            projections.insert(vertex_family_projection(family), ());
                        }
                    }
                }
            }
            _ => {}
        }
        match projections.len() {
            0 => LiveFamilyResolution::Unresolved,
            1 => LiveFamilyResolution::Unique,
            _ => LiveFamilyResolution::Ambiguous,
        }
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
                || family.anchor_constituent >= self.entries.get(&family.formation_signature).map(|f| f.constituents.len()).unwrap_or(0)
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
            serde_json::to_writer(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
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
            { continue; }
            let signature = family.signature();
            if !self.rigid_point_contact_families.contains_key(&signature) {
                unique.insert(signature, family);
            }
        }
        if unique.is_empty() { return Ok(0); }
        let path = self.root.join("rigid_point_contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for family in unique.values() {
            serde_json::to_writer(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
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
            serde_json::to_writer(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
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
            serde_json::to_writer(&mut file, family)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;
        let added = unique.len();
        self.fluid_boundary_families.extend(unique);
        Ok(added)
    }

    pub fn insert_contact_family(&mut self, family: GeometryContactFamily) -> std::io::Result<bool> {
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION || !family.contact_angle_radians.is_finite() || !family.curvature_radius.is_finite() || !family.contact_length.is_finite() || !family.edge_parameter_start.is_finite() || !family.edge_parameter_end.is_finite() || family.edge_parameter_end < family.edge_parameter_start { return Ok(false); }
        let signature = family.signature();
        if self.contact_families.contains_key(&signature) { return Ok(false); }
        let path = self.root.join("contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, &family).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
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
            serde_json::to_writer(&mut file, family)
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
        for formation in formations {
            if let Some(canonical) = formation.canonicalized(catalog) {
                if self.entries.contains_key(&canonical.signature)
                    || self
                        .entries
                        .values()
                        .any(|existing| formations_equivalent_within_tolerance(existing, &canonical))
                    || unique
                        .values()
                        .any(|existing| formations_equivalent_within_tolerance(existing, &canonical))
                {
                    continue;
                }
                unique.insert(canonical.signature.clone(), canonical);
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
            serde_json::to_writer(&mut file, formation)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            file.write_all(b"\n")?;
        }
        file.sync_data()?;

        let added = unique.len();
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

fn canonical_pose_candidates(formation: &GeometryFormation, catalog: &[BaseResource]) -> Vec<GeometryFormation> {
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

fn formations_equivalent_within_tolerance(
    a: &GeometryFormation,
    b: &GeometryFormation,
) -> bool {
    if a.schema_version != b.schema_version
        || a.constituents.len() != b.constituents.len()
        || a.bonds != b.bonds
    {
        return false;
    }

    a.constituents.iter().zip(&b.constituents).all(|(left, right)| {
        left.resource == right.resource
            && angular_difference(
                left.placement.rotation_radians,
                right.placement.rotation_radians,
            ) <= QUANTUM
            && (left.placement.x - right.placement.x).hypot(
                left.placement.y - right.placement.y,
            ) <= GEOMETRY_EQUIVALENCE_TOLERANCE
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
    (angle + std::f64::consts::PI)
        .rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI
}

fn canonical_shape_rotation(form: &Form, angle: f64) -> f64 {
    let angle = normalized_angle(angle);
    let period = match form {
        Form::Circle { .. } => return 0.0,
        Form::Rectangle { .. } => std::f64::consts::PI,
        Form::RegularPolygon { sides, .. } => std::f64::consts::TAU / (*sides as f64),
        Form::Polygon { vertices } => rotational_symmetry_period(vertices),
        Form::Line { .. } => std::f64::consts::PI,
        Form::Fluid { boundary, .. } => boundary.as_deref()
            .map(rotational_symmetry_period)
            .unwrap_or(std::f64::consts::TAU),
    };
    if period >= std::f64::consts::TAU - 1e-12 { return angle; }
    normalized_angle((angle / period).round() * period)
}

fn rotational_symmetry_period(vertices: &[(f64, f64)]) -> f64 {
    if vertices.len() < 3 { return std::f64::consts::TAU; }
    let n = vertices.len() as f64;
    let center = vertices.iter().fold((0.0, 0.0), |(x, y), (vx, vy)| (x + vx, y + vy));
    let center = (center.0 / n, center.1 / n);
    let points: Vec<(f64, f64)> = vertices.iter().map(|(x, y)| (x - center.0, y - center.1)).collect();
    let first = points[0];
    if first.0.hypot(first.1) <= 1e-12 { return std::f64::consts::TAU; }
    let first_angle = first.1.atan2(first.0);
    let tol = 1e-9;
    for shift in 1..vertices.len() {
        let target = points[shift];
        let rotation = normalized_angle(target.1.atan2(target.0) - first_angle);
        let (sin, cos) = rotation.sin_cos();
        if points.iter().all(|p| {
            let rotated = (p.0 * cos - p.1 * sin, p.0 * sin + p.1 * cos);
            points.iter().any(|q| (rotated.0 - q.0).hypot(rotated.1 - q.1) <= tol)
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
        || formation.constituents.len() > 20
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
        let resource = catalog.iter().find(|r| r.name == constituent.resource).unwrap();
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
    let Some(anchor_resource) = catalog.iter().find(|r| r.name == formation.constituents[anchor_index].resource) else {
        return Vec::new();
    };
    let Some(anchor_vertices) = anchor_resource.shape.form.polygon_vertices() else {
        return Vec::new();
    };
    let anchor_placement = formation.constituents[anchor_index].placement;
    let mut out = Vec::new();

    for edge in 0..anchor_vertices.len() {
        let a = world_point(anchor_vertices[edge], anchor_placement);
        let b = world_point(anchor_vertices[(edge + 1) % anchor_vertices.len()], anchor_placement);
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
            let Some(resource) = catalog.iter().find(|r| r.name == other.resource) else { continue; };
            if resource.physical_state == crate::resources::PhysicalState::Fluid {
                continue;
            }
            let Some(vertices) = resource.shape.form.polygon_vertices() else { continue; };
            let p = other.placement;
            for other_edge in 0..vertices.len() {
                let c = world_point(vertices[other_edge], p);
                let d = world_point(vertices[(other_edge + 1) % vertices.len()], p);
                let ex = d.0 - c.0;
                let ey = d.1 - c.1;
                let cross = dx * (c.1 - a.1) - dy * (c.0 - a.0);
                let cross_end = dx * (d.1 - a.1) - dy * (d.0 - a.0);
                if cross.abs() > 1e-9 * length_sq.sqrt() || cross_end.abs() > 1e-9 * length_sq.sqrt() {
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
                out.push(ExposedEdgeInterval { edge, start: cursor, end: start.min(1.0) });
            }
            cursor = cursor.max(end);
            if cursor >= 1.0 - 1e-10 {
                break;
            }
        }
        if cursor < 1.0 - 1e-10 {
            out.push(ExposedEdgeInterval { edge, start: cursor, end: 1.0 });
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
) -> BTreeMap<String, GeometryFluidBoundaryFamily> {
    let mut out = BTreeMap::new();
    let Ok(file) = File::open(path) else { return out; };
    let lines = BufReader::new(file).lines().collect::<Result<Vec<_>, _>>().unwrap_or_default();
    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() { continue; }
        let Ok(family) = serde_json::from_str::<GeometryFluidBoundaryFamily>(line) else {
            if index + 1 == lines.len() { continue; }
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
            || family.anchor_constituent >= entries.get(&family.formation_signature).map(|f| f.constituents.len()).unwrap_or(0)
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
            free_arc_angle_radians: 2.0
                * (std::f64::consts::PI - family.contact_angle_radians),
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
            Form::Line { length: other_length } => {
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
                    let t0 = ((c.0 - anchor_start.0) * dx + (c.1 - anchor_start.1) * dy) / length_sq;
                    let t1 = ((d.0 - anchor_start.0) * dx + (d.1 - anchor_start.1) * dy) / length_sq;
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
            out.push(ExposedEdgeInterval { edge: 0, start: cursor, end: start.min(1.0) });
        }
        cursor = cursor.max(end);
        if cursor >= 1.0 - 1e-10 {
            return out;
        }
    }
    if cursor < 1.0 - 1e-10 {
        out.push(ExposedEdgeInterval { edge: 0, start: cursor, end: 1.0 });
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
    let anchor_resource = catalog
        .iter()
        .find(|resource| resource.name == formation.constituents[family.anchor_constituent].resource)?;
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
) -> BTreeMap<String, GeometryRigidPointContactFamily> {
    let mut out = BTreeMap::new();
    let Ok(file) = File::open(path) else { return out; };
    let lines = BufReader::new(file).lines().collect::<Result<Vec<_>, _>>().unwrap_or_default();
    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() { continue; }
        let Ok(family) = serde_json::from_str::<GeometryRigidPointContactFamily>(line) else {
            if index + 1 == lines.len() { continue; }
            continue;
        };
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
            || !family.anchor_parameter_start.is_finite()
            || !family.anchor_parameter_end.is_finite()
            || !family.candidate_rotation_start_radians.is_finite()
            || !family.candidate_rotation_end_radians.is_finite()
            || family.anchor_parameter_start > family.anchor_parameter_end
            || !entries.contains_key(&family.formation_signature)
            || family.anchor_constituent >= entries.get(&family.formation_signature).map(|f| f.constituents.len()).unwrap_or(0)
            || family.candidate_endpoint > 1
            || catalog.iter().all(|r| r.name != family.candidate_resource)
        { continue; }
        out.insert(family.signature(), family);
    }
    out
}

fn load_rigid_vertex_contact_families(
    path: &Path,
    entries: &BTreeMap<String, GeometryFormation>,
    catalog: &[BaseResource],
) -> BTreeMap<String, GeometryRigidVertexContactFamily> {
    let mut out = BTreeMap::new();
    let Ok(file) = File::open(path) else { return out; };
    let lines = BufReader::new(file).lines().collect::<Result<Vec<_>, _>>().unwrap_or_default();
    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() { continue; }
        let Ok(family) = serde_json::from_str::<GeometryRigidVertexContactFamily>(line) else {
            if index + 1 == lines.len() { continue; }
            continue;
        };
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
            || !family.anchor_parameter_start.is_finite()
            || !family.anchor_parameter_end.is_finite()
            || !family.candidate_rotation_start_radians.is_finite()
            || !family.candidate_rotation_end_radians.is_finite()
            || family.anchor_parameter_start > family.anchor_parameter_end
            || !entries.contains_key(&family.formation_signature)
            || family.anchor_constituent >= entries.get(&family.formation_signature).map(|f| f.constituents.len()).unwrap_or(0)
            || catalog.iter().all(|r| r.name != family.candidate_resource)
        { continue; }
        let Some(anchor_resource) = catalog.iter().find(|r| r.name == entries[&family.formation_signature].constituents[family.anchor_constituent].resource) else { continue; };
        let Some(anchor_vertices) = anchor_resource.shape.form.polygon_vertices() else { continue; };
        let Some(candidate_resource) = catalog.iter().find(|r| r.name == family.candidate_resource) else { continue; };
        let Some(candidate_vertices) = candidate_resource.shape.form.polygon_vertices() else { continue; };
        if family.anchor_edge >= anchor_vertices.len() || family.candidate_vertex >= candidate_vertices.len() {
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
        let Some(anchor_resource) = catalog.iter().find(|r| r.name == formation.constituents[anchor_index].resource) else { continue; };

        // A zero-thickness line can accept a candidate line endpoint at any
        // exposed point along its segment. Unlike a polygon edge, there is no
        // positive-area interior to avoid, so the candidate endpoint has the
        // full exact 2*pi orientation family.
        if matches!(anchor_resource.shape.form, Form::Line { .. }) {
            let exposed = exposed_line_intervals(formation, anchor_index, catalog);
            for interval in exposed {
                for endpoint in 0..2 {
                    let family = GeometryRigidPointContactFamily {
                        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
                        formation_signature: formation.signature.clone(),
                        candidate_resource: candidate_resource.name.clone(),
                        anchor_constituent: anchor_index,
                        anchor_edge: interval.edge,
                        candidate_endpoint: endpoint,
                        anchor_parameter_start: interval.start,
                        anchor_parameter_end: interval.end,
                        candidate_rotation_start_radians: 0.0,
                        candidate_rotation_end_radians: std::f64::consts::TAU,
                    };
                    unique.insert(family.signature(), family);
                }
            }
            continue;
        }

        let Some(vertices) = anchor_resource.shape.form.polygon_vertices() else { continue; };
        let exposed = exposed_polygon_edge_intervals(formation, anchor_index, catalog);
        let placement = formation.constituents[anchor_index].placement;
        let signed_area = vertices.iter().enumerate().map(|(i, &(x0,y0))| {
            let (x1,y1) = vertices[(i+1)%vertices.len()];
            x0*y1-y0*x1
        }).sum::<f64>();

        for interval in exposed {
            let Some(&a0) = vertices.get(interval.edge) else { continue; };
            let a1 = vertices[(interval.edge + 1) % vertices.len()];
            let dx = a1.0-a0.0;
            let dy = a1.1-a0.1;
            let length = dx.hypot(dy);
            if length <= QUANTUM { continue; }

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
                let local_interior_angle = if endpoint == 0 { 0.0 } else { std::f64::consts::PI };
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

#[cfg(test)]
mod live_interface_tests {
    use super::*;

    #[test]
    fn live_family_resolution_never_guesses_from_topology_alone() {
        let interface = resolve_live_contact_interface(
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            "Hydrogen",
            ConnectionEndpoint::LineEndpoint { point_index: 0 },
        );
        assert_eq!(
            classify_live_family_resolution(&interface),
            LiveFamilyResolution::Unresolved
        );
    }

    #[test]
    fn live_interface_is_independent_of_endpoint_order() {
        let a = resolve_live_contact_interface(
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            "Hydrogen",
            ConnectionEndpoint::LineEndpoint { point_index: 1 },
        );
        let b = resolve_live_contact_interface(
            "Hydrogen",
            ConnectionEndpoint::LineEndpoint { point_index: 1 },
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
        );
        assert_eq!(a, b);
        assert_eq!(a.interface_class, "rigid_point");
    }

    #[test]
    fn live_interface_distinguishes_endpoint_topology() {
        let edge = resolve_live_contact_interface(
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: std::f64::consts::PI },
        );
        let vertex = resolve_live_contact_interface(
            "Carbon",
            ConnectionEndpoint::Corner { point_index: 0 },
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
        );
        assert_eq!(edge.interface_class, "rigid_edge");
        assert_eq!(vertex.interface_class, "rigid_vertex");
        assert_ne!(edge.signature, vertex.signature);
    }

    #[test]
    fn live_interface_does_not_search_or_generate_geometry() {
        let interface = resolve_live_contact_interface(
            "Carbon",
            ConnectionEndpoint::Corner { point_index: 2 },
            "Phosphorus",
            ConnectionEndpoint::Corner { point_index: 1 },
        );
        assert!(interface.signature.contains("corner:2"));
        assert!(interface.signature.contains("corner:1"));
    }
}

#[cfg(test)]
mod point_contact_family_tests {
    use super::*;
    use crate::resources::Shape;

    #[test]
    fn line_anchor_exposes_endpoint_to_interior_contact_family() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Hydrogen");
        let line = catalog.iter().find(|r| r.name == "Hydrogen").unwrap();
        assert!(matches!(line.shape.form, Form::Rectangle { .. }));

        let mut test_catalog = catalog.clone();
        test_catalog.push(BaseResource {
            name: "TestLine".to_string(),
            physical_state: crate::resources::PhysicalState::Rigid,
            shape: Shape { form: Form::Line { length: 2.0 } },
            properties: line.properties.clone(),
        });

        let line_formation = GeometryFormation::single("TestLine");
        let candidate = test_catalog.iter().find(|r| r.name == "TestLine").unwrap();
        let families = generate_rigid_point_contact_families(&line_formation, candidate, &test_catalog);
        assert!(families.iter().any(|family| {
            family.candidate_resource == "TestLine"
                && family.anchor_parameter_start == 0.0
                && family.anchor_parameter_end == 1.0
                && family.candidate_rotation_start_radians == 0.0
                && (family.candidate_rotation_end_radians - std::f64::consts::TAU).abs() < 1e-12
        }));
    }
}

/// Record the continuous manifold where a rigid polygon vertex touches an
/// exposed polygon edge. The contact point may translate over the exposed
/// edge; the candidate polygon may rotate while its selected vertex remains
/// on the supporting line and its interior stays in the outward half-plane.
/// No angle or position sampling is used.
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
        let Some(anchor_resource) = catalog.iter().find(|r| r.name == formation.constituents[anchor_index].resource) else { continue; };
        let Some(anchor_vertices) = anchor_resource.shape.form.polygon_vertices() else { continue; };
        let exposed = exposed_polygon_edge_intervals(formation, anchor_index, catalog);
        let placement = formation.constituents[anchor_index].placement;
        let signed_area = anchor_vertices.iter().enumerate().map(|(i, &(x0,y0))| {
            let (x1,y1) = anchor_vertices[(i+1)%anchor_vertices.len()];
            x0*y1-y0*x1
        }).sum::<f64>();

        for interval in exposed {
            let a0 = anchor_vertices[interval.edge];
            let a1 = anchor_vertices[(interval.edge + 1) % anchor_vertices.len()];
            let dx = a1.0-a0.0;
            let dy = a1.1-a0.1;
            let length = dx.hypot(dy);
            if length <= QUANTUM { continue; }

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
                let Some((start, end)) =
                    exact_vertex_rotation_interval(&candidate_vertices, candidate_vertex, outward_angle)
                else {
                    continue;
                };
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
        let Some(anchor_resource) = catalog.iter().find(|r| r.name == formation.constituents[anchor_index].resource) else { continue; };
        let anchor_segments = rigid_boundary_segments(&anchor_resource.shape.form);
        let exposed = if matches!(anchor_resource.shape.form, Form::Line { .. }) {
            exposed_line_intervals(formation, anchor_index, catalog)
        } else {
            exposed_polygon_edge_intervals(formation, anchor_index, catalog)
        };
        let candidate_segments = rigid_boundary_segments(&candidate_resource.shape.form);
        for interval in exposed {
            let Some(&(a0, a1)) = anchor_segments.get(interval.edge) else { continue; };
            let anchor_length = (a1.0-a0.0).hypot(a1.1-a0.1);
            if anchor_length <= QUANTUM { continue; }
            let anchor = formation.constituents[anchor_index].placement;
            let world_a0 = world_point(a0, anchor);
            let world_a1 = world_point(a1, anchor);
            let anchor_angle = (world_a1.1-world_a0.1).atan2(world_a1.0-world_a0.0);
            for (candidate_edge, &(c0,c1)) in candidate_segments.iter().enumerate() {
                let candidate_length = (c1.0-c0.0).hypot(c1.1-c0.1);
                if candidate_length <= QUANTUM { continue; }
                let candidate_angle = (c1.1-c0.1).atan2(c1.0-c0.0);
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
    if target.constituents.len() != 1 { return Vec::new(); }
    let Some(target_resource) = catalog.iter().find(|r| r.name == target.constituents[0].resource) else {
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
                    &candidate_resource.shape, ci, &target_resource.shape, ti,
                    target_placement.rotation_radians,
                ) {
                    let (tx, ty) = crate::rigid_boundary::world_vertex(
                        &target_resource.shape, ti, target_placement).unwrap();
                    let (cx, cy) = rotated_point(cv[ci], rotation);
                    push_candidate(&mut out, target, candidate_resource, Placement {
                        x: tx - cx, y: ty - cy, rotation_radians: rotation,
                    }, catalog);
                }
            }
        }

        // Exact flat-to-flat contacts: align boundary edges and coincide one
        // endpoint. No angular sampling is used.
        for te in 0..tv.len() {
            let tn = (te + 1) % tv.len();
            for ce in 0..cv.len() {
                let cn = (ce + 1) % cv.len();
                let Some(ta) = edge_angle_world(tv[te], tv[tn], target_placement.rotation_radians) else { continue };
                let Some(ca) = edge_angle(cv[ce], cv[cn]) else { continue };
                for flip in [0.0, std::f64::consts::PI] {
                    let rotation = normalize_angle(ta + flip - ca);
                    let (tx, ty) = world_point(tv[te], target_placement);
                    let (cx, cy) = rotated_point(cv[ce], rotation);
                    push_candidate(&mut out, target, candidate_resource, Placement {
                        x: tx - cx, y: ty - cy, rotation_radians: rotation,
                    }, catalog);
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
                        endpoint, 0, target_placement.rotation_radians,
                    ) {
                        let (tx, ty) = world_point(tv[ti], target_placement);
                        let (cx, cy) = rotated_point(endpoints[endpoint], rotation);
                        push_candidate(&mut out, target, candidate_resource, Placement {
                            x: tx - cx, y: ty - cy, rotation_radians: rotation,
                        }, catalog);
                    }
                }
            }
        }
    }

    if let (Form::Line { length: target_length }, Form::Line { length: candidate_length }) =
        (&target_resource.shape.form, &candidate_resource.shape.form)
    {
        let target_half = *target_length / 2.0;
        let candidate_half = *candidate_length / 2.0;
        let target_endpoints = [(-target_half, 0.0), (target_half, 0.0)];
        let candidate_endpoints = [(-candidate_half, 0.0), (candidate_half, 0.0)];
        let target_angle = target_placement.rotation_radians;

        // Exact endpoint-to-endpoint line contacts. This supplies finite
        // representatives for the rigid line-line contact space; the
        // continuous contact family remains represented by the library's
        // rigid-contact records rather than angular sampling.
        for target_endpoint in target_endpoints {
            let (tx, ty) = world_point(target_endpoint, target_placement);
            for candidate_endpoint in candidate_endpoints {
                for flip in [0.0, std::f64::consts::PI] {
                    let rotation = normalize_angle(target_angle + flip);
                    let (cx, cy) = rotated_point(candidate_endpoint, rotation);
                    push_candidate(&mut out, target, candidate_resource, Placement {
                        x: tx - cx,
                        y: ty - cy,
                        rotation_radians: rotation,
                    }, catalog);
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
    if two_constituent.constituents.len() != 2 { return Vec::new(); }
    let mut out = Vec::new();
    for anchor_index in 0..2 {
        let anchor = &two_constituent.constituents[anchor_index];
        let anchor_formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![anchor.clone()],
            bonds: Vec::new(),
            signature: String::new(),
        };
        for pair in generate_two_constituent_candidates(&anchor_formation, candidate_resource, catalog) {
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
            if validate_formation(&formation, catalog) { out.push(formation); }
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
            GeometryConstituent { resource: resource.name.clone(), placement },
        ],
        bonds: vec![GeometryBond { constituent_a: 0, constituent_b: 1 }],
        signature: String::new(),
    };
    if validate_formation(&formation, catalog) { out.push(formation); }
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
    if dx.hypot(dy) <= f64::EPSILON { None } else { Some(dy.atan2(dx)) }
}

fn edge_angle_world(a: (f64, f64), b: (f64, f64), rotation: f64) -> Option<f64> {
    Some(normalize_angle(edge_angle(a, b)? + rotation))
}

fn exact_vertex_rotation_interval(
    vertices: &[(f64, f64)],
    vertex_index: usize,
    outward_angle: f64,
) -> Option<(f64, f64)> {
    if vertices.len() < 3 || vertex_index >= vertices.len() {
        return None;
    }

    // A selected candidate vertex is placed on the anchor supporting line.
    // The candidate must remain entirely in the anchor's outward half-plane
    // (touching the line is allowed). For every vector v from the selected
    // vertex to another candidate vertex, this is exactly:
    //
    //     dot(R(theta) v, outward_normal) <= 0
    //
    // Each constraint is therefore one exact semicircle of admissible
    // rotations. We intersect those semicircles on one unwrapped 2*pi sheet.
    // No angular sampling is involved.
    let selected = vertices[vertex_index];
    let mut centers = Vec::with_capacity(vertices.len() - 1);

    for (index, point) in vertices.iter().enumerate() {
        if index == vertex_index {
            continue;
        }
        let dx = point.0 - selected.0;
        let dy = point.1 - selected.1;
        if dx.hypot(dy) <= QUANTUM {
            return None;
        }

        // dot <= 0 means the rotated vector is at least 90 degrees away
        // from the outward normal. The center of that admissible semicircle
        // is pi past the vector direction.
        centers.push(outward_angle - dy.atan2(dx) + std::f64::consts::PI);
    }

    let reference = centers[0];
    let mut lo = reference - std::f64::consts::FRAC_PI_2;
    let mut hi = reference + std::f64::consts::FRAC_PI_2;

    for center in centers.into_iter().skip(1) {
        // Choose the equivalent copy whose center is nearest the current
        // interval. This unwraps all constraints onto the same real line.
        let shift = ((reference - center) / std::f64::consts::TAU).round();
        let center = center + shift * std::f64::consts::TAU;
        let next_lo = (center - std::f64::consts::FRAC_PI_2).max(lo);
        let next_hi = (center + std::f64::consts::FRAC_PI_2).min(hi);
        if next_hi - next_lo <= QUANTUM {
            return None;
        }
        lo = next_lo;
        hi = next_hi;
    }

    (hi - lo > QUANTUM).then_some((lo, hi))
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
    if formation.constituents.is_empty() || formation.constituents.len() >= 20 {
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

        for pair in generate_two_constituent_candidates(
            &anchor_formation,
            candidate_resource,
            catalog,
        ) {
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
            candidates.extend(generate_two_constituent_candidates(&target, resource, catalog));
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
            candidates.extend(expand_three_constituent_candidates(&formation, resource, catalog));
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

pub fn open_default_library() -> std::io::Result<GeometryLibrary> {
    let catalog = default_catalog();
    GeometryLibrary::open("geometry_library/data", &catalog)
}

#[test]
fn canonicalization_collapses_intrinsic_rotation() {
    let catalog = default_catalog();
    let base = GeometryFormation::single("Carbon");
    let mut rotated = base.clone();
    rotated.constituents[0].placement.rotation_radians = std::f64::consts::PI / 3.0;
    assert_eq!(base.canonicalized(&catalog).unwrap().signature,
               rotated.canonicalized(&catalog).unwrap().signature);

    let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
    let vertex = carbon.shape.form.polygon_vertices().unwrap()[0];
    let mut pair = GeometryFormation {
        schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
        constituents: vec![
            GeometryConstituent { resource: "Carbon".into(), placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 } },
            GeometryConstituent { resource: "Carbon".into(), placement: Placement { x: vertex.0 * 2.0, y: vertex.1 * 2.0, rotation_radians: 0.0 } },
        ],
        bonds: vec![GeometryBond { constituent_a: 0, constituent_b: 1 }],
        signature: String::new(),
    };
    if !validate_formation(&pair, &catalog) {
        pair = generate_two_constituent_candidates(&base, carbon, &catalog).into_iter().next().unwrap();
    }
    let mut rotated_pair = pair.clone();
    rotated_pair.constituents[1].placement.rotation_radians += std::f64::consts::PI / 3.0;
    assert_eq!(pair.canonicalized(&catalog).unwrap().signature,
               rotated_pair.canonicalized(&catalog).unwrap().signature);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn catalog_with_test_line() -> Vec<BaseResource> {
        let mut catalog = default_catalog();
        catalog.push(BaseResource {
            name: "TestLine".into(),
            properties: crate::resources::ResourceProperties {
                mass: 1.0,
                potential_energy: 1.0,
                reactivity: 0.0,
                chemical_position: None,
                cohesion: 0.5,
            },
            physical_state: crate::resources::PhysicalState::Rigid,
            shape: crate::resources::Shape {
                form: Form::Line { length: 1.0 },
            },
        });
        catalog
    }

    fn temp_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("evosim-geometry-library-{nonce}"))
    }


    #[test]
    fn bob_equivalence_collapses_sub_half_unit_face_to_face_variation() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let base = GeometryFormation::single("Carbon");
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let pair = generate_two_constituent_candidates(&base, carbon, &catalog)
            .into_iter()
            .find(|formation| exposed_polygon_edge_intervals(formation, 0, &catalog).len() == 5)
            .expect("face-to-face carbon pair");

        assert!(validate_formation(&pair, &catalog));
        assert!(library.insert(pair.clone(), &catalog).unwrap());

        let mut near_duplicate = pair.clone();
        near_duplicate.constituents[1].placement.x += 0.01;
        let canonical = near_duplicate.canonicalized(&catalog).unwrap();
        assert!(formations_equivalent_within_tolerance(
            library.get(&pair.canonicalized(&catalog).unwrap().signature).unwrap(),
            &canonical,
        ));
        assert!(!library.insert(near_duplicate, &catalog).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bob_equivalence_includes_the_half_unit_boundary() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let base = GeometryFormation::single("Carbon");
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let pair = generate_two_constituent_candidates(&base, carbon, &catalog)
            .into_iter()
            .find(|formation| exposed_polygon_edge_intervals(formation, 0, &catalog).len() == 5)
            .expect("face-to-face carbon pair");
        assert!(library.insert(pair.clone(), &catalog).unwrap());

        let mut boundary = pair.clone();
        boundary.constituents[1].placement.x += GEOMETRY_EQUIVALENCE_TOLERANCE;
        assert!(!library.insert(boundary, &catalog).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn isolated_rigid_boundary_is_fully_exposed() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Carbon");
        let intervals = exposed_polygon_edge_intervals(&formation, 0, &catalog);
        assert_eq!(intervals.len(), 6);
        assert!(intervals.iter().all(|interval| {
            (interval.start - 0.0).abs() <= 1e-12 && (interval.end - 1.0).abs() <= 1e-12
        }));
    }

    #[test]
    fn face_to_face_bond_removes_the_shared_edge_from_exposure() {
        let catalog = default_catalog();
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let base = GeometryFormation::single("Carbon");
        let candidates = generate_two_constituent_candidates(&base, carbon, &catalog);
        let formation = candidates
            .into_iter()
            .find(|formation| {
                let intervals = exposed_polygon_edge_intervals(formation, 0, &catalog);
                intervals.len() == 5
            })
            .expect("exact face-to-face carbon contact should expose five of six edges");
        let intervals = exposed_polygon_edge_intervals(&formation, 0, &catalog);
        assert_eq!(intervals.len(), 5);
        assert!(intervals.iter().all(|interval| interval.end > interval.start));
    }

    #[test]
    fn line_line_candidates_include_endpoint_contact() {
        let catalog = catalog_with_test_line();
        let line = catalog.iter().find(|r| r.name == "TestLine").unwrap();
        let base = GeometryFormation::single("TestLine");
        let candidates = generate_two_constituent_candidates(&base, line, &catalog);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|formation| {
            formation.constituents.len() == 2
                && formation.bonds.len() == 1
                && validate_formation(formation, &catalog)
        }));
    }

    #[test]
    fn water_capillary_family_supports_rigid_line_boundaries() {
        let catalog = catalog_with_test_line();
        let formation = GeometryFormation::single("TestLine");
        let water = catalog.iter().find(|r| r.name == "Water").unwrap();
        let families = generate_water_contact_families(&formation, water, &catalog);
        assert!(!families.is_empty());
        assert!(families.iter().all(|family| {
            family.anchor_edge == 0
                && family.edge_parameter_end >= family.edge_parameter_start
                && family.edge_parameter_start >= 0.0
                && family.edge_parameter_end <= 1.0
        }));
    }

    #[test]
    fn exposed_line_boundary_can_be_fully_occluded() {
        let catalog = catalog_with_test_line();
        let formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "TestLine".into(),
                    placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                },
                GeometryConstituent {
                    resource: "TestLine".into(),
                    placement: Placement { x: -0.25, y: 0.0, rotation_radians: 0.0 },
                },
                GeometryConstituent {
                    resource: "TestLine".into(),
                    placement: Placement { x: 0.25, y: 0.0, rotation_radians: 0.0 },
                },
            ],
            bonds: vec![
                GeometryBond { constituent_a: 0, constituent_b: 1 },
                GeometryBond { constituent_a: 0, constituent_b: 2 },
            ],
            signature: String::new(),
        };
        assert!(validate_formation(&formation, &catalog));
        let intervals = exposed_line_intervals(&formation, 0, &catalog);
        assert_eq!(intervals.len(), 0);
    }

    #[test]
    fn rigid_contact_families_are_persistent() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let families = generate_rigid_contact_families(&formation, carbon, &catalog);
        assert!(!families.is_empty());
        let count = library.insert_rigid_contact_families(families).unwrap();
        assert!(count > 0);
        drop(library);
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert!(reopened.rigid_contact_families().next().is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fluid_boundary_state_preserves_conserved_area_and_persists() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        let water = catalog.iter().find(|r| r.name == "Water").unwrap();
        let states = generate_fluid_boundary_families(&formation, water, &catalog);
        assert!(!states.is_empty());
        assert!(states.iter().all(|state| {
            (state.area - 0.5).abs() < 1e-12
                && state.curvature_radius.is_finite()
                && state.free_arc_angle_radians.is_finite()
        }));
        assert!(library.insert_fluid_boundary_families(states).unwrap() > 0);
        drop(library);
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert!(reopened.fluid_boundary_families().next().is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inconsistent_fluid_boundary_state_is_rejected_before_persistence() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        let water = catalog.iter().find(|r| r.name == "Water").unwrap();
        let mut states = generate_fluid_boundary_families(&formation, water, &catalog);
        assert!(!states.is_empty());
        states[0].curvature_radius *= 2.0;
        assert_eq!(library.insert_fluid_boundary_families(states).unwrap(), 0);
        assert_eq!(library.fluid_boundary_families().count(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn water_capillary_family_is_exact_and_persistent() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let carbon = GeometryFormation::single("Carbon");
        let water = catalog.iter().find(|r| r.name == "Water").unwrap();
        let families = generate_water_contact_families(&carbon, water, &catalog);
        assert!(!families.is_empty());
        assert!(families.iter().all(|family| family.edge_parameter_end >= family.edge_parameter_start));
        for family in families {
            assert!(library.insert_contact_family(family).unwrap());
        }
        assert!(!library.contact_families().collect::<Vec<_>>().is_empty());
        drop(library);
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert!(!reopened.contact_families().collect::<Vec<_>>().is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn convex_polygon_vertex_contact_is_continuous_and_persistent() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let formation = GeometryFormation::single("Carbon");
        let methane = catalog.iter().find(|r| r.name == "Methane").unwrap();
        let families = generate_rigid_vertex_contact_families(&formation, methane, &catalog);
        assert!(!families.is_empty());
        assert!(families.iter().all(|family| {
            family.anchor_parameter_start >= 0.0
                && family.anchor_parameter_end <= 1.0
                && family.anchor_parameter_end >= family.anchor_parameter_start
                && family.candidate_rotation_end_radians > family.candidate_rotation_start_radians
        }));
        let count = library.insert_rigid_vertex_contact_families(families).unwrap();
        assert!(count > 0);
        drop(library);
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert!(reopened.rigid_vertex_contact_families().next().is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn convex_vertex_rotation_interval_matches_hexagonal_wedge() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Carbon");
        let methane = catalog.iter().find(|r| r.name == "Methane").unwrap();
        let families = generate_rigid_vertex_contact_families(&formation, methane, &catalog);
        assert!(!families.is_empty());

        // Methane is a convex triangle. At a vertex touching a flat boundary,
        // the two incident edges define a 60-degree admissible rotation wedge.
        for family in families {
            let width = family.candidate_rotation_end_radians
                - family.candidate_rotation_start_radians;
            assert!((width - std::f64::consts::PI / 3.0).abs() < 1e-10);
        }
    }

    #[test]
    fn concave_polygon_vertex_contact_is_not_claimed_by_convex_manifold() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Carbon");
        let phosphorus = catalog.iter().find(|r| r.name == "Phosphorus").unwrap();
        assert!(generate_rigid_vertex_contact_families(&formation, phosphorus, &catalog).is_empty());
    }

    #[test]
    fn interrupted_final_json_record_does_not_destroy_durable_library() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        seed_base_catalogue(&mut library, &catalog).unwrap();
        drop(library);

        let path = root.join("formations.jsonl");
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"schema_version\":1,\"truncated\":").unwrap();
        file.sync_data().unwrap();

        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert_eq!(reopened.len(), catalog.len());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn frontier_state_persists_across_reopen() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        library
            .set_frontier_state(
                "formation-a",
                "Carbon",
                GeometryFrontierState::Exhausted,
            )
            .unwrap();
        drop(library);

        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        let key = "formation-a|Carbon";
        assert_eq!(
            reopened.frontier().records.get(key).map(|r| &r.state),
            Some(&GeometryFrontierState::Exhausted)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rigid_contact_family_can_be_instantiated_without_angle_search() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Carbon");
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let families = generate_rigid_contact_families(&formation, carbon, &catalog);
        assert!(families.iter().any(|family| {
            let parameter = (family.anchor_parameter_start + family.anchor_parameter_end) * 0.5;
            instantiate_rigid_contact_family(&formation, family, parameter, &catalog).is_some()
        }));
    }

    #[test]
    fn hydrogen_is_treated_as_a_finite_strip_in_contact_generation() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Carbon");
        let hydrogen = catalog.iter().find(|r| r.name == "Hydrogen").unwrap();
        let families = generate_rigid_vertex_contact_families(&formation, hydrogen, &catalog);
        assert!(!families.is_empty());
        assert!(families.iter().all(|family| {
            family.anchor_parameter_start <= family.anchor_parameter_end
                && family.candidate_rotation_start_radians
                    <= family.candidate_rotation_end_radians
        }));
    }

    #[test]
    fn rigid_contact_family_represents_continuous_edge_overlap() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Carbon");
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let families = generate_rigid_contact_families(&formation, carbon, &catalog);
        assert!(!families.is_empty());
        assert!(families.iter().all(|family| {
            family.anchor_parameter_end >= family.anchor_parameter_start
                && family.candidate_rotation_radians.is_finite()
        }));
    }

    #[test]
    fn two_constituent_generator_finds_exact_carbon_contacts() {
        let catalog = default_catalog();
        let base = GeometryFormation::single("Carbon");
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let candidates = generate_two_constituent_candidates(&base, carbon, &catalog);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|f| validate_formation(f, &catalog)));
    }

    #[test]
    fn three_constituent_expansion_stays_physically_valid() {
        let catalog = default_catalog();
        let base = GeometryFormation::single("Carbon");
        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let two = generate_two_constituent_candidates(&base, carbon, &catalog);
        assert!(!two.is_empty());
        let three = expand_three_constituent_candidates(&two[0], carbon, &catalog);
        assert!(!three.is_empty());
        assert!(three.iter().all(|f| f.constituents.len() == 3 && validate_formation(f, &catalog)));
    }


    #[test]
    fn two_resource_catalogue_seeding_is_deduplicated() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        seed_base_catalogue(&mut library, &catalog).unwrap();
        let added = seed_two_constituent_catalogue(&mut library, &catalog).unwrap();
        assert!(added > 0);
        let count = library.len();
        assert_eq!(seed_two_constituent_catalogue(&mut library, &catalog).unwrap(), 0);
        assert_eq!(library.len(), count);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn batched_insertion_deduplicates_before_persisting() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let carbon = GeometryFormation::single("Carbon");
        let hydrogen = GeometryFormation::single("Hydrogen");

        assert_eq!(
            library
                .insert_many(
                    vec![carbon.clone(), carbon, hydrogen.clone(), hydrogen],
                    &catalog,
                )
                .unwrap(),
            2
        );
        assert_eq!(library.len(), 2);

        drop(library);
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert_eq!(reopened.len(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn single_resource_formations_are_persistent_and_deduplicated() {
        let root = temp_root();
        let catalog = default_catalog();
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();

        assert_eq!(seed_base_catalogue(&mut library, &catalog).unwrap(), catalog.len());
        assert_eq!(library.len(), catalog.len());
        assert_eq!(seed_base_catalogue(&mut library, &catalog).unwrap(), 0);

        drop(library);
        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        assert_eq!(reopened.len(), catalog.len());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn declared_bond_requires_physical_contact() {
        let catalog = default_catalog();
        let formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                },
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 10.0, y: 0.0, rotation_radians: 0.0 },
                },
            ],
            bonds: vec![GeometryBond { constituent_a: 0, constituent_b: 1 }],
            signature: String::new(),
        };
        assert!(!validate_formation(&formation, &catalog));
    }

    #[test]
    fn declared_bond_rejects_penetrating_constituents() {
        let catalog = default_catalog();
        let formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                },
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 },
                },
            ],
            bonds: vec![GeometryBond { constituent_a: 0, constituent_b: 1 }],
            signature: String::new(),
        };
        assert!(!validate_formation(&formation, &catalog));
    }

    #[test]
    fn realized_live_interface_is_translation_and_endpoint_order_invariant() {
        let catalog = default_catalog();
        let a = crate::structure::StructuralUnit::new(
            "Carbon",
            Placement { x: 10.0, y: 20.0, rotation_radians: 0.0 },
        );
        let b = crate::structure::StructuralUnit::new(
            "Carbon",
            Placement { x: 11.0, y: 20.0, rotation_radians: 0.0 },
        );
        let candidate = crate::contact::ConnectionPairCandidate {
            endpoint_a: ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            endpoint_b: ConnectionEndpoint::Boundary { angle_radians: std::f64::consts::PI },
            distance: 0.0,
            facing: 1.0,
            load_a: 0.0,
            load_b: 0.0,
            available_a: true,
            available_b: true,
        };
        let forward = resolve_live_contact_candidate("Carbon", &a, "Carbon", &b, candidate, &catalog).unwrap();

        let reversed_candidate = crate::contact::ConnectionPairCandidate {
            endpoint_a: candidate.endpoint_b,
            endpoint_b: candidate.endpoint_a,
            ..candidate
        };
        let reverse = resolve_live_contact_candidate("Carbon", &b, "Carbon", &a, reversed_candidate, &catalog).unwrap();
        assert_eq!(forward, reverse);
        assert!(forward.signature.contains("edge:"));
    }

    #[test]
    fn topology_only_live_resolution_refuses_to_guess_a_persistent_family() {
        let interface = resolve_live_contact_interface(
            "Carbon",
            ConnectionEndpoint::Corner { point_index: 0 },
            "Carbon",
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
        );
        assert_eq!(
            classify_live_family_resolution(&interface),
            LiveFamilyResolution::Unresolved
        );
    }

}
#[derive(Clone, Debug, PartialEq)]
struct LiveEdgeDescriptor { material: String, edge: usize, parameter: f64, rotation: f64 }

fn parse_edge_pair(signature: &str) -> Option<(LiveEdgeDescriptor, LiveEdgeDescriptor)> {
    let mut parts = signature.split('|');
    let _prefix = parts.next()?;
    let first = parts.next()?.split_once(':')?;
    let second = parts.next()?.split_once(':')?;
    Some((parse_edge_descriptor(first.0, first.1)?, parse_edge_descriptor(second.0, second.1)?))
}

fn parse_edge_descriptor(material: &str, value: &str) -> Option<LiveEdgeDescriptor> {
    let value = value.strip_prefix("edge:")?;
    let mut fields = value.strip_prefix("edge:")?.split('@');
    let edge = fields.next()?;
    let parameter = fields.next()?;
    let rotation = fields.next()?;
    Some(LiveEdgeDescriptor {
        material: material.to_owned(),
        edge: edge.parse().ok()?,
        parameter: parameter.parse::<i64>().ok()? as f64 / 1_000_000_000.0,
        rotation: rotation.parse::<i64>().ok()? as f64 / 1_000_000_000.0,
    })
}

fn edge_pair_matches_family(
    a: &LiveEdgeDescriptor,
    b: &LiveEdgeDescriptor,
    family: &GeometryRigidContactFamily,
) -> bool {
    let candidate_is_a = a.material == family.candidate_resource;
    let (candidate, anchor) = if candidate_is_a { (a, b) } else { (b, a) };
    if candidate.material != family.candidate_resource || anchor.edge != family.anchor_edge
        || candidate.edge != family.candidate_edge
        || anchor.parameter < family.anchor_parameter_start - QUANTUM
        || anchor.parameter > family.anchor_parameter_end + QUANTUM
    {
        return false;
    }
    let relative = normalize_angle(candidate.rotation - anchor.rotation);
    (normalize_angle(relative - family.candidate_rotation_radians)).abs() <= 1e-7
}

fn edge_point_pair_matches_family(
    a: &LiveEdgeDescriptor,
    b: &LiveEdgeDescriptor,
    family: &GeometryRigidPointContactFamily,
) -> bool {
    (a.material == family.candidate_resource || b.material == family.candidate_resource)
        && (a.edge == family.anchor_edge || b.edge == family.anchor_edge)
        && ((a.parameter >= family.anchor_parameter_start - QUANTUM
            && a.parameter <= family.anchor_parameter_end + QUANTUM)
            || (b.parameter >= family.anchor_parameter_start - QUANTUM
                && b.parameter <= family.anchor_parameter_end + QUANTUM))
}

fn edge_vertex_pair_matches_family(
    a: &LiveEdgeDescriptor,
    b: &LiveEdgeDescriptor,
    family: &GeometryRigidVertexContactFamily,
) -> bool {
    (a.material == family.candidate_resource || b.material == family.candidate_resource)
        && (a.edge == family.anchor_edge || b.edge == family.anchor_edge)
        && ((a.parameter >= family.anchor_parameter_start - QUANTUM
            && a.parameter <= family.anchor_parameter_end + QUANTUM)
            || (b.parameter >= family.anchor_parameter_start - QUANTUM
                && b.parameter <= family.anchor_parameter_end + QUANTUM))
}

fn rigid_family_projection(family: &GeometryRigidContactFamily) -> String {
    format!("edge|{}|{}|{}|{}|{}", family.anchor_edge, family.candidate_edge, quantize(family.candidate_rotation_radians), quantize(family.anchor_parameter_start), quantize(family.anchor_parameter_end))
}
fn point_family_projection(family: &GeometryRigidPointContactFamily) -> String {
    format!("point|{}|{}|{}|{}|{}", family.anchor_edge, family.candidate_endpoint, quantize(family.anchor_parameter_start), quantize(family.anchor_parameter_end), quantize(family.candidate_rotation_start_radians))
}
fn vertex_family_projection(family: &GeometryRigidVertexContactFamily) -> String {
    format!("vertex|{}|{}|{}|{}|{}", family.anchor_edge, family.candidate_vertex, quantize(family.anchor_parameter_start), quantize(family.anchor_parameter_end), quantize(family.candidate_rotation_start_radians))
}

