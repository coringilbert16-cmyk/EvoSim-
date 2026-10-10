//! Persistent chemistry knowledge keyed by canonical physical interfaces.
use crate::geometry_reference_library::{
    GeometryContactFamily, GeometryFluidBoundaryFamily, GeometryRigidContactFamily,
    GeometryRigidPointContactFamily, GeometryRigidVertexContactFamily, LiveGeometryInterface,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const CHEMISTRY_LIBRARY_SCHEMA_VERSION: u32 = 3;

fn quantize(value: f64) -> i64 {
    (value * 1_000_000_000.0).round() as i64
}
pub fn quantized_amount(value: f64) -> i64 {
    quantize(value)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChemistryKey {
    pub schema_version: u32,
    pub material_a: String,
    pub material_b: String,
    pub interface_class: String,
    pub interface_signature: String,
}

impl ChemistryKey {
    pub fn new(
        a: impl Into<String>,
        b: impl Into<String>,
        class: impl Into<String>,
        interface: impl Into<String>,
    ) -> Self {
        let mut material_a = a.into();
        let mut material_b = b.into();
        if material_b < material_a {
            std::mem::swap(&mut material_a, &mut material_b);
        }
        Self {
            schema_version: CHEMISTRY_LIBRARY_SCHEMA_VERSION,
            material_a,
            material_b,
            interface_class: class.into(),
            interface_signature: interface.into(),
        }
    }
    pub fn signature(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}",
            self.schema_version,
            self.material_a,
            self.material_b,
            self.interface_class,
            self.interface_signature
        )
    }

    pub fn from_live_geometry(
        a: impl Into<String>,
        b: impl Into<String>,
        interface: &LiveGeometryInterface,
    ) -> Self {
        Self::new(a, b, interface.interface_class, interface.signature.clone())
    }

    pub fn from_geometry_contact(
        a: impl Into<String>,
        b: impl Into<String>,
        family: &GeometryContactFamily,
    ) -> Self {
        Self::new(
            a,
            b,
            "rigid_surface",
            format!(
                "v{}|anchor_edge={}|angle={}|radius={}|edge_start={}|edge_end={}|length={}",
                family.schema_version,
                family.anchor_edge,
                quantize(family.contact_angle_radians),
                quantize(family.curvature_radius),
                quantize(family.edge_parameter_start),
                quantize(family.edge_parameter_end),
                quantize(family.contact_length),
            ),
        )
    }

    pub fn from_geometry_fluid_boundary(
        a: impl Into<String>,
        b: impl Into<String>,
        family: &GeometryFluidBoundaryFamily,
    ) -> Self {
        Self::new(a, b, "fluid_boundary", format!(
            "v{}|anchor_edge={}|area={}|angle={}|radius={}|arc={}|length={}|edge_start={}|edge_end={}",
            family.schema_version,
            family.anchor_edge,
            quantize(family.area),
            quantize(family.contact_angle_radians),
            quantize(family.curvature_radius),
            quantize(family.free_arc_angle_radians),
            quantize(family.contact_length),
            quantize(family.edge_parameter_start),
            quantize(family.edge_parameter_end),
        ))
    }

    pub fn from_geometry_rigid_contact(
        a: impl Into<String>,
        b: impl Into<String>,
        family: &GeometryRigidContactFamily,
    ) -> Self {
        Self::new(
            a,
            b,
            "rigid_edge",
            format!(
                "v{}|anchor_edge={}|candidate_edge={}|rotation={}|start={}|end={}",
                family.schema_version,
                family.anchor_edge,
                family.candidate_edge,
                quantize(family.candidate_rotation_radians),
                quantize(family.anchor_parameter_start),
                quantize(family.anchor_parameter_end),
            ),
        )
    }

    pub fn from_geometry_rigid_point(
        a: impl Into<String>,
        b: impl Into<String>,
        family: &GeometryRigidPointContactFamily,
    ) -> Self {
        Self::new(a, b, "rigid_point", format!(
            "v{}|anchor_edge={}|candidate_endpoint={}|start={}|end={}|rotation_start={}|rotation_end={}",
            family.schema_version,
            family.anchor_edge,
            family.candidate_endpoint,
            quantize(family.anchor_parameter_start),
            quantize(family.anchor_parameter_end),
            quantize(family.candidate_rotation_start_radians),
            quantize(family.candidate_rotation_end_radians),
        ))
    }

    pub fn from_geometry_rigid_vertex(
        a: impl Into<String>,
        b: impl Into<String>,
        family: &GeometryRigidVertexContactFamily,
    ) -> Self {
        Self::new(a, b, "rigid_vertex", format!(
            "v{}|anchor_edge={}|candidate_vertex={}|start={}|end={}|rotation_start={}|rotation_end={}",
            family.schema_version,
            family.anchor_edge,
            family.candidate_vertex,
            quantize(family.anchor_parameter_start),
            quantize(family.anchor_parameter_end),
            quantize(family.candidate_rotation_start_radians),
            quantize(family.candidate_rotation_end_radians),
        ))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChemistryRecord {
    pub key: ChemistryKey,
    pub static_potential: f64,
}

impl ChemistryRecord {
    fn is_valid(&self) -> bool {
        matches!(self.key.schema_version, 2 | CHEMISTRY_LIBRARY_SCHEMA_VERSION)
            && !self.key.material_a.is_empty()
            && !self.key.material_b.is_empty()
            && !self.key.interface_class.is_empty()
            && !self.key.interface_signature.is_empty()
            && self.static_potential.is_finite()
            && self.static_potential >= 0.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChemistryLibraryManifest {
    pub schema_version: u32,
    pub entries: u64,
}

#[derive(Clone)]
pub struct ChemistryLibrary {
    root: PathBuf,
    entries: HashMap<ChemistryKey, ChemistryRecord>,
    manifest: ChemistryLibraryManifest,
}

impl ChemistryLibrary {
    pub fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let data = root.join("chemistry.jsonl");
        let manifest_path = root.join("manifest.json");
        let mut entries = HashMap::new();
        if data.exists() {
            let file = File::open(&data)?;
            let lines: Vec<String> = BufReader::new(file).lines().collect::<Result<_, _>>()?;
            for (i, line) in lines.iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let record: ChemistryRecord = serde_json::from_str(line).map_err(|error| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid chemistry record at line {}: {error}", i + 1),
                    )
                })?;
                if !record.is_valid() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid chemistry record at line {}", i + 1),
                    ));
                }
                let key = record.key.clone();
                if entries.insert(key.clone(), record).is_some() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("duplicate chemistry key in persisted store: {}", key.signature()),
                    ));
                }
            }
        }
        let manifest = if manifest_path.exists() {
            serde_json::from_slice(&fs::read(manifest_path)?)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        } else {
            ChemistryLibraryManifest {
                schema_version: CHEMISTRY_LIBRARY_SCHEMA_VERSION,
                entries: entries.len() as u64,
            }
        };
        if !matches!(manifest.schema_version, 2 | CHEMISTRY_LIBRARY_SCHEMA_VERSION) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsupported chemistry library schema version: {}", manifest.schema_version),
            ));
        }
        if manifest.entries != entries.len() as u64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "chemistry manifest entry count mismatch: manifest={}, loaded_unique={}",
                    manifest.entries,
                    entries.len()
                ),
            ));
        }
        Ok(Self {
            root,
            entries,
            manifest,
        })
    }

    pub fn get(&self, key: &ChemistryKey) -> Option<&ChemistryRecord> {
        self.entries.get(key)
    }
    pub fn get_or_insert_static_potential(
        &mut self,
        key: ChemistryKey,
        static_potential: f64,
    ) -> std::io::Result<Option<f64>> {
        if !static_potential.is_finite() || static_potential < 0.0 {
            return Ok(None);
        }
        if let Some(record) = self.get(&key) {
            return Ok(Some(record.static_potential));
        }
        let record = ChemistryRecord {
            key,
            static_potential,
        };
        self.insert(record)?;
        Ok(Some(static_potential))
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn insert(&mut self, record: ChemistryRecord) -> std::io::Result<bool> {
        if !record.is_valid() {
            return Ok(false);
        }
        let key = record.key.clone();
        if self.entries.contains_key(&key) {
            return Ok(false);
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("chemistry.jsonl"))?;
        serde_json::to_writer(&mut file, &record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        self.entries.insert(key, record);
        self.manifest.entries = self.entries.len() as u64;
        let bytes = serde_json::to_vec_pretty(&self.manifest)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(self.root.join("manifest.json"), bytes)?;
        Ok(true)
    }
}

pub fn open_default_library() -> std::io::Result<ChemistryLibrary> {
    ChemistryLibrary::open("chemistry_library/data")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_geometry_interface_can_form_chemistry_key() {
        let interface = crate::geometry_reference_library::resolve_live_contact_interface(
            "Carbon",
            crate::structure::ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            "Hydrogen",
            crate::structure::ConnectionEndpoint::LineEndpoint { point_index: 0 },
        );
        let key = ChemistryKey::from_live_geometry("Carbon", "Hydrogen", &interface);
        assert_eq!(key.interface_class, "rigid_point");
        assert_eq!(key.interface_signature, interface.signature);
    }

    #[test]
    fn key_canonicalizes_material_order() {
        assert_eq!(
            ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "g"),
            ChemistryKey::new("Hydrogen", "Carbon", "rigid_edge", "g")
        );
    }
    #[test]
    fn bob_interface_key_ignores_formation_context() {
        let a = GeometryRigidContactFamily {
            schema_version: 1,
            formation_signature: "formation-a".into(),
            candidate_resource: "Hydrogen".into(),
            anchor_constituent: 0,
            anchor_edge: 2,
            candidate_edge: 1,
            candidate_rotation_radians: 0.5,
            anchor_parameter_start: 0.25,
            anchor_parameter_end: 0.75,
        };
        let mut b = a.clone();
        b.formation_signature = "formation-b".into();
        b.candidate_resource = "Carbon".into();
        let ka = ChemistryKey::from_geometry_rigid_contact("Carbon", "Hydrogen", &a);
        let kb = ChemistryKey::from_geometry_rigid_contact("Hydrogen", "Carbon", &b);
        assert_eq!(ka, kb);
    }

    #[test]
    fn bob_interface_key_distinguishes_interface_class() {
        let rigid = GeometryRigidContactFamily {
            schema_version: 1,
            formation_signature: "formation".into(),
            candidate_resource: "Hydrogen".into(),
            anchor_constituent: 0,
            anchor_edge: 1,
            candidate_edge: 1,
            candidate_rotation_radians: 0.0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
        };
        let point = GeometryRigidPointContactFamily {
            schema_version: 1,
            formation_signature: "formation".into(),
            candidate_resource: "Hydrogen".into(),
            anchor_constituent: 0,
            anchor_edge: 1,
            candidate_endpoint: 0,
            anchor_parameter_start: 0.0,
            anchor_parameter_end: 1.0,
            candidate_rotation_start_radians: 0.0,
            candidate_rotation_end_radians: 0.0,
        };
        assert_ne!(
            ChemistryKey::from_geometry_rigid_contact("Carbon", "Hydrogen", &rigid),
            ChemistryKey::from_geometry_rigid_point("Carbon", "Hydrogen", &point)
        );
    }

    #[test]
    fn calculate_on_miss_persists_static_potential() {
        let root =
            std::env::temp_dir().join(format!("evosim-chemistry-miss-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut lib = ChemistryLibrary::open(&root).unwrap();
        let key = ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "edge");
        assert_eq!(lib.get(&key), None);
        assert_eq!(
            lib.get_or_insert_static_potential(key.clone(), 0.625)
                .unwrap(),
            Some(0.625)
        );
        assert_eq!(
            lib.get(&key).map(|record| record.static_potential),
            Some(0.625)
        );
        let reopened = ChemistryLibrary::open(&root).unwrap();
        assert_eq!(
            reopened.get(&key).map(|record| record.static_potential),
            Some(0.625)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn library_deduplicates_keys() {
        let root = std::env::temp_dir().join(format!("evosim-chemistry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut lib = ChemistryLibrary::open(&root).unwrap();
        let key = ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "g");
        let record = ChemistryRecord {
            key: key.clone(),
            static_potential: 0.5,
        };
        assert!(lib.insert(record.clone()).unwrap());
        assert!(!lib.insert(record).unwrap());
        assert!(lib.get(&key).is_some());
        assert_eq!(ChemistryLibrary::open(&root).unwrap().len(), 1);
        let _ = fs::remove_dir_all(root);
    }
}
