mod boundary_contact;
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
mod interface_geometry;
mod material_geometry;
mod math;
mod membrane_geometry;
mod organism_boundary;
mod organism_geometry;
mod permeability;
mod physical_acquisition;
mod physical_environment;
mod physical_formation;
mod physical_interface;
mod physical_realization;
mod physical_spatial_index;
mod reproduction;
mod resources;
mod structural_blueprint;
mod structural_combine;
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
