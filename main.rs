mod combine;
mod combine_runtime;
mod connection_geometry;
mod contact;
mod core_geometry;
mod core_integrity;
mod decision;
mod decision_runtime;
mod environment;
mod genome;
mod math;
mod membrane_geometry;
mod reproduction;
mod resources;
mod structural_blueprint;
mod structural_combine;
mod structural_geometry;
mod structural_material;
mod structure;

mod memory;
mod movement;
mod perception;
mod server;
mod simulation;
mod state;
mod transformation;

#[cfg(test)]
mod simulation_tests;

#[tokio::main]
async fn main() {
    server::run().await;
}
