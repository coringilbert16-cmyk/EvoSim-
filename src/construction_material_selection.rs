use crate::material_storage::{MaterialStorage, StoredMaterial};
use crate::resources::{BaseResource, Form};

/// Minimum structural similarity required before a physical material may be
/// substituted for the material preferred at the current blueprint location.
///
/// This is deliberately a threshold, not a ranking: a candidate below it is
/// not "least bad" construction material. Its absence becomes a material need.
pub(crate) const MIN_CONSTRUCTION_MATERIAL_MATCH: f64 = 0.60;

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

fn normalized_similarity(a: f64, b: f64, min: f64, max: f64) -> f64 {
    let range = (max - min).abs();
    if range <= f64::EPSILON {
        return if (a - b).abs() <= f64::EPSILON {
            1.0
        } else {
            0.0
        };
    }
    (1.0 - (a - b).abs() / range).clamp(0.0, 1.0)
}

fn form_family(form: &Form) -> u8 {
    match form {
        Form::Circle { .. } => 0,
        Form::Line { .. } => 1,
        Form::Rectangle { .. } => 2,
        Form::RegularPolygon { .. } => 3,
        Form::Polygon { .. } => 4,
        Form::Fluid { .. } => 5,
    }
}

fn structural_similarity(
    preferred: &BaseResource,
    candidate: &BaseResource,
    catalog: &[BaseResource],
) -> f64 {
    if preferred.name == candidate.name {
        return 1.0;
    }

    let ranges = |f: fn(&crate::resources::ResourceProperties) -> f64| {
        let values = catalog.iter().map(|r| f(&r.properties));
        let min = values.clone().fold(f64::INFINITY, f64::min);
        let max = values.fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };

    let (mass_min, mass_max) = ranges(|p| p.mass);
    let (energy_min, energy_max) = ranges(|p| p.potential_energy);
    let (reactivity_min, reactivity_max) = ranges(|p| p.reactivity);
    let (cohesion_min, cohesion_max) = ranges(|p| p.cohesion);

    let property_score = [
        normalized_similarity(
            preferred.properties.mass,
            candidate.properties.mass,
            mass_min,
            mass_max,
        ),
        normalized_similarity(
            preferred.properties.potential_energy,
            candidate.properties.potential_energy,
            energy_min,
            energy_max,
        ),
        normalized_similarity(
            preferred.properties.reactivity,
            candidate.properties.reactivity,
            reactivity_min,
            reactivity_max,
        ),
        normalized_similarity(
            preferred.properties.cohesion,
            candidate.properties.cohesion,
            cohesion_min,
            cohesion_max,
        ),
    ]
    .into_iter()
    .sum::<f64>()
        / 4.0;

    let shape_score = if form_family(&preferred.shape.form) == form_family(&candidate.shape.form) {
        1.0
    } else {
        0.0
    };

    // Geometry is a first-class structural property. The four resource
    // properties provide the remaining material signature. Equal weighting
    // keeps the selector small and prevents energy value from becoming a
    // hidden "food" preference.
    (property_score * 0.8 + shape_score * 0.2).clamp(0.0, 1.0)
}

pub(crate) fn rank_available_construction_materials(
    storage: &MaterialStorage,
    preferred_resource_name: &str,
    catalog: &[BaseResource],
) -> Result<Vec<(usize, String, f64)>, String> {
    let preferred = catalog
        .iter()
        .find(|resource| resource.name == preferred_resource_name)
        .ok_or_else(|| {
            format!("unknown preferred construction resource {preferred_resource_name}")
        })?;

    let mut candidates = storage
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let StoredMaterial::Physical(instance) = entry;
            if !instance.is_realized() || instance.material.parts.len() != 1 {
                return None;
            }
            let (resource_name, amount) = instance.material.parts.first()?;
            if (amount - 1.0).abs() > 1e-12 {
                return None;
            }
            let candidate = catalog
                .iter()
                .find(|resource| resource.name == *resource_name)?;
            let score = structural_similarity(preferred, candidate, catalog);
            Some((index, resource_name.clone(), score))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    Ok(candidates)
}

pub(crate) fn select_construction_material(
    storage: &MaterialStorage,
    preferred_resource_name: &str,
    catalog: &[BaseResource],
) -> Result<ConstructionMaterialDecision, String> {
    let candidates =
        rank_available_construction_materials(storage, preferred_resource_name, catalog)?;

    if let Some((storage_index, resource_name, score)) = candidates
        .iter()
        .find(|(_, _, score)| *score >= MIN_CONSTRUCTION_MATERIAL_MATCH)
    {
        return Ok(ConstructionMaterialDecision::Selected {
            storage_index: *storage_index,
            resource_name: resource_name.clone(),
            score: *score,
        });
    }

    Ok(ConstructionMaterialDecision::Need {
        preferred_resource: preferred_resource_name.to_string(),
        best_available_score: candidates.first().map_or(0.0, |candidate| candidate.2),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, Material};
    use crate::structure::Placement;

    fn storage_with(name: &str) -> MaterialStorage {
        let catalog = default_catalog();
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical(
            Material::free_base(name, 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        ));
        storage
    }

    #[test]
    fn exact_preference_is_selected() {
        let catalog = default_catalog();
        let storage = storage_with("Carbon");
        assert_eq!(
            select_construction_material(&storage, "Carbon", &catalog),
            Ok(ConstructionMaterialDecision::Selected {
                storage_index: 0,
                resource_name: "Carbon".into(),
                score: 1.0,
            })
        );
    }

    #[test]
    fn structurally_close_substitute_is_accepted() {
        let catalog = default_catalog();
        let storage = storage_with("Nitrogen");
        let decision = select_construction_material(&storage, "Carbon", &catalog).unwrap();
        assert!(matches!(
            decision,
            ConstructionMaterialDecision::Selected {
                resource_name,
                score,
                ..
            } if resource_name == "Nitrogen" && score >= MIN_CONSTRUCTION_MATERIAL_MATCH
        ));
    }

    #[test]
    fn poor_match_becomes_need_instead_of_forced_substitution() {
        let catalog = default_catalog();
        let storage = storage_with("Hydrogen");
        let decision = select_construction_material(&storage, "Carbon", &catalog).unwrap();
        assert!(matches!(
            decision,
            ConstructionMaterialDecision::Need {
                preferred_resource,
                ..
            } if preferred_resource == "Carbon"
        ));
    }
}
