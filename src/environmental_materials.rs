#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::environment::ActiveMaterialField;
use crate::resources::{combine_materials, Material};
use crate::structure::Placement;

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

fn compound(parts: &[(&str, f64)]) -> Material {
    let inputs = parts
        .iter()
        .map(|(name, amount)| Material::free_base(*name, *amount))
        .collect::<Vec<_>>();
    combine_materials(&inputs)
}

/// Populate the active field with a deterministic, spatially correlated
/// starting landscape. No terrain categories are introduced: local character
/// comes entirely from material composition, quantity, and neighboring cells.
/// The vent emits ten independently selected physical packets per tick.
pub(crate) const VENT_EMISSION_QUANTITY: usize = 10;
pub(crate) const VENT_POSITION: (f64, f64) = (500.0, 450.0);
const VENT_SPREAD_RADIUS: f64 = 6.0;

pub(crate) fn emit_vents(
    field: &mut ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
    rng: &mut impl rand::Rng,
) {
    let compounds = seed_compounds();
    if compounds.is_empty() {
        return;
    }

    for _ in 0..VENT_EMISSION_QUANTITY {
        let material = compounds[rng.gen_range(0..compounds.len())].clone();
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let radius = rng.gen_range(0.0..VENT_SPREAD_RADIUS);
        let x = VENT_POSITION.0 + radius * angle.cos();
        let y = VENT_POSITION.1 + radius * angle.sin();
        let placements = compound_placements(
            &material,
            x,
            y,
            angle,
            rng.gen_range(0.25..0.65),
        );
        let Some(physical) =
            crate::physical_material::PhysicalMaterial::realized(material, placements, catalog)
        else {
            continue;
        };
        let _ = field.deposit(x, y, physical);
    }
}

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
pub(crate) fn seed_initial_landscape(
    field: &mut ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
) {
    use crate::physical_material::PhysicalMaterial;
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
            let placements =
                compound_placements(material, x, y, local_rotation, rng.gen_range(0.25..0.65));
            let Some(physical) = PhysicalMaterial::realized(material.clone(), placements, catalog)
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

fn formation_material_indices(compounds: &[Material], formation_index: usize) -> Vec<usize> {
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

fn formation_seed(compounds: &[Material], indices: &[usize], formation_index: usize) -> u64 {
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
    use rand::SeedableRng;

    use super::{
        seed_compounds, seed_initial_landscape, FORMATION_PARTICLES, INITIAL_FORMATION_COUNT,
    };
    use crate::environment::ActiveMaterialField;
    use std::collections::BTreeSet;

    fn physical_signature(material: &crate::physical_material::PhysicalMaterial) -> Vec<String> {
        let mut names = material
            .material
            .parts
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn vent_emits_ten_realized_material_packets() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let catalog = crate::resources::default_catalog();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        emit_vents(&mut field, &catalog, &mut rng);

        let packets: Vec<_> = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(packets.len(), VENT_EMISSION_QUANTITY);
        assert!(packets.iter().all(|packet| packet.is_realized()));
        assert!(packets.iter().all(|packet| {
            packet.placements.as_ref().is_some_and(|placements| {
                placements.iter().all(|placement| {
                    let dx = placement.x - VENT_POSITION.0;
                    let dy = placement.y - VENT_POSITION.1;
                    dx * dx + dy * dy <= VENT_SPREAD_RADIUS * VENT_SPREAD_RADIUS
                })
            })
        }));
    }

    #[test]
    fn seed_set_has_expected_size_and_structure() {
        let compounds = seed_compounds();
        assert_eq!(compounds.len(), 12);
        assert!(compounds.iter().all(|material| material.is_valid()
            && material.has_internal_structure()
            && material.parts.len() >= 2));
    }

    #[test]
    fn initial_landscape_uses_a_small_number_of_physical_formations() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, &crate::resources::default_catalog());
        let physical_count: usize = field
            .cells
            .iter()
            .map(|cell| cell.physical_materials.len())
            .sum();
        assert!(physical_count > INITIAL_FORMATION_COUNT);
        assert!(physical_count <= INITIAL_FORMATION_COUNT * FORMATION_PARTICLES);
    }

    #[test]
    fn initial_landscape_contains_empty_field_between_formations() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, &crate::resources::default_catalog());
        let occupied = field
            .cells
            .iter()
            .filter(|cell| !cell.physical_materials.is_empty())
            .count();
        assert!(occupied < field.cells.len());
    }

    #[test]
    fn formations_contain_multiple_compositions() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, &crate::resources::default_catalog());
        let signatures = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(physical_signature)
            .collect::<BTreeSet<_>>();
        assert!(signatures.len() >= 3);
    }

    #[test]
    fn initial_landscape_is_physically_realized() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        seed_initial_landscape(&mut field, &crate::resources::default_catalog());
        assert!(field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .all(|material| material.is_realized()));
    }

    #[test]
    fn initial_landscape_seeding_is_deterministic() {
        let mut first = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut second = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let catalog = crate::resources::default_catalog();
        seed_initial_landscape(&mut first, &catalog);
        seed_initial_landscape(&mut second, &catalog);
        assert_eq!(first.cells, second.cells);
    }
}
