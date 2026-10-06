//! Persistent geometry reference library.
//!
//! This module is deliberately separate from the live construction runtime.
//! The library is durable knowledge: tests use isolated temporary stores, while
//! the production catalogue lives outside `target/` and survives test runs and
//! process restarts.

use crate::capillary_geometry::{solve_water_against_solid, ContactTranslationInterval};
use crate::material_geometry::{placed_forms_penetrate, placed_forms_rigid_contact, PlacedMaterialPart};
use crate::resources::{default_catalog, BaseResource, Form};
use crate::structure::Placement;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const GEOMETRY_LIBRARY_SCHEMA_VERSION: u32 = 2;
const QUANTUM: f64 = 1e-9;

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
    rigid_contact_families: BTreeMap<String, GeometryRigidContactFamily>,
}

impl GeometryLibrary {
    pub fn open(root: impl AsRef<Path>, catalog: &[BaseResource]) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let data_path = root.join("formations.jsonl");
        let manifest_path = root.join("manifest.json");
        let frontier_path = root.join("frontier.json");
        let contact_family_path = root.join("contact_families.jsonl");
        let rigid_contact_family_path = root.join("rigid_contact_families.jsonl");

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
            for line in BufReader::new(file).lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let family: GeometryRigidContactFamily = match serde_json::from_str(&line) {
                    Ok(value) => value,
                    Err(_) => continue,
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

        let mut library = Self {
            root,
            entries,
            manifest,
            frontier,
            contact_families,
            rigid_contact_families,
        };
        library.manifest.entries = library.entries.len() as u64;
        library.write_manifest()?;
        Ok(library)
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

    pub fn rigid_contact_families(&self) -> impl Iterator<Item = &GeometryRigidContactFamily> {
        self.rigid_contact_families.values()
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

    pub fn insert_contact_family(&mut self, family: GeometryContactFamily) -> std::io::Result<bool> {
        if family.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION || !family.contact_angle_radians.is_finite() || !family.curvature_radius.is_finite() || !family.contact_length.is_finite() || !family.edge_parameter_start.is_finite() || !family.edge_parameter_end.is_finite() || family.edge_parameter_end < family.edge_parameter_start { return Ok(false); }
        let signature = family.signature();
        if self.contact_families.contains_key(&signature) { return Ok(false); }
        let path = self.root.join("contact_families.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, &family).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(b"\\n")?;
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
            file.write_all(b"\\n")?;
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
                if !self.entries.contains_key(&canonical.signature) {
                    unique.insert(canonical.signature.clone(), canonical);
                }
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
            file.write_all(b"\\n")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("evosim-geometry-library-{nonce}"))
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
    fn water_capillary_family_supports_rigid_line_boundaries() {
        let catalog = default_catalog();
        let formation = GeometryFormation::single("Hydrogen");
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
        let catalog = default_catalog();
        let formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "Hydrogen".into(),
                    placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                },
                GeometryConstituent {
                    resource: "Hydrogen".into(),
                    placement: Placement { x: -0.25, y: 0.0, rotation_radians: 0.0 },
                },
                GeometryConstituent {
                    resource: "Hydrogen".into(),
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
    fn touching_bonded_fluids_can_be_valid_against_a_rigid_boundary() {
        let catalog = default_catalog();
        let formation = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {