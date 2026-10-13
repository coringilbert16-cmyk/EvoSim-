use crate::geometry_reference_library::{
    expand_formation_candidates, fluid_formations_from_boundaries,
    generate_collinear_multi_contact_formations,
    generate_fluid_boundary_families_from_contact_families, generate_rigid_contact_families,
    generate_rigid_point_contact_families, generate_rigid_vertex_contact_families,
    generate_water_contact_families_with_diagnostics, open_default_library, seed_base_catalogue,
    GeometryFrontierState, GeometryLibrary,
};
use crate::resources::{default_catalog, BaseResource};
use std::thread;
use std::time::{Duration, Instant};

const IDLE_SLEEP: Duration = Duration::from_secs(1);

/// Bob catalogs useful local assemblies, not whole organisms. Keep clean builds bounded.
const MAX_LIBRARY_CONSTITUENTS: usize = 3;

pub fn run() {
    run_continuously_with_limit(MAX_LIBRARY_CONSTITUENTS);
}

/// Run continuously while limiting generated formations to max_constituents.
/// Bob is intentionally bounded to singleton, two-piece, and three-piece local formations.
/// Reject wider limits rather than silently expanding repository storage.
pub fn run_continuously_with_limit(max_constituents: usize) {
    assert!(
        (1..=MAX_LIBRARY_CONSTITUENTS).contains(&max_constituents),
        "Bob's maximum constituent count is fixed at 3"
    );
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
    if !(1..=MAX_LIBRARY_CONSTITUENTS).contains(&max_constituents) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Bob's maximum constituent count is fixed at 3",
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
    let mut fluid_formations_added = 0usize;
    let mut multi_contact_fluid_formations = 0usize;
    let mut multi_contact_fluid_formations_added = 0usize;

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
        fluid_formations_added += metrics.fluid_formations_added;
        multi_contact_fluid_formations += metrics.multi_contact_fluid_formations;
        multi_contact_fluid_formations_added += metrics.multi_contact_fluid_formations_added;
    }

    let formations_before_reopen = library.len();
    let fluid_formations_before_reopen = library.fluid_formations().count();
    drop(library);

    let reopened = open_default_library()?;
    let fluid_formations_after_reopen = reopened.fluid_formations().count();
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
        "geometry worker complete: max_constituents={max_constituents} seeded={seeded} passes={processed} generated_candidates={generated} added_formations={added} not_added={} edge_families_added={edge_added} point_families_added={point_added} vertex_families_added={vertex_added} water_families_added={water_added} fluid_boundary_families_added={fluid_boundary_added} fluid_formations_added={fluid_formations_added} multi_contact_fluid_formations={multi_contact_fluid_formations} multi_contact_fluid_formations_added={multi_contact_fluid_formations_added} multi_contact_not_added={} fluid_formations_before_reopen={fluid_formations_before_reopen} fluid_formations_after_reopen={fluid_formations_after_reopen} formations_before_reopen={formations_before_reopen} formations_after_reopen={} oversized_formations={oversized} unfinished_frontiers={unfinished} elapsed_ms={}",
        generated.saturating_sub(added),
        multi_contact_fluid_formations.saturating_sub(multi_contact_fluid_formations_added),
        reopened.len(),
        started.elapsed().as_millis(),
    );

    if oversized > 0
        || unfinished > 0
        || reopened.len() != formations_before_reopen
        || fluid_formations_after_reopen != fluid_formations_before_reopen
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "bounded generation verification failed: oversized={oversized}, unfinished={unfinished}, before_reopen={formations_before_reopen}, after_reopen={}, fluid_before_reopen={fluid_formations_before_reopen}, fluid_after_reopen={fluid_formations_after_reopen}",
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
    water_frontiers_without_boundary_candidates: usize,
    water_unresolved_anchor_resources: usize,
    water_unsupported_anchor_geometry: usize,
    water_capillary_solver_failures: usize,
    water_anchors_without_exposed_intervals: usize,
    water_invalid_contact_intervals: usize,
    water_anchors_without_valid_contact_overlap: usize,
    fluid_formations: usize,
    fluid_formations_added: usize,
    multi_contact_fluid_formations: usize,
    multi_contact_fluid_formations_added: usize,
    elapsed: Duration,
    total_formations: usize,
    expansion_limit_reached: bool,
}

impl std::fmt::Display for WorkerPassMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "geometry worker pass: size={} generated={} added={} not_added={} edge_families={} edge_added={} point_families={} point_added={} vertex_families={} vertex_added={} water_families={} water_added={} fluid_boundary_families={} fluid_boundary_added={} water_frontiers_without_boundary_candidates={} water_unresolved_anchor_resources={} water_unsupported_anchor_geometry={} water_capillary_solver_failures={} water_anchors_without_exposed_intervals={} water_invalid_contact_intervals={} water_anchors_without_valid_contact_overlap={} fluid_formations={} fluid_formations_added={} multi_contact_fluid_formations={} multi_contact_fluid_formations_added={} multi_contact_fluid_formations_not_added={} total_formations={} expansion_limit_reached={} elapsed_ms={}",
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
            self.water_frontiers_without_boundary_candidates,
            self.water_unresolved_anchor_resources,
            self.water_unsupported_anchor_geometry,
            self.water_capillary_solver_failures,
            self.water_anchors_without_exposed_intervals,
            self.water_invalid_contact_intervals,
            self.water_anchors_without_valid_contact_overlap,
            self.fluid_formations,
            self.fluid_formations_added,
            self.multi_contact_fluid_formations,
            self.multi_contact_fluid_formations_added,
            self.multi_contact_fluid_formations.saturating_sub(self.multi_contact_fluid_formations_added),
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
    if !(1..=MAX_LIBRARY_CONSTITUENTS).contains(&max_constituents) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Bob's maximum constituent count is fixed at 3",
        ));
    }

    // The library is already held in a BTreeMap keyed by canonical
    // signature. Do not clone and sort the entire catalogue on every worker
    // step; that turns catalogue growth itself into the hot path. We only
    // borrow the first formation with unfinished resource frontiers, process
    // its complete seven-resource row, then return so the next pass advances.
    let Some(formation) = library
        .formations()
        .filter(|formation| formation.constituents.len() <= max_constituents)
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
                // Persist solved Water/rigid contact families and aggregate
                // per-anchor outcomes so empty results are actionable.
                let (families, diagnostics) =
                    generate_water_contact_families_with_diagnostics(&formation, resource, catalog);
                metrics.water_unresolved_anchor_resources +=
                    diagnostics.unresolved_anchor_resources;
                metrics.water_unsupported_anchor_geometry +=
                    diagnostics.unsupported_anchor_geometry;
                metrics.water_capillary_solver_failures += diagnostics.capillary_solver_failures;
                metrics.water_anchors_without_exposed_intervals +=
                    diagnostics.anchors_without_exposed_intervals;
                metrics.water_invalid_contact_intervals += diagnostics.invalid_contact_intervals;
                metrics.water_anchors_without_valid_contact_overlap +=
                    diagnostics.anchors_without_valid_contact_overlap;
                metrics.water_families += families.len();
                let boundary_states =
                    generate_fluid_boundary_families_from_contact_families(resource, &families);
                metrics.fluid_boundary_families += boundary_states.len();
                if boundary_states.is_empty() {
                    metrics.water_frontiers_without_boundary_candidates += 1;
                }
                // The configured limit bounds rigid parent formations, not the
                // additional deformable fluid body. Generate fluid records for
                // every eligible parent at or below the limit, including parents
                // exactly at max_constituents.
                let fluid_formations = fluid_formations_from_boundaries(&boundary_states);
                let multi_contact_formations =
                    generate_collinear_multi_contact_formations(&formation, resource, catalog);
                metrics.fluid_formations += fluid_formations.len() + multi_contact_formations.len();
                metrics.multi_contact_fluid_formations += multi_contact_formations.len();
                metrics.water_families_added += library.insert_contact_families(families)?;
                metrics.fluid_boundary_families_added +=
                    library.insert_fluid_boundary_families(boundary_states)?;
                let (fluid_added, multi_contact_added) = library
                    .insert_fluid_formations_with_multi_contact_counts(
                        fluid_formations,
                        multi_contact_formations,
                        catalog,
                    )?;
                metrics.fluid_formations_added += fluid_added;
                metrics.multi_contact_fluid_formations_added += multi_contact_added;
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
                // limit. This makes a fresh catalogue finite.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_reference_library::GeometryFormation;

    #[test]
    fn worker_generates_fluid_records_for_parent_at_constituent_limit() {
        let catalog = default_catalog();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let seed = GeometryFormation::single("Carbon");
        let parent = expand_formation_candidates(&seed, carbon, &catalog)
            .into_iter()
            .find(|formation| formation.constituents.len() == 2)
            .expect("Bob should produce at least one valid two-constituent Carbon parent");
        let signature = parent.signature.clone();
        let root =
            std::env::temp_dir().join(format!("evosim-bob-fluid-at-limit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        library.insert(parent, &catalog).unwrap();

        let metrics = process_one_frontier(&mut library, &catalog, 2)
            .unwrap()
            .expect("the two-constituent parent should have an unfinished frontier");
        assert!(metrics.fluid_formations > 0);
        assert!(
            library.fluid_formations().any(|formation| {
                formation.parent_formation_signature == signature
                    && formation.fluid_resource == "Water"
            }),
            "the parent at the rigid-constituent limit must still receive fluid formations"
        );
        drop(library);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn worker_accepts_three_piece_limit_and_rejects_wider_limits() {
        let error = run_to_completion(4).expect_err("Bob must reject a four-piece limit");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

        let catalog = default_catalog();
        let root = std::env::temp_dir().join(format!(
            "evosim-bob-three-piece-limit-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let seed = GeometryFormation::single("Carbon");
        let parent = expand_formation_candidates(&seed, carbon, &catalog)
            .into_iter()
            .find(|formation| formation.constituents.len() == 2)
            .expect("Bob should produce a valid two-piece parent");
        library.insert(parent, &catalog).unwrap();

        let first_pass = process_one_frontier(&mut library, &catalog, 3)
            .unwrap()
            .expect("the two-piece parent should have work remaining");
        assert!(first_pass.added_formations > 0);
        assert!(library
            .formations()
            .any(|formation| formation.constituents.len() == 3));
        assert!(library
            .formations()
            .all(|formation| formation.constituents.len() <= 3));

        let three_piece = library
            .formations()
            .find(|formation| formation.constituents.len() == 3)
            .expect("the first pass should create three-piece formations")
            .signature
            .clone();
        let second_pass = process_one_frontier(&mut library, &catalog, 3)
            .unwrap()
            .expect("a three-piece formation should have its own contact frontier");
        assert_eq!(second_pass.formation_size, 3);
        assert!(second_pass.expansion_limit_reached);
        assert!(library.get(&three_piece).is_some());
        assert!(library
            .formations()
            .all(|formation| formation.constituents.len() <= 3));

        let error = process_one_frontier(&mut library, &catalog, 4)
            .err()
            .expect("frontier processing must reject a four-piece limit");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        drop(library);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn worker_persists_water_inclusive_formations_and_reopens_them() {
        let catalog = default_catalog();
        let root =
            std::env::temp_dir().join(format!("evosim-bob-water-worker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut library = GeometryLibrary::open(&root, &catalog).unwrap();
        let carbon = GeometryFormation::single("Carbon");
        let carbon_signature = carbon.signature.clone();
        library.insert(carbon, &catalog).unwrap();

        let metrics = process_one_frontier(&mut library, &catalog, 2)
            .unwrap()
            .expect("the seeded Carbon formation must have an unfinished frontier");
        assert!(metrics.fluid_formations > 0);
        assert!(metrics.fluid_formations_added > 0);
        assert!(library.fluid_formations().all(|formation| {
            formation.parent_formation_signature == carbon_signature
                && formation.fluid_resource == "Water"
        }));
        drop(library);

        let reopened = GeometryLibrary::open(&root, &catalog).unwrap();
        let persisted_signatures = reopened
            .fluid_formations()
            .map(|formation| formation.signature().to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(!persisted_signatures.is_empty());
        assert!(reopened.fluid_formations().all(|formation| {
            formation.parent_formation_signature == carbon_signature
                && formation.fluid_resource == "Water"
        }));

        let parent = reopened
            .get(&carbon_signature)
            .expect("the canonical parent formation must survive reopening");
        let (realized_shape, realized_pose) = reopened
            .fluid_formations_for_parent(&carbon_signature, "Water")
            .find_map(|suggestion| suggestion.realize_against_parent(parent, &catalog, true))
            .expect("a generated, reopened Water formation must realize against its parent");
        assert!(realized_shape.is_valid());
        assert!(matches!(
            realized_shape.form,
            crate::resources::Form::Fluid {
                nominal_area,
                boundary: None,
                arc_boundary: Some(_), ..} if nominal_area.is_finite() && nominal_area > 0.0
        ));
        assert!(realized_pose.x.is_finite());
        assert!(realized_pose.y.is_finite());
        assert!(realized_pose.rotation_radians.is_finite());

        let mut resumed = reopened;
        let _ = process_one_frontier(&mut resumed, &catalog, 2).unwrap();
        let resumed_signatures = resumed
            .fluid_formations()
            .map(|formation| formation.signature().to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(persisted_signatures.is_subset(&resumed_signatures));
        assert_eq!(
            resumed_signatures.len(),
            resumed.fluid_formations().count(),
            "resuming work must not create duplicate fluid formation signatures"
        );
        drop(resumed);
        let _ = std::fs::remove_dir_all(root);
    }
}
