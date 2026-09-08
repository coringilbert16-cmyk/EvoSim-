use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use parking_lot::Mutex;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::observation::{
    ObservationContext, ObservationProjection, OrganismObservation, StructureObservation,
    WorldObservation,
};
use crate::resource_visualization::appearance;
use crate::state::{AppState, Simulation};

pub(crate) fn start_tick_loop(simulation: Arc<Mutex<Simulation>>) {
    tokio::spawn(async move {
        loop {
            let tick_duration = {
                let sim = simulation.lock();
                if !sim.running {
                    Duration::from_millis(100)
                } else {
                    let tps = sim.ticks_per_second.max(0.001);
                    Duration::from_secs_f64(1.0 / tps)
                }
            };

            tokio::time::sleep(tick_duration).await;
            let mut sim = simulation.lock();
            sim.step();
        }
    });
}

async fn index_handler() -> impl IntoResponse {
    let page = include_str!("../ui/index.html");
    let resource_visualization = include_str!("../ui/resource_visualization.js");
    Html(format!(
        "{page}\n<script>{resource_visualization}</script>"
    ))
}

#[derive(serde::Serialize)]
struct ObservationStatus {
    tick: u64,
    running: bool,
}

async fn observation_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    let simulation = state.simulation.lock();
    Json(ObservationStatus {
        tick: simulation.tick,
        running: simulation.running,
    })
}

async fn world_observation_handler(State(state): State<AppState>) -> impl IntoResponse {
    let simulation = state.simulation.lock();
    Json(ObservationProjection::world(WorldObservation::from_simulation(
        &simulation,
    )))
}

async fn organism_observation_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let simulation = state.simulation.lock();
    let Some(observation) = OrganismObservation::from_simulation(&simulation, &id) else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };
    let context = ObservationContext::organism(vec![id]);
    Json(ObservationProjection::organism(context, observation).expect("validated level"))
        .into_response()
}

async fn structure_observation_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let simulation = state.simulation.lock();
    let Some(observation) = StructureObservation::from_simulation(&simulation, &id) else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };
    let context = ObservationContext::structure(vec![id.clone()], Some(id));
    Json(ObservationProjection::structure(context, observation).expect("validated level"))
        .into_response()
}

#[derive(serde::Serialize)]
struct ResourceVisualizationObservation {
    resources: Vec<(String, crate::resource_visualization::ResourceAppearance)>,
    field_cell_size: f64,
}

async fn resource_visualization_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let simulation = state.simulation.lock();
    let resources = simulation
        .environment
        .catalog
        .iter()
        .map(|resource| (resource.name.clone(), appearance(resource)))
        .collect::<Vec<_>>();
    Json(ResourceVisualizationObservation {
        resources,
        field_cell_size: simulation.environment.field.cell_size,
    })
}

pub(crate) async fn run() {
    let simulation = Arc::new(Mutex::new(Simulation::new(42, 10.0)));

    let state = AppState {
        simulation: simulation.clone(),
    };

    start_tick_loop(simulation);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/observation/status", get(observation_status_handler))
        .route("/observation/world", get(world_observation_handler))
        .route("/observation/organism/{id}", get(organism_observation_handler))
        .route("/observation/structure/{id}", get(structure_observation_handler))
        .route("/observation/resources", get(resource_visualization_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", address);
    let listener = TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
