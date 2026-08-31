use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

mod environment;
mod genome;
mod math;
mod resources;
mod structure;
mod contact;
mod connection_geometry;

use environment::{
    apply_settling, apply_vents, ActiveMaterialField, DeepReservoir, Vent,
    DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION, DEFAULT_RESERVOIR_BLOCK_SIZE,
    DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS,
};
use genome::{initial_genome, Genome};
use math::exponential_influence;
use resources::{BaseResource, Material};


// ============================================================
// APPLICATION STATE
// ============================================================

#[derive(Clone)]
struct AppState {
    simulation: Arc<Mutex<Simulation>>,
    broadcaster: broadcast::Sender<String>,
}


// ============================================================
// RESOURCE PERCEPTION
// ============================================================
//
// A ResourceObservation is what the organism currently perceives.
// It is NOT itself a stored environmental resource.
//
// Perception now reads directly from the active material field's
// grid cells (see environment.rs) instead of the legacy
// ResourceCloud list. A field cell holds exactly one bonded stack