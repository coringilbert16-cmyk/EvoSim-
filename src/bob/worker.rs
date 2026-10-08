use crate::chemistry::evaluate_static_chemical_interaction;
use crate::chemistry_library::{ChemistryEvaluationState, ChemistryKey, ChemistryLibrary};
use crate::combine::{evaluate_formation, formation_cost};
use crate::contact::ConnectionCompatibilityCache;
use crate::geometry_reference_library::{
    expand_formation_candidates, generate_fluid_boundary_families, generate_rigid_contact_families,
    generate_rigid_point_contact_families, generate_rigid_vertex_contact_families,
    generate_water_contact_families, open_default_library, seed_base_catalogue, GeometryFormation,
    GeometryFrontierState, GeometryLibrary,
};
use crate::resources::{default_catalog, BaseResource};
use crate::structure::{OrganismStructure, StructuralUnit};
use std::thread;
use std::time::{Duration, Instant};

const IDLE_SLEEP: Duration = Duration::from_secs(1);

pub fn run() {
    let catalog = default_catalog();
    let mut library = open_default_library().expect("geometry library must open");
    let mut chemistry_library =
        crate::chemistry_library::open_default_library().expect("chemistry library must open");
    seed_base_catalogue(&mut library, &catalog).expect("geometry library seed must succeed");

    loop {
        match process_one_frontier(&mut library, &mut chemistry_library, &catalog)
            .expect("geometry worker failed")
        {
            Some(metrics) => eprintln!("{metrics}"),
            None => thread::sleep(IDLE_SLEEP),
        }
    }
}

/// Process exactly one durable frontier pass and exit.
///
/// This is the safe smoke-test entry point: it exercises the same persistent
/// store, seeding, candidate generation, validation, and frontier writes as the
/// continuous worker without starting an unbounded process.
pub fn run_once() -> std::io::Result<bool> {
    let catalog = default_catalog();
    let mut library = open_default_library()?;
    let mut chemistry_library = crate::chemistry_library::open_default_library()?;
    let seeded = seed_base_catalogue(&mut library, &catalog)?;
    let metrics = process_one_frontier(&mut library, &mut chemistry_library, &catalog)?;
    if let Some(metrics) = metrics {
        eprintln!("geometry worker once: seeded={seeded}; {metrics}");
        Ok(true)
    } else {
        eprintln!(
            "geometry worker once: formations={}, seeded={}, processed=false",
            library.len(),
            seeded
        );
        Ok(false)
    }
}

#[derive(Default)]
struct WorkerPassMetrics {
    formation_size: usize,
    generated_candidates: usize,
    added_formations: usize,
    rigid_edge_families: usize,
    rigid_point_families: usize,
    rigid_vertex_families: usize,
    water_families: usize,
    chemistry_rejections: usize,
    fluid_boundary_families: usize,
    elapsed: Duration,
    total_formations: usize,
}

impl std::fmt::Display for WorkerPassMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "geometry worker pass: size={} generated={} added={} not_added={} chemistry_rejections={} edge_families={} point_families={} vertex_families={} water_families={} fluid_boundary_families={} total_formations={} elapsed_ms={}",
            self.formation_size,
            self.generated_candidates,
            self.added_formations,
            self.generated_candidates.saturating_sub(self.added_formations),
            self.chemistry_rejections,
            self.rigid_edge_families,
            self.rigid_point_families,
            self.rigid_vertex_families,
            self.water_families,
            self.fluid_boundary_families,
            self.total_formations,
            self.elapsed.as_millis(),
        )
    }
}

fn evaluate_formation_chemistry(
    formation: &GeometryFormation,
    catalog: &[BaseResource],
    chemistry_library: &mut ChemistryLibrary,
) -> std::io::Result<Result<(), String>> {
    let mut structure = OrganismStructure::new();
    for constituent in &formation.constituents {
        structure.add_unit(StructuralUnit::new(
            constituent.resource.clone(),
            constituent.placement,
        ));
    }

    let mut cache = ConnectionCompatibilityCache::new();

    for bond in &formation.bonds {
        let candidates = crate::combine::eligible_candidates(
            &structure,
            bond.constituent_a,
            bond.constituent_b,
            catalog,
            &mut cache,
        );
        if candidates.is_empty() {
            return Ok(Err(format!(
                "no realized physical contact for formation bond {}-{}",
                bond.constituent_a, bond.constituent_b
            )));
        }

        // A formation is accepted only when at least one realized contact
        // interface for every required bond is chemically realizable.
        let mut valid_interface = false;
        let mut last_rejection = None;

        for candidate in candidates {
            let unit_a = &structure.units[bond.constituent_a];
            let unit_b = &structure.units[bond.constituent_b];
            let properties_a = unit_a
                .properties(catalog)
                .ok_or_else(|| std::io::Error::other("missing resource properties"))?;
            let properties_b = unit_b
                .properties(catalog)
                .ok_or_else(|| std::io::Error::other("missing resource properties"))?;

            let interface = crate::geometry_reference_library::resolve_live_contact_interface(
                &formation.constituents[bond.constituent_a].resource,
                candidate.endpoint_a,
                &formation.constituents[bond.constituent_b].resource,
                candidate.endpoint_b,
            );
            let key = ChemistryKey::from_live_geometry(
                &formation.constituents[bond.constituent_a].resource,
                &formation.constituents[bond.constituent_b].resource,
                &interface,
            );

            if let Some(record) = chemistry_library.get(&key) {
                if record.state == ChemistryEvaluationState::Valid {
                    valid_interface = true;
                    break;
                }
                last_rejection = record.rejection_reason.clone();
                continue;
            }

            let Some(evaluation) = evaluate_static_chemical_interaction(
                properties_a.chemical_position,
                properties_b.chemical_position,
                properties_a.cohesion,
                properties_b.cohesion,
            ) else {
                let reason = "no defined static chemical interaction".to_string();
                chemistry_library.record_rejection(key, reason.clone())?;
                last_rejection = Some(reason);
                continue;
            };

            let formation_evaluation =
                evaluate_formation(candidate, properties_a.cohesion, properties_b.cohesion);
            if formation_cost(properties_a, properties_b, formation_evaluation).is_err() {
                let reason = "invalid chemical formation cost".to_string();
                chemistry_library.record_rejection(key, reason.clone())?;
                last_rejection = Some(reason);
                continue;
            }

            chemistry_library.record_valid_evaluation(
                key,
                evaluation.static_potential,
                evaluation.bond_strength,
            )?;
            valid_interface = true;
            break;
        }

        if !valid_interface {
            return Ok(Err(last_rejection.unwrap_or_else(|| {
                "no chemically realizable contact interface".to_string()
            })));
        }
    }

    Ok(Ok(()))
}

fn process_one_frontier(
    library: &mut GeometryLibrary,
    chemistry_library: &mut ChemistryLibrary,
    catalog: &[BaseResource],
) -> std::io::Result<Option<WorkerPassMetrics>> {
    // The library is already held in a BTreeMap keyed by canonical
    // signature. Do not clone and sort the entire catalogue on every worker
    // step; that turns catalogue growth itself into the hot path. We only
    // borrow the first formation with unfinished resource frontiers, process
    // its complete seven-resource row, then return so the next pass advances.
    let Some(formation) = library
        .formations()
        .filter(|formation| formation.constituents.len() < 20)
        .filter(|formation| {
            catalog.iter().any(|resource| {
                !matches!(
                    library
                        .frontier()
                        .records
                        .get(&format!("{}|{}", formation.signature, resource.name))
                        .map(|record| &record.state),
                    Some(GeometryFrontierState::Exhausted)
                        | Some(GeometryFrontierState::ContinuousFamilyPending)
                )
            })
        })
        .min_by(|a, b| {
            a.constituents
                .len()
                .cmp(&b.constituents.len())
                .then_with(|| a.signature.cmp(&b.signature))
        })
        .cloned()
    else {
        return Ok(None);
    };

    let started = Instant::now();
    let formation_size = formation.constituents.len();
    let mut metrics = WorkerPassMetrics {
        formation_size,
        ..WorkerPassMetrics::default()
    };

    {
        for resource in catalog {
            let key = format!("{}|{}", formation.signature, resource.name);
            let state = library
                .frontier()
                .records
                .get(&key)
                .map(|record| &record.state);
            if matches!(
                state,
                Some(GeometryFrontierState::Exhausted)
                    | Some(GeometryFrontierState::ContinuousFamilyPending)
            ) {
                continue;
            }

            library.set_frontier_state(
                formation.signature.clone(),
                resource.name.clone(),
                GeometryFrontierState::InProgress,
            )?;

            if resource.name == "Water" {
                // Every currently supported Water/rigid case is represented
                // symbolically as an exact capillary contact family. If no
                // family exists, the current boundary geometry has no valid
                // Water contact; it is exhausted rather than left in a
                // permanently "pending" state.
                let families = generate_water_contact_families(&formation, resource, catalog);
                metrics.water_families += families.len();
                library.insert_contact_families(families)?;
                let boundary_states =
                    generate_fluid_boundary_families(&formation, resource, catalog);
                metrics.fluid_boundary_families += boundary_states.len();
                library.insert_fluid_boundary_families(boundary_states)?;
                library.set_frontier_state(
                    formation.signature.clone(),
                    resource.name.clone(),
                    GeometryFrontierState::Exhausted,
                )?;
                continue;
            }

            let families = generate_rigid_contact_families(&formation, resource, catalog);
            metrics.rigid_edge_families += families.len();
            library.insert_rigid_contact_families(families)?;

            let point_families =
                generate_rigid_point_contact_families(&formation, resource, catalog);
            metrics.rigid_point_families += point_families.len();
            library.insert_rigid_point_contact_families(point_families)?;

            let vertex_families =
                generate_rigid_vertex_contact_families(&formation, resource, catalog);
            metrics.rigid_vertex_families += vertex_families.len();
            library.insert_rigid_vertex_contact_families(vertex_families)?;

            let candidates = expand_formation_candidates(&formation, resource, catalog);
            metrics.generated_candidates += candidates.len();
            let mut chemistry_valid = Vec::new();
            for candidate in candidates {
                let Some(canonical) = candidate.canonicalized(catalog) else {
                    continue;
                };
                if library.rejection(&canonical.signature).is_some() {
                    metrics.chemistry_rejections += 1;
                    continue;
                }
                match evaluate_formation_chemistry(&canonical, catalog, chemistry_library)? {
                    Ok(()) => chemistry_valid.push(canonical),
                    Err(reason) => {
                        library.insert_rejection(canonical.signature.clone(), reason)?;
                        metrics.chemistry_rejections += 1;
                    }
                }
            }
            metrics.added_formations += library.insert_many(chemistry_valid, catalog)?;

            library.set_frontier_state(
                formation.signature.clone(),
                resource.name.clone(),
                GeometryFrontierState::Exhausted,
            )?;
        }

        // All resource frontiers for this formation were handled in one pass.
        // The next worker pass advances to the next formation rather than
        // rebuilding and rescanning this same seven-resource row.
    }

    metrics.elapsed = started.elapsed();
    metrics.total_formations = library.len();
    Ok(Some(metrics))
}
