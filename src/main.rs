#![allow(clippy::too_many_arguments)]

// Core environment, resources, and physical material/structure.
#[path = "organism/developmental_blueprint.rs"]
mod developmental_blueprint;
#[path = "simulation/environment.rs"]
mod environment;
#[path = "materials/environmental_materials.rs"]
mod environmental_materials;
#[path = "chemistry/expulsion.rs"]
mod expulsion;
#[path = "geometry/material_geometry.rs"]
mod material_geometry;
#[path = "materials/material_restoration.rs"]
mod material_restoration;
#[path = "materials/material_storage.rs"]
mod material_storage;
#[path = "materials/material_transfer.rs"]
mod material_transfer;
#[path = "materials/physical_material.rs"]
mod physical_material;
#[path = "materials/resources.rs"]
mod resources;
#[path = "geometry/structure.rs"]
mod structure;
#[path = "geometry/structure_authority.rs"]
mod structure_authority;

// Active physical geometry authority stack.
#[path = "geometry/capillary_geometry.rs"]
mod capillary_geometry;
#[path = "geometry/cavity.rs"]
mod cavity;
#[path = "geometry/connection_geometry.rs"]
mod connection_geometry;
#[path = "construction/material_selection.rs"]
mod construction_material_selection;
#[path = "construction/runtime.rs"]
mod construction_runtime;
#[path = "geometry/contact.rs"]
mod contact;
#[path = "bob/worker.rs"]
mod geometry_library_worker;
#[path = "bob/library.rs"]
mod geometry_reference_library;
#[path = "bob/server.rs"]
mod geometry_server;
#[path = "construction/initial_organism_constructor.rs"]
mod initial_organism_constructor;
#[path = "geometry/interior_geometry.rs"]
mod interior_geometry;
#[path = "organism/juvenile.rs"]
mod juvenile;
#[path = "geometry/organism_geometry.rs"]
mod organism_geometry;
#[path = "organism/viability.rs"]
mod organism_viability;
#[path = "geometry/physical_geometry.rs"]
mod physical_geometry;
#[path = "geometry/rigid_boundary.rs"]
mod rigid_boundary;
#[path = "geometry/surface_geometry.rs"]
mod surface_geometry;

// Material transformation and bonding.
#[path = "chemistry/chemical_reaction.rs"]
mod chemical_reaction;
#[path = "chemistry/chemistry.rs"]
mod chemistry;
#[path = "chemistry/chemistry_library.rs"]
mod chemistry_library;
#[path = "chemistry/combine.rs"]
mod combine;
#[path = "chemistry/combine_runtime.rs"]
mod combine_runtime;
#[path = "chemistry/decomposition.rs"]
mod decomposition;
#[path = "infrastructure/diagnostics.rs"]
mod diagnostics;
#[path = "chemistry/energy_ledger.rs"]
mod energy_ledger;

// Organism genome, behavior, and lifecycle.
#[path = "organism/decision.rs"]
mod decision;
#[path = "organism/decision_runtime.rs"]
mod decision_runtime;
#[path = "organism/developmental_decision.rs"]
mod developmental_decision;
#[path = "organism/genome.rs"]
mod genome;
#[path = "perception/harmonics.rs"]
mod harmonics;
#[path = "organism/memory.rs"]
mod memory;
#[path = "perception/movement.rs"]
mod movement;
#[path = "perception/movement_direction.rs"]
mod movement_direction;
#[path = "perception/observation.rs"]
mod observation;
#[path = "chemistry/recycling.rs"]
mod recycling;
#[path = "organism/reproduction.rs"]
mod reproduction;
#[path = "organism/structural_blueprint_unified.rs"]
#[allow(clippy::needless_range_loop, unused_variables)]
mod structural_blueprint;
#[path = "chemistry/transformation.rs"]
mod transformation;

// Simulation and application runtime.
#[path = "infrastructure/math.rs"]
mod math;
#[path = "ui/resource_visualization.rs"]
mod resource_visualization;
#[path = "simulation/runtime.rs"]
mod runtime;
#[path = "simulation/server.rs"]
mod server;
#[path = "simulation/sim_process.rs"]
mod sim_process;
#[path = "simulation/simulation.rs"]
mod simulation;
#[path = "simulation/simulation_runner.rs"]
mod simulation_runner;
#[path = "simulation/state.rs"]
mod state;

// Integration and contract tests.
#[cfg(test)]
#[path = "tests/blueprint_diagnostics.rs"]
mod blueprint_diagnostics;
#[cfg(test)]
#[path = "tests/blueprint_spatial_target_tests.rs"]
mod blueprint_spatial_target_tests;
#[cfg(test)]
#[path = "tests/bond_driven_contract_tests.rs"]
mod bond_driven_contract_tests;
#[cfg(test)]
#[path = "tests/observation_contract_tests.rs"]
mod observation_contract_tests;
#[cfg(test)]
#[path = "tests/phase1_acquisition_contract_tests.rs"]
mod phase1_acquisition_contract_tests;
#[cfg(test)]
#[path = "tests/phase2_structure_contract_tests.rs"]
mod phase2_structure_contract_tests;
#[cfg(test)]
#[path = "tests/phase3_authority_contract_tests.rs"]
mod phase3_authority_contract_tests;
#[cfg(test)]
#[path = "tests/simulation_tests.rs"]
mod simulation_tests;

#[tokio::main]
async fn main() {
    let command = std::env::args().nth(1);

    if command.as_deref() == Some("--simulation-child") {
        let port = std::env::args()
            .nth(2)
            .and_then(|value| value.parse::<u16>().ok())
            .expect("--simulation-child requires a TCP port");
        sim_process::run_child(port).await;
        return;
    }

    if command.as_deref() == Some("--geometry-worker-once") {
        geometry_library_worker::run_once().expect("geometry worker smoke test failed");
        return;
    }

    if command.as_deref() == Some("--geometry-worker-passes") {
        let passes = std::env::args()
            .nth(2)
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|passes| *passes > 0)
            .expect("--geometry-worker-passes requires a positive integer");
        geometry_library_worker::run_passes(passes).expect("bounded geometry worker run failed");
        return;
    }

    if command.as_deref() == Some("--geometry-worker") {
        geometry_library_worker::run();
        return;
    }

    if command.as_deref() == Some("--geometry-viewer") {
        geometry_server::run().await;
        return;
    }

    if command.as_deref() == Some("--headless") {
        simulation_runner::run_from_args(std::env::args());
        return;
    }

    server::run().await;
}
