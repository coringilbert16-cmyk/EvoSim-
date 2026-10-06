use crate::material_storage::{MaterialStorage, StoredMaterial};
use crate::resources::{BaseResource, Form};

/// Enumerate realized physical materials available for construction.
///
/// Construction is material-neutral: no resource is preferred, ranked, or
/// substituted according to a developmental material preference. Geometry and
/// the actual physical state of the available material determine whether it
/// can participate in construction.
pub(crate) fn available_construction_materials(
    storage: &MaterialStorage,
) -> Vec<(usize, String)> {
    storage
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let StoredMaterial::Physical(instance) = entry;
            if !instance.is_realized() || instance.material.parts.is_empty() {
                return None;
            }
            let resource_name = instance
                .material
                .parts
                .iter()
                .find(|(_, amount)| (*amount - 1.0).abs() <= 1e-12)
                .map(|(name, _)| name.clone())?;
            Some((index, resource_name))
        })
        .collect()
}

