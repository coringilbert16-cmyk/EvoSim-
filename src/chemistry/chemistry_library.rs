//! Persistent chemistry knowledge keyed by canonical physical interfaces.
use crate::geometry_reference_library::LiveGeometryInterface;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const CHEMISTRY_LIBRARY_SCHEMA_VERSION: u32 = 5;

/// Increment this fingerprint whenever chemistry equations, constants, or
/// resource-chemistry semantics change. Existing derived cache rows are never
/// relabeled or migrated across fingerprints.
pub const CHEMISTRY_EQUATION_FINGERPRINT: &str = "evosim-chemistry-v1|potential=exp-k-dmax|contact=(1-d/r)^2|Dmax=13|k=1/13|contact_radius=1|max_force=1|energy_distance=contact_radius|energy_scale=max_force*energy_distance|reaction_dissipation=0.1/tick|catalog=resource-chemistry-v2|chemical-position=quantized-key-v1";

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
    /// Quantized chemical positions used by the static-potential equation.
    /// They belong in the cache key so a catalog position change cannot reuse
    /// a potential calculated from the previous chemistry spectrum.
    #[serde(default)]
    pub chemical_position_a: Option<i64>,
    #[serde(default)]
    pub chemical_position_b: Option<i64>,
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
            chemical_position_a: None,
            chemical_position_b: None,
        }
    }
    pub fn signature(&self) -> String {
        format!(
            "v{}|{}|{}|{}|{}|chemical_positions={:?},{:?}",
            self.schema_version,
            self.material_a,
            self.material_b,
            self.interface_class,
            self.interface_signature,
            self.chemical_position_a,
            self.chemical_position_b
        )
    }

    pub fn from_live_geometry(
        a: impl Into<String>,
        b: impl Into<String>,
        interface: &LiveGeometryInterface,
        chemical_position_a: f64,
        chemical_position_b: f64,
    ) -> Self {
        let material_a = a.into();
        let material_b = b.into();
        // Keep each quantized position attached to its canonical material name.
        let (position_a, position_b) = if material_a <= material_b {
            (chemical_position_a, chemical_position_b)
        } else {
            (chemical_position_b, chemical_position_a)
        };
        let mut key = Self::new(
            material_a,
            material_b,
            interface.interface_class,
            interface.signature.clone(),
        );
        key.chemical_position_a = Some(quantize(position_a));
        key.chemical_position_b = Some(quantize(position_b));
        key
    }


}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChemistryRecord {
    pub key: ChemistryKey,
    pub static_potential: f64,
}

impl ChemistryRecord {
    fn is_valid(&self) -> bool {
        self.key.schema_version == CHEMISTRY_LIBRARY_SCHEMA_VERSION
            && !self.key.material_a.is_empty()
            && !self.key.material_b.is_empty()
            && !self.key.interface_class.is_empty()
            && !self.key.interface_signature.is_empty()
            // Live chemistry keys carry both positions; geometry-only library
            // keys carry neither. A half-populated key is never a valid identity.
            && (self.key.chemical_position_a.is_some()
                == self.key.chemical_position_b.is_some())
            && self.static_potential.is_finite()
            && self.static_potential >= 0.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChemistryLibraryManifest {
    pub schema_version: u32,
    pub equation_fingerprint: String,
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
        let _lock = lock_chemistry_store(&root)?;
        Self::open_unlocked(root)
    }

    fn open_unlocked(root: PathBuf) -> std::io::Result<Self> {
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
                        format!(
                            "duplicate chemistry key in persisted store: {}",
                            key.signature()
                        ),
                    ));
                }
            }
        }
        let manifest = if manifest_path.exists() {
            serde_json::from_slice(&fs::read(manifest_path)?)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        } else {
            if !entries.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "chemistry manifest is missing for a non-empty cache; refusing to infer schema or equation fingerprint",
                ));
            }
            ChemistryLibraryManifest {
                schema_version: CHEMISTRY_LIBRARY_SCHEMA_VERSION,
                equation_fingerprint: CHEMISTRY_EQUATION_FINGERPRINT.into(),
                entries: 0,
            }
        };
        if manifest.schema_version != CHEMISTRY_LIBRARY_SCHEMA_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "unsupported chemistry library schema version: {}",
                    manifest.schema_version
                ),
            ));
        }
        if manifest.equation_fingerprint != CHEMISTRY_EQUATION_FINGERPRINT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "chemistry equation fingerprint mismatch: manifest={}, current={}",
                    manifest.equation_fingerprint, CHEMISTRY_EQUATION_FINGERPRINT
                ),
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
            key: key.clone(),
            static_potential,
        };
        self.insert(record)?;
        Ok(self.get(&key).map(|record| record.static_potential))
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn insert(&mut self, record: ChemistryRecord) -> std::io::Result<bool> {
        self.insert_with_persistence(record, append_record_and_sync, persist_manifest_atomically)
    }

    #[cfg(test)]
    fn insert_with_manifest_persistence<F>(
        &mut self,
        record: ChemistryRecord,
        persist_manifest: F,
    ) -> std::io::Result<bool>
    where
        F: FnOnce(&Path, &ChemistryLibraryManifest) -> std::io::Result<()>,
    {
        self.insert_with_persistence(record, append_record_and_sync, persist_manifest)
    }

    fn insert_with_persistence<A, F>(
        &mut self,
        record: ChemistryRecord,
        append_record: A,
        persist_manifest: F,
    ) -> std::io::Result<bool>
    where
        A: FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
        F: FnOnce(&Path, &ChemistryLibraryManifest) -> std::io::Result<()>,
    {
        if !record.is_valid() {
            return Ok(false);
        }
        let _lock = lock_chemistry_store(&self.root)?;
        // Another instance may have changed the store since this instance opened.
        // Refresh under the lock before duplicate detection or manifest counting.
        let current = Self::open_unlocked(self.root.clone())?;
        self.entries = current.entries;
        self.manifest = current.manifest;

        let key = record.key.clone();
        if self.entries.contains_key(&key) {
            return Ok(false);
        }
        let data_path = self.root.join("chemistry.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&data_path)?;
        let original_len = file.metadata()?.len();
        let mut row = serde_json::to_vec(&record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        row.push(b'\n');

        if let Err(write_error) = append_record(&mut file, &row) {
            return Err(rollback_appended_record(
                &mut file,
                original_len,
                write_error,
            ));
        }

        let mut next_manifest = self.manifest.clone();
        next_manifest.entries = (self.entries.len() + 1) as u64;
        if let Err(manifest_error) = persist_manifest(&self.root, &next_manifest) {
            return Err(rollback_appended_record(
                &mut file,
                original_len,
                manifest_error,
            ));
        }

        // Publish the new in-memory state only after both persisted updates succeed.
        self.entries.insert(key, record);
        self.manifest = next_manifest;
        Ok(true)
    }
}

fn append_record_and_sync(file: &mut File, row: &[u8]) -> std::io::Result<()> {
    file.write_all(row)?;
    file.sync_data()
}

fn rollback_appended_record(
    file: &mut File,
    original_len: u64,
    original_error: std::io::Error,
) -> std::io::Error {
    match file.set_len(original_len).and_then(|()| file.sync_data()) {
        Ok(()) => original_error,
        Err(rollback_error) => std::io::Error::new(
            rollback_error.kind(),
            format!(
                "chemistry persistence failed ({original_error}); additionally failed to roll back appended data ({rollback_error})"
            ),
        ),
    }
}

// Advisory file locking coordinates independent ChemistryLibrary instances and
// separate processes that use the same store path.
fn lock_chemistry_store(root: &Path) -> std::io::Result<File> {
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(".chemistry.lock"))?;
    lock_file.lock()?;
    Ok(lock_file)
}

// Unique names prevent independent library instances from colliding on a temporary
// manifest file. Atomic rename keeps readers from observing a partially-written JSON.
static MANIFEST_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn persist_manifest_atomically(
    root: &Path,
    manifest: &ChemistryLibraryManifest,
) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let manifest_path = root.join("manifest.json");
    let (temp_path, mut file) = loop {
        let sequence = MANIFEST_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = root.join(format!(
            ".manifest.json.tmp-{}-{sequence}",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };

    let write_result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, &manifest_path)?;
        // The rename is the commit point. Directory syncing is best-effort because
        // reporting failure after rename would falsely imply the manifest was unchanged.
        let _ = File::open(root).and_then(|directory| directory.sync_all());
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
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
        let key = ChemistryKey::from_live_geometry("Carbon", "Hydrogen", &interface, 10.5, 5.5);
        assert_eq!(key.interface_class, "rigid_point");
        assert_eq!(key.interface_signature, interface.signature);
        assert_eq!(key.chemical_position_a, Some(10_500_000_000));
        assert_eq!(key.chemical_position_b, Some(5_500_000_000));
    }

    #[test]
    fn live_geometry_key_distinguishes_chemical_positions_and_canonicalizes_them() {
        let interface = crate::geometry_reference_library::resolve_live_contact_interface(
            "Carbon",
            crate::structure::ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            "Hydrogen",
            crate::structure::ConnectionEndpoint::LineEndpoint { point_index: 0 },
        );
        let canonical =
            ChemistryKey::from_live_geometry("Carbon", "Hydrogen", &interface, 10.5, 5.5);
        let reversed =
            ChemistryKey::from_live_geometry("Hydrogen", "Carbon", &interface, 5.5, 10.5);
        let changed_position =
            ChemistryKey::from_live_geometry("Carbon", "Hydrogen", &interface, 10.25, 5.5);
        assert_eq!(canonical, reversed);
        assert_ne!(canonical, changed_position);
        assert_ne!(canonical.signature(), changed_position.signature());
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
    fn pre_unification_sulfur_interaction_cache_record_is_not_reused() {
        // D6 rejects pre-unification chemistry caches without migration. The
        // repository cache was reset to the current schema; the old cached
        // interaction must not be treated as authoritative under schema five.
        let library = ChemistryLibrary::open("chemistry_library/data").unwrap();
        let key = ChemistryKey::new(
            "Hydrogen",
            "Sulfur",
            "rigid_point",
            "live-v1|Hydrogen:line:0|Sulfur:corner:0",
        );
        assert!(
            library.get(&key).is_none(),
            "pre-unification Hydrogen–Sulfur cache output must not be reused"
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

    #[test]
    fn manifest_failure_rolls_back_appended_record_and_reopens_cleanly() {
        let root = test_root("manifest-failure");
        let _ = fs::remove_dir_all(&root);
        let mut library = ChemistryLibrary::open(&root).unwrap();
        let record = ChemistryRecord {
            key: ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "rollback"),
            static_potential: 0.55,
        };
        let manifest_path = root.join("manifest.json");
        let original_manifest = match fs::read(&manifest_path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("failed to read original manifest: {error}"),
        };
        let error = library
            .insert_with_manifest_persistence(record, |_, _| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected manifest replacement failure",
                ))
            })
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(library.len(), 0);
        assert_eq!(library.manifest.entries, 0);
        assert_eq!(fs::metadata(root.join("chemistry.jsonl")).unwrap().len(), 0);
        let manifest_after_failure = match fs::read(&manifest_path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("failed to read manifest after failed insert: {error}"),
        };
        assert_eq!(manifest_after_failure, original_manifest);

        let reopened = ChemistryLibrary::open(&root).unwrap();
        assert!(reopened.is_empty());
        assert_eq!(reopened.manifest.entries, 0);
        drop(reopened);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn insert_partial_append_failure_rolls_back_original_cache_bytes_and_reopens_cleanly() {
        let root = test_root("append-failure");
        let _ = fs::remove_dir_all(&root);
        let mut library = ChemistryLibrary::open(&root).unwrap();
        let original = ChemistryRecord {
            key: ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "append-rollback"),
            static_potential: 0.45,
        };
        assert!(library.insert(original.clone()).unwrap());

        let data_path = root.join("chemistry.jsonl");
        let manifest_path = root.join("manifest.json");
        let original_data = fs::read(&data_path).unwrap();
        let original_manifest = fs::read(&manifest_path).unwrap();
        let failed_record = ChemistryRecord {
            key: ChemistryKey::new(
                "Carbon",
                "Hydrogen",
                "rigid_edge",
                "injected-append-failure",
            ),
            static_potential: 0.75,
        };
        let failed_key = failed_record.key.clone();

        let error = library
            .insert_with_persistence(
                failed_record,
                |file, row| {
                    let prefix_len = row.len().min(5);
                    file.write_all(&row[..prefix_len])?;
                    file.sync_data()?;
                    Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "injected partial chemistry record append failure",
                    ))
                },
                persist_manifest_atomically,
            )
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::WriteZero);
        assert_eq!(fs::read(&data_path).unwrap(), original_data);
        assert_eq!(fs::read(&manifest_path).unwrap(), original_manifest);
        assert_eq!(library.len(), 1);
        assert_eq!(library.manifest.entries, 1);
        assert!(library.get(&failed_key).is_none());

        let reopened = ChemistryLibrary::open(&root).unwrap();
        assert_eq!(reopened.len(), 1);
        assert_eq!(reopened.manifest.entries, 1);
        assert!(reopened.get(&failed_key).is_none());
        assert_eq!(
            reopened
                .get(&original.key)
                .map(|record| record.static_potential),
            Some(0.45)
        );
        drop(reopened);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn independent_instances_serialize_writes_and_refresh_stale_state() {
        let root = test_root("concurrent-writers");
        let _ = fs::remove_dir_all(&root);
        let first = ChemistryLibrary::open(&root).unwrap();
        let second = ChemistryLibrary::open(&root).unwrap();
        let first_record = ChemistryRecord {
            key: ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "concurrent-a"),
            static_potential: 0.4,
        };
        let second_record = ChemistryRecord {
            key: ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "concurrent-b"),
            static_potential: 0.6,
        };

        let first_thread = std::thread::spawn(move || {
            let mut library = first;
            library.insert(first_record)
        });
        let second_thread = std::thread::spawn(move || {
            let mut library = second;
            library.insert(second_record)
        });
        assert!(first_thread.join().unwrap().unwrap());
        assert!(second_thread.join().unwrap().unwrap());
        assert_eq!(ChemistryLibrary::open(&root).unwrap().len(), 2);

        // Two stale instances racing to insert the same key must not create a
        // duplicate persisted row or inflate the manifest count.
        let third = ChemistryLibrary::open(&root).unwrap();
        let fourth = ChemistryLibrary::open(&root).unwrap();
        let duplicate = ChemistryRecord {
            key: ChemistryKey::new("Carbon", "Hydrogen", "rigid_edge", "concurrent-duplicate"),
            static_potential: 0.7,
        };
        let third_thread = std::thread::spawn({
            let duplicate = duplicate.clone();
            move || {
                let mut library = third;
                library.insert(duplicate)
            }
        });
        let fourth_thread = std::thread::spawn(move || {
            let mut library = fourth;
            library.insert(duplicate)
        });
        let inserted = [
            third_thread.join().unwrap().unwrap(),
            fourth_thread.join().unwrap().unwrap(),
        ];
        assert_eq!(inserted.into_iter().filter(|value| *value).count(), 1);
        let reopened = ChemistryLibrary::open(&root).unwrap();
        assert_eq!(reopened.len(), 3);
        let rows = BufReader::new(File::open(root.join("chemistry.jsonl")).unwrap())
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(rows.len(), 3);
        drop(reopened);
        let _ = fs::remove_dir_all(root);
    }

    fn test_root(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock must be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "evosim-chemistry-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn write_test_store(root: &Path, schema_version: u32, rows: &[String], manifest_entries: u64) {
        fs::create_dir_all(root).unwrap();
        let mut data = rows.join("\n");
        if !data.is_empty() {
            data.push('\n');
        }
        fs::write(root.join("chemistry.jsonl"), data).unwrap();
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&ChemistryLibraryManifest {
                schema_version,
                equation_fingerprint: CHEMISTRY_EQUATION_FINGERPRINT.into(),
                entries: manifest_entries,
            })
            .unwrap(),
        )
        .unwrap();
    }

    fn test_record_json(schema_version: u32, signature: &str) -> String {
        serde_json::to_string(&ChemistryRecord {
            key: ChemistryKey {
                schema_version,
                material_a: "Carbon".into(),
                material_b: "Hydrogen".into(),
                interface_class: "rigid_edge".into(),
                interface_signature: signature.into(),
                chemical_position_a: None,
                chemical_position_b: None,
            },
            static_potential: 0.5,
        })
        .unwrap()
    }

    #[test]
    fn loads_current_schema_five_store() {
        let root = test_root("schema-five");
        write_test_store(&root, 5, &[test_record_json(5, "edge")], 1);
        let library = ChemistryLibrary::open(&root).unwrap();
        assert_eq!(library.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_nonempty_cache_without_manifest_without_migration() {
        let root = test_root("missing-manifest");
        fs::create_dir_all(&root).unwrap();
        let data_path = root.join("chemistry.jsonl");
        let record = test_record_json(5, "edge");
        fs::write(&data_path, format!("{record}\n")).unwrap();
        let original_data = fs::read(&data_path).unwrap();

        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("non-empty cache without a manifest must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("manifest is missing"));
        assert_eq!(fs::read(&data_path).unwrap(), original_data);
        assert!(!root.join("manifest.json").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_legacy_schema_two_store_without_migration() {
        let root = test_root("schema-two");
        write_test_store(&root, 2, &[], 0);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("legacy cache must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error
            .to_string()
            .contains("unsupported chemistry library schema version"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_schema_four_store_after_chemical_position_key_migration() {
        let root = test_root("schema-four");
        write_test_store(&root, 4, &[], 0);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("schema-four cache must be rejected after the key contract changes");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error
            .to_string()
            .contains("unsupported chemistry library schema version"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_legacy_schema_three_store_without_migration() {
        let root = test_root("schema-three");
        write_test_store(&root, 3, &[], 0);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("pre-unification cache must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error
            .to_string()
            .contains("unsupported chemistry library schema version"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_legacy_record_keys_even_in_current_schema_manifest() {
        let root = test_root("legacy-record");
        write_test_store(&root, 5, &[test_record_json(3, "legacy-edge")], 1);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("legacy record keys must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("invalid chemistry record"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_equation_fingerprint_mismatch() {
        let root = test_root("fingerprint-mismatch");
        write_test_store(&root, 5, &[test_record_json(5, "edge")], 1);
        let manifest_path = root.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["equation_fingerprint"] = serde_json::Value::String("old-equations".into());
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("mismatched equation cache must be rejected");
        assert!(error.to_string().contains("fingerprint mismatch"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_malformed_final_nonempty_record() {
        let root = test_root("truncated-tail");
        write_test_store(&root, 5, &["{".into()], 1);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("corrupt store must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_duplicate_persisted_keys() {
        let root = test_root("duplicate");
        let row = test_record_json(5, "same-edge");
        write_test_store(&root, 5, &[row.clone(), row], 1);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("corrupt store must be rejected");
        assert!(error.to_string().contains("duplicate chemistry key"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_manifest_count_mismatch() {
        let root = test_root("count-mismatch");
        write_test_store(&root, 5, &[test_record_json(5, "edge")], 2);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("corrupt store must be rejected");
        assert!(error.to_string().contains("entry count mismatch"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_partial_chemical_position_keys() {
        let root = test_root("partial-chemical-position");
        let mut value: serde_json::Value =
            serde_json::from_str(&test_record_json(5, "edge")).unwrap();
        value["key"]["chemical_position_a"] = serde_json::Value::from(10_i64);
        value["key"]["chemical_position_b"] = serde_json::Value::Null;
        write_test_store(&root, 5, &[serde_json::to_string(&value).unwrap()], 1);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("partial chemical position identity must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("invalid chemistry record"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_semantically_invalid_persisted_records() {
        let root = test_root("invalid-record");
        let invalid = serde_json::to_string(&ChemistryRecord {
            key: ChemistryKey {
                schema_version: 5,
                material_a: "Carbon".into(),
                material_b: "Hydrogen".into(),
                interface_class: "rigid_edge".into(),
                interface_signature: "edge".into(),
                chemical_position_a: None,
                chemical_position_b: None,
            },
            static_potential: f64::NAN,
        })
        .unwrap();
        // JSON does not represent NaN, so serialize a parseable record with a negative value.
        let invalid = invalid.replace("null", "-1.0");
        write_test_store(&root, 5, &[invalid], 1);
        let error = ChemistryLibrary::open(&root)
            .err()
            .expect("corrupt store must be rejected");
        assert!(error.to_string().contains("invalid chemistry record"));
        let _ = fs::remove_dir_all(root);
    }
}
