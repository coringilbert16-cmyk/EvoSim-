#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::environment::ActiveMaterialField;
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
pub(crate) const INITIAL_FORMATION_COUNT: usize = 6;
const FORMATION_PARTICLES: usize = 120;
const FORMATION_RADIUS: f64 = 45.0;
const FORMATION_CENTER_FRACTIONS: [(f64, f64); INITIAL_FORMATION_COUNT] = [
    (0.18, 0.18),
    (0.50, 0.18),
    (0.82, 0.18),
    (0.18, 0.82),
    (0.50, 0.82),
    (0.82, 0.82),
];

/// Populate the active field with a small number of physically realized,
/// composition-driven formations. Empty field remains between formations.
/// Each formation uses a composition-specific repeating pattern with bounded
/// random variation so its physical outline is irregular rather than circular.
pub(crate) fn seed_initial_landscape(field: &mut ActiveMaterialField) {
    use crate::physical_material::PhysicalMaterial;
    use crate::structure::Placement;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    let compounds = seed_compounds();
    if compounds.is_empty() || field.width_cells == 0 || field.height_cells == 0 {
        return;
    }

    let width = field.width_cells as f64 * field.cell_size;
    let height = field.height_cells as f64 * field.cell_size;

    for (formation_index, &(fx, fy)) in FORMATION_CENTER_FRACTIONS.iter().enumerate() {
        let center_x = fx * width;
        let center_y = fy * height;
        let composition_indices = formation_material_indices(&compounds, formation_index);
        let seed = formation_seed(&compounds, &composition_indices, formation_index);
        let mut rng = StdRng::seed_from_u64(seed);

        for particle_index in 0..FORMATION_PARTICLES {
            let t = particle_index as f64 / FORMATION_PARTICLES as f64;
            let angle = t * std::f64::consts::TAU * (1.0 + (seed % 3) as f64 * 0.05)
                + rng.gen_range(-0.08..0.08);
            let radial = t.sqrt();
            let harmonic = 1.0
                + 0.12 * (angle * (2.0 + (seed % 4) as f64)).sin()
                + 0.07 * (angle * (3.0 + (seed % 3) as f64) + 1.1).cos();
            let jitter = rng.gen_range(-0.12..0.12);
            let radius = FORMATION_RADIUS * radial * (harmonic + jitter).max(0.25);
            let x = center_x + radius * angle.cos();
            let y = (center_y + radius * angle.sin()).rem_euclid(height);
            if x < 0.0 || x >= width {
                continue;
            }

            let material_index = composition_indices
                [((particle_index as u64 + seed) as usize) % composition_indices.len()];
            let material = &compounds[material_index];
            let local_rotation = angle + rng.gen_range(-0.35..0.35);
            let placements = compound_placements(
                material,
                x,
                y,
                local_rotation,
                rng.gen_range(0.25..0.65),
            );
            let Some(physical) =
                PhysicalMaterial::realized(material.clone(), placements, &crate::resources::default_catalog())
            else {
                continue;
            };

            let Some(index) = field.index_for_position(x, y) else {
                continue;
            };
            field.deposit_physical_at_index(index, physical);
        }
    }
}

fn formation_material_indices(
    compounds: &[Material],
    formation_index: usize,
) -> Vec<usize> {
    let count = compounds.len();
    let first = (formation_index * 5 + 1) % count;
    let second = (first + 3 + formation_index % 4) % count;
    let third = (second + 4 + formation_index % 3) % count;
    let mut indices = vec![first, second];
    if third != first && third != second {
        indices.push(third);
    }
    indices
}

fn formation_seed(
    compounds: &[Material],
    indices: &[usize],
    formation_index: usize,
) -> u64 {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64 ^ formation_index as u64;
    for &index in indices {
        for (name, amount) in &compounds[index].parts {
            for byte in name.bytes() {
                seed = seed.rotate_left(7) ^ u64::from(byte);
            }
            seed ^= amount.to_bits().rotate_left(17);
        }
    }
    seed
}

fn compound_placements(
    material: &Material,
    x: f64,
    y: f64,
    rotation: f64,
    spacing: f64,
) -> Vec<crate::structure::Placement> {
    let count = material.parts.len();
    (0..count)
        .map(|index| {
            let angle = rotation + index as f64 * std::f64::consts::TAU / count.max(1) as f64;
            Placement {
                x: x + spacing * angle.cos(),
                y: y + spacing * angle.sin(),
                rotation_radians: angle,
            }
        })
        .collect()
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
        seed_initial_landscape(&mut field);
        assert!(field.total_amount() > 0.0);
        assert!(field.cells.iter().all(|cell| !cell.materials.is_empty()));
        assert!(field.cells.iter().any(|cell| cell
            .materials
            .iter()
            .any(|material| material.has_internal_structure())));
        assert!(field.cells.iter().any(|cell| cell
            .materials
            .iter()
            .any(|material| material.parts.iter().any(|(name, _)| name == "Water"))));
    }
    #[test]
    fn initial_landscape_uses_all_compound_varieties() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field);
        let signatures = field
            .cells
            .iter()
            .flat_map(|cell| cell.materials.iter())
            .filter_map(structured_signature)
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
                    .materials
                    .iter()
                    .find_map(structured_signature);
                let right = field.cells[right_index]
                    .materials
                    .iter()
                    .find_map(structured_signature);
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
            .filter_map(|cell| cell.materials.iter().find_map(structured_signature))
            .collect::<BTreeSet<_>>();
        assert!(signatures.len() > 1);
    }
    #[test]
    fn initial_landscape_retains_unstructured_material() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field);
        assert!(field.cells.iter().any(|cell| cell
            .materials
            .iter()
            .any(|material| !material.has_internal_structure())));
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
            assert_eq!(first_cell.materials.len(), second_cell.materials.len());
            for (first_material, second_material) in first_cell
                .materials
                .iter()
                .zip(second_cell.materials.iter())
            {
                assert_eq!(first_material.parts, second_material.parts);
                assert_eq!(
                    first_material.internal_bonds,
                    second_material.internal_bonds
                );
            }
        }
    }
}
