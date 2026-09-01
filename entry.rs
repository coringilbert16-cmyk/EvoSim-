// Thin crate entrypoint used to compile the simulation and its supporting
// architecture without duplicating the runtime transport boundary.
mod combine;
mod structural_combine;
mod decision;
mod decision_runtime;
mod runtime;

include!("main.rs");
