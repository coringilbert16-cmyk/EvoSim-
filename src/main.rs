#![allow(unfulfilled_lint_expectations)]
#![allow(clippy::too_many_arguments)]

// Core environment, resources, and physical material/structure.
mod developmental_blueprint;
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
mod diagnostics;
mod energy_ledger;

// Organism genome, behavior, and lifecycle.
mod decision;
mod decision_runtime;
mod genome;
mod harmonics;
mod memory;
mod movement;
mod observation;
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
mod simulation_runner;
mod state;

// Integration and contract tests.
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
mod phase3_authority_contract_tests;
#[cfg(test)]
mod simulation_tests;

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("--headless") {
        simulation_runner::run_from_args(std::env::args());
        return;
    }

    server::run().await;
}
