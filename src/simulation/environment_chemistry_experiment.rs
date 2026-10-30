//! Reproducible, organism-free starting state for environmental chemistry validation.

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::state::{EnergyLedger, Simulation};
use crate::structure::Placement;

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

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let catalog = simulation.environment.catalog.clone();
    let resource_names = catalog
        .iter()
        .map(|resource| resource.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        resource_names.len(),
        7,
        "the chemistry experiment requires the seven canonical base resources"
    );

    let mut identities = resource_names.clone();
    while identities.len() < 128 {
        identities.push(resource_names[rng.gen_range(0..resource_names.len())].clone());
    }
    identities.shuffle(&mut rng);

    let pile_sizes = randomized_pile_sizes(&mut rng);
    assert_eq!(pile_sizes.iter().sum::<usize>(), 128);
    assert!(pile_sizes.iter().all(|size| (6..=20).contains(size)));

    let hotspot = crate::hotspot::RadialHotspot::right_bend_for_world(
        simulation.environment.width,
        simulation.environment.height,
    )
    .expect("experiment world must support a right-bend hotspot");
    assert!(simulation.enable_chemistry_experiment_hotspot());

    let centers = randomized_pile_centers(
        pile_sizes.len(),
        hotspot.center_x,
        hotspot.center_y,
        simulation.environment.width,
        simulation.environment.height,
        &mut rng,
    );
    let mut identity_index = 0;
    for (pile_index, pile_size) in pile_sizes.iter().copied().enumerate() {
        let (center_x, center_y) = centers[pile_index];
        let columns = (pile_size as f64).sqrt().ceil() as usize;
        let rows = pile_size.div_ceil(columns);
        for local_index in 0..pile_size {
            let column = local_index % columns;
            let row = local_index / columns;
            let x = center_x + (column as f64 - (columns - 1) as f64 * 0.5) * 2.0;
            let y = center_y + (row as f64 - (rows - 1) as f64 * 0.5) * 2.0;
            let rotation_radians = rng.gen_range(0.0..std::f64::consts::TAU);
            let name = identities[identity_index].clone();
            identity_index += 1;
            let physical = PhysicalMaterial::realized(
                Material::free_base(&name, 1.0),
                vec![Placement {
                    x,
                    y,
                    rotation_radians,
                }],
                &catalog,
            )
            .expect("every canonical base resource must have valid physical geometry");
            assert!(
                simulation.environment.field.deposit_physical(x, y, physical, &catalog),
                "pile {pile_index} resource {local_index} must be inside the world"
            );
        }
    }
    assert_eq!(identity_index, 128);
    assert_eq!(
        simulation
            .environment
            .field
            .cells
            .iter()
            .map(|cell| cell.physical_materials.len())
            .sum::<usize>(),
        128
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

fn randomized_pile_sizes(rng: &mut ChaCha8Rng) -> Vec<usize> {
    let mut remaining = 128;
    let mut sizes = Vec::new();
    while remaining > 20 {
        let maximum = 20.min(remaining - 6);
        let size = rng.gen_range(6..=maximum);
        sizes.push(size);
        remaining -= size;
    }
    sizes.push(remaining);
    sizes
}

fn randomized_pile_centers(
    pile_count: usize,
    hotspot_x: f64,
    hotspot_y: f64,
    width: f64,
    height: f64,
    rng: &mut ChaCha8Rng,
) -> Vec<(f64, f64)> {
    let mut centers = vec![(hotspot_x, hotspot_y)];
    let mut attempts = 0;
    while centers.len() < pile_count && attempts < 20_000 {
        attempts += 1;
        let candidate = (
            rng.gen_range(width * 0.08..width * 0.92),
            rng.gen_range(height * 0.08..height * 0.92),
        );
        let separated = centers.iter().all(|(x, y)| {
            let vertical = (candidate.1 - y + height * 0.5).rem_euclid(height) - height * 0.5;
            (candidate.0 - x).hypot(vertical) >= 70.0
        });
        if separated {
            centers.push(candidate);
        }
    }
    assert_eq!(
        centers.len(),
        pile_count,
        "seeded pile placement must find separated positions"
    );
    centers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_has_128_whole_resources_in_randomized_valid_piles_and_no_organisms() {
        let first = build(42);
        let second = build(42);
        assert_eq!(first.pile_sizes, second.pile_sizes);
        assert_eq!(
            first.pile_sizes.iter().sum::<usize>(),
            128
        );
        assert!(first.pile_sizes.iter().all(|size| (6..=20).contains(size)));
        assert!(first.simulation.organisms.is_empty());
        assert!(first.simulation.hotspot.is_some());
        let material_count = first.simulation.environment.field.cells.iter()
            .map(|cell| cell.physical_materials.len())
            .sum::<usize>();
        assert_eq!(material_count, 128);
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
        let total_material = simulation
            .environment
            .field
            .cells
            .iter()
            .map(|cell| cell.total_amount())
            .sum::<f64>();
        assert!((total_material - 128.0).abs() < 1e-6);
        assert!(
            simulation
                .energy_inventory_snapshot()
                .is_balanced(1e-7),
            "environment-only chemistry ticks must conserve energy"
        );
        assert_eq!(
            simulation
                .environmental_chemistry_events
                .iter()
                .filter(|event| event.reaction == "COMBINE" && event.outcome == "committed")
                .count(),
            simulation.environmental_chemistry_diagnostics.combine_committed
        );
        assert_eq!(
            simulation
                .environmental_chemistry_events
                .iter()
                .filter(|event| event.reaction == "BREAK" && event.outcome == "committed")
                .count(),
            simulation.environmental_chemistry_diagnostics.break_committed
        );
    }
}
