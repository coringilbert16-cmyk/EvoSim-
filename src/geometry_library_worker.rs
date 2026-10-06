use crate::geometry_reference_library::{
    expand_formation_candidates, generate_rigid_contact_families, generate_rigid_point_contact_families,
    generate_rigid_vertex_contact_families, generate_water_contact_families, open_default_library,
    seed_base_catalogue, GeometryFrontierState, GeometryLibrary,
};
use crate::resources::{default_catalog, BaseResource};
use std::thread;
use std::time::Duration;

const IDLE_SLEEP: Duration = Duration::from_secs(1);

pub fn run() {
    let catalog = default_catalog();
    let mut library = open_default_library().expect("geometry library must open");
    seed_base_catalogue(&mut library, &catalog).expect("geometry library seed must succeed");

    loop {
        if !process_one_frontier(&mut library, &catalog).expect("geometry worker failed") {
            thread::sleep(IDLE_SLEEP);
        }
    }
}

fn process_one_frontier(
    library: &mut GeometryLibrary,
    catalog: &[BaseResource],
) -> std::io::Result<bool> {
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
                    library.frontier().records.get(&format!("{}|{}", formation.signature, resource.name)).map(|record| &record.state),
                    Some(GeometryFrontierState::Exhausted)
                        | Some(GeometryFrontierState::ContinuousFamilyPending)
                )
            })
        })
        .min_by(|a, b| {
            a.constituents.len()
                .cmp(&b.constituents.len())
                .then_with(|| a.signature.cmp(&b.signature))
        })
        .cloned()
    else {
        return Ok(false);
    };

    {
        for resource in catalog {
            let key = format!("{}|{}", formation.signature, resource.name);
            let state = library.frontier().records.get(&key).map(|record| &record.state);
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
                library.insert_contact_families(families)?;
                library.set_frontier_state(
                    formation.signature.clone(),
                    resource.name.clone(),
                    GeometryFrontierState::Exhausted,
                )?;
                continue;
            }

            let families = generate_rigid_contact_families(&formation, resource, catalog);
            library.insert_rigid_contact_families(families)?;

            let point_families = generate_rigid_point_contact_families(&formation, resource, catalog);
            library.insert_rigid_point_contact_families(point_families)?;

            let vertex_families = generate_rigid_vertex_contact_families(&formation, resource, catalog);
            library.insert_rigid_vertex_contact_families(vertex_families)?;

            let candidates = expand_formation_candidates(&formation, resource, catalog);
            library.insert_many(candidates, catalog)?;

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

    Ok(true)
}

