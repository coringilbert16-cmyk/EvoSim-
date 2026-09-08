// Environment subsystem facade.
//
// The implementation is split by responsibility so each file remains
// independently readable and editable while preserving the active API.

#[path = "field.rs"]
mod field;
#[path = "reservoir.rs"]
mod reservoir;
#[path = "settling.rs"]
mod settling;
#[path = "vents.rs"]
mod vents;

pub use field::{ActiveMaterialField, DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION};
pub use reservoir::{DeepReservoir, DEFAULT_RESERVOIR_BLOCK_SIZE};
pub use settling::{apply_settling, DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS};
pub use vents::{apply_vents, Vent};

#[cfg(test)]
#[path = "environment_tests.rs"]
mod environment_tests;
