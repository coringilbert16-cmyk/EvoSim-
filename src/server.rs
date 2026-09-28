use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use tokio::io::AsyncBufReadExt;
use tower_http::cors::CorsLayer;

use crate::runtime::SimulationProcess;
use crate::state::AppState;

async fn request(state: &AppState, command: Value) -> Result<Value, &'static str> {
    let mut process = state.simulation.lock().await;
    process.request(command).await
}

fn error_response(error: &'static str) -> axum::response::Response {
    match error {
        "not_found" => axum::http::StatusCode::NOT_FOUND.into_response(),
        "invalid_speed" | "invalid_command" => axum::http::StatusCode::BAD_REQUEST.into_response(),
        _ => axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn index_handler() -> impl IntoResponse {
    let page = include_str!("../ui/index.html");
    let resource_visualization = include_str!("../ui/resource_visualization.js");
    let organism_inspector = include_str!("../ui/organism_inspector.js");
    Html(format!(
        "{page}\n<script>{resource_visualization}</script>\n<script>{organism_inspector}</script>"
    ))
}

async fn observation_stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_observations(socket, state))
}

async fn stream_observations(mut socket: WebSocket, state: AppState) {
    let stream = {
        let mut process = state.simulation.lock().await;
        process.subscribe().await
    };
    let Ok(stream) = stream else {
        return;
    };

    let mut reader = tokio::io::BufReader::new(stream);
    let mut line = String::new();

    loop {
        tokio::select! {
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let message = Message::Text(line.trim_end().to_owned().into());
                        if socket.send(message).await.is_err() {
                            break;
                        }
                        line.clear();
                    }
                }
            }
            result = socket.recv() => {
                match result {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

async fn observation_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    match request(&state, serde_json::json!({"command": "status"})).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

#[derive(serde::Deserialize, Default)]
struct OrganismObservationQuery {
    known_structure_revision: Option<u64>,
    known_position_revision: Option<u64>,
}

#[derive(serde::Deserialize, Default)]
struct ViewBounds {
    min_x: Option<f64>,
    max_x: Option<f64>,
    min_y: Option<f64>,
    max_y: Option<f64>,
    known_organisms: Option<String>,
    known_cells: Option<String>,
}

async fn world_observation_handler(
    Query(query): Query<ViewBounds>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let bounds = match (query.min_x, query.max_x, query.min_y, query.max_y) {
        (Some(min_x), Some(max_x), Some(min_y), Some(max_y))
            if min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite()
                && min_x <= max_x && min_y <= max_y => Some((min_x, max_x, min_y, max_y)),
        (None, None, None, None) => None,
        _ => return error_response("invalid_view_bounds"),
    };
    let known_organisms = query
        .known_organisms
        .as_deref()
        .map(serde_json::from_str::<Vec<(String, u64, u64)>>)
        .transpose()
        .map_err(|_| "invalid_view_bounds");
    let known_cells = query
        .known_cells
        .as_deref()
        .map(serde_json::from_str::<Vec<(usize, u64)>>)
        .transpose()
        .map_err(|_| "invalid_view_bounds");
    let (Ok(known_organisms), Ok(known_cells)) = (known_organisms, known_cells) else {
        return error_response("invalid_view_bounds");
    };
    match request(
        &state,
        serde_json::json!({
            "command": "world",
            "bounds": bounds,
            "known_organisms": known_organisms,
            "known_cells": known_cells,
        }),
    ).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn historical_world_observation_handler(
    Path(tick): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match request(
        &state,
        serde_json::json!({"command": "history_world", "tick": tick}),
    )
    .await
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn organism_observation_handler(
    Path(id): Path<String>,
    Query(query): Query<OrganismObservationQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match request(
        &state,
        serde_json::json!({
            "command": "organism",
            "id": id,
            "known_structure_revision": query.known_structure_revision,
            "known_position_revision": query.known_position_revision,
        }),
    )
    .await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn structure_observation_handler(
    Path(id): Path<String>,
    Query(query): Query<OrganismObservationQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match request(
        &state,
        serde_json::json!({
            "command": "structure",
            "id": id,
            "known_structure_revision": query.known_structure_revision,
        }),
    )
    .await
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn resource_visualization_handler(State(state): State<AppState>) -> impl IntoResponse {
    match request(&state, serde_json::json!({"command": "resources"})).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn pause_handler(State(state): State<AppState>) -> impl IntoResponse {
    match request(&state, serde_json::json!({"command": "pause"})).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn resume_handler(State(state): State<AppState>) -> impl IntoResponse {
    match request(&state, serde_json::json!({"command": "resume"})).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn step_handler(State(state): State<AppState>) -> impl IntoResponse {
    match request(&state, serde_json::json!({"command": "step"})).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn speed_handler(
    Path(ticks_per_second): Path<f64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match request(
        &state,
        serde_json::json!({
            "command": "speed",
            "ticks_per_second": ticks_per_second
        }),
    )
    .await
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn restore_handler(
    Path(tick): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match request(
        &state,
        serde_json::json!({"command": "restore", "tick": tick}),
    )
    .await
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

pub(crate) async fn run() {
    let process = SimulationProcess::spawn().await;
    let state = AppState {
        simulation: std::sync::Arc::new(tokio::sync::Mutex::new(process)),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/observation/status", get(observation_status_handler))
        .route("/observation/stream", get(observation_stream_handler))
        .route("/observation/world", get(world_observation_handler))
        .route(
            "/observation/history/{tick}/world",
            get(historical_world_observation_handler),
        )
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
        .route("/control/resume", axum::routing::post(resume_handler))
        .route("/control/step", axum::routing::post(step_handler))
        .route(
            "/control/speed/{ticks_per_second}",
            axum::routing::post(speed_handler),
        )
        .route(
            "/control/restore/{tick}",
            axum::routing::post(restore_handler),
        )
        .with_state(state)
        .layer(CorsLayer::permissive());

    let address = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on http://{address}");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("could not bind viewer server");
    axum::serve(listener, app)
        .await
        .expect("viewer server stopped");
}
