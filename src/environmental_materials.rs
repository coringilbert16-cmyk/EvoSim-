#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::environment::ActiveMaterialField;
use serde::{Deserialize, Serialize};
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

/// Configuration for the finite, physically realized resource cloud surrounding the initial organism.
/// The cloud is a spatial formation only; its contents live in the active field as ordinary physical materials.\n/// Once acquired, material is no longer environmental cloud material and cannot be reclaimed by the cloud.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct ResourceCloud {
    pub(crate) center_x: f64,
    pub(crate) center_y: f64,
    pub(crate) radius: f64,
    pub(crate) max_material_moves_per_tick: usize,
    pub(crate) movement_step: f64,
}

impl ResourceCloud {
    pub(crate) fn initial(center_x: f64, center_y: f64) -> Self {
        Self {
            center_x,
            center_y,
            radius: 70.0,
            max_material_moves_per_tick: 24,
            movement_step: 1.5,
        }
    }
}

const RESOURCE_CLOUD_PARTICLES: usize = 360;

/// Seed a dense, finite population of atomic and composite physical materials
/// around the starting organism. No logical resource inventory is created.
pub(crate) fn seed_resource_cloud(
    field: &mut ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
    cloud: &ResourceCloud,
    seed: u64,
) {
    use crate::physical_material::PhysicalMaterial;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    let compounds = seed_compounds();
    let mut rng = StdRng::seed_from_u64(seed ^ 0xC10D_5EED);
    for index in 0..RESOURCE_CLOUD_PARTICLES {
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let radial = rng.gen::<f64>().sqrt();
        let radius = cloud.radius * radial;
        let x = cloud.center_x + radius * angle.cos();
        let y = cloud.center_y + radius * angle.sin();
        let material = if rng.gen_bool(0.33) {
            let name =
                ["Carbon", "Hydrogen", "Nitrogen", "Phosphorus", "Sulfur"][rng.gen_range(0..5)];
            Material::free_base(name, 1.0)
        } else {
            compounds[rng.gen_range(0..compounds.len())].clone()
        };
        let placements = if material.parts.len() == 1 {
            vec![Placement {
                x,
                y,
                rotation_radians: angle,
            }]
        } else {
            compound_placements(&material, x, y, angle, rng.gen_range(0.25..0.65))
        };
        let Some(physical) = PhysicalMaterial::realized(material, placements, catalog) else {
            continue;
        };
        if let Some(cell) = field.index_for_position(x, y) {
            field.deposit_physical_at_index(cell, physical);
        }
    }
}

/// Move a bounded number of cloud materials by small random displacements.
/// This deliberately avoids pairwise diffusion physics while allowing the
/// cloud contents to rearrange and remain a persistent environmental source.
pub(crate) fn advance_resource_cloud<R: Rng + ?Sized>(
    field: &mut ActiveMaterialField,
    cloud: &ResourceCloud,
    rng: &mut R,
) {
    let candidate_indices = field.cells_within_radius(
        cloud.center_x,
        cloud.center_y,
        cloud.radius + cloud.movement_step + 2.0,
    );
    let mut moved = Vec::new();
    let mut moved_count = 0;
    for index in candidate_indices {
        let cell = &mut field.cells[index];
        let mut remaining = Vec::with_capacity(cell.physical_materials.len());
        for mut physical in cell.physical_materials.drain(..) {
            let should_move = moved_count < cloud.max_material_moves_per_tick
                && physical
                    .placements
                    .as_ref()
                    .and_then(|placements| placements.first())
                    .map(|placement| {
                        let dx = placement.x - cloud.center_x;
                        let dy = placement.y - cloud.center_y;
                        dx * dx + dy * dy <= cloud.radius * cloud.radius
                    })
                    .unwrap_or(false)
                && rng.gen_bool(0.5);
            if !should_move {
                remaining.push(physical);
                continue;
            }
            let angle = rng.gen_range(0.0..std::f64::consts::TAU);
            let distance = rng.gen_range(0.0..=cloud.movement_step);
            let dx = distance * angle.cos();
            let dy = distance * angle.sin();
            if let Some(placements) = physical.placements.as_mut() {
                for placement in placements {
                    placement.x += dx;
                    placement.y += dy;
                }
            }
            let Some(first) = physical.placements.as_ref().and_then(|p| p.first()) else {
                remaining.push(physical);
                continue;
            };
            let boundary_dx = first.x - cloud.center_x;
            let boundary_dy = first.y - cloud.center_y;
            if boundary_dx * boundary_dx + boundary_dy * boundary_dy > cloud.radius * cloud.radius {
                if let Some(placements) = physical.placements.as_mut() {
                    for placement in placements {
                        placement.x -= dx;
                        placement.y -= dy;
                    }
                }
                remaining.push(physical);
            } else {
                moved_count += 1;
                moved.push(physical);
            }
        }
        cell.physical_materials = remaining;
    }
    for physical in moved {
        if let Some(placement) = physical.placements.as_ref().and_then(|p| p.first()) {
            let _ = field.deposit_physical(placement.x, placement.y, physical);
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
    use super::{
        seed_compounds, seed_initial_landscape, FORMATION_PARTICLES, INITIAL_FORMATION_COUNT,
    };
    use crate::environment::ActiveMaterialField;
    use std::collections::BTreeSet;
    use rand::SeedableRng;

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
    fn seed_set_has_expected_size_and_structure() {
        let compounds = seed_compounds();
        assert_eq!(compounds.len(), 12);
        assert!(compounds.iter().all(|material| material.is_valid()
            && material.has_internal_structure()
            && material.parts.len() >= 2));
    }

    #[test]
    fn resource_cloud_is_dense_finite_and_mixed() {
        let mut field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let cloud = super::ResourceCloud::initial(500.0, 500.0);
        super::seed_resource_cloud(&mut field, &crate::resources::default_catalog(), &cloud, 7);
        let materials = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect::<Vec<_>>();
        assert_eq!(materials.len(), super::RESOURCE_CLOUD_PARTICLES);
        assert!(materials.iter().any(|material| material.material.parts.len() == 1));
        assert!(materials.iter().any(|material| material.material.parts.len() > 1));
        assert!(materials.iter().all(|material| material.is_realized()));
        assert!(materials.iter().all(|material| {
            let placement = material.placements.as_ref().unwrap().first().unwrap();
            let dx = placement.x - cloud.center_x;
            let dy = placement.y - cloud.center_y;
            dx * dx + dy * dy <= cloud.radius * cloud.radius
        }));
    }

    #[test]
    fn resource_cloud_movement_is_bounded_and_deterministic() {
        let catalog = crate::resources::default_catalog();
        let cloud = super::ResourceCloud::initial(500.0, 500.0);
        let mut first = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut second = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        super::seed_resource_cloud(&mut first, &catalog, &cloud, 7);
        super::seed_resource_cloud(&mut second, &catalog, &cloud, 7);
        let before = first.clone();
        let mut rng_a = rand::rngs::StdRng::seed_from_u64(99);
        let mut rng_b = rand::rngs::StdRng::seed_from_u64(99);
        super::advance_resource_cloud(&mut first, &cloud, &mut rng_a);
        super::advance_resource_cloud(&mut second, &cloud, &mut rng_b);
        assert_eq!(first, second);
        assert_eq!(
            first
                .cells
                .iter()
                .map(|cell| cell.physical_materials.len())
                .sum::<usize>(),
            before
                .cells
                .iter()
                .map(|cell| cell.physical_materials.len())
                .sum::<usize>()
        );
        assert_ne!(first, before);
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
