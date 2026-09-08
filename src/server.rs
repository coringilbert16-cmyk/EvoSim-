use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use crate::observation::{
    ObservationContext, ObservationProjection, OrganismObservation, StructureObservation,
    WorldObservation,
};
use crate::resource_visualization::appearance;
use crate::state::{AppState, Simulation};

pub(crate) fn start_tick_loop(
    simulation: Arc<Mutex<Simulation>>,
    broadcaster: broadcast::Sender<String>,
) {
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
            let snapshot = {
                let mut sim = simulation.lock();
                sim.step()
            };

            if let Ok(json) = serde_json::to_string(&snapshot) {
                let _ = broadcaster.send(json);
            }
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

async fn snapshot_handler(State(state): State<AppState>) -> impl IntoResponse {
    let simulation = state.simulation.lock();
    Json(simulation.snapshot())
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

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.broadcaster.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            if sender.send(Message::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    let mut receive_task =
        tokio::spawn(async move { while let Some(Ok(_message)) = receiver.next().await {} });

    tokio::select! {
        _ = (&mut send_task) => receive_task.abort(),
        _ = (&mut receive_task) => send_task.abort(),
    }
}

pub(crate) async fn run() {
    let (tx, _rx) = broadcast::channel::<String>(128);
    let simulation = Arc::new(Mutex::new(Simulation::new(42, 10.0)));

    let state = AppState {
        simulation: simulation.clone(),
        broadcaster: tx.clone(),
    };

    start_tick_loop(simulation, tx);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/snapshot", get(snapshot_handler))
        .route("/observation/world", get(world_observation_handler))
        .route("/observation/organism/{id}", get(organism_observation_handler))
        .route("/observation/structure/{id}", get(structure_observation_handler))
        .route("/observation/resources", get(resource_visualization_handler))
        .route("/ws", get(ws_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", address);
    let listener = TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
