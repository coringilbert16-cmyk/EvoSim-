use crate::field::ActiveMaterialField;
use crate::resources::{combine_materials, Material};

/// Small, reusable structured-material seeds for the initial environment.
///
/// These are deliberately compositions rather than terrain types. Their
environmental meaning comes from composition + internal structure + where
they are placed in the active field.
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
pub(crate) fn seed_initial_landscape(field: &mut ActiveMaterialField) {
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
        let field_c = ((nx * 5.0 + ny * 3.7 + 1.7).sin() + 1.0) * 0.5;
        let selector = ((field_a * 0.55 + field_b * 0.35 + field_c * 0.10)
            * compounds.len() as f64) as usize
            % compounds.len();

        field.deposit_at_index(index, scaled_material(&compounds[selector], 4.0));

        if field_c > 0.62 {
            let secondary = (selector + 3 + (field_b * 4.0) as usize) % compounds.len();
            field.deposit_at_index(index, scaled_material(&compounds[secondary], 2.0));
        }

        if field_a > 0.72 {
            field.deposit_at_index(index, Material::free_base("Hydrogen", 2.0));
        }
        if field_b < 0.22 {
            field.deposit_at_index(index, Material::free_base("Water", 3.0));
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
    use crate::field::ActiveMaterialField;

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
    fn seed_set_contains_carbon_and_non_carbon_compositions() {
        let compounds = seed_compounds();
        assert!(compounds.iter().any(|material| {
            material.parts.iter().any(|(name, _)| name == "Carbon")
        }));
        assert!(compounds.iter().any(|material| {
            material.parts.iter().all(|(name, _)| name != "Carbon")
        }));
    }

    #[test]
    fn initial_landscape_is_populated_and_structured() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field);

        assert!(field.total_amount() > 0.0);
        assert!(field.cells.iter().all(|cell| !cell.materials.is_empty()));
        assert!(field.cells.iter().any(|cell| {
            cell.materials.iter().any(|material| material.has_internal_structure())
        }));
        assert!(field.cells.iter().any(|cell| {
            cell.materials.iter().any(|material| material.parts.iter().any(|(name, _)| name == "Water"))
        }));
    }
}
