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

    for index in 0..field.cells.len() {
        let (x, y) = field.cell_center(index);
        let nx = x / (field.width_cells as f64 * field.cell_size).max(1.0);
        let ny = y / (field.height_cells as f64 * field.cell_size).max(1.0);

        let field_a = ((nx * 2.4 + ny * 1.3).sin() + 1.0) * 0.5;
        let field_b = ((nx * 1.1 - ny * 2.7 + 0.8).cos() + 1.0) * 0.5;
        let field_c = ((nx * 3.0 + ny * 2.0 + 1.7).sin() + 1.0) * 0.5;
        let selector = ((field_c * compounds.len() as f64) as usize).min(compounds.len() - 1);

        let mut composition = compounds[selector].parts.clone();
        if field_a > 0.72 {
            let secondary = (selector + 3 + (field_b * 4.0) as usize) % compounds.len();
            for (name, amount) in &compounds[secondary].parts {
                if let Some(existing) = composition.iter_mut().find(|(candidate, _)| candidate == name) {
                    *existing += amount * 0.5;
                } else {
                    composition.push((name.clone(), amount * 0.5));
                }
            }
        }
        if field_a > 0.72 {
            composition.push(("Hydrogen".to_string(), 2.0));
        }
        if field_b < 0.5 {
            composition.push(("Water".to_string(), 3.0));
        }

        let composition = composition
            .into_iter()
            .filter(|(_, amount)| amount.is_finite() && *amount > 0.0)
            .collect::<Vec<_>>();
        if let Some(formation) =
            Formation::new(composition, resolved_extent * 2.0, catalog)
        {
            field.cells[index].formations.push(formation);
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
        seed_initial_landscape(
            &mut field,
            10.0,
            &crate::resources::default_catalog(),
        );
        assert!(field.total_amount() > 0.0);
        assert!(field.cells.iter().all(|cell| !cell.formations.is_empty()));
        assert!(field.cells.iter().any(|cell| cell
            .formations
            .iter()
            .any(|formation| !formation.pattern.material.material.is_empty())));
        assert!(field.cells.iter().any(|cell| cell
            .formations
            .iter()
            .any(|formation| formation.bulk.composition.iter().any(|(name, _)| name == "Water"))));
    }
    #[test]
    fn initial_landscape_uses_all_compound_varieties() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(
            &mut field,
            10.0,
            &crate::resources::default_catalog(),
        );
        let signatures = field
            .cells
            .iter()
            .flat_map(|cell| cell.formations.iter())
            .map(|formation| formation.pattern.material.material.clone())
            .filter_map(|material| structured_signature(&material))
            .collect::<BTreeSet<_>>();
        assert_eq!(signatures.len(), ENVIRONMENTAL_COMPOUND_COUNT);
    }
    #[test]
    fn initial_landscape_has_neighboring_material_coherence() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field);
        let mut comparable_pairs = 0usize;
        let mut matching_pairs = 0usize;
        for y in 0..field.height_cells {
            for x in 0..field.width_cells.saturating_sub(1) {
                let left_index = y * field.width_cells + x;
                let right_index = left_index + 1;
                let left = field.cells[left_index]
                    .formations
                    .iter()
                    .find_map(|formation| structured_signature(&formation.pattern.material.material));
                let right = field.cells[right_index]
                    .formations
                    .iter()
                    .find_map(|formation| structured_signature(&formation.pattern.material.material));
                if let (Some(left), Some(right)) = (left, right) {
                    comparable_pairs += 1;
                    if left == right {
                        matching_pairs += 1;
                    }
                }
            }
        }
        assert!(comparable_pairs > 0);
        assert!(matching_pairs * 4 > comparable_pairs);
    }
    #[test]
    fn initial_landscape_is_not_globally_uniform() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field);
        let signatures = field
            .cells
            .iter()
            .filter_map(|cell| cell.formations.iter().find_map(|formation| structured_signature(&formation.pattern.material.material)))
            .collect::<BTreeSet<_>>();
        assert!(signatures.len() > 1);
    }
    #[test]
    fn initial_landscape_retains_unstructured_material() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field);
        assert!(field.cells.iter().any(|cell| cell
            .formations
            .iter()
            .any(|formation| !formation.pattern.material.material.has_internal_structure())));
    }
    #[test]
    fn initial_landscape_totals_are_finite_and_positive() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field);
        let total = field.total_amount();
        assert!(total.is_finite());
        assert!(total > 0.0);
    }
    #[test]
    fn initial_landscape_seeding_is_deterministic() {
        let mut first = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut second = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut first);
        seed_initial_landscape(&mut second);
        assert_eq!(first.total_amount(), second.total_amount());
        for (first_cell, second_cell) in first.cells.iter().zip(second.cells.iter()) {
            assert_eq!(first_cell.formations, second_cell.formations);
        }
    }
}
