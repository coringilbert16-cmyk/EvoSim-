mod environment;
mod genome;
mod math;
mod resources;
mod structure;
mod connection_geometry;
mod contact;
mod combine;
mod combine_runtime;
mod structural_combine;
mod decision;
mod decision_runtime;

mod state;
mod simulation;
mod perception;
mod memory;
mod movement;
mod transformation;
mod server;

#[cfg(test)]
mod simulation_tests;

#[tokio::main]
async fn main() {
    server::run().await;
}
