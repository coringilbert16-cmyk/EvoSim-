//! Persistent chemistry knowledge keyed by canonical physical interfaces.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const CHEMISTRY_LIBRARY_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChemistryKey {
    pub schema_version: u32,
    pub material_a: String,
    pub material_b: String,
    pub interface_class: String,
    pub interface_signature: String,
}

impl ChemistryKey {
    pub fn new(a: impl Into<String>, b: impl Into<String>, class: impl Into<String>, interface: impl Into<String>) -> Self {
        let mut material_a = a.into();
        let mut material_b = b.into();
        if material_b < material_a { std::mem::swap(&mut material_a, &mut material_b); }
        Self { schema_version: CHEMISTRY_LIBRARY_SCHEMA_VERSION, material_a, material_b, interface_class: class.into(), interface_signature: interface.into() }
    }
    pub fn signature(&self) -> String {
        format!("v{}|{}|{}|{}|{}", self.schema_version, self.material_a, self.material_b, self.interface_class, self.interface_signature)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChemistryRecord {
    pub key: ChemistryKey,
    pub static_potential: f64,
    pub activation_barrier: f64,
}

impl ChemistryRecord {
    fn is_valid(&self) -> bool {
        self.key.schema_version == CHEMISTRY_LIBRARY_SCHEMA_VERSION
            && !self.key.material_a.is_empty()
            && !self.key.material_b.is_empty()
            && !self.key.interface_class.is_empty()
            && !self.key.interface_signature.is_empty()
            && self.static_potential.is_finite() && self.static_potential >= 0.0
            && self.activation_barrier.is_finite() && self.activation_barrier >= 0.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChemistryLibraryManifest {
    pub schema_version: u32,
    pub entries: u64,
}

pub struct ChemistryLibrary {
    root: PathBuf,
    entries: BTreeMap<String, ChemistryRecord>,
    manifest: ChemistryLibraryManifest,
}

impl ChemistryLibrary {
    pub fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let data = root.join("chemistry.jsonl");
        let manifest_path = root.join("manifest.json");
        let mut entries = BTreeMap::new();
        if data.exists() {
            let file = File::open(&data)?;
            let lines: Vec<String> = BufReader::new(file).lines().collect::<Result<_, _>>()?;
            for (i, line) in lines.iter().enumerate() {
                if line.trim().is_empty() { continue; }
                let record: ChemistryRecord = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) if i + 1 == lines.len() => { let _ = e; continue; }
                    Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                };
                if record.is_valid() { entries.insert(record.key.signature(), record); }
            }
        }
        let manifest = if manifest_path.exists() {
            serde_json::from_slice(&fs::read(manifest_path)?)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        } else {
            ChemistryLibraryManifest { schema_version: CHEMISTRY_LIBRARY_SCHEMA_VERSION, entries: entries.len() as u64 }
        };
        if manifest.schema_version != CHEMISTRY_LIBRARY_SCHEMA_VERSION {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "chemistry library schema mismatch"));
        }
        Ok(Self { root, entries, manifest })
    }

    pub fn get(&self, key: &ChemistryKey) -> Option<&ChemistryRecord> { self.entries.get(&key.signature()) }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    pub fn insert(&mut self, record: ChemistryRecord) -> std::io::Result<bool> {
        if !record.is_valid() { return Ok(false); }
        let key = record.key.signature();
        if self.entries.contains_key(&key) { return Ok(false); }
        let mut file = OpenOptions::new().create(true).append(true).open(self.root.join("chemistry.jsonl"))?;
        serde_json::to_writer(&mut file, &record).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        self.entries.insert(key, record);
        self.manifest.entries = self.entries.len() as u64;
        let bytes = serde_json::to_vec_pretty(&self.manifest).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(self.root.join("manifest.json"), bytes)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_canonicalizes_material_order() {
        assert_eq!(ChemistryKey::new("Carbon","Hydrogen","rigid_edge","g"), ChemistryKey::new("Hydrogen","Carbon","rigid_edge","g"));
    }
    #[test]
    fn library_deduplicates_keys() {
        let root = std::env::temp_dir().join(format!("evosim-chemistry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut lib = ChemistryLibrary::open(&root).unwrap();
        let key = ChemistryKey::new("Carbon","Hydrogen","rigid_edge","g");
        let record = ChemistryRecord { key: key.clone(), static_potential: 0.5, activation_barrier: 1.0 };
        assert!(lib.insert(record.clone()).unwrap());
        assert!(!lib.insert(record).unwrap());
        assert!(lib.get(&key).is_some());
        assert_eq!(ChemistryLibrary::open(&root).unwrap().len(), 1);
        let _ = fs::remove_dir_all(root);
    }
}