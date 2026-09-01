//! EvoSim shared decision-layer library.
//!
//! The executable remains in `main.rs`. This small library target gives the
//! decision architecture an independently testable boundary before it is
//! wired into the simulation loop.

pub mod decision;
