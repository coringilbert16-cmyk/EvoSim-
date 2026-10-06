//! Persistent geometry reference library.
//!
//! This module is deliberately separate from the live construction runtime.
//! The library is durable knowledge: tests use isolated temporary stores, while
//! the production catalogue lives outside `target/` and survives test runs and
//! process restarts.

use crate::material_geometry::{placed_forms_penetrate, PlacedMaterialPart};
use crate::resources::{default_catalog, BaseResource, Form};
use crate::structure::Placement;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const GEOMETRY_LIBRARY_SCHEMA_VERSION: u32 = 1;
const QUANTUM: f64 = 1e-9;

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

        normalize_global_pose(&mut self, catalog);
        self.constituents.sort_by(|a, b| {
            a.resource
                .cmp(&b.resource)
                .then_with(|| quantize(a.placement.x).cmp(&quantize(b.placement.x)))
                .then_with(|| quantize(a.placement.y).cmp(&quantize(b.placement.y)))
                .then_with(|| {
                    quantize(normalized_angle(a.placement.rotation_radians))
                        .cmp(&quantize(normalized_angle(b.placement.rotation_radians)))
                })
        });

        // Rebuild through the sorted constituents using the original identities.
        // We cannot safely infer identity from equal values, so canonicalization
        // uses a second deterministic pass over the original formation.
        let original = self.constituents.clone();
        let mut indexed: Vec<(usize, GeometryConstituent)> =
            original.into_iter().enumerate().collect();
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
        self.constituents = indexed.iter().map(|(_, c)| c.clone()).collect();

        let mut remap = vec![0usize; indexed.len()];
        for (new_index, (old_index, _)) in indexed.iter().enumerate() {
            remap[*old_index] = new_index;
        }
        for bond in &mut self.bonds {
            bond.constituent_a = remap[bond.constituent_a];
            bond.constituent_b = remap[bond.constituent_b];
            if bond.constituent_a > bond.constituent_b {
                std::mem::swap(&mut bond.constituent_a, &mut bond.constituent_b);
            }
        }
        self.bonds.sort_by_key(|b| (b.constituent_a, b.constituent_b));
        self.signature = self.canonical_signature();
        Some(self)
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

#[derive(Debug)]
pub struct GeometryLibrary {
    root: PathBuf,
    entries: BTreeMap<String, GeometryFormation>,
    manifest: GeometryLibraryManifest,
}

impl GeometryLibrary {
    pub fn open(root: impl AsRef<Path>, catalog: &[BaseResource]) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let data_path = root.join("formations.jsonl");
        let manifest_path = root.join("manifest.json");

        let mut entries = BTreeMap::new();
        if data_path.exists() {
            let file = File::open(&data_path)?;
            for line in BufReader::new(file).lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let formation: GeometryFormation = serde_json::from_str(&line).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                })?;
                if formation.schema_version != GEOMETRY_LIBRARY_SCHEMA_VERSION
                    || !validate_formation(&formation, catalog)
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

        let mut library = Self {
            root,
            entries,
            manifest,
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

    pub fn insert(
        &mut self,
        formation: GeometryFormation,
        catalog: &[BaseResource],
    ) -> std::io::Result<bool> {
        let Some(formation) = formation.canonicalized(catalog) else {
            return Ok(false);
        };
        if self.entries.contains_key(&formation.signature) {
            return Ok(false);
        }

        let data_path = self.root.join("formations.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(data_path)?;
        serde_json::to_writer(&mut file, &formation)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(b"\n")?;
        file.sync_data()?;

        self.entries
            .insert(formation.signature.clone(), formation);
        self.manifest.entries = self.entries.len() as u64;
        self.write_manifest()?;
        Ok(true)
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
    fn rigid_two_constituent_formation_is_valid_when_touching_without_penetration() {
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
                    placement: Placement { x: 2.0, y: 0.0, rotation_radians: 0.0 },
                },
            ],
            bonds: vec![GeometryBond { constituent_a: 0, constituent_b: 1 }],
            signature: String::new(),
        };

        assert!(validate_formation(&formation, &catalog));
        let canonical = formation.canonicalized(&catalog).unwrap();
        assert_eq!(canonical.signature, canonical.canonical_signature());
    }

    #[test]
    fn global_rotation_does_not_change_canonical_signature() {
        let catalog = default_catalog();
        let a = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                },
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 2.0, y: 0.0, rotation_radians: 0.0 },
                },
            ],
            bonds: vec![GeometryBond { constituent_a: 0, constituent_b: 1 }],
            signature: String::new(),
        };
        let angle = std::f64::consts::FRAC_PI_2;
        let b = GeometryFormation {
            schema_version: GEOMETRY_LIBRARY_SCHEMA_VERSION,
            constituents: vec![
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 0.0, y: 0.0, rotation_radians: angle },
                },
                GeometryConstituent {
                    resource: "Carbon".into(),
                    placement: Placement { x: 0.0, y: 2.0, rotation_radians: angle },
                },
            ],
            bonds: vec![GeometryBond { constituent_a: 0, constituent_b: 1 }],
            signature: String::new(),
        };
        let ca = a.canonicalized(&catalog).unwrap();
        let cb = b.canonicalized(&catalog).unwrap();
        assert_eq!(ca.signature, cb.signature);
    }
}
