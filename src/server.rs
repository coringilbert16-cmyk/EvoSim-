use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use crate::state::AppState;

struct SimulationProcess {
    child: Child,
    port: u16,
}

impl Drop for SimulationProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

impl SimulationProcess {
    async fn spawn() -> Self {
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .expect("could not allocate simulation control port")
            .local_addr()
            .expect("could not read simulation control port")
            .port();

        let executable = std::env::current_exe().expect("could not locate EvoSim executable");
        let child = Command::new(executable)
            .arg("--simulation-child")
            .arg(port.to_string())
            .spawn()
            .expect("could not start simulation child");

        let process = Self { child, port };

        for _ in 0..100 {
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                return process;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        panic!("simulation child did not become ready");
    }

    async fn request(&mut self, command: Value) -> Result<Value, &'static str> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .map_err(|_| "simulation_unavailable")?;

        let mut encoded = serde_json::to_vec(&command).map_err(|_| "invalid_command")?;
        encoded.push(b'\n');
        stream
            .write_all(&encoded)
            .await
            .map_err(|_| "simulation_unavailable")?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|_| "simulation_unavailable")?;

        let response: Value =
            serde_json::from_str(&line).map_err(|_| "invalid_simulation_response")?;

        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("simulation_error"));
        }

        response
            .get("value")
            .cloned()
            .ok_or("invalid_simulation_response")
    }
}

async fn request(
    state: &AppState,
    command: Value,
) -> Result<Value, &'static str> {
    let mut process = state.simulation.lock().await;
    process.request(command).await
}

fn error_response(error: &'static str) -> axum::response::Response {
    match error {
        "not_found" => axum::http::StatusCode::NOT_FOUND.into_response(),
        "invalid_speed" | "invalid_command" => {
            axum::http::StatusCode::BAD_REQUEST.into_response()
        }
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

async fn observation_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    match request(&state, serde_json::json!({"command": "status"})).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn world_observation_handler(State(state): State<AppState>) -> impl IntoResponse {
    match request(&state, serde_json::json!({"command": "world"})).await {
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
    State(state): State<AppState>,
) -> impl IntoResponse {
    match request(
        &state,
        serde_json::json!({"command": "organism", "id": id}),
    )
    .await
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn structure_observation_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match request(
        &state,
        serde_json::json!({"command": "structure", "id": id}),
    )
    .await
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

async fn resource_visualization_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
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
        simulation: Arc::new(Mutex::new(process)),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/observation/status", get(observation_status_handler))
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
        .route("/control/pause", axum::routing::post(pause_handler))
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
