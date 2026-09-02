mod combine;
mod combine_runtime;
mod connection_geometry;
mod contact;
mod decision;
mod decision_runtime;
mod environment;
mod genome;
mod math;
mod resources;
mod structure;

mod memory;
mod movement;
mod perception;
mod server;
mod simulation;
mod state;
mod transformation;

#[cfg(test)]
mod phase3_tests;
#[cfg(test)]
mod simulation_tests;

#[tokio::main]
async fn main() {
    server::run().await;
}
