//! Reproducible, organism-free starting state for environmental chemistry validation.

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::state::{EnergyLedger, Simulation};
use crate::structure::Placement;

const ENVIRONMENT_OBJECT_COUNT: usize = 1_280;
const ENVIRONMENT_SINGLE_RESOURCE_COUNT: usize = 1_200;
const ENVIRONMENT_MOLECULE_COUNT: usize = 80;

/// Seed 1,280 physical objects in randomized piles. The benchmark variant
/// substitutes 80 intact molecules for 80 single-resource objects.
pub(crate) fn seed_randomized_resources(
    field: &mut ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
    seed: u64,
    width: f64,
    height: f64,
    include_molecules: bool,
) -> Vec<usize> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let resource_names = catalog
        .iter()
        .map(|resource| resource.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        resource_names.len(),
        7,
        "the environment requires seven canonical base resources"
    );

    let mut identities = resource_names.clone();
    let single_resource_count = if include_molecules {
        ENVIRONMENT_SINGLE_RESOURCE_COUNT
    } else {
        ENVIRONMENT_OBJECT_COUNT
    };
    let molecule_count = if include_molecules {
        ENVIRONMENT_MOLECULE_COUNT
    } else {
        0
    };
    while identities.len() < single_resource_count {
        identities.push(resource_names[rng.gen_range(0..resource_names.len())].clone());
    }
    identities.shuffle(&mut rng);
    let mut objects: Vec<(Vec<String>, bool)> = identities
        .into_iter()
        .map(|name| (vec![name], false))
        .collect();
    for _ in 0..(molecule_count / 2) {
        let hydrogen_sulfur = (vec!["Hydrogen".into(), "Sulfur".into()], true);
        let carbon_hydrogen = (vec!["Carbon".into(), "Hydrogen".into()], true);
        let hs_index = rng.gen_range(0..=objects.len());
        objects.insert(hs_index, hydrogen_sulfur);
        let ch_index = rng.gen_range(0..=objects.len());
        objects.insert(ch_index, carbon_hydrogen);
    }

    let pile_count = rng.gen_range(3..=10);
    let mut cuts: Vec<usize> = (1_usize..ENVIRONMENT_OBJECT_COUNT).collect();
    cuts.shuffle(&mut rng);
    cuts.truncate(pile_count - 1);
    cuts.sort_unstable();
    let mut previous = 0;
    let mut pile_sizes = Vec::with_capacity(pile_count);
    for cut in cuts
        .into_iter()
        .chain(std::iter::once(ENVIRONMENT_OBJECT_COUNT))
    {
        pile_sizes.push(cut - previous);
        previous = cut;
    }

    let hotspot = crate::hotspot::RadialHotspot::right_bend_for_world(width, height)
        .expect("the configured world must support the right-bend hotspot");
    let centers = randomized_pile_centers(
        &pile_sizes,
        hotspot.center_x,
        hotspot.center_y,
        width,
        height,
        &mut rng,
    );
    let mut object_index = 0;
    for (pile_index, pile_size) in pile_sizes.iter().copied().enumerate() {
        let (center_x, center_y) = centers[pile_index];
        let columns = (pile_size as f64).sqrt().ceil() as usize;
        let rows = pile_size.div_ceil(columns);
        for local_index in 0..pile_size {
            let column = local_index % columns;
            let row = local_index / columns;
            let (parts, is_molecule) = &objects[object_index];
            let max_x = if *is_molecule {
                width - 2.8
            } else {
                width - 1.0
            };
            let x = (center_x + (column as f64 - (columns - 1) as f64 * 0.5) * 1.5)
                .clamp(1.0, max_x);
            let y =
                (center_y + (row as f64 - (rows - 1) as f64 * 0.5) * 1.5).clamp(1.0, height - 1.0);
            let rotation = rng.gen_range(0.0..std::f64::consts::TAU);
            let placement = Placement {
                x,
                y,
                rotation_radians: rotation,
            };
            let material = if *is_molecule {
                Material {
                    parts: parts.iter().cloned().map(|name| (name, 1.0)).collect(),
                    internal_bonds: vec![crate::resources::InternalBond {
                        part_a: 0,
                        part_b: 1,
                    }],
                }
            } else {
                Material::free_base(&parts[0], 1.0)
            };
            let placements = if *is_molecule {
                vec![
                    placement,
                    Placement {
                        x: x + 1.8,
                        y,
                        rotation_radians: rotation,
                    },
                ]
            } else {
                vec![placement]
            };
            let physical = PhysicalMaterial::realized(material, placements, catalog)
                .expect("seeded resource or molecule must have valid physical geometry");
            object_index += 1;
            assert!(
                field.deposit_physical(x, y, physical, catalog),
                "randomized pile {pile_index} object {local_index} must remain in bounds"
            );
        }
    }
    assert_eq!(object_index, ENVIRONMENT_OBJECT_COUNT);
    pile_sizes
}

pub(crate) struct EnvironmentalChemistryFixture {
    pub(crate) simulation: Simulation,
    pub(crate) pile_sizes: Vec<usize>,
}

pub(crate) fn build(seed: u64) -> EnvironmentalChemistryFixture {
    let mut simulation = Simulation::new(seed, 10.0);
    simulation.organisms.clear();
    simulation.active_transformations.clear();
    simulation.decomposing_bodies.clear();
    simulation.chemical_reaction_accumulation.clear();
    simulation.environment.chemistry_usable_energy = 0.0;
    simulation.environment.resonant_energy = Default::default();
    simulation.environment.field = ActiveMaterialField::new(
        simulation.environment.width,
        simulation.environment.height,
        DEFAULT_CELL_SIZE,
    );
    let catalog = simulation.environment.catalog.clone();

    let pile_sizes = seed_randomized_resources(
        &mut simulation.environment.field,
        &catalog,
        seed,
        simulation.environment.width,
        simulation.environment.height,
        true,
    );

    // The default constructor's organism is deliberately removed. Rebase the
    // ledger on the actual environment-only tick-zero inventory so no organism
    // energy or structure can act as a hidden environmental source.
    simulation.energy_ledger = EnergyLedger::default();
    let initial = simulation.energy_inventory_snapshot();
    simulation.energy_ledger.initial_usable_energy_held = initial.current_usable_energy;
    simulation.energy_ledger.total_usable_energy_held = initial.current_usable_energy;
    simulation.energy_ledger.initial_resonant_energy_held = initial.current_resonant_energy;
    simulation.energy_ledger.total_resonant_energy_held = initial.current_resonant_energy;
    simulation.energy_ledger.initial_structural_energy_held = initial.current_structural_energy;
    simulation.energy_ledger.total_structural_energy_held = initial.current_structural_energy;

    EnvironmentalChemistryFixture {
        simulation,
        pile_sizes,
    }
}

fn randomized_pile_centers(
    pile_sizes: &[usize],
    hotspot_x: f64,
    hotspot_y: f64,
    width: f64,
    height: f64,
    rng: &mut ChaCha8Rng,
) -> Vec<(f64, f64)> {
    let pile_radii = pile_sizes
        .iter()
        .map(|pile_size| {
            let columns = (*pile_size as f64).sqrt().ceil() as usize;
            let rows = pile_size.div_ceil(columns);
            let half_width = (columns.saturating_sub(1) as f64) * 0.75 + 1.0;
            let half_height = (rows.saturating_sub(1) as f64) * 0.75 + 1.0;
            half_width.hypot(half_height)
        })
        .collect::<Vec<_>>();
    let mut centers = vec![(hotspot_x, hotspot_y)];
    let mut attempts = 0;
    while centers.len() < pile_sizes.len() && attempts < 20_000 {
        attempts += 1;
        let radius = pile_radii[centers.len()];
        let candidate = (
            rng.gen_range(radius..width - radius),
            rng.gen_range(radius..height - radius),
        );
        let separated = centers
            .iter()
            .zip(pile_radii.iter())
            .all(|((x, y), existing_radius)| {
                let vertical = (candidate.1 - *y + height * 0.5).rem_euclid(height) - height * 0.5;
                (candidate.0 - *x).hypot(vertical) >= radius + *existing_radius + 3.0
            });
        if separated {
            centers.push(candidate);
        }
    }
    assert_eq!(
        centers.len(),
        pile_sizes.len(),
        "seeded pile placement must find separated positions"
    );
    centers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_simulation_keeps_1280_individual_base_resources() {
        let simulation = Simulation::new(73, 10.0);
        assert_eq!(simulation.environment.width, 250.0);
        assert_eq!(simulation.environment.height, 250.0);
        let materials = simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect::<Vec<_>>();
        assert_eq!(materials.len(), ENVIRONMENT_OBJECT_COUNT);
        assert!(materials.iter().all(|material| {
            material.material.parts.len() == 1 && material.material.parts[0].1 == 1.0
        }));
        assert!(simulation.hotspot.is_some());
    }

    #[test]
    fn fixture_has_1280_objects_with_80_intact_molecules_and_no_organisms() {
        let first = build(42);
        let second = build(42);
        assert_eq!(first.pile_sizes, second.pile_sizes);
        assert_eq!(
            first.pile_sizes.iter().sum::<usize>(),
            ENVIRONMENT_OBJECT_COUNT
        );
        assert!((3..=10).contains(&first.pile_sizes.len()));
        assert!(first.pile_sizes.iter().all(|size| *size >= 1));
        assert!(first.pile_sizes.iter().any(|size| *size < 6 || *size > 20));
        assert!(first.simulation.organisms.is_empty());
        assert!(first.simulation.hotspot.is_some());
        assert_eq!(first.simulation.environment.width, 250.0);
        assert_eq!(first.simulation.environment.height, 250.0);
        let material_count = first
            .simulation
            .environment
            .field
            .cells
            .iter()
            .map(|cell| cell.physical_materials.len())
            .sum::<usize>();
        assert_eq!(material_count, ENVIRONMENT_OBJECT_COUNT);
        let materials = first
            .simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect::<Vec<_>>();
        assert_eq!(
            materials
                .iter()
                .filter(|m| m.material.parts.len() == 1)
                .count(),
            ENVIRONMENT_SINGLE_RESOURCE_COUNT
        );
        assert_eq!(
            materials
                .iter()
                .filter(|m| m.material.parts.len() == 2)
                .count(),
            ENVIRONMENT_MOLECULE_COUNT
        );
        let hs = materials
            .iter()
            .filter(|m| {
                m.material.parts.iter().any(|p| p.0 == "Sulfur")
                    && m.material.parts.iter().any(|p| p.0 == "Hydrogen")
            })
            .count();
        let ch = materials
            .iter()
            .filter(|m| {
                m.material.parts.iter().any(|p| p.0 == "Carbon")
                    && m.material.parts.iter().any(|p| p.0 == "Hydrogen")
            })
            .count();
        assert_eq!(hs, 40);
        assert_eq!(ch, 40);
        let mut represented_types = std::collections::BTreeSet::new();
        for cell in &first.simulation.environment.field.cells {
            for material in &cell.physical_materials {
                for (name, amount) in &material.material.parts {
                    assert_eq!(*amount, 1.0);
                    represented_types.insert(name.clone());
                }
            }
        }
        assert_eq!(represented_types.len(), 7);
        assert_eq!(
            first.simulation.energy_inventory_snapshot().residual(),
            Some(0.0)
        );

        let mut simulation = first.simulation;
        for _ in 0..4 {
            simulation.step();
        }
        assert_eq!(simulation.tick, 4);
        assert_eq!(simulation.environmental_chemistry_diagnostics.tick, 4);
        assert_eq!(
            simulation
                .environmental_chemistry_diagnostics
                .hotspot_settlement_succeeded,
            Some(true)
        );
        let total_material = simulation
            .environment
            .field
            .cells
            .iter()
            .map(|cell| cell.total_amount())
            .sum::<f64>();
        assert!(
            (total_material
                - (ENVIRONMENT_SINGLE_RESOURCE_COUNT + 2 * ENVIRONMENT_MOLECULE_COUNT) as f64)
                .abs()
                < 1e-6
        );
        assert!(
            simulation.energy_inventory_snapshot().is_balanced(1e-7),
            "environment-only chemistry ticks must conserve energy"
        );
        assert_eq!(
            simulation
                .environmental_chemistry_events
                .iter()
                .filter(|event| event.reaction == "COMBINE" && event.outcome == "committed")
                .count(),
            simulation
                .environmental_chemistry_diagnostics
                .combine_committed
        );
        assert_eq!(
            simulation
                .environmental_chemistry_events
                .iter()
                .filter(|event| event.reaction == "BREAK" && event.outcome == "committed")
                .count(),
            simulation
                .environmental_chemistry_diagnostics
                .break_committed
        );
    }
}
