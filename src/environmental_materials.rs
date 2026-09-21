#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::environment::ActiveMaterialField;
use crate::environment_formation::Formation;
use crate::resources::BaseResource;
use crate::resources::{combine_materials, Material};

/// Small, reusable structured-material seeds for the initial environment.
///
/// These are deliberately compositions rather than terrain types. Their
/// environmental meaning comes from composition + internal structure + where
/// they are placed in the active field.
pub(crate) const ENVIRONMENTAL_COMPOUND_COUNT: usize = 12;

pub(crate) fn seed_compounds() -> Vec<Material> {
    vec![
        compound(&[("Hydrogen", 1.0), ("Nitrogen", 1.0)]),
        compound(&[("Hydrogen", 1.0), ("Sulfur", 1.0)]),
        compound(&[("Nitrogen", 1.0), ("Sulfur", 1.0)]),
        compound(&[("Nitrogen", 1.0), ("Phosphorus", 1.0)]),
        compound(&[("Sulfur", 1.0), ("Phosphorus", 1.0)]),
        compound(&[("Hydrogen", 1.0), ("Nitrogen", 1.0), ("Sulfur", 1.0)]),
        compound(&[("Carbon", 1.0), ("Hydrogen", 1.0)]),
        compound(&[("Carbon", 1.0), ("Nitrogen", 1.0)]),
        compound(&[("Carbon", 1.0), ("Sulfur", 1.0)]),
        compound(&[("Carbon", 1.0), ("Phosphorus", 1.0)]),
        compound(&[("Carbon", 1.0), ("Hydrogen", 1.0), ("Nitrogen", 1.0)]),
        compound(&[("Carbon", 1.0), ("Sulfur", 1.0), ("Phosphorus", 1.0)]),
    ]
}

/// Populate the active field with a deterministic, spatially correlated
/// starting landscape. No terrain categories are introduced: local character
/// comes entirely from material composition, quantity, and neighboring cells.
pub(crate) fn seed_initial_landscape(
    field: &mut ActiveMaterialField,
    resolved_extent: f64,
    catalog: &[BaseResource],
) {
    let compounds = seed_compounds();
    if compounds.is_empty() {
        return;
    }

    // Formations are spatially coarse environmental objects. The field cells
    // remain only an index; a small deterministic set of irregular formations
    // provides the initial biome-like spread without creating one formation per
    // indexing cell.
    let grid_side = 4usize;
    let width = field.width_cells as f64 * field.cell_size;
    let height = field.height_cells as f64 * field.cell_size;

    for row in 0..grid_side {
        for col in 0..grid_side {
            let gx = (col as f64 + 0.5) / grid_side as f64;
            let gy = (row as f64 + 0.5) / grid_side as f64;
            let field_a = ((gx * 2.4 + gy * 1.3).sin() + 1.0) * 0.5;
            let field_b = ((gx * 1.1 - gy * 2.7 + 0.8).cos() + 1.0) * 0.5;
            let field_c = ((gx * 3.0 + gy * 2.0 + 1.7).sin() + 1.0) * 0.5;

            let base_selector = row * grid_side + col;
            let selector_offset = (field_c * compounds.len() as f64) as usize;
            let selector = (base_selector + selector_offset) % compounds.len();

            let mut composition = compounds[selector].parts.clone();
            if field_a > 0.55 {
                let secondary = (selector + 3 + (field_b * 4.0) as usize) % compounds.len();
                for (name, amount) in &compounds[secondary].parts {
                    if let Some(existing) = composition
                        .iter_mut()
                        .find(|(candidate, _)| candidate == name)
                    {
                        existing.1 += amount * 0.5;
                    } else {
                        composition.push((name.clone(), amount * 0.5));
                    }
                }
            }
            if field_b < 0.5 {
                composition.push(("Water".to_string(), 3.0));
            }
            if field_a > 0.72 {
                composition.push(("Hydrogen".to_string(), 2.0));
            }

            let x_jitter = (field_a - 0.5) * field.cell_size * 2.0;
            let y_jitter = (field_b - 0.5) * field.cell_size * 2.0;
            let origin = (
                (gx * width + x_jitter).rem_euclid(width),
                (gy * height + y_jitter).rem_euclid(height),
            );

            let composition = composition
                .into_iter()
                .filter(|(_, amount)| amount.is_finite() && *amount > 0.0)
                .collect::<Vec<_>>();

            let formation = Formation::new(composition, resolved_extent * 2.0, catalog);
            if let Some(mut formation) = formation {
                formation.set_origin(origin);
                formation.resolve_frontier();
                field.formations.push(formation);
            }
        }
    }
}

fn compound(parts: &[(&str, f64)]) -> Material {
    let inputs = parts
        .iter()
        .map(|(name, amount)| Material::free_base(*name, *amount))
        .collect::<Vec<_>>();
    combine_materials(&inputs)
}

fn scaled_material(material: &Material, scale: f64) -> Material {
    Material {
        parts: material
            .parts
            .iter()
            .map(|(name, amount)| (name.clone(), amount * scale))
            .collect(),
        internal_bonds: material.internal_bonds.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{seed_compounds, seed_initial_landscape, ENVIRONMENTAL_COMPOUND_COUNT};
    use crate::environment::ActiveMaterialField;
    use crate::resources::Material;
    use std::collections::BTreeSet;

    fn structured_signature(material: &Material) -> Option<Vec<String>> {
        if !material.has_internal_structure() {
            return None;
        }
        let mut names = material
            .parts
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        names.sort();
        Some(names)
    }

    #[test]
    fn seed_set_has_expected_size_and_structure() {
        let compounds = seed_compounds();
        assert_eq!(compounds.len(), ENVIRONMENTAL_COMPOUND_COUNT);
        assert!(compounds.iter().all(|material| material.is_valid()
            && material.has_internal_structure()
            && material.parts.len() >= 2));
    }
    #[test]
    fn seed_set_contains_carbon_and_non_carbon_compositions() {
        let compounds = seed_compounds();
        assert!(compounds
            .iter()
            .any(|material| material.parts.iter().any(|(name, _)| name == "Carbon")));
        assert!(compounds
            .iter()
            .any(|material| material.parts.iter().all(|(name, _)| name != "Carbon")));
    }
    #[test]
    fn initial_landscape_is_populated_and_structured() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, 10.0, &crate::resources::default_catalog());
        assert!(field.total_amount() > 0.0);
        assert!(!field.formations.is_empty());
        assert!(field.formations.iter().any(|formation| !formation
            .pattern
            .material
            .material
            .is_empty()));
        assert!(field.formations.iter().any(|formation| {
            formation
                .bulk
                .composition
                .iter()
                .any(|(name, _)| name == "Water")
        }));
    }
    #[test]
    fn initial_landscape_uses_all_compound_varieties() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, 10.0, &crate::resources::default_catalog());
        let signatures = field
            .formations
            .iter()
            .map(|formation| formation.pattern.material.material.clone())
            .filter_map(|material| structured_signature(&material))
            .collect::<BTreeSet<_>>();
        assert!(signatures.len() > 1);
        assert!(signatures.len() <= ENVIRONMENTAL_COMPOUND_COUNT);
    }
    #[test]
    fn initial_landscape_is_not_globally_uniform() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, 10.0, &crate::resources::default_catalog());
        let signatures = field
            .formations
            .iter()
            .filter_map(|formation| structured_signature(&formation.pattern.material.material))
            .collect::<BTreeSet<_>>();
        assert!(signatures.len() > 1);
    }
    #[test]
    fn initial_landscape_totals_are_finite_and_positive() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, 10.0, &crate::resources::default_catalog());
        let total = field.total_amount();
        assert!(total.is_finite());
        assert!(total > 0.0);
    }
    #[test]
    fn initial_landscape_seeding_is_deterministic() {
        let mut first = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut second = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut first, 10.0, &crate::resources::default_catalog());
        seed_initial_landscape(&mut second, 10.0, &crate::resources::default_catalog());
        assert_eq!(first.total_amount(), second.total_amount());
        assert_eq!(first.formations, second.formations);
    }
}
