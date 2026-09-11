use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::environment::ActiveMaterialField;
use crate::resources::Material;

/// A vent is an environmental source, not a chemical recipe or reservoir
/// outlet. Each emission independently chooses a valid material and a
/// fluctuating quantity around the vent's long-term average.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vent {
    pub x: f64,
    pub y: f64,
    pub emission_amount: f64,
    pub emission_interval: u64,
    pub emission_timer: u64,
}

fn scale_material(material: &Material, amount: f64) -> Material {
    let base_amount = material.total_amount();
    if base_amount <= f64::EPSILON {
        return material.clone();
    }
    let scale = amount / base_amount;
    Material {
        parts: material
            .parts
            .iter()
            .map(|(name, part_amount)| (name.clone(), part_amount * scale))
            .collect(),
        internal_bonds: material.internal_bonds.clone(),
    }
}

/// Build the pool from which vents may emit. It contains both elemental/raw
/// materials and the approved environmental compounds. No material category
/// is preferred; selection is uniformly random among valid material kinds.
pub fn valid_vent_materials(catalog: &[crate::resources::BaseResource]) -> Vec<Material> {
    let mut materials = catalog
        .iter()
        .map(|resource| Material::free_base(resource.name.clone(), 1.0))
        .filter(Material::is_valid)
        .collect::<Vec<_>>();
    materials.extend(
        crate::environmental_materials::seed_compounds()
            .into_iter()
            .filter(Material::is_valid),
    );
    materials
}

pub fn apply_vents<R: Rng + ?Sized>(
    field: &mut ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
    vents: &mut [Vent],
    rng: &mut R,
) {
    let available = valid_vent_materials(catalog);
    if available.is_empty() {
        return;
    }

    for vent in vents.iter_mut() {
        if vent.emission_timer > 0 {
            vent.emission_timer -= 1;
            continue;
        }
        vent.emission_timer = vent.emission_interval;

        let Some(field_index) = field.index_for_position(vent.x, vent.y) else {
            continue;
        };
        let template = &available[rng.gen_range(0..available.len())];
        let fluctuation = rng.gen_range(0.5..1.5);
        let amount = (vent.emission_amount.max(0.0) * fluctuation).max(f64::EPSILON);
        field.deposit_at_index(field_index, scale_material(template, amount));
    }
}
