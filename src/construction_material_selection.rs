use crate::material_storage::{MaterialStorage, StoredMaterial};

/// Construction has no preferred material and no material-similarity threshold.
///
/// The compatibility arguments are retained temporarily because staged
/// developmental callers still pass a blueprint resource name. They are
/// deliberately ignored: every realized physical material is equally eligible
/// and geometry decides whether it can actually be placed and bonded.
pub(crate) const MIN_CONSTRUCTION_MATERIAL_MATCH: f64 = 0.0;

pub(crate) fn rank_available_construction_materials(
    storage: &MaterialStorage,
    _preferred_resource_name: &str,
    _catalog: &[crate::resources::BaseResource],
) -> Result<Vec<(usize, String, f64)>, String> {
    Ok(storage
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
            Some((index, resource_name, 1.0))
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ConstructionMaterialDecision {
    Selected {
        storage_index: usize,
        resource_name: String,
        score: f64,
    },
    Need {
        preferred_resource: String,
        best_available_score: f64,
    },
}

/// Select the first realized physical material only as an inventory ordering
/// decision. The resource identity has no preference or similarity advantage.
pub(crate) fn select_construction_material(
    storage: &MaterialStorage,
    _preferred_resource_name: &str,
    _catalog: &[crate::resources::BaseResource],
) -> Result<ConstructionMaterialDecision, String> {
    let candidates = rank_available_construction_materials(storage, "", &[])?;
    Ok(candidates.first().map_or(
        ConstructionMaterialDecision::Need {
            preferred_resource: String::new(),
            best_available_score: 0.0,
        },
        |(storage_index, resource_name, _)| ConstructionMaterialDecision::Selected {
            storage_index: *storage_index,
            resource_name: resource_name.clone(),
            score: 1.0,
        },
    ))
}

/// Enumerate realized physical materials without assigning any material a
/// construction preference.
pub(crate) fn available_construction_materials(
    storage: &MaterialStorage,
) -> Vec<(usize, String)> {
    rank_available_construction_materials(storage, "", &[])
        .unwrap_or_default()
        .into_iter()
        .map(|(index, name, _)| (index, name))
        .collect()
}
