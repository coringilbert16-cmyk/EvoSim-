//! Separate persistence for complete, already-qualified cavity candidates.
//!
//! These records intentionally do not enter Bob's small GeometryFormation
//! catalogue or its three-constituent expansion frontier.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::resources::BaseResource;
use crate::structure::OrganismStructure;

const SCHEMA_VERSION: u32 = 1;
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

/// Stable signature for a realized geometry, independent of physical IDs.
/// Unit order is preserved because mutation operators preserve the parent order;
/// IDs are normalized so the same geometry reached through repeated runs deduplicates.
fn geometry_signature(structure: &OrganismStructure) -> Result<String, String> {
    let mut normalized = structure.clone();
    let mut id_map = std::collections::HashMap::new();
    for (index, unit) in normalized.units.iter_mut().enumerate() {
        let old = unit.physical_id;
        let new = crate::structure::PhysicalConstituentId(index as u64 + 1);
        id_map.insert(old, new);
        unit.physical_id = new;
    }
    for bond in &mut normalized.bonds {
        bond.endpoint_a.constituent_id = *id_map
            .get(&bond.endpoint_a.constituent_id)
            .ok_or_else(|| "candidate bond references a missing constituent".to_string())?;
        bond.endpoint_b.constituent_id = *id_map
            .get(&bond.endpoint_b.constituent_id)
            .ok_or_else(|| "candidate bond references a missing constituent".to_string())?;
    }
    normalized.clear_genome_constituent_ids();
    let payload = serde_json::to_vec(&(&normalized.units, &normalized.bonds))
        .map_err(|error| format!("serialize candidate geometry signature: {error}"))?;
    let digest = Sha256::digest(payload);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
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
        "initial validated 5x5 nitrogen perimeter".to_string(),
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

    let candidate_signature = geometry_signature(&structure)?;
    let mut existing_count = 0usize;
    let mut existing_bytes = 0u64;
    if path.exists() {
        existing_bytes = fs::metadata(path)
            .map_err(|error| format!("inspect cavity store size: {error}"))?
            .len();
        let file = std::fs::File::open(path)
            .map_err(|error| format!("open cavity store for duplicate check: {error}"))?;
        for (line_number, line) in BufReader::new(file).lines().enumerate() {
            let line = line
                .map_err(|error| format!("read cavity store line {}: {error}", line_number + 1))?;
            if line.trim().is_empty() {
                continue;
            }
            let existing: KnownQualifyingCavity = serde_json::from_str(&line).map_err(|error| {
                format!("decode cavity store line {}: {error}", line_number + 1)
            })?;
            existing_count += 1;
            if existing.id == id || geometry_signature(&existing.structure)? == candidate_signature {
                return Ok(SeedOutcome::AlreadyPresent { id: existing.id });
            }
        }
    }

    if existing_count >= MAX_QUALIFYING_CANDIDATES {
        return Err(format!("known qualifying cavity store reached its hard limit of {MAX_QUALIFYING_CANDIDATES} candidates"));
    }

    let record = KnownQualifyingCavity {
        schema_version: SCHEMA_VERSION,
        id: id.to_string(),
        parent_id,
        change,
        constituent_count: structure.units.len(),
        cavity_area: cavity.area,
        minimum_cavity_area: cavity.minimum_area,
        structure,
    };
    let mut serialized = serde_json::to_vec(&record)
        .map_err(|error| format!("serialize qualifying cavity: {error}"))?;
    serialized.push(b'\n');
    if existing_bytes.saturating_add(serialized.len() as u64) > MAX_STORE_BYTES {
        return Err(format!("known qualifying cavity store would exceed its hard {} MiB disk budget", MAX_STORE_BYTES / (1024 * 1024)));
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("open cavity store for append: {error}"))?;
    file.write_all(&serialized)
        .map_err(|error| format!("finish cavity record: {error}"))?;
    file.sync_data()
        .map_err(|error| format!("sync cavity store: {error}"))?;

    Ok(SeedOutcome::Added {
        id: record.id,
        constituent_count: record.constituent_count,
        cavity_area: record.cavity_area,
    })
}

#[cfg(test)]
pub(crate) fn load_records(path: impl AsRef<Path>) -> Result<Vec<KnownQualifyingCavity>, String> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).map_err(|error| format!("open cavity store: {error}"))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            Ok(line) => Some(
                serde_json::from_str(&line)
                    .map_err(|error| format!("decode cavity store line {}: {error}", index + 1)),
            ),
            Err(error) => Some(Err(format!(
                "read cavity store line {}: {error}",
                index + 1
            ))),
        })
        .collect()
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
        assert_eq!(constituent_count, 16);
        assert!(cavity_area >= crate::cavity::minimum_genome_cavity_area(&catalog).unwrap());

        let records = load_records(&path).expect("store should reopen");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].constituent_count, 16);
        assert_eq!(records[0].structure.units.len(), 16);

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
}
