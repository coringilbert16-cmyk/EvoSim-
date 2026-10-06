use crate::capillary_geometry::solve_water_against_solid;
use crate::geometry_reference_library::{
    expand_formation_candidates, generate_water_contact_families, open_default_library,
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
                return Ok(true);
            }

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
        if formation.constituents.len() < 20 {
            return Ok(true);
        }
    }

    Ok(false)
}


fn continuous_contact_family_exists(
    formation: &crate::geometry_reference_library::GeometryFormation,
    candidate: &BaseResource,
    catalog: &[BaseResource],
) -> bool {
    // A fluid circle is no longer treated as an unexplained infinite search.
    // Its continuous placement family is now recognized through the exact
    // capillary solution. The worker still defers persistence of that family
    // until the boundary-feature representation can carry the solution.
    if candidate.name != "Water"
        || candidate.physical_state != crate::resources::PhysicalState::Fluid
    {
        return false;
    }

    let Some(water) = catalog.iter().find(|resource| resource.name == "Water") else {
        return false;
    };

    let area = match water.shape.form {
        crate::resources::Form::Circle { radius } => std::f64::consts::PI * radius * radius,
        crate::resources::Form::Fluid { nominal_area, .. } => nominal_area,
        _ => return false,
    };

    formation.constituents.iter().any(|constituent| {
        let Some(resource) = catalog.iter().find(|resource| resource.name == constituent.resource)
        else {
            return false;
        };
        if resource.physical_state == crate::resources::PhysicalState::Fluid {
            return false;
        }

        let Some(vertices) = resource.shape.form.polygon_vertices() else {
            return false;
        };
        let Some(family) = solve_water_against_solid(
            area,
            water.properties.cohesion,
            resource.properties.cohesion,
        ) else {
            return false;
        };
        (0..vertices.len()).any(|edge| {
            let a = vertices[edge];
            let b = vertices[(edge + 1) % vertices.len()];
            let edge_length = (b.0 - a.0).hypot(b.1 - a.1);
            edge_length + 1e-12 >= family.contact_length
        })
    })
}
