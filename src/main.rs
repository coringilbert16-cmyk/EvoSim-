#![allow(clippy::too_many_arguments)]

// Core environment, resources, and physical material/structure.
mod environment;
mod environmental_materials;
mod material_storage;
mod material_transfer;
mod resources;
mod structure;

// Physical geometry, interfaces, contact, and construction.
mod boundary_contact;
mod cavity;
mod connection_geometry;
mod contact;
mod construction_realization;
mod interface_geometry;
#[allow(unused_variables)]
mod material_geometry;
mod organism_boundary;
mod organism_geometry;
mod permeability;
mod physical_geometry;
mod physical_interface;
mod surface_geometry;

// Material transformation and bonding.
mod combine;
mod combine_runtime;
mod decomposition;

// Organism genome, behavior, and lifecycle.
mod decision;
mod decision_runtime;
mod genome;
mod memory;
mod movement;
mod observation;
mod perception;
mod reproduction;
#[allow(clippy::needless_range_loop, unused_variables)]
mod structural_blueprint;
mod transformation;

// Simulation and application runtime.
mod math;
mod resource_visualization;
mod server;
mod simulation;
mod state;

// Integration and contract tests.
#[cfg(test)]
mod blueprint_diagnostics;
#[cfg(test)]
mod blueprint_spatial_target_tests;
#[cfg(test)]
mod observation_contract_tests;
#[cfg(test)]
mod simulation_tests;

#[tokio::main]
async fn main() {
    server::run().await;
}
