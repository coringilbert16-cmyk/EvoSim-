#![allow(clippy::too_many_arguments)]

mod combine;
mod combine_runtime;
mod connection_geometry;
mod contact;
mod construction_realization;
mod decision;
mod decision_runtime;
mod decomposition;
mod environment;
mod environmental_materials;
mod genome;
mod interface_geometry;
mod math;
#[allow(unused_variables)]
mod material_geometry;
mod material_storage;
mod material_transfer;
mod organism_boundary;
mod organism_geometry;
mod boundary_contact;
mod physical_geometry;
mod physical_interface;
mod permeability;
mod reproduction;
mod resource_visualization;
mod resources;
#[allow(clippy::needless_range_loop, unused_variables)]
mod structural_blueprint;
mod structure;
mod surface_geometry;

mod memory;
mod movement;
mod observation;
mod perception;
mod server;
mod simulation;
mod state;
mod transformation;

#[cfg(test)]
mod blueprint_diagnostics;

#[cfg(test)]
mod observation_contract_tests;

#[cfg(test)]
mod simulation_tests;

#[tokio::main]
async fn main() {
    server::run().await;
}
