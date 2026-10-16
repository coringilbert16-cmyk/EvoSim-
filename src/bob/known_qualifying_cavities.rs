//! Separate persistence for complete, already-qualified cavity candidates.
//!
//! These records intentionally do not enter Bob's small GeometryFormation
//! catalogue or its three-constituent expansion frontier.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::resources::{BaseResource, Form};
use crate::structure::OrganismStructure;

#[path = "cavity_mutation.rs"]
mod cavity_mutation;
pub(crate) use cavity_mutation::{mutate_once, mutate_until_full};

const SCHEMA_VERSION: u32 = 3;

/// Persist cavity-area metadata at four decimal places; geometry remains authoritative.
fn rounded_cavity_area(area: f64) -> f64 {
    (area * 10_000.0).round() / 10_000.0
}
const PREVIOUS_SCHEMA_VERSION: u32 = 2;
const LEGACY_SCHEMA_VERSION: u32 = 1;
const NITROGEN_RING_SEED_ID: &str = "nitrogen-ring-seed-v1";
pub(crate) const MAX_QUALIFYING_CANDIDATES: usize = 5_000;
pub(crate) const MAX_MUTATION_ATTEMPTS_PER_RUN: usize = 5_000;
const MAX_STORE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct KnownQualifyingCavity {
    pub schema_version: u32,
    pub id: String,
    pub parent_id: Option<String>,
    pub change: String,
    pub constituent_count: usize,
    pub cavity_area: f64,
    pub minimum_cavity_area: f64,
    pub structure: OrganismStructure,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SeedOutcome {
    Added {
        id: String,
        constituent_count: usize,
        cavity_area: f64,
    },
    AlreadyPresent {
        id: String,
    },
}

/// Stable formation signature, independent of physical IDs and bond bookkeeping.
/// Formation identity is defined by constituent materials and realized geometry, not
/// by arbitrary bond additions/removals. Stored records must still pass cavity analysis,
/// so only a bond change that alters cavity qualification can affect whether a record
/// is eligible for storage; it does not create a second copy of the same formation.
fn quantize_signature_value(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn polygon_rotation_period(vertices: &[(f64, f64)]) -> f64 {
    let count = vertices.len();
    for order in (2..=count).rev() {
        if count % order != 0 {
            continue;
        }
        let angle = std::f64::consts::TAU / order as f64;
        let (sin, cos) = angle.sin_cos();
        let symmetric = vertices.iter().all(|&(x, y)| {
            let rotated = (x * cos - y * sin, x * sin + y * cos);
            vertices
                .iter()
                .any(|&(other_x, other_y)| (rotated.0 - other_x).hypot(rotated.1 - other_y) <= 1e-7)
        });
        if symmetric {
            return angle;
        }
    }
    std::f64::consts::TAU
}

/// The smallest rotation period that leaves a resource's local physical shape
/// unchanged. None means the shape has no meaningful orientation.
fn rotational_symmetry_period(form: &Form) -> Option<f64> {
    match form {
        Form::Circle { .. } => None,
        Form::Line { .. } => Some(std::f64::consts::PI),
        Form::Rectangle { width, height } => {
            if (*width - *height).abs() <= 1e-9 * (*width).max(*height).max(1.0) {
                Some(std::f64::consts::FRAC_PI_2)
            } else {
                Some(std::f64::consts::PI)
            }
        }
        Form::RegularPolygon { sides, .. } => Some(std::f64::consts::TAU / f64::from(*sides)),
        Form::Polygon { vertices } => Some(polygon_rotation_period(vertices)),
        Form::Fluid {
            boundary: Some(vertices),
            arc_boundary: None,
            multi_contact_boundary: None,
            ..
        } => Some(polygon_rotation_period(vertices)),
        Form::Fluid {
            boundary: None,
            arc_boundary: None,
            multi_contact_boundary: None,
            ..
        } => None,
        Form::Fluid { .. } => Some(std::f64::consts::TAU),
    }
}

/// Stable physical-formation identity, independent of constituent IDs, unit
/// ordering, global translation/rotation, and rotations that leave an individual
/// resource shape unchanged. Reflections are not normalized as a canonical frame:
/// mirror arrangements remain distinct unless an allowed rotation/shape symmetry
/// already makes their physical representation equivalent. Bond bookkeeping is excluded.
fn geometry_signature(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Result<String, String> {
    type CanonicalUnit = (String, String, i64, i64, i64);

    let forms = structure
        .units
        .iter()
        .map(|unit| {
            unit.shape(catalog)
                .map(|shape| shape.form.clone())
                .ok_or_else(|| {
                    "candidate formation has a constituent with no physical shape".to_string()
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // A unit's orientation (including every symmetry-equivalent orientation)
    // and every center-to-center direction can define a global reference frame.
    // The smallest normalized representation is independent of that choice.
    let mut frame_angles = Vec::new();
    for (unit, form) in structure.units.iter().zip(&forms) {
        if let Some(period) = rotational_symmetry_period(form) {
            let order = (std::f64::consts::TAU / period).round().max(1.0) as usize;
            for symmetry_step in 0..order {
                frame_angles.push(unit.placement.rotation_radians + symmetry_step as f64 * period);
            }
        }
    }
    for left in 0..structure.units.len() {
        for right in (left + 1)..structure.units.len() {
            let dx = structure.units[right].placement.x - structure.units[left].placement.x;
            let dy = structure.units[right].placement.y - structure.units[left].placement.y;
            if dx.hypot(dy) > 1e-9 {
                frame_angles.push(dy.atan2(dx));
            }
        }
    }
    if frame_angles.is_empty() {
        frame_angles.push(0.0);
    }

    let mut representations = Vec::<Vec<CanonicalUnit>>::with_capacity(frame_angles.len());
    for frame_angle in frame_angles {
        let (sin, cos) = frame_angle.sin_cos();
        let transformed_positions = structure
            .units
            .iter()
            .map(|unit| {
                (
                    unit.placement.x * cos + unit.placement.y * sin,
                    -unit.placement.x * sin + unit.placement.y * cos,
                )
            })
            .collect::<Vec<_>>();
        let min_x = transformed_positions
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::INFINITY, f64::min);
        let min_y = transformed_positions
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::INFINITY, f64::min);

        let mut canonical_units = Vec::with_capacity(structure.units.len());
        for (index, unit) in structure.units.iter().enumerate() {
            let mut parts = unit.material.parts.clone();
            parts.sort_by(|left, right| {
                left.0
                    .cmp(&right.0)
                    .then_with(|| left.1.total_cmp(&right.1))
            });
            let material = serde_json::to_string(&parts)
                .map_err(|error| format!("serialize candidate material identity: {error}"))?;
            let form = serde_json::to_string(&forms[index])
                .map_err(|error| format!("serialize candidate shape identity: {error}"))?;
            let (x, y) = transformed_positions[index];
            let angle = rotational_symmetry_period(&forms[index])
                .map(|period| (unit.placement.rotation_radians - frame_angle).rem_euclid(period))
                .unwrap_or(0.0);
            canonical_units.push((
                material,
                form,
                quantize_signature_value(x - min_x),
                quantize_signature_value(y - min_y),
                quantize_signature_value(angle),
            ));
        }
        canonical_units.sort();
        representations.push(canonical_units);
    }

    let canonical = representations.into_iter().min().unwrap_or_default();
    let payload = serde_json::to_vec(&canonical)
        .map_err(|error| format!("serialize candidate formation signature: {error}"))?;
    let digest = Sha256::digest(payload);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Reject candidates whose placed shapes penetrate. This uses the same strict
/// overlap predicate as full validation, so legal edge/corner contact remains valid.
fn validate_candidate_non_overlap(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Result<(), String> {
    for left in 0..structure.units.len() {
        for right in (left + 1)..structure.units.len() {
            if crate::structure::units_strictly_overlap(
                &structure.units[left],
                &structure.units[right],
                catalog,
            ) {
                return Err(format!("constituents {left} and {right} overlap"));
            }
        }
    }
    Ok(())
}

/// Validate physical shapes and bond endpoints after the early overlap precheck.
fn validate_candidate_shape_and_bonds(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Result<(), String> {
    for (index, unit) in structure.units.iter().enumerate() {
        if !unit
            .shape(catalog)
            .is_some_and(|shape| shape.is_valid_physical_shape())
        {
            return Err(format!("constituent {index} has no valid physical shape"));
        }
    }
    for (index, bond) in structure.bonds.iter().enumerate() {
        if !structure.is_valid_bond(bond, catalog) {
            return Err(format!(
                "bond {index} violates physical endpoint/contact rules"
            ));
        }
    }
    Ok(())
}

/// Full validation remains authoritative for stored records and other callers.
fn validate_candidate_geometry(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Result<(), String> {
    validate_candidate_shape_and_bonds(structure, catalog)?;
    validate_candidate_non_overlap(structure, catalog)
}

/// Atomically replace a cavity store during schema migration.
fn replace_records_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary_path = path.with_extension("migration.tmp");
    let result = (|| {
        let mut temporary = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_path)
            .map_err(|error| format!("open temporary cavity migration: {error}"))?;
        temporary
            .write_all(bytes)
            .map_err(|error| format!("write temporary cavity migration: {error}"))?;
        temporary
            .sync_all()
            .map_err(|error| format!("sync temporary cavity migration: {error}"))?;
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("publish migrated cavity store: {error}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

/// Publish a JSONL append by atomically replacing the file. An interrupted write
/// leaves either the previous complete store or the complete new store visible.
pub(super) fn append_records_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary_path = path.with_extension("tmp");
    let result = (|| {
        let mut temporary = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_path)
            .map_err(|error| format!("open temporary cavity store: {error}"))?;
        if path.exists() {
            let mut existing = std::fs::File::open(path)
                .map_err(|error| format!("open existing cavity store: {error}"))?;
            std::io::copy(&mut existing, &mut temporary)
                .map_err(|error| format!("copy existing cavity store: {error}"))?;
        }
        temporary
            .write_all(bytes)
            .map_err(|error| format!("write temporary cavity store: {error}"))?;
        temporary
            .sync_all()
            .map_err(|error| format!("sync temporary cavity store: {error}"))?;
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("publish cavity store atomically: {error}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

struct StoreLock {
    path: PathBuf,
}

impl StoreLock {
    fn acquire(store_path: &Path) -> Result<Self, String> {
        let path = store_path.with_extension("lock");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                format!(
                    "known qualifying cavity store lock {} could not be acquired: {error}; \
                     if no Bob process is running, inspect and manually remove the stale lock before retrying",
                    path.display()
                )
            })?;
        Ok(Self { path })
    }
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn default_store_path() -> PathBuf {
    std::env::var_os("EVOSIM_KNOWN_CAVITIES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("geometry_library/known_qualifying_cavities"))
        .join("candidates.jsonl")
}

/// Seed Bob's separate cavity category with the known nitrogen ring.
/// Validation happens before the record is written; invalid structures are
/// never stored in the qualifying-cavity file.
pub(crate) fn seed_nitrogen_ring(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
) -> Result<SeedOutcome, String> {
    let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(catalog)?;
    persist_if_qualifying(
        path,
        NITROGEN_RING_SEED_ID,
        None,
        "initial validated corner-free 12-piece nitrogen cavity ring".to_string(),
        structure,
        catalog,
    )
}

pub(crate) fn persist_if_qualifying(
    path: impl AsRef<Path>,
    id: &str,
    parent_id: Option<String>,
    change: String,
    structure: OrganismStructure,
    catalog: &[BaseResource],
) -> Result<SeedOutcome, String> {
    if id.trim().is_empty() {
        return Err("known cavity candidate ID must not be empty".to_string());
    }

    validate_candidate_geometry(&structure, catalog)?;

    // Use the authoritative realized-geometry validator. Extra processing
    // structure is deliberately not required for this category.
    let cavity = crate::cavity::analyze_genome_cavity(&structure, catalog)?.ok_or_else(|| {
        "candidate has no sealed cavity meeting the three-Carbon minimum".to_string()
    })?;
    if !cavity.qualifies() {
        return Err("candidate cavity failed the authoritative qualification checks".to_string());
    }
    crate::organism_viability::validate_realized_organism(
        &structure,
        catalog,
        crate::organism_viability::OrganismViabilityRequirements {
            require_sealed_genome_cavity: true,
            require_extra_structure: false,
        },
    )?;

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create cavity store directory: {error}"))?;
    }

    let _store_lock = StoreLock::acquire(path)?;
    let candidate_signature = geometry_signature(&structure, catalog)?;
    let mut existing_count = 0usize;
    let mut existing_bytes = 0u64;
    if path.exists() {
        existing_bytes = fs::metadata(path)
            .map_err(|error| format!("inspect cavity store size: {error}"))?
            .len();
        let existing_records = load_records(path)?;
        existing_count = existing_records.len();
        for existing in existing_records {
            if existing.id == id
                || geometry_signature(&existing.structure, catalog)? == candidate_signature
            {
                return Ok(SeedOutcome::AlreadyPresent { id: existing.id });
            }
        }
    }

    if existing_count >= MAX_QUALIFYING_CANDIDATES {
        return Err(format!(
            "known qualifying cavity store reached its hard limit of {MAX_QUALIFYING_CANDIDATES} candidates"
        ));
    }

    let record = KnownQualifyingCavity {
        schema_version: SCHEMA_VERSION,
        id: id.to_string(),
        parent_id,
        change,
        constituent_count: structure.units.len(),
        cavity_area: rounded_cavity_area(cavity.area),
        minimum_cavity_area: cavity.minimum_area,
        structure,
    };
    let mut serialized = serde_json::to_vec(&record)
        .map_err(|error| format!("serialize qualifying cavity: {error}"))?;
    serialized.push(b'\n');
    if existing_bytes.saturating_add(serialized.len() as u64) > MAX_STORE_BYTES {
        return Err(format!(
            "known qualifying cavity store would exceed its hard {} MiB disk budget",
            MAX_STORE_BYTES / (1024 * 1024)
        ));
    }
    append_records_atomically(path, &serialized)?;

    Ok(SeedOutcome::Added {
        id: record.id,
        constituent_count: record.constituent_count,
        cavity_area: record.cavity_area,
    })
}

pub(crate) fn load_records(path: impl AsRef<Path>) -> Result<Vec<KnownQualifyingCavity>, String> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents =
        fs::read_to_string(path).map_err(|error| format!("open cavity store: {error}"))?;
    let mut records = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let record: KnownQualifyingCavity = serde_json::from_str(line)
            .map_err(|error| format!("decode cavity store line {line_number}: {error}"))?;
        if record.schema_version != SCHEMA_VERSION
            && record.schema_version != PREVIOUS_SCHEMA_VERSION
            && record.schema_version != LEGACY_SCHEMA_VERSION
        {
            return Err(format!(
                "unsupported cavity-store schema version {} on line {line_number}; expected {SCHEMA_VERSION}",
                record.schema_version
            ));
        }
        records.push(record);
    }

    // One-time migration for the old store: preserve only the original nitrogen
    // ring and discard every candidate generated under bond-sensitive identity.
    // Writing schema v2 makes this migration idempotent; subsequent Bob runs keep
    // newly generated candidates rather than resetting the store again.
    if records
        .iter()
        .any(|record| record.schema_version != SCHEMA_VERSION)
    {
        let mut seed = records
            .into_iter()
            .find(|record| record.id == NITROGEN_RING_SEED_ID)
            .ok_or_else(|| {
                "legacy cavity store has no original nitrogen-ring seed; refusing destructive migration"
                    .to_string()
            })?;
        seed.schema_version = SCHEMA_VERSION;
        let mut bytes = serde_json::to_vec(&seed)
            .map_err(|error| format!("serialize migrated nitrogen-ring seed: {error}"))?;
        bytes.push(b'\n');
        replace_records_atomically(path, &bytes)?;
        return Ok(vec![seed]);
    }

    let mut ids = std::collections::HashSet::new();
    for (index, record) in records.iter().enumerate() {
        let line_number = index + 1;
        if record.id.trim().is_empty() || ids.contains(&record.id) {
            return Err(format!(
                "empty or duplicate cavity candidate ID on line {line_number}"
            ));
        }
        if record.constituent_count != record.structure.units.len() {
            return Err(format!(
                "cavity candidate {} has inconsistent constituent_count metadata",
                record.id
            ));
        }
        if !record.cavity_area.is_finite()
            || record.cavity_area <= 0.0
            || !record.minimum_cavity_area.is_finite()
            || record.minimum_cavity_area <= 0.0
        {
            return Err(format!(
                "cavity candidate {} has invalid stored area metadata",
                record.id
            ));
        }
        if let Some(parent_id) = &record.parent_id {
            if !ids.contains(parent_id) {
                return Err(format!(
                    "cavity candidate {} references a parent not already present: {}",
                    record.id, parent_id
                ));
            }
        }
        ids.insert(record.id.clone());
        if index + 1 > MAX_QUALIFYING_CANDIDATES {
            return Err(format!(
                "cavity store exceeds hard limit of {MAX_QUALIFYING_CANDIDATES} records"
            ));
        }
    }
    Ok(records)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_store() -> PathBuf {
        std::env::temp_dir().join(format!(
            "evosim-known-cavities-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ))
    }

    #[test]
    fn overlap_precheck_preserves_valid_seed_contacts_and_rejects_penetration() {
        let catalog = crate::resources::default_catalog();
        let path = temporary_store();
        let _ = fs::remove_file(&path);
        seed_nitrogen_ring(&path, &catalog).expect("seed must pass authoritative validation");
        let records = load_records(&path).expect("seed record should load");
        let seed = &records[0].structure;
        assert!(
            validate_candidate_non_overlap(seed, &catalog).is_ok(),
            "legal contacts in the qualifying seed must not count as overlap"
        );

        let mut overlapping = seed.clone();
        let first_pose = overlapping.units[0].placement;
        overlapping.units[1].placement.x = first_pose.x;
        overlapping.units[1].placement.y = first_pose.y;
        assert!(validate_candidate_non_overlap(&overlapping, &catalog)
            .expect_err("coincident rigid pieces must be rejected")
            .contains("overlap"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn cavity_store_lock_prevents_concurrent_writers() {
        let path = temporary_store();
        let lock_path = path.with_extension("lock");
        let _ = fs::remove_file(&lock_path);
        let first = StoreLock::acquire(&path).expect("first writer should acquire lock");
        assert!(StoreLock::acquire(&path).is_err());
        drop(first);
        assert!(StoreLock::acquire(&path).is_ok());
        let _ = fs::remove_file(lock_path);
    }

    #[test]
    fn one_step_mutations_never_increase_constituent_count() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let parent = KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: NITROGEN_RING_SEED_ID.to_string(),
            parent_id: None,
            change: "test seed".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: 0.0,
            minimum_cavity_area: 0.0,
            structure,
        };
        let mutations = cavity_mutation::one_step_mutations(&parent, &catalog);
        assert!(!mutations.is_empty());
        assert!(mutations
            .iter()
            .all(|(_, candidate)| { candidate.units.len() <= parent.constituent_count }));
        assert!(mutations.iter().any(|(change, candidate)| {
            change.starts_with("remove constituent")
                && candidate.units.len() + 1 == parent.constituent_count
        }));
        assert!(mutations.iter().any(|(change, candidate)| {
            change.starts_with("replace constituent")
                && candidate.units.len() == parent.constituent_count
        }));

        assert!(
            mutations[0].0.starts_with("replace constituent"),
            "resource substitution must be the first attempted mutation"
        );
        let first_translation = mutations
            .iter()
            .position(|(change, _)| change.starts_with("translate constituent"))
            .expect("mutation list should include translation trials");
        let first_rotation = mutations
            .iter()
            .position(|(change, _)| change.starts_with("rotate constituent"))
            .expect("mutation list should include rotation trials");
        assert!(
            first_translation < mutations.len() - 1 && first_rotation < mutations.len() - 1,
            "translation and rotation must be scheduled before the final removal family"
        );
        assert!(
            first_translation < mutations.iter().rposition(|(change, _)| change.starts_with("replace constituent")).unwrap(),
            "translations should be interleaved before the replacement family is exhausted"
        );
        assert!(
            first_rotation < mutations.iter().rposition(|(change, _)| change.starts_with("replace constituent")).unwrap(),
            "rotations should be interleaved before the replacement family is exhausted"
        );
        let first_removal = mutations
            .iter()
            .position(|(change, _)| change.starts_with("remove constituent"))
            .expect("mutation list should include constituent removal");
        assert!(
            mutations[first_removal..]
                .iter()
                .all(|(change, _)| change.starts_with("remove constituent")),
            "constituent removals must be the final mutation family"
        );
    }

    #[test]
    fn mutation_frontier_changes_one_thing_at_a_time_in_stages() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .expect("cavity analysis should succeed")
            .expect("seed should have a cavity");
        let parent = KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: NITROGEN_RING_SEED_ID.to_string(),
            parent_id: None,
            change: "test seed".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: rounded_cavity_area(cavity.area),
            minimum_cavity_area: cavity.minimum_area,
            structure,
        };
        let mutations = cavity_mutation::one_step_mutations(&parent, &catalog);
        assert!(
            !mutations
                .iter()
                .any(|(change, _)| change.contains(" with Water")),
            "circle-shaped Water cannot form the polygonal wall required by this cavity validator"
        );
        assert!(mutations[0].0.starts_with("replace constituent"));
        assert!(mutations[0].0.ends_with("(existing pose)"));

        let first_translation = mutations
            .iter()
            .position(|(change, _)| change.starts_with("translate constituent"))
            .expect("frontier should include independent translations");
        let first_rotation = mutations
            .iter()
            .position(|(change, _)| change.starts_with("rotate constituent"))
            .expect("frontier should include independent rotations");
        let last_replacement = mutations
            .iter()
            .rposition(|(change, _)| change.starts_with("replace constituent"))
            .expect("frontier should include resource substitutions");
        assert!(
            first_translation < last_replacement && first_rotation < last_replacement,
            "translation and rotation attempts must be interleaved before replacement trials are exhausted"
        );
        assert!(
            mutations[..first_translation]
                .iter()
                .any(|(change, _)| change.ends_with("(existing pose)")),
            "the exchange search should include an unchanged-pose trial before translations"
        );

        let replacement = &mutations[0];
        let changed_index = replacement
            .0
            .strip_prefix("replace constituent at index ")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|index| index.parse::<usize>().ok())
            .expect("replacement label should identify its constituent");
        assert_eq!(
            replacement.1.units[changed_index].placement,
            parent.structure.units[changed_index].placement,
            "replacement must first be tested at its unchanged pose"
        );
        assert_ne!(
            replacement.1.units[changed_index].material,
            parent.structure.units[changed_index].material,
            "replacement trial should change only the resource"
        );

        let translated = mutations
            .iter()
            .find(|(change, _)| {
                change.starts_with("translate constituent") && change.contains(" 0.25")
            })
            .expect("frontier should include an independently tested quarter-unit translation");
        assert!(translated.0.starts_with("translate constituent"));
        let translated_index = translated
            .0
            .strip_prefix("translate constituent at index ")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|index| index.parse::<usize>().ok())
            .expect("translation label should identify its constituent");
        assert_eq!(
            translated.1.units[translated_index].material,
            parent.structure.units[translated_index].material,
            "translation must not also change the resource"
        );
        assert_eq!(
            translated.1.units[translated_index]
                .placement
                .rotation_radians,
            parent.structure.units[translated_index]
                .placement
                .rotation_radians,
            "translation must not also rotate the constituent"
        );
        assert!(validate_candidate_non_overlap(&translated.1, &catalog).is_ok());

        let unit_translation = mutations
            .iter()
            .find(|(change, _)| {
                change.starts_with("translate constituent") && change.contains(" 1.00")
            })
            .expect(
                "frontier should include independent one-unit translations when non-overlapping",
            );
        assert!(unit_translation.0.starts_with("translate constituent"));
        let unit_translated_index = unit_translation
            .0
            .strip_prefix("translate constituent at index ")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|index| index.parse::<usize>().ok())
            .expect("one-unit translation should identify its constituent");
        let before = &parent.structure.units[unit_translated_index];
        let after = &unit_translation.1.units[unit_translated_index];
        let dx = (after.placement.x - before.placement.x).abs();
        let dy = (after.placement.y - before.placement.y).abs();
        assert!(
            (dx - 1.0).abs() < 1e-10 || (dy - 1.0).abs() < 1e-10,
            "one-unit translation must move exactly one unit on one axis"
        );
        assert_eq!(after.material, before.material);
        assert_eq!(
            after.placement.rotation_radians,
            before.placement.rotation_radians
        );
        assert!(validate_candidate_non_overlap(&unit_translation.1, &catalog).is_ok());

        let rotated = mutations
            .iter()
            .find(|(change, _)| change == "rotate constituent at index 0 by 180deg")
            .expect("frontier should include an independent 180-degree rotation");
        assert_eq!(
            rotated.1.units[0].placement.x,
            parent.structure.units[0].placement.x
        );
        assert_eq!(
            rotated.1.units[0].placement.y,
            parent.structure.units[0].placement.y
        );
        assert_eq!(
            rotated.1.units[0].material,
            parent.structure.units[0].material
        );
        assert!(
            (rotated.1.units[0].placement.rotation_radians
                - (parent.structure.units[0].placement.rotation_radians + std::f64::consts::PI))
                .abs()
                < 1e-10
        );

        let first_removal = mutations
            .iter()
            .position(|(change, _)| change.starts_with("remove constituent"))
            .expect("removal should remain available as a last resort");
        assert!(mutations[first_removal..]
            .iter()
            .all(|(change, _)| change.starts_with("remove constituent")));
    }

    #[test]
    fn next_lineage_step_prioritizes_the_constituent_changed_by_its_parent() {
        let catalog = crate::resources::default_catalog();
        let mut structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        structure.units[3].material =
            crate::resources::Material::free_base("Methane".to_string(), 1.0);
        structure.units[3].geometry = None;
        let parent = KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: "lineage-parent".to_string(),
            parent_id: Some(NITROGEN_RING_SEED_ID.to_string()),
            change: "replace constituent at index 3 with Methane (existing pose)".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: 9.0,
            minimum_cavity_area: crate::cavity::minimum_genome_cavity_area(&catalog).unwrap(),
            structure,
        };
        let mutations = cavity_mutation::one_step_mutations(&parent, &catalog);
        assert!(
            mutations[0]
                .0
                .starts_with("replace constituent at index 3 "),
            "resource substitutions must remain the first mutation family, got {}",
            mutations[0].0
        );
        let first_non_replacement = mutations
            .iter()
            .position(|(change, _)| !change.starts_with("replace constituent"))
            .expect("lineage frontier should include non-replacement mutations");
        assert!(
            mutations[first_non_replacement]
                .0
                .starts_with("translate constituent at index 3 "),
            "after substitutions, continue locally with the recently changed piece first; got {}",
            mutations[first_non_replacement].0
        );
    }

    #[test]
    fn replacement_trial_is_generated_at_the_original_pose_before_other_mutations() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let parent = KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: NITROGEN_RING_SEED_ID.to_string(),
            parent_id: None,
            change: "test seed".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: 9.0,
            minimum_cavity_area: crate::cavity::minimum_genome_cavity_area(&catalog).unwrap(),
            structure,
        };
        let mutations = cavity_mutation::one_step_mutations(&parent, &catalog);
        let (change, trial) = mutations
            .first()
            .expect("replacement frontier should not be empty");
        assert!(change.starts_with("replace constituent"));
        assert!(change.ends_with("(existing pose)"));
        assert_eq!(trial.units.len(), parent.structure.units.len());
        let index = change
            .strip_prefix("replace constituent at index ")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|value| value.parse::<usize>().ok())
            .expect("replacement label should identify its constituent");
        assert_eq!(
            trial.units[index].placement,
            parent.structure.units[index].placement
        );
        assert_ne!(
            trial.units[index].material,
            parent.structure.units[index].material
        );
    }

    #[test]
    fn replacement_trial_must_rebuild_a_live_physical_contact() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("nitrogen ring seed should build");
        let parent = KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: NITROGEN_RING_SEED_ID.to_string(),
            parent_id: None,
            change: "test seed".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: 9.0,
            minimum_cavity_area: crate::cavity::minimum_genome_cavity_area(&catalog).unwrap(),
            structure,
        };
        let mutations = cavity_mutation::one_step_mutations(&parent, &catalog);
        let mut repaired_candidate = None;

        for (change, mut candidate) in mutations
            .into_iter()
            .filter(|(change, _)| change.starts_with("replace constituent"))
        {
            let Some(unit_index) = change
                .strip_prefix("replace constituent at index ")
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|index| index.parse::<usize>().ok())
            else {
                continue;
            };
            if cavity_mutation::rebuild_replacement_connections(
                &mut candidate,
                unit_index,
                &catalog,
            ) {
                repaired_candidate = Some((change, candidate, unit_index));
                break;
            }
        }

        let (change, structure, unit_index) = repaired_candidate
            .expect("at least one non-overlapping replacement pose should repair a live contact");
        let changed_id = structure.units[unit_index].physical_id;
        assert!(
            structure.bonds.iter().any(|bond| {
                bond.endpoint_a.constituent_id == changed_id
                    || bond.endpoint_b.constituent_id == changed_id
            }),
            "the changed constituent in {change} should be connected through a live physical bond"
        );
    }

    #[test]
    fn mutation_batch_stays_within_candidate_and_attempt_budgets() {
        let catalog = crate::resources::default_catalog();
        let path = temporary_store();
        let cursor = path.with_extension("cursor");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&cursor);

        let stats = mutate_once(&path, &catalog).expect("bounded mutation batch should complete");
        assert!(stats.attempts <= MAX_MUTATION_ATTEMPTS_PER_RUN);
        if stats.attempts == MAX_MUTATION_ATTEMPTS_PER_RUN {
            assert!(
                stats.stopped_at_attempt_limit,
                "a batch that reaches its attempt budget must report the stop reason"
            );
        }
        assert!(
            stats.cavity_checks_performed > 0,
            "mutator should run cavity analysis on physically valid novel candidates"
        );
        assert!(
            stats.cavity_checks_performed <= stats.attempts,
            "cavity checks cannot exceed candidate attempts"
        );
        assert!(
            stats.accepted > 0,
            "seed should produce at least one qualifying one-step variant"
        );
        let records = load_records(&path).expect("mutated store should reopen");
        assert!(!records.is_empty());
        assert!(records.len() <= MAX_QUALIFYING_CANDIDATES);
        assert_eq!(records[0].id, NITROGEN_RING_SEED_ID);
        assert!(records.iter().all(|record| record.constituent_count <= 12));

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(cursor);
    }

    #[test]
    fn legacy_store_migration_keeps_only_the_original_nitrogen_ring() {
        let catalog = crate::resources::default_catalog();
        let path = temporary_store();
        let _ = fs::remove_file(&path);
        let seed_structure =
            crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
                .expect("seed structure should build");
        let seed_cavity = crate::cavity::analyze_genome_cavity(&seed_structure, &catalog)
            .expect("seed cavity analysis should succeed")
            .expect("seed should have a qualifying cavity");
        let seed = KnownQualifyingCavity {
            schema_version: PREVIOUS_SCHEMA_VERSION,
            id: NITROGEN_RING_SEED_ID.to_string(),
            parent_id: None,
            change: "initial validated corner-free 12-piece nitrogen cavity ring".to_string(),
            constituent_count: seed_structure.units.len(),
            cavity_area: seed_cavity.area,
            minimum_cavity_area: seed_cavity.minimum_area,
            structure: seed_structure.clone(),
        };
        let mut obsolete = seed.clone();
        obsolete.id = "legacy-bond-variant".to_string();
        obsolete.parent_id = Some(NITROGEN_RING_SEED_ID.to_string());
        obsolete.change = "remove bond at index 0".to_string();
        obsolete.structure.break_bond(0);
        let mut contents = serde_json::to_vec(&seed).unwrap();
        contents.push(b'\n');
        contents.extend(serde_json::to_vec(&obsolete).unwrap());
        contents.push(b'\n');
        fs::write(&path, contents).expect("legacy fixture should be writable");

        let migrated = load_records(&path).expect("legacy store should migrate");
        assert_eq!(migrated.len(), 1);
        assert_eq!(migrated[0].id, NITROGEN_RING_SEED_ID);
        assert_eq!(migrated[0].schema_version, SCHEMA_VERSION);
        let on_disk = fs::read_to_string(&path).expect("migrated store should be readable");
        assert_eq!(on_disk.lines().count(), 1);
        assert!(on_disk.contains(NITROGEN_RING_SEED_ID));
        assert!(!on_disk.contains("legacy-bond-variant"));

        // A second load must not reset newly written v2 records.
        let mut evolved = migrated[0].clone();
        evolved.id = "new-v2-candidate".to_string();
        evolved.parent_id = Some(NITROGEN_RING_SEED_ID.to_string());
        evolved.change = "test generated candidate".to_string();
        let mut next_contents = on_disk;
        next_contents.push_str(&serde_json::to_string(&evolved).unwrap());
        next_contents.push('\n');
        fs::write(&path, next_contents).unwrap();
        assert_eq!(load_records(&path).unwrap().len(), 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn geometry_signature_ignores_bond_only_differences() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let original = geometry_signature(&structure, &catalog).expect("signature should build");
        let mut changed_bonds = structure.clone();
        changed_bonds.break_bond(0);
        assert_eq!(
            original,
            geometry_signature(&changed_bonds, &catalog).expect("signature should build"),
            "bond-only changes must not be counted as a new formation"
        );
        let mut translated = structure.clone();
        for unit in &mut translated.units {
            unit.placement.x += 17.25;
            unit.placement.y -= 8.5;
        }
        assert_eq!(
            original,
            geometry_signature(&translated, &catalog).expect("translated signature should build"),
            "global translation must not create a duplicate formation"
        );
    }

    #[test]
    fn geometry_signature_ignores_physical_id_values() {
        let catalog = crate::resources::default_catalog();
        let mut first = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let original = geometry_signature(&first, &catalog).expect("signature should build");
        let mut id_map = std::collections::HashMap::new();
        for (index, unit) in first.units.iter_mut().enumerate() {
            let old = unit.physical_id;
            let new = crate::structure::PhysicalConstituentId(100 + index as u64);
            id_map.insert(old, new);
            unit.physical_id = new;
        }
        for bond in &mut first.bonds {
            bond.endpoint_a.constituent_id = id_map[&bond.endpoint_a.constituent_id];
            bond.endpoint_b.constituent_id = id_map[&bond.endpoint_b.constituent_id];
        }
        assert_eq!(original, geometry_signature(&first, &catalog).unwrap());
    }

    #[test]
    fn geometry_signature_canonicalizes_equivalent_physical_poses() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let original = geometry_signature(&structure, &catalog).expect("signature should build");

        let global_angle = 0.37_f64;
        let (sin, cos) = global_angle.sin_cos();
        let mut globally_rotated = structure.clone();
        for unit in &mut globally_rotated.units {
            let x = unit.placement.x;
            let y = unit.placement.y;
            unit.placement.x = x * cos - y * sin;
            unit.placement.y = x * sin + y * cos;
            unit.placement.rotation_radians += global_angle;
        }
        assert_eq!(
            original,
            geometry_signature(&globally_rotated, &catalog)
                .expect("rotated signature should build"),
            "global rotation must not create a duplicate formation"
        );

        let mut reordered = structure.clone();
        reordered.units.reverse();
        assert_eq!(
            original,
            geometry_signature(&reordered, &catalog).expect("reordered signature should build"),
            "constituent ordering must not define formation identity"
        );

        let mut symmetric_piece_rotation = structure.clone();
        symmetric_piece_rotation.units[0].placement.rotation_radians += std::f64::consts::FRAC_PI_2;
        assert_eq!(
            original,
            geometry_signature(&symmetric_piece_rotation, &catalog)
                .expect("symmetry-equivalent signature should build"),
            "rotating a square nitrogen by 90 degrees must not create a new formation"
        );

        let mut meaningful_rotation = structure;
        meaningful_rotation.units[0].placement.rotation_radians += std::f64::consts::PI / 12.0;
        assert_ne!(
            original,
            geometry_signature(&meaningful_rotation, &catalog)
                .expect("distinct rotation signature should build"),
            "a non-symmetric 15-degree rotation must remain a distinct physical formation"
        );
    }

    #[test]
    fn geometry_signature_does_not_implicitly_collapse_mirror_arrangements() {
        let catalog = crate::resources::default_catalog();
        let mut asymmetric = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        asymmetric.units[0].placement.rotation_radians += std::f64::consts::PI / 12.0;
        let original = geometry_signature(&asymmetric, &catalog)
            .expect("original signature should build");

        let mut mirrored = asymmetric.clone();
        for unit in &mut mirrored.units {
            unit.placement.y = -unit.placement.y;
            unit.placement.rotation_radians = -unit.placement.rotation_radians;
        }
        assert_ne!(
            original,
            geometry_signature(&mirrored, &catalog).expect("mirrored signature should build"),
            "reflection is not a rotation; a chiral placement must not be collapsed implicitly"
        );
    }

    #[test]
    fn cavity_area_metadata_is_rounded_to_four_decimal_places() {
        assert_eq!(rounded_cavity_area(7.123456), 7.1235);
        assert_eq!(rounded_cavity_area(7.12344), 7.1234);
        assert_eq!(rounded_cavity_area(9.0), 9.0);
    }

    #[test]
    fn nitrogen_ring_is_validated_and_persisted_separately_and_idempotently() {
        let catalog = crate::resources::default_catalog();
        let path = temporary_store();
        let _ = fs::remove_file(&path);

        let first = seed_nitrogen_ring(&path, &catalog).expect("nitrogen ring should qualify");
        let SeedOutcome::Added {
            constituent_count,
            cavity_area,
            ..
        } = first
        else {
            panic!("first seed should be added");
        };
        assert_eq!(constituent_count, 12);
        assert!(cavity_area >= crate::cavity::minimum_genome_cavity_area(&catalog).unwrap());

        let records = load_records(&path).expect("store should reopen");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].constituent_count, 12);
        assert_eq!(records[0].structure.units.len(), 12);

        let second = seed_nitrogen_ring(&path, &catalog).expect("repeat seed should be safe");
        assert_eq!(
            second,
            SeedOutcome::AlreadyPresent {
                id: NITROGEN_RING_SEED_ID.to_string()
            }
        );
        assert_eq!(load_records(&path).unwrap().len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn overlapping_mutation_is_rejected_before_cavity_persistence() {
        let catalog = crate::resources::default_catalog();
        let mut structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        // Shift one top-edge square into its neighbor; cavity shape alone must
        // not be enough to admit this physically overlapping candidate.
        structure.units[1].placement.x += 0.25;
        assert!(validate_candidate_geometry(&structure, &catalog).is_err());
        let path = temporary_store();
        let _ = fs::remove_file(&path);
        let result = persist_if_qualifying(
            &path,
            "rejected-overlap",
            None,
            "move a boundary piece into its neighbor".to_string(),
            structure,
            &catalog,
        );
        assert!(result.is_err());
        assert!(!path.exists() || load_records(&path).unwrap().is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn unsealed_candidate_is_never_persisted() {
        let catalog = crate::resources::default_catalog();
        let path = temporary_store();
        let _ = fs::remove_file(&path);
        let mut structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        // Remove the midpoint of a side, not a corner: a corner removal can
        // leave the remaining boundary segments touching at a vertex.
        let removed_id = structure.units[2].physical_id;
        structure
            .units
            .retain(|unit| unit.physical_id != removed_id);
        structure.bonds.retain(|bond| {
            bond.endpoint_a.constituent_id != removed_id
                && bond.endpoint_b.constituent_id != removed_id
        });
        let result = persist_if_qualifying(
            &path,
            "rejected-open-ring",
            None,
            "remove all bonds from one boundary piece".to_string(),
            structure,
            &catalog,
        );
        assert!(result.is_err());
        assert!(!path.exists() || load_records(&path).unwrap().is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_records_rejects_inconsistent_constituent_metadata() {
        let catalog = crate::resources::default_catalog();
        let path = temporary_store();
        let _ = fs::remove_file(&path);
        seed_nitrogen_ring(&path, &catalog).expect("valid seed should persist");
        let contents = fs::read_to_string(&path).expect("store should be readable");
        let corrupted = contents.replace("\"constituent_count\":12", "\"constituent_count\":999");
        assert_ne!(
            contents, corrupted,
            "fixture must contain the expected metadata"
        );
        fs::write(&path, corrupted).expect("corrupt test fixture should be writable");
        let error = load_records(&path).expect_err("inconsistent metadata must be rejected");
        assert!(error.contains("inconsistent constituent_count"), "{error}");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_records_rejects_unsupported_schema_version() {
        let catalog = crate::resources::default_catalog();
        let path = temporary_store();
        let _ = fs::remove_file(&path);
        seed_nitrogen_ring(&path, &catalog).expect("valid seed should persist");
        let contents = fs::read_to_string(&path).expect("store should be readable");
        let corrupted = contents.replace("\"schema_version\":3", "\"schema_version\":999");
        assert_ne!(
            contents, corrupted,
            "fixture must contain the expected schema"
        );
        fs::write(&path, corrupted).expect("corrupt test fixture should be writable");
        let error = load_records(&path).expect_err("unknown schema must be rejected");
        assert!(
            error.contains("unsupported cavity-store schema version"),
            "{error}"
        );
        let _ = fs::remove_file(path);
    }
}
