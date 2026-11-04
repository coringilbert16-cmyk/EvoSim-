//! Small, naturally powered environmental BREAK experiment.
//!
//! This fixture reuses existing compound recipes and the normal simulation
//! tick. It does not alter chemistry properties, activation thresholds, bond
//! energies, or environmental energy sources.

use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::physical_material::PhysicalMaterial;
use crate::resources::{Material, ResourceBaselines};
use crate::state::{EnergyLedger, Simulation};
use crate::structure::Placement;
use std::time::{Duration, Instant};

const REPLICATES_PER_RECIPE: usize = 4;
const CANDIDATE_RECIPES: [(&str, &str); 2] = [("Hydrogen", "Sulfur"), ("Carbon", "Hydrogen")];

pub(crate) struct NaturalBreakFixture {
    pub(crate) simulation: Simulation,
    pub(crate) molecule_count: usize,
}

fn contains_recipe(material: &Material, a: &str, b: &str) -> bool {
    material.parts.len() == 2
        && material.parts.iter().any(|(name, _)| name == a)
        && material.parts.iter().any(|(name, _)| name == b)
        && material.internal_bonds.len() == 1
}

fn placements(x: f64, y: f64, rotation: f64, spacing: f64, part_count: usize) -> Vec<Placement> {
    (0..part_count)
        .map(|index| {
            let angle = rotation + index as f64 * std::f64::consts::TAU / part_count.max(1) as f64;
            Placement {
                x: x + spacing * angle.cos(),
                y: y + spacing * angle.sin(),
                rotation_radians: angle,
            }
        })
        .collect()
}

pub(crate) fn build(seed: u64) -> NaturalBreakFixture {
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
    let compounds = crate::environmental_materials::seed_compounds();
    let mut molecule_count = 0;

    // Cluster the candidate molecules around the configured hotspot center.
    // Eight separated molecule centers fit inside its high-intensity core, so
    // the normal hotspot can energize them without changing source strength,
    // resonance rules, field flow, or chemistry scheduling.
    let hotspot = simulation
        .hotspot
        .expect("the default experiment environment must include its hotspot");
    let hotspot_center_x = hotspot.center_x;
    let hotspot_center_y = hotspot.center_y;
    const HOTSPOT_POPULATION_RADIUS: f64 = 8.0;
    for (recipe_index, &(a, b)) in CANDIDATE_RECIPES.iter().enumerate() {
        let material = compounds
            .iter()
            .find(|candidate| contains_recipe(candidate, a, b))
            .unwrap_or_else(|| panic!("existing compound catalog lacks {a}-{b}"));
        for replicate in 0..REPLICATES_PER_RECIPE {
            let index = recipe_index * REPLICATES_PER_RECIPE + replicate;
            let angle = std::f64::consts::TAU * index as f64
                / (REPLICATES_PER_RECIPE * CANDIDATE_RECIPES.len()) as f64;
            let x = hotspot_center_x + HOTSPOT_POPULATION_RADIUS * angle.cos();
            let y = hotspot_center_y + HOTSPOT_POPULATION_RADIUS * angle.sin();
            let rotation = 0.37 * index as f64;
            // Use the same radial constituent layout as the existing environmental
            // compound seeder. Try deterministic spacings and retain only a
            // realization accepted by the normal physical contact validator.
            let physical = (0..9)
                .filter_map(|attempt| {
                    let spacing = 0.25 + attempt as f64 * 0.05;
                    PhysicalMaterial::realized(
                        material.clone(),
                        placements(x, y, rotation, spacing, material.parts.len()),
                        &catalog,
                    )
                })
                .next()
                .unwrap_or_else(|| {
                    panic!("existing {a}-{b} compound has no valid physical realization")
                });
            let cell_index = simulation
                .environment
                .field
                .index_for_position(x, y)
                .expect("candidate molecule position must be inside the experiment field");
            assert!(
                simulation
                    .environment
                    .field
                    .deposit_physical_at_index(cell_index, physical, &catalog),
                "candidate molecule must remain inside the experiment field"
            );
            molecule_count += 1;
        }
    }

    // The default constructor's initial organism is removed. Rebase the energy
    // ledger on the actual molecule-only tick-zero state; no synthetic energy
    // grant or preloaded resonance is introduced.
    simulation.energy_ledger = EnergyLedger::default();
    let initial = simulation.energy_inventory_snapshot();
    simulation.energy_ledger.initial_usable_energy_held = initial.current_usable_energy;
    simulation.energy_ledger.total_usable_energy_held = initial.current_usable_energy;
    simulation.energy_ledger.initial_resonant_energy_held = initial.current_resonant_energy;
    simulation.energy_ledger.total_resonant_energy_held = initial.current_resonant_energy;
    simulation.energy_ledger.initial_structural_energy_held = initial.current_structural_energy;
    simulation.energy_ledger.total_structural_energy_held = initial.current_structural_energy;

    NaturalBreakFixture {
        simulation,
        molecule_count,
    }
}

fn report_candidate_profiles(simulation: &Simulation) {
    let catalog = &simulation.environment.catalog;
    let baselines = ResourceBaselines::from_catalog(catalog);
    for &(a, b) in &CANDIDATE_RECIPES {
        let properties_a = catalog
            .iter()
            .find(|resource| resource.name == a)
            .unwrap()
            .properties;
        let properties_b = catalog
            .iter()
            .find(|resource| resource.name == b)
            .unwrap()
            .properties;
        let strength = crate::combine::bond_strength(properties_a, properties_b);
        let barrier = crate::chemistry::activation_barrier_from_bond_properties(
            strength,
            properties_a.reactivity,
            properties_b.reactivity,
            baselines.reactivity,
        )
        .unwrap_or(f64::NAN);
        let potential = crate::chemistry::interaction_potential(
            properties_a.chemical_position.unwrap(),
            properties_b.chemical_position.unwrap(),
            crate::chemistry::CHEMICAL_K,
            crate::chemistry::CHEMICAL_D_MAX,
        )
        .unwrap_or(f64::NAN);
        let frequency_a =
            crate::harmonics::natural_frequency_cycles_per_day(properties_a, baselines);
        let frequency_b =
            crate::harmonics::natural_frequency_cycles_per_day(properties_b, baselines);
        let pair_frequency = (frequency_a * frequency_b).sqrt();
        println!(
            "BREAK CANDIDATE {a}-{b}: bond_strength={strength:.6} activation_barrier={barrier:.6} static_potential={potential:.6} natural_pair_frequency={pair_frequency:.3} cycles/day"
        );
    }
}

pub(crate) fn run(seed: u64, duration: Duration) {
    let fixture = build(seed);
    let mut simulation = fixture.simulation;
    report_candidate_profiles(&simulation);
    let started = Instant::now();
    let mut last_report = started;
    println!(
        "NATURAL BREAK EXPERIMENT START: seed={seed} duration_seconds={} molecules={} organisms={} world={}x{} hotspot={:?} hotspot_population_radius={}",
        duration.as_secs(),
        fixture.molecule_count,
        simulation.organisms.len(),
        simulation.environment.width,
        simulation.environment.height,
        simulation.hotspot,
        8.0
    );

    let mut total_ready = 0_usize;
    let mut total_scheduled = 0_usize;
    let mut total_committed = 0_usize;
    let mut total_rejected = 0_usize;
    while started.elapsed() < duration {
        simulation.step();
        let counters = simulation.environmental_chemistry_diagnostics;
        total_ready = total_ready.saturating_add(counters.break_ready_candidates);
        total_scheduled = total_scheduled.saturating_add(counters.break_scheduled);
        total_committed = total_committed.saturating_add(counters.break_committed);
        total_rejected = total_rejected.saturating_add(counters.break_rejected);

        if last_report.elapsed() >= Duration::from_secs(5) {
            last_report = Instant::now();
            let material_count = simulation
                .environment
                .field
                .cells
                .iter()
                .map(|cell| cell.physical_materials.len())
                .sum::<usize>();
            let bond_count = simulation
                .environment
                .field
                .cells
                .iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .map(|material| material.internal_connections.as_ref().map_or(0, Vec::len))
                .sum::<usize>();
            let energy = simulation.energy_inventory_snapshot();
            let intact_hydrogen_sulfur = simulation
                .environment
                .field
                .cells
                .iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .filter(|physical| contains_recipe(&physical.material, "Hydrogen", "Sulfur"))
                .count();
            let intact_carbon_hydrogen = simulation
                .environment
                .field
                .cells
                .iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .filter(|physical| contains_recipe(&physical.material, "Carbon", "Hydrogen"))
                .count();
            println!(
                "[natural-break] elapsed_seconds={:.3} tick={} ready/scheduled/committed/rejected={}/{}/{}/{} intact_HS={} intact_CH={} bonds={} materials={} accumulation_entries={} resonance={:.6} usable={:.6} energy_residual={:?}",
                started.elapsed().as_secs_f64(),
                simulation.tick,
                total_ready,
                total_scheduled,
                total_committed,
                total_rejected,
                intact_hydrogen_sulfur,
                intact_carbon_hydrogen,
                bond_count,
                material_count,
                counters.accumulation_entries,
                simulation.environment.resonant_energy.total_energy().unwrap_or(f64::NAN),
                simulation.environment.chemistry_usable_energy,
                energy.residual()
            );
        }
    }

    let final_inventory = simulation.energy_inventory_snapshot();
    let final_bonds = simulation
        .environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .map(|material| material.internal_connections.as_ref().map_or(0, Vec::len))
        .sum::<usize>();
    let final_materials = simulation
        .environment
        .field
        .cells
        .iter()
        .map(|cell| cell.physical_materials.len())
        .sum::<usize>();
    println!(
        "NATURAL BREAK EXPERIMENT END: elapsed_seconds={:.3} ticks={} ready/scheduled/committed/rejected={}/{}/{}/{} remaining_bonds={} remaining_materials={} energy_balanced={} energy_residual={:?}",
        started.elapsed().as_secs_f64(),
        simulation.tick,
        total_ready,
        total_scheduled,
        total_committed,
        total_rejected,
        final_bonds,
        final_materials,
        final_inventory.is_balanced(1e-7),
        final_inventory.residual()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_uses_existing_weak_bond_recipes_without_preloading_energy() {
        let fixture = build(42);
        let simulation = fixture.simulation;
        assert_eq!(fixture.molecule_count, 8);
        assert!(simulation.organisms.is_empty());
        let materials = simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect::<Vec<_>>();
        assert_eq!(materials.len(), 8);
        let hotspot = simulation.hotspot.expect("fixture must retain its hotspot");
        assert!(materials.iter().all(|material| {
            material.placements.as_ref().is_some_and(|placements| {
                placements.iter().all(|placement| {
                    hotspot
                        .intensity_at(placement.x, placement.y)
                        .is_some_and(|intensity| intensity >= hotspot.peak_energy_per_tick_per_area * 0.85)
                })
            })
        }), "every candidate constituent must start in the hotspot high-intensity core");
        assert!(materials.iter().all(|material| {
            material.material.internal_bonds.len() == 1
                && material
                    .internal_connections
                    .as_ref()
                    .is_some_and(|bonds| bonds.len() == 1)
                && material.internal_connections.as_ref().unwrap()[0].bond_energy == 0.0
        }));
        assert_eq!(simulation.environment.chemistry_usable_energy, 0.0);
        assert_eq!(
            simulation.environment.resonant_energy.total_energy(),
            Some(0.0)
        );
        assert_eq!(simulation.energy_ledger.total_external_energy_input, 0.0);
        assert!(simulation.energy_inventory_snapshot().is_balanced(1e-9));
    }

    #[test]
    fn natural_discovery_runs_without_forcing_break_or_granting_energy() {
        let mut simulation = build(42).simulation;
        let mut accumulation = std::collections::BTreeMap::new();
        for tick in 0..128 {
            let mut remaining_work =
                crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK;
            let _operations = crate::chemical_reaction::accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                &mut simulation.chemistry_library,
                tick,
                &mut remaining_work,
            );
        }
        assert_eq!(simulation.environment.chemistry_usable_energy, 0.0);
        assert_eq!(
            simulation.environment.resonant_energy.total_energy(),
            Some(0.0)
        );
        assert!(simulation.energy_inventory_snapshot().is_balanced(1e-7));
    }
}
