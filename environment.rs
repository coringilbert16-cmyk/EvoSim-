// Environment subsystem facade.
//
// The implementation is split by responsibility so each file remains
// independently readable and editable while preserving the original public API.

mod field;
mod reservoir;
mod vents;
mod settling;

pub use field::{ActiveMaterialField, FieldCell, DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION, MATERIAL_EPSILON};
pub use reservoir::{DeepReservoir, ReservoirCell, DEFAULT_RESERVOIR_BLOCK_SIZE};
pub use vents::{apply_vents, Vent};
pub use settling::{apply_settling, DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS};

#[cfg(test)]
mod environment_tests;
