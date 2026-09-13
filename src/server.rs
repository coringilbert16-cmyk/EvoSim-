use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use parking_lot::Mutex;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::observation::SeedObservation;
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
    Html(include_str!("../ui/index.html"))
}

async fn seed_observation_handler(State(state): State<AppState>) -> impl IntoResponse {
    let simulation = state.simulation.lock();
    match SeedObservation::from_simulation(&simulation) {
        Some(observation) => Json(observation).into_response(),
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}

pub(crate) async fn run() {
    let simulation = Arc::new(Mutex::new(Simulation::new(42, 10.0)));
    let state = AppState {
        simulation: simulation.clone(),
    };

    start_tick_loop(simulation);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/observation/seed", get(seed_observation_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", address);
    let listener = TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
