use crate::geometry_reference_library::{
    expand_formation_candidates, open_default_library, GeometryFrontierState, GeometryLibrary,
};
use crate::resources::{default_catalog, BaseResource};
use std::thread;
use std::time::Duration;

const IDLE_SLEEP: Duration = Duration::from_secs(1);

pub fn run() {
    let catalog = default_catalog();
    let mut library = open_default_library().expect("geometry library must open");

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
    let mut formations: Vec<_> = library
        .formations()
        .filter(|formation| formation.constituents.len() < 20)
        .cloned()
        .collect();
    formations.sort_by(|a, b| {
        a.constituents
            .len()
            .cmp(&b.constituents.len())
            .then_with(|| a.signature.cmp(&b.signature))
    });

    for formation in formations {
        for resource in catalog {
            let key = format!("{}|{}", formation.signature, resource.name);
            let state = library.frontier().records.get(&key).map(|record| &record.state);
            if matches!(state, Some(GeometryFrontierState::Exhausted)) {
                continue;
            }

            library.set_frontier_state(
                formation.signature.clone(),
                resource.name.clone(),
                GeometryFrontierState::InProgress,
            )?;

            let candidates = expand_formation_candidates(&formation, resource, catalog);
            for candidate in candidates {
                library.insert(candidate, catalog)?;
            }

            library.set_frontier_state(
                formation.signature.clone(),
                resource.name.clone(),
                GeometryFrontierState::Exhausted,
            )?;
            return Ok(true);
        }
    }

    Ok(false)
}
