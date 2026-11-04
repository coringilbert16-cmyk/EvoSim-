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

fn placements(material: &Material, x: f64, y: f64, rotation: f64) -> Vec<Placement> {
    let count = material.parts.len();
    (0..count)
        .map(|index| {
            let angle = rotation + index as f64 * std::f64::consts::TAU / count.max(1) as f64;
            Placement {
                x: x + 0.45 * angle.cos(),
                y: y + 0.45 * angle.sin(),
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

    // Put a few isolated copies near the same exposed side of the world. The
    // normal rotating source, hotspot, field flow, and chemistry scheduler stay
    // enabled; only the starting molecular population is controlled.
    for (recipe_index, &(a, b)) in CANDIDATE_RECIPES.iter().enumerate() {
        let material = compounds
            .iter()
            .find(|candidate| contains_recipe(candidate, a, b))
            .unwrap_or_else(|| panic!("existing compound catalog lacks {a}-{b}"));
        for replicate in 0..REPLICATES_PER_RECIPE {
            let index = recipe_index * REPLICATES_PER_RECIPE + replicate;
            let x = 12.5 + (replicate % 2) as f64 * 25.0;
            let y = 12.5 + (index / 2) as f64 * 25.0;
            let rotation = 0.37 * index as f64;
            let physical = PhysicalMaterial::realized(
                material.clone(),
                placements(material, x, y, rotation),
                &catalog,
            )
            .unwrap_or_else(|| panic!("existing {a}-{b} compound must realize physically"));
            assert!(
                simulation.environment.field.deposit_physical(x, y, physical, &catalog),
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
        let properties_a = catalog.iter().find(|resource| resource.name == a).unwrap().properties;
        let properties_b = catalog.iter().find(|resource| resource.name == b).unwrap().properties;
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
        let frequency_a = crate::harmonics::natural_frequency_cycles_per_day(properties_a, baselines);
        let frequency_b = crate::harmonics::natural_frequency_cycles_per_day(properties_b, baselines);
        let pair_frequency = (frequency_a * frequency_b).sqrt();
        println!(
            "BREAK CANDIDATE {a}-{b}: bond_strength={strength:.6} activation_barrier={barrier:.6} static_potential={potential:.6} natural_pair_frequency={pair_frequency:.3} cycles/day"
        );
    }
}

pub(crate) fn run(seed: u64, max_ticks: u64) {
    let fixture = build(seed);
    let mut simulation = fixture.simulation;
    report_candidate_profiles(&simulation);
    println!(
        "NATURAL BREAK EXPERIMENT START: seed={seed} ticks={max_ticks} molecules={} organisms={} world={}x{} hotspot={:?}",
        fixture.molecule_count,
        simulation.organisms.len(),
        simulation.environment.width,
        simulation.environment.height,
        simulation.hotspot
    );

    let mut total_ready = 0_usize;
    let mut total_scheduled = 0_usize;
    let mut total_committed = 0_usize;
    let mut total_rejected = 0_usize;
    for _ in 0..max_ticks {
        simulation.step();
        let counters = simulation.environmental_chemistry_diagnostics;
        total_ready = total_ready.saturating_add(counters.break_ready_candidates);
        total_scheduled = total_scheduled.saturating_add(counters.break_scheduled);
        total_committed = total_committed.saturating_add(counters.break_committed);
        total_rejected = total_rejected.saturating_add(counters.break_rejected);

        if simulation.tick % 1_000 == 0 {
            let material_count = simulation.environment.field.cells.iter()
                .map(|cell| cell.physical_materials.len())
                .sum::<usize>();
            let bond_count = simulation.environment.field.cells.iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .map(|material| material.internal_connections.as_ref().map_or(0, Vec::len))
                .sum::<usize>();
            let energy = simulation.energy_inventory_snapshot();
            println!(
                "[natural-break] tick={} ready/scheduled/committed/rejected={}/{}/{}/{} bonds={} materials={} accumulation_entries={} resonance={:.6} usable={:.6} energy_residual={:?}",
                simulation.tick,
                total_ready,
                total_scheduled,
                total_committed,
                total_rejected,
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
    let final_bonds = simulation.environment.field.cells.iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .map(|material| material.internal_connections.as_ref().map_or(0, Vec::len))
        .sum::<usize>();
    let final_materials = simulation.environment.field.cells.iter()
        .map(|cell| cell.physical_materials.len())
        .sum::<usize>();
    println!(
        "NATURAL BREAK EXPERIMENT END: tick={} ready/scheduled/committed/rejected={}/{}/{}/{} remaining_bonds={} remaining_materials={} energy_balanced={} energy_residual={:?}",
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
        let materials = simulation.environment.field.cells.iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect::<Vec<_>>();
        assert_eq!(materials.len(), 8);
        assert!(materials.iter().all(|material| {
            material.material.internal_bonds.len() == 1
                && material.internal_connections.as_ref().is_some_and(|bonds| bonds.len() == 1)
                && material.internal_connections.as_ref().unwrap()[0].bond_energy == 0.0
        }));
        assert_eq!(simulation.environment.chemistry_usable_energy, 0.0);
        assert_eq!(simulation.environment.resonant_energy.total_energy(), Some(0.0));
        assert_eq!(simulation.energy_ledger.total_external_energy_input, 0.0);
        assert!(simulation.energy_inventory_snapshot().is_balanced(1e-9));
    }

    #[test]
    fn natural_discovery_accumulates_from_live_molecule_geometry() {
        let mut simulation = build(42).simulation;
        let mut accumulation = std::collections::BTreeMap::new();
        let mut observed_natural_progress = false;
        for tick in 0..128 {
            let mut remaining_work =
                crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK;
            let operations = crate::chemical_reaction::accumulate_environmental(
                &simulation.environment,
                &mut accumulation,
                &mut simulation.chemistry_library,
                tick,
                &mut remaining_work,
            );
            if !operations.is_empty() || accumulation.values().any(|value| *value > 0.0) {
                observed_natural_progress = true;
                break;
            }
        }
        assert!(
            observed_natural_progress,
            "existing H-S/C-H molecules should produce live BREAK activation progress without a synthetic operation"
        );
    }
}
