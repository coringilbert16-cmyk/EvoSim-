#![allow(clippy::too_many_arguments)]

// Isolated Phase 3B hotspot profile mathematics; not connected to live sources.
#[path = "physics/hotspot_profile.rs"]
#[allow(dead_code)]
mod hotspot_profile;

// Core environment, resources, and physical material/structure.
#[path = "organism/developmental_blueprint.rs"]
mod developmental_blueprint;
#[path = "simulation/environment.rs"]
mod environment;
#[path = "simulation/environment_chemistry_experiment.rs"]
mod environment_chemistry_experiment;
#[path = "simulation/environment_flow.rs"]
mod environment_flow;
#[path = "materials/environmental_materials.rs"]
mod environmental_materials;
#[path = "simulation/environment_movement.rs"]
mod environmental_movement;
#[path = "chemistry/expulsion.rs"]
mod expulsion;
#[path = "simulation/hotspot.rs"]
mod hotspot;
#[path = "geometry/material_geometry.rs"]
mod material_geometry;
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
#[path = "simulation/world_tone.rs"]
mod world_tone;
#[path = "simulation/world_tone_snapshot.rs"]
#[allow(dead_code)] // Live settlement uses this snapshot; audit helpers remain test-only.
mod world_tone_snapshot;

// Active physical geometry authority stack.
#[path = "geometry/capillary_geometry.rs"]
mod capillary_geometry;
#[path = "geometry/cavity.rs"]
mod cavity;
#[path = "geometry/connection_faces.rs"]
mod connection_faces;
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
#[path = "geometry/molecular_layout.rs"]
mod molecular_layout;
#[path = "geometry/organism_geometry.rs"]
mod organism_geometry;
#[path = "organism/viability.rs"]
mod organism_viability;
#[path = "geometry/phase3_boundary.rs"]
#[allow(dead_code)] // Consumed by the next Phase 3 integration slice.
mod phase3_boundary;
#[path = "geometry/phase3_geometry.rs"]
#[allow(dead_code)]
// Strict Phase 3 reference geometry contract; integrated in later Phase 3 stages.
mod phase3_geometry;
#[path = "geometry/phase3_numeric.rs"]
mod phase3_numeric;
#[path = "geometry/phase3_polygon.rs"]
mod phase3_polygon;
#[path = "geometry/phase3_source_footprint.rs"]
#[allow(dead_code)] // Geometry-only source-footprint measure; source coupling is a later phase.
mod phase3_source_footprint;
#[path = "geometry/phase3_union.rs"]
#[allow(dead_code)] // Strict checked polygon union measurement; integration is staged.
mod phase3_union;
#[path = "geometry/physical_geometry.rs"]
mod physical_geometry;
#[path = "geometry/rigid_boundary.rs"]
mod rigid_boundary;
#[path = "geometry/source_exposure.rs"]
#[allow(dead_code)] // Live sunlight uses analytic exposure; remaining helpers are test-only.
mod source_exposure;
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
#[allow(
    dead_code,
    reason = "Staged graph analysis API is integrated in a later chemistry stage"
)]
#[path = "chemistry/emergent_cohesion.rs"]
mod emergent_cohesion;
