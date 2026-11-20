//! Deterministic, experiment-only replenishment for PLAN-004.
//!
//! This injects free base elements as external matter. It does not inject
//! compounds, reactions, usable energy, or resonance.

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::state::Simulation;
use crate::structure::Placement;

pub(crate) const PLAN004_RESOURCE_FEED_PULSE_COUNT: usize = 6;
pub(crate) const PLAN004_RESOURCE_FEED_PULSE_INTERVAL_SECONDS: u64 = 600;
pub(crate) const PLAN004_RESOURCE_FEED_FIRST_PULSE_SECONDS: u64 = 300;
pub(crate) const PLAN004_RESOURCES_PER_ELEMENT_PER_PULSE: usize = 12;
pub(crate) const PLAN004_RESOURCE_FEED_PULSE_SIZE: usize =
    PLAN004_RESOURCES_PER_ELEMENT_PER_PULSE * 6;
pub(crate) const PLAN004_RESOURCE_FEED_TOTAL: usize =
    PLAN004_RESOURCE_FEED_PULSE_SIZE * PLAN004_RESOURCE_FEED_PULSE_COUNT;

const ACTIVE_ELEMENTS: [&str; 6] = [
    "Carbon",
    "Hydrogen",
    "Nitrogen",
    "Oxygen",
    "Phosphorus",
    "Sulfur",
];

#[derive(Clone)]
pub(crate) struct ResourceFeedParticle {
    pub(crate) element: String,
    pub(crate) material: PhysicalMaterial,
}

/// Capture the tick-zero positions of free resources. Feed placements sample
/// this fixed reference distribution so the intervention does not introduce
/// a separate uniform-spatial-distribution treatment.
pub(crate) fn initial_free_resource_positions(simulation: &Simulation) -> Vec<(f64, f64)> {
    let positions = simulation
        .environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .filter(|material| material.material.parts.len() == 1)
        .flat_map(|material| material.placements.iter().flatten())
        .map(|placement| (placement.x, placement.y))
        .collect::<Vec<_>>();
    assert_eq!(
        positions.len(),
        1_200,
        "PLAN-004 feed placement must sample the standard tick-zero free-resource distribution"
    );
    positions
}

/// Add one deterministic pulse of 72 free atoms: 12 of each active element.
/// A separate RNG stream ensures feeding does not consume or reseed the
/// simulation's own random-number stream. Positions are sampled from the
/// unchanged tick-zero free-resource spatial distribution with small jitter.
pub(crate) fn inject_plan004_resource_pulse(
    simulation: &mut Simulation,
    seed: u64,
    pulse_index: usize,
    reference_positions: &[(f64, f64)],
) -> Vec<ResourceFeedParticle> {
    assert!(pulse_index < PLAN004_RESOURCE_FEED_PULSE_COUNT);
    assert!(!reference_positions.is_empty());
    let energy_before = simulation.energy_inventory_snapshot();
    let mut rng = ChaCha8Rng::seed_from_u64(
        seed ^ 0x504c_414e_3030_3400_u64.wrapping_add(pulse_index as u64),
    );
    let mut elements = ACTIVE_ELEMENTS
        .iter()
        .flat_map(|element| std::iter::repeat_n(*element, PLAN004_RESOURCES_PER_ELEMENT_PER_PULSE))
        .collect::<Vec<_>>();
    elements.shuffle(&mut rng);

    let catalog = simulation.environment.catalog.clone();
    let width = simulation.environment.width;
    let height = simulation.environment.height;
    let mut locations = Vec::with_capacity(PLAN004_RESOURCE_FEED_PULSE_SIZE);

    for element in elements {
        let (reference_x, reference_y) =
            reference_positions[rng.gen_range(0..reference_positions.len())];
        let x = (reference_x + rng.gen_range(-0.4..0.4)).clamp(1.0, width - 1.0);
        let y = (reference_y + rng.gen_range(-0.4..0.4)).clamp(1.0, height - 1.0);
        let placement = Placement {
            x,
            y,
            rotation_radians: rng.gen_range(0.0..std::f64::consts::TAU),
        };
        let material = PhysicalMaterial::realized(
            Material::free_base(element, 1.0),
            vec![placement],
            &catalog,
        )
        .expect("canonical free base resources must have valid physical geometry");
        let cell_index = simulation
            .environment
            .field
            .index_for_position(x, y)
            .expect("feed position must be inside the environment");
        let material_index = simulation.environment.field.cells[cell_index]
            .physical_materials
            .len();
        assert!(
            simulation
                .environment
                .field
                .deposit_physical(x, y, material, &catalog),
            "external base resource must be accepted by the environment"
        );
        locations.push((element.to_owned(), x, y, cell_index, material_index));
    }

    simulation
        .environment
        .field
        .assign_missing_material_identities();

    let particles = locations
        .into_iter()
        .map(|(element, _, _, cell_index, material_index)| {
            let material = simulation.environment.field.cells[cell_index].physical_materials
                [material_index]
                .clone();
            assert_ne!(
                material.environmental_identity, 0,
                "every externally supplied atom must receive a stable identity"
            );
            ResourceFeedParticle { element, material }
        })
        .collect::<Vec<_>>();

    assert_eq!(particles.len(), PLAN004_RESOURCE_FEED_PULSE_SIZE);
    let mut counts = std::collections::BTreeMap::new();
    for particle in &particles {
        *counts.entry(particle.element.as_str()).or_insert(0_usize) += 1;
    }
    assert!(ACTIVE_ELEMENTS
        .iter()
        .all(|element| counts.get(*element) == Some(&PLAN004_RESOURCES_PER_ELEMENT_PER_PULSE)));

    let energy_after = simulation.energy_inventory_snapshot();
    assert_eq!(
        energy_after, energy_before,
        "free-matter input must not change any energy inventory or ledger flow"
    );

    particles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_feed_pulse_adds_72_free_atoms_with_balanced_elemental_composition() {
        let mut simulation = crate::environment_chemistry_experiment::build(42).simulation;
        let before_amount = simulation.environment.field.total_amount();
        let before_energy = simulation.energy_inventory_snapshot();
        let reference_positions = initial_free_resource_positions(&simulation);
        assert_eq!(reference_positions.len(), 1_200);
        let particles = inject_plan004_resource_pulse(&mut simulation, 42, 0, &reference_positions);

        assert_eq!(particles.len(), 72);
        assert_eq!(
            simulation.environment.field.total_amount(),
            before_amount + 72.0
        );
        let mut counts = std::collections::BTreeMap::new();
        for particle in &particles {
            *counts.entry(particle.element.as_str()).or_insert(0_usize) += 1;
            assert_eq!(particle.material.material.parts.len(), 1);
            assert_eq!(particle.material.material.parts[0].1, 1.0);
            assert!(particle
                .material
                .internal_connections
                .as_ref()
                .is_none_or(|connections| connections.is_empty()));
        }
        assert!(ACTIVE_ELEMENTS
            .iter()
            .all(|element| counts.get(*element) == Some(&12)));
        let ids = particles
            .iter()
            .map(|particle| particle.material.environmental_identity)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), 72);
        let after_energy = simulation.energy_inventory_snapshot();
        assert_eq!(
            after_energy.current_usable_energy,
            before_energy.current_usable_energy
        );
        assert_eq!(
            after_energy.current_resonant_energy,
            before_energy.current_resonant_energy
        );
        assert_eq!(
            after_energy.current_structural_energy,
            before_energy.current_structural_energy
        );
        assert!(after_energy.is_balanced(1e-7));
    }

    #[test]
    fn feed_pulse_is_reproducible_and_does_not_depend_on_simulation_rng() {
        let mut first = crate::environment_chemistry_experiment::build(43).simulation;
        let mut second = crate::environment_chemistry_experiment::build(43).simulation;
        let first_positions = initial_free_resource_positions(&first);
        let second_positions = initial_free_resource_positions(&second);
        let first_particles = inject_plan004_resource_pulse(&mut first, 43, 2, &first_positions);
        let second_particles = inject_plan004_resource_pulse(&mut second, 43, 2, &second_positions);
        assert_eq!(
            first_particles
                .iter()
                .map(|particle| (
                    particle.element.clone(),
                    particle.material.environmental_identity,
                    particle.material.placements.as_ref().unwrap()[0].x,
                    particle.material.placements.as_ref().unwrap()[0].y
                ))
                .collect::<Vec<_>>(),
            second_particles
                .iter()
                .map(|particle| (
                    particle.element.clone(),
                    particle.material.environmental_identity,
                    particle.material.placements.as_ref().unwrap()[0].x,
                    particle.material.placements.as_ref().unwrap()[0].y
                ))
                .collect::<Vec<_>>()
        );
    }
}
