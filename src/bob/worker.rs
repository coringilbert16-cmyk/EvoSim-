use crate::geometry_reference_library::{
    expand_formation_candidates, generate_fluid_boundary_families_from_contact_families,
    generate_rigid_contact_families, generate_rigid_point_contact_families,
    generate_rigid_vertex_contact_families, generate_water_contact_families, open_default_library,
    seed_base_catalogue, GeometryFrontierState, GeometryLibrary,
};
use crate::resources::{default_catalog, BaseResource};
use std::thread;
use std::time::{Duration, Instant};

const IDLE_SLEEP: Duration = Duration::from_secs(1);

/// Bob catalogs useful local assemblies, not whole organisms. Keep clean builds bounded.
const MAX_LIBRARY_CONSTITUENTS: usize = 20;

pub fn run() {
    run_continuously_with_limit(MAX_LIBRARY_CONSTITUENTS);
}

/// Run continuously while limiting generated formations to max_constituents.
/// Existing worker invocations preserve the historical 20-constituent default.
pub fn run_continuously_with_limit(max_constituents: usize) {
    assert!(max_constituents > 0, "maximum constituents must be positive");
    let catalog = default_catalog();
    let mut library = open_default_library().expect("geometry library must open");
    seed_base_catalogue(&mut library, &catalog).expect("geometry library seed must succeed");

    loop {
        match process_one_frontier(&mut library, &catalog, max_constituents)
            .expect("geometry worker failed")
        {
            Some(metrics) => eprintln!("{metrics}"),
            None => thread::sleep(IDLE_SLEEP),
        }
    }
}

/// Exhaust every durable frontier at or below max_constituents, then reopen
/// the store and verify that generation stayed within the requested bound.
/// Unlike run_passes, this terminates only when no eligible frontier remains.
pub fn run_to_completion(max_constituents: usize) -> std::io::Result<()> {
    if max_constituents == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "maximum constituents must be positive",
        ));
    }

    let catalog = default_catalog();
    let mut library = open_default_library()?;
    let seeded = seed_base_catalogue(&mut library, &catalog)?;
    let started = Instant::now();
    let mut processed = 0usize;
    let mut generated = 0usize;
    let mut added = 0usize;
    let mut edge_added = 0usize;
    let mut point_added = 0usize;
    let mut vertex_added = 0usize;
    let mut water_added = 0usize;
    let mut fluid_boundary_added = 0usize;

    while let Some(metrics) = process_one_frontier(&mut library, &catalog, max_constituents)? {
        eprintln!("{metrics}");
        processed += 1;
        generated += metrics.generated_candidates;
        added += metrics.added_formations;
        edge_added += metrics.rigid_edge_families_added;
        point_added += metrics.rigid_point_families_added;
        vertex_added += metrics.rigid_vertex_families_added;
        water_added += metrics.water_families_added;
        fluid_boundary_added += metrics.fluid_boundary_families_added;
    }

    let formations_before_reopen = library.len();
    drop(library);

    let reopened = open_default_library()?;
    let oversized = reopened
        .formations()
        .filter(|formation| formation.constituents.len() > max_constituents)
        .count();
    let unfinished = reopened
        .frontier()
        .records
        .values()
        .filter(|record| {
            !matches!(
                &record.state,
                GeometryFrontierState::Exhausted | GeometryFrontierState::ContinuousFamilyPending
            )
        })
        .count();

    eprintln!(
        "geometry worker complete: max_constituents={max_constituents} seeded={seeded} passes={processed} generated_candidates={generated} added_formations={added} not_added={} edge_families_added={edge_added} point_families_added={point_added} vertex_families_added={vertex_added} water_families_added={water_added} fluid_boundary_families_added={fluid_boundary_added} formations_before_reopen={formations_before_reopen} formations_after_reopen={} oversized_formations={oversized} unfinished_frontiers={unfinished} elapsed_ms={}",
        generated.saturating_sub(added),
        reopened.len(),
        started.elapsed().as_millis(),
    );

    if oversized > 0 || unfinished > 0 || reopened.len() != formations_before_reopen {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "bounded generation verification failed: oversized={oversized}, unfinished={unfinished}, before_reopen={formations_before_reopen}, after_reopen={}",
                reopened.len()
            ),
        ));
    }
    Ok(())
}

/// Process exactly one durable frontier pass and exit.
///
/// This is the safe smoke-test entry point: it exercises the same persistent
/// store, seeding, candidate generation, validation, and frontier writes as the
/// continuous worker without starting an unbounded process.
pub fn run_once() -> std::io::Result<bool> {
    let catalog = default_catalog();
    let mut library = open_default_library()?;
    let seeded = seed_base_catalogue(&mut library, &catalog)?;
    let metrics = process_one_frontier(&mut library, &catalog, MAX_LIBRARY_CONSTITUENTS)?;
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

/// Process at most `max_passes` formation frontiers without reopening the
/// catalogue between passes. This is useful for bounded generation benchmarks.
pub fn run_passes(max_passes: usize) -> std::io::Result<usize> {
    let catalog = default_catalog();
    let mut library = open_default_library()?;
    let seeded = seed_base_catalogue(&mut library, &catalog)?;
    let started = Instant::now();
    let mut processed = 0usize;

    for _ in 0..max_passes {
        match process_one_frontier(&mut library, &catalog, MAX_LIBRARY_CONSTITUENTS)? {
            Some(metrics) => {
                eprintln!("{metrics}");
                processed += 1;
            }
            None => {
                eprintln!("geometry worker passes: frontier idle; stopping early");
                break;
            }
        }
    }

    eprintln!(
        "geometry worker passes complete: processed={processed} requested={max_passes} seeded={seeded} total_formations={} elapsed_ms={}",
        library.len(),
        started.elapsed().as_millis()
    );
    Ok(processed)
}

#[derive(Default)]
struct WorkerPassMetrics {
    formation_size: usize,
    generated_candidates: usize,
    added_formations: usize,
    rigid_edge_families: usize,
    rigid_edge_families_added: usize,
    rigid_point_families: usize,
    rigid_point_families_added: usize,
    rigid_vertex_families: usize,
    rigid_vertex_families_added: usize,
    water_families: usize,
    water_families_added: usize,
    fluid_boundary_families: usize,
    fluid_boundary_families_added: usize,
    elapsed: Duration,
    total_formations: usize,
    expansion_limit_reached: bool,
}

impl std::fmt::Display for WorkerPassMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "geometry worker pass: size={} generated={} added={} not_added={} edge_families={} edge_added={} point_families={} point_added={} vertex_families={} vertex_added={} water_families={} water_added={} fluid_boundary_families={} fluid_boundary_added={} total_formations={} expansion_limit_reached={} elapsed_ms={}",
            self.formation_size,
            self.generated_candidates,
            self.added_formations,
            self.generated_candidates.saturating_sub(self.added_formations),
            self.rigid_edge_families,
            self.rigid_edge_families_added,
            self.rigid_point_families,
            self.rigid_point_families_added,
            self.rigid_vertex_families,
            self.rigid_vertex_families_added,
            self.water_families,
            self.water_families_added,
            self.fluid_boundary_families,
            self.fluid_boundary_families_added,
            self.total_formations,
            self.expansion_limit_reached,
            self.elapsed.as_millis(),
        )
    }
}

fn process_one_frontier(
    library: &mut GeometryLibrary,
    catalog: &[BaseResource],
    max_constituents: usize,
) -> std::io::Result<Option<WorkerPassMetrics>> {
    // The library is already held in a BTreeMap keyed by canonical
    // signature. Do not clone and sort the entire catalogue on every worker
    // step; that turns catalogue growth itself into the hot path. We only
    // borrow the first formation with unfinished resource frontiers, process
    // its complete seven-resource row, then return so the next pass advances.
    let Some(formation) = library
        .formations()
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
        let mut completed_frontiers = Vec::with_capacity(catalog.len());
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

            if resource.name == "Water" {
                // Every currently supported Water/rigid case is represented
                // symbolically as an exact capillary contact family. If no
                // family exists, the current boundary geometry has no valid
                // Water contact; it is exhausted rather than left in a
                // permanently "pending" state.
                let families = generate_water_contact_families(&formation, resource, catalog);
                metrics.water_families += families.len();
                let boundary_states =
                    generate_fluid_boundary_families_from_contact_families(resource, &families);
                metrics.fluid_boundary_families += boundary_states.len();
                metrics.water_families_added += library.insert_contact_families(families)?;
                metrics.fluid_boundary_families_added +=
                    library.insert_fluid_boundary_families(boundary_states)?;
                completed_frontiers.push((
                    formation.signature.clone(),
                    resource.name.clone(),
                    GeometryFrontierState::Exhausted,
                ));
                continue;
            }

            let families = generate_rigid_contact_families(&formation, resource, catalog);
            metrics.rigid_edge_families += families.len();
            metrics.rigid_edge_families_added += library.insert_rigid_contact_families(families)?;

            let point_families =
                generate_rigid_point_contact_families(&formation, resource, catalog);
            metrics.rigid_point_families += point_families.len();
            metrics.rigid_point_families_added +=
                library.insert_rigid_point_contact_families(point_families)?;

            let vertex_families =
                generate_rigid_vertex_contact_families(&formation, resource, catalog);
            metrics.rigid_vertex_families += vertex_families.len();
            metrics.rigid_vertex_families_added +=
                library.insert_rigid_vertex_contact_families(vertex_families)?;

            if formation.constituents.len() < max_constituents {
                let candidates = expand_formation_candidates(&formation, resource, catalog);
                metrics.generated_candidates += candidates.len();
                metrics.added_formations += library.insert_many(candidates, catalog)?;
            } else {
                // Still record local contact knowledge for formations at the
                // limit, but do not create 21-constituent rows from a 20-unit
                // library target. This makes a fresh catalogue finite.
                metrics.expansion_limit_reached = true;
            }

            completed_frontiers.push((
                formation.signature.clone(),
                resource.name.clone(),
                GeometryFrontierState::Exhausted,
            ));
        }

        // Persist the complete formation frontier once. If interrupted before
        // this checkpoint, durable results are safe to regenerate because all
        // insert paths deduplicate by canonical identity.
        library.set_frontier_states(completed_frontiers)?;
    }

    metrics.elapsed = started.elapsed();
    metrics.total_formations = library.len();
    Ok(Some(metrics))
}
