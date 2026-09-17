#![allow(clippy::too_many_arguments)]

// Core environment, resources, and physical material/structure.
mod environment;
mod environmental_materials;
mod material_geometry;
mod material_restoration;
mod material_storage;
mod material_transfer;
mod physical_material;
mod resources;
mod structure;
mod structure_authority;

// Active physical geometry authority stack.
mod architecture;
mod cavity;
mod connection_geometry;
mod construction_runtime;
mod contact;
mod juvenile;
mod juvenile_requirements;
mod organism_geometry;
mod physical_geometry;
mod rigid_boundary;
mod surface_geometry;

// Material transformation and bonding.
mod combine;
mod combine_runtime;
mod decomposition;
mod energy_ledger;

// Organism genome, behavior, and lifecycle.
mod decision;
mod decision_runtime;
mod genome;
mod memory;
mod movement;
mod observation;
mod perception;
mod reproduction;
#[path = "structural_blueprint_unified.rs"]
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
mod architecture_contract_tests;
#[cfg(test)]
mod blueprint_diagnostics;
#[cfg(test)]
mod blueprint_spatial_target_tests;
#[cfg(test)]
mod observation_contract_tests;
#[cfg(test)]
mod phase1_acquisition_contract_tests;
#[cfg(test)]
mod phase2_structure_contract_tests;
#[cfg(test)]
mod simulation_tests;

#[tokio::main]
async fn main() {
    server::run().await;
}
