use crate::resources::{combine_materials, Material};

/// Small, reusable structured-material seeds for the initial environment.
///
/// These are deliberately compositions rather than terrain types. Their
/// environmental meaning comes from composition + internal structure + where
/// they are placed in the active field.
pub(crate) const ENVIRONMENTAL_COMPOUND_COUNT: usize = 6;

pub(crate) fn seed_compounds() -> Vec<Material> {
    vec![
        compound(&[("Carbon", 1.0), ("Hydrogen", 1.0)]),
        compound(&[("Carbon", 1.0), ("Sulfur", 1.0)]),
        compound(&[("Carbon", 1.0), ("Nitrogen", 1.0)]),
        compound(&[("Carbon", 1.0), ("Phosphorus", 1.0)]),
        compound(&[("Carbon", 1.0), ("Hydrogen", 1.0), ("Nitrogen", 1.0)]),
        compound(&[("Carbon", 1.0), ("Sulfur", 1.0), ("Hydrogen", 1.0)]),
    ]
}

fn compound(parts: &[(&str, f64)]) -> Material {
    let inputs = parts
        .iter()
        .map(|(name, amount)| Material::free_base(*name, *amount))
        .collect::<Vec<_>>();
    combine_materials(&inputs)
}

#[cfg(test)]
mod tests {
    use super::{seed_compounds, ENVIRONMENTAL_COMPOUND_COUNT};

    #[test]
    fn seed_set_has_expected_size_and_structure() {
        let compounds = seed_compounds();
        assert_eq!(compounds.len(), ENVIRONMENTAL_COMPOUND_COUNT);
        assert!(compounds.iter().all(|material| {
            material.is_valid()
                && material.has_internal_structure()
                && material.parts.len() >= 2
        }));
    }

    #[test]
    fn seed_set_contains_no_terrain_categories() {
        let compounds = seed_compounds();
        assert!(compounds.iter().all(|material| {
            material.parts.iter().all(|(name, _)| {
                matches!(
                    name.as_str(),
                    "Carbon" | "Hydrogen" | "Nitrogen" | "Sulfur" | "Phosphorus"
                )
            })
        }));
    }
}
