// Environment subsystem facade.
//
// The implementation is split by responsibility so each file remains
// independently readable and editable while preserving the original public API.

#[path = "field.rs"]
mod field;
#[path = "reservoir.rs"]
mod reservoir;
#[path = "vents.rs"]
mod vents;
#[path = "settling.rs"]
mod settling;

pub use field::{ActiveMaterialField, FieldCell, DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION, MATERIAL_EPSILON};
pub use reservoir::{DeepReservoir, ReservoirCell, DEFAULT_RESERVOIR_BLOCK_SIZE};
pub use vents::{apply_vents, Vent};
pub use settling::{apply_settling, DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS};

#[cfg(test)]
#[path = "environment_tests.rs"]
mod environment_tests;
