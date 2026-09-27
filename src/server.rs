use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::{get, post},
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
use crate::runtime::RuntimeState;
use crate::state::{AppState, Simulation};

pub(crate) fn start_tick_loop(runtime: Arc<Mutex<RuntimeState>>) {
    tokio::spawn(async move {
        loop {
            let tick_duration = {
                let state = runtime.lock();
                if !state.simulation.running {
                    Duration::from_millis(100)
                } else {
                    let tps = state.simulation.ticks_per_second.max(0.001);
                    Duration::from_secs_f64(1.0 / tps)
                }
            };

            tokio::time::sleep(tick_duration).await;

            let mut state = runtime.lock();
            if state.simulation.running {
                state.step();
            }
        }
    });
}

async fn index_handler() -> impl IntoResponse {
    let page = include_str!("../ui/index.html");
    let resource_visualization = include_str!("../ui/resource_visualization.js");
    let organism_inspector = include_str!("../ui/organism_inspector.js");
    Html(format!(
        "{page}\n<script>{resource_visualization}</script>\n<script>{organism_inspector}</script>"
    ))
}

#[derive(serde::Serialize)]
struct ObservationStatus {
    tick: u64,
    running: bool,
    ticks_per_second: f64,
    history_ticks: Vec<u64>,
    observed_tick: u64,
    session_id: String,
}

async fn observation_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    let runtime = state.runtime.lock();
    Json(ObservationStatus {
        tick: runtime.simulation.tick,
        running: runtime.simulation.running,
        ticks_per_second: runtime.simulation.ticks_per_second,
        history_ticks: runtime.history_ticks(),
        observed_tick: runtime.observed_tick(),
        session_id: runtime.session_id.clone(),
    })
}

async fn world_observation_handler(State(state): State<AppState>) -> impl IntoResponse {
    let runtime = state.runtime.lock();
    Json(ObservationProjection::world(
        WorldObservation::from_simulation(&runtime.simulation),
    ))
}

async fn organism_observation_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let runtime = state.runtime.lock();
    let Some(observation) = OrganismObservation::from_simulation(&runtime.simulation, &id) else {
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
    let runtime = state.runtime.lock();
    let Some(observation) = StructureObservation::from_simulation(&runtime.simulation, &id) else {
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

async fn resource_visualization_handler(State(state): State<AppState>) -> impl IntoResponse {
    let runtime = state.runtime.lock();
    let resources = runtime
        .simulation
        .environment
        .catalog
        .iter()
        .map(|resource| (resource.name.clone(), appearance(resource)))
        .collect::<Vec<_>>();
    Json(ResourceVisualizationObservation {
        resources,
        field_cell_size: runtime.simulation.environment.field.cell_size,
    })
}

async fn pause_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut runtime = state.runtime.lock();
    runtime.simulation.running = false;
    Json(serde_json::json!({"running": false}))
}

async fn resume_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut runtime = state.runtime.lock();
    runtime.simulation.running = true;
    Json(serde_json::json!({"running": true}))
}

async fn step_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut runtime = state.runtime.lock();
    runtime.step();
    Json(serde_json::json!({"tick": runtime.simulation.tick}))
}

async fn speed_handler(
    Path(ticks_per_second): Path<f64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !ticks_per_second.is_finite() || ticks_per_second <= 0.0 {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let mut runtime = state.runtime.lock();
    runtime.simulation.ticks_per_second = ticks_per_second;
    Json(serde_json::json!({"ticks_per_second": ticks_per_second})).into_response()
}

async fn restore_handler(
    Path(tick): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut runtime = state.runtime.lock();
    if !runtime.restore_tick(tick) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    Json(serde_json::json!({"tick": runtime.simulation.tick})).into_response()
}

pub(crate) async fn run() {
    let runtime = Arc::new(Mutex::new(RuntimeState::new(Simulation::new(42, 10.0))));
    let state = AppState {
        runtime: runtime.clone(),
    };

    start_tick_loop(runtime);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/observation/status", get(observation_status_handler))
        .route("/observation/world", get(world_observation_handler))
        .route(
            "/observation/organism/{id}",
            get(organism_observation_handler),
        )
        .route(
            "/observation/structure/{id}",
            get(structure_observation_handler),
        )
        .route(
            "/observation/resources",
            get(resource_visualization_handler),
        )
        .route("/control/pause", post(pause_handler))
        .route("/control/resume", post(resume_handler))
        .route("/control/step", post(step_handler))
        .route("/control/speed/{ticks_per_second}", post(speed_handler))
        .route("/control/restore/{tick}", post(restore_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let address = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on http://{address}");
    let listener = TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
