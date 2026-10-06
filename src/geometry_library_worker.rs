use crate::geometry_reference_library::{
    expand_formation_candidates, open_default_library, seed_base_catalogue, GeometryFrontierState,
    GeometryLibrary,
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

            if continuous_contact_family_exists(&formation, resource) {
                library.set_frontier_state(
                    formation.signature.clone(),
                    resource.name.clone(),
                    GeometryFrontierState::ContinuousFamilyPending,
                )?;
                return Ok(true);
            }

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


fn continuous_contact_family_exists(
    formation: &crate::geometry_reference_library::GeometryFormation,
    candidate: &BaseResource,
) -> bool {
    let catalog = default_catalog();
    let is_circle = |resource: &BaseResource| {
        matches!(resource.shape.form, crate::resources::Form::Circle { .. })
    };
    let candidate_is_circle = is_circle(candidate);
    if !candidate_is_circle
        && !formation.constituents.iter().any(|constituent| {
            catalog.iter()
                .find(|resource| resource.name == constituent.resource)
                .map(is_circle)
                .unwrap_or(false)
        })
    {
        return false;
    }

    if candidate_is_circle {
        return formation.constituents.iter().any(|constituent| {
            catalog.iter()
                .find(|resource| resource.name == constituent.resource)
                .map(|resource| resource.physical_state != crate::resources::PhysicalState::Fluid)
                .unwrap_or(false)
        });
    }

    formation.constituents.iter().any(|constituent| {
        catalog.iter()
            .find(|resource| resource.name == constituent.resource)
            .map(|resource| is_circle(resource))
            .unwrap_or(false)
    })
}
