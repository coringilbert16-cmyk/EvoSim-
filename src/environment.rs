// Environment subsystem facade.
//
// The active field is the complete environmental material layer. Vents are
// independent sources that inject valid materials directly into that field.

#[path = "field.rs"]
mod field;
#[path = "vents.rs"]
mod vents;

pub use field::{ActiveMaterialField, DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION};
pub use vents::{apply_vents, Vent};

#[cfg(test)]
#[path = "environment_tests.rs"]
mod environment_tests;
