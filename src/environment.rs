use crate::environmental_materials::EnvironmentMaterial;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{BaseResource, Material};
use crate::state::{Environment, FieldCell};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ActiveMaterialField {
    pub cells: Vec<FieldCell>,
    pub width: usize,
    pub height: usize,
    pub cell_size: f64,
}

