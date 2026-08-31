// Thin crate-root wrapper. The simulation implementation is kept in simulation.rs
// so the crate can declare cross-cutting modules before including it.
mod connection_geometry;
mod combine;

include!("simulation.rs");
