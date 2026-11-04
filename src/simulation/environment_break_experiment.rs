//! Small, naturally powered environmental BREAK experiment.
//!
//! This fixture reuses existing compound recipes and the normal simulation
//! tick. It does not alter chemistry properties, activation thresholds, bond
//! energies, or environmental energy sources.

use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::physical_material::PhysicalMaterial;
use crate::resources::{Material, ResourceBaselines};
use crate::state::{EnergyLedger, Simulation};
use crate::structure::{OrganismStructure, Placement};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const REPLICATES_PER_RECIPE: usize = 40;
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

fn restorable_physical(
    material: &Material,
    catalog: &[crate::resources::BaseResource],
    x: f64,
    y: f64,
    rotation: f64,
) -> PhysicalMaterial {
    assert_eq!(
        material.parts.len(),
        2,
        "natural BREAK candidates must remain two-constituent molecules"
    );

    // Search real constituent shapes and orientations. A successful
    // PhysicalMaterial::realized() only proves that bond endpoints can be
    // assigned; restoration additionally requires nonpenetrating geometry.
    // Accept a candidate only when the exact stored arrangement restores.
    const DISTANCE_STEPS: usize = 41;
    const ORIENTATION_STEPS: usize = 16;
    let mut best_contact_distance = f64::INFINITY;
    let mut best_facing = f64::NEG_INFINITY;
    let mut best_contact_factor = 0.0_f64;
    let mut best_engagement = 0.0_f64;
    let mut best_realization: Option<(PhysicalMaterial, f64, f64, f64, f64)> = None;
    let mut best_score = 0.0_f64;
    for distance_step in 0..DISTANCE_STEPS {
        let center_distance = distance_step as f64 * 0.025;
        for axis_step in 0..ORIENTATION_STEPS {
            let axis =
                rotation + std::f64::consts::TAU * axis_step as f64 / ORIENTATION_STEPS as f64;
            let (axis_sin, axis_cos) = axis.sin_cos();
            for orientation_a in 0..ORIENTATION_STEPS {
                let angle_a = rotation
                    + std::f64::consts::TAU * orientation_a as f64 / ORIENTATION_STEPS as f64;
                for orientation_b in 0..ORIENTATION_STEPS {
                    let angle_b = rotation
                        + std::f64::consts::TAU * orientation_b as f64 / ORIENTATION_STEPS as f64;
                    let candidate_placements = vec![
                        Placement {
                            x: x - center_distance * 0.5 * axis_cos,
                            y: y - center_distance * 0.5 * axis_sin,
                            rotation_radians: angle_a,
                        },
                        Placement {
                            x: x + center_distance * 0.5 * axis_cos,
                            y: y + center_distance * 0.5 * axis_sin,
                            rotation_radians: angle_b,
                        },
                    ];
                    let Some(candidate) =
                        PhysicalMaterial::realized(material.clone(), candidate_placements, catalog)
                    else {
                        continue;
                    };
                    let relative = candidate.placements.as_ref().unwrap();
                    let shape_a = candidate.realized_shape_for_part(0, catalog).unwrap();
                    let shape_b = candidate.realized_shape_for_part(1, catalog).unwrap();
                    let part_a = crate::material_geometry::PlacedMaterialPart {
                        part_index: 0,
                        form: shape_a.form,
                        placement: relative[0],
                    };
                    let part_b = crate::material_geometry::PlacedMaterialPart {
                        part_index: 1,
                        form: shape_b.form,
                        placement: relative[1],
                    };
                    if crate::material_geometry::placed_forms_penetrate(
                        &part_a,
                        &part_b,
                        crate::material_geometry::GEOMETRY_EPSILON,
                    ) {
                        continue;
                    }

                    let mut restored = OrganismStructure::new();
                    let Some(restored_indices) = crate::material_restoration::restore_material(
                        &mut restored,
                        &candidate,
                        Placement {
                            x: 0.0,
                            y: 0.0,
                            rotation_radians: 0.0,
                        },
                        catalog,
                    ) else {
                        continue;
                    };
                    let Some(connection) = candidate
                        .internal_connections
                        .as_ref()
                        .and_then(|connections| connections.first())
                    else {
                        continue;
                    };
                    let Some(&unit_a) = restored_indices.get(connection.part_a) else {
                        continue;
                    };
                    let Some(&unit_b) = restored_indices.get(connection.part_b) else {
                        continue;
                    };
                    let Some(contact) = crate::contact::connection_pair_candidates(
                        &restored, unit_a, unit_b, catalog,
                    )
                    .into_iter()
                    .find(|contact| {
                        (contact.endpoint_a == connection.endpoint_a
                            && contact.endpoint_b == connection.endpoint_b)
                            || (contact.endpoint_a == connection.endpoint_b
                                && contact.endpoint_b == connection.endpoint_a)
                    }) else {
                        continue;
                    };
                    let contact_factor = crate::chemistry::contact_factor(
                        contact.distance,
                        crate::chemistry::CHEMICAL_CONTACT_RADIUS,
                    ).unwrap_or(0.0);
                    let engagement =
                        crate::chemistry::interface_engagement(contact.facing).unwrap_or(0.0);
                    if best_contact_distance.is_infinite()
                        || contact_factor * engagement > best_contact_factor * best_engagement
                    {
                        best_contact_distance = contact.distance;
                        best_facing = contact.facing;
                        best_contact_factor = contact_factor;
                        best_engagement = engagement;
                    }
                    // Some(0.0) is a valid "no effective contact" result, not
                    // evidence that this geometry can accumulate chemistry.
                    if contact_factor <= 0.0 || engagement <= 0.0 {
                        continue;
                    }
                    let geometry_material_a = restored.units[unit_a]
                        .material
                        .parts
                        .first()
                        .map(|part| part.0.as_str())
                        .unwrap_or("unknown");
                    let geometry_material_b = restored.units[unit_b]
                        .material
                        .parts
                        .first()
                        .map(|part| part.0.as_str())
                        .unwrap_or("unknown");
                    if crate::geometry_reference_library::resolve_live_contact_candidate(
                        geometry_material_a,
                        &restored.units[unit_a],
                        geometry_material_b,
                        &restored.units[unit_b],
                        contact,
                        catalog,
                    )
                    .is_some()
                    {
                        let score = contact_factor * engagement;
                        if score > best_score {
                            best_score = score;
                            best_realization = Some((
                                candidate,
                                contact.distance,
                                contact.facing,
                                contact_factor,
                                engagement,
                            ));
                        }
                    }
                }
            }
        }
    }

    if let Some((candidate, distance, facing, contact_factor, engagement)) = best_realization {
        println!(
            "[seed-contact] material={:?} distance={:.6} facing={:.6} contact_factor={:.6} engagement={:.6} combined={:.6}",
            material.parts, distance, facing, contact_factor, engagement, best_score
        );
        return candidate;
    }

    panic!(
        "existing compound {:?} has no positive-contact physical realization; best_distance={:.6} best_facing={:.6} best_contact_factor={:.6} best_engagement={:.6} radius={:.6}",
        material.parts,
        best_contact_distance,
        best_facing,
        best_contact_factor,
        best_engagement,
        crate::chemistry::CHEMICAL_CONTACT_RADIUS
    );
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
    // Eighty separated molecule centers fit inside the high-intensity core.
    // Keep source strength, resonance rules, and chemistry scheduling fixed.
    let hotspot = simulation
        .hotspot
        .expect("the default experiment environment must include its hotspot");
    let hotspot_center_x = hotspot.center_x;
    let hotspot_center_y = hotspot.center_y;
    const GRID_COLUMNS: usize = 10;
    const GRID_SPACING: f64 = 1.3;
    for (recipe_index, &(a, b)) in CANDIDATE_RECIPES.iter().enumerate() {
        let material = compounds
            .iter()
            .find(|candidate| contains_recipe(candidate, a, b))
            .unwrap_or_else(|| panic!("existing compound catalog lacks {a}-{b}"));
        for replicate in 0..REPLICATES_PER_RECIPE {
            let index = recipe_index * REPLICATES_PER_RECIPE + replicate;
            let column = index % GRID_COLUMNS;
            let row = index / GRID_COLUMNS;
            let x = hotspot_center_x
                + (column as f64 - (GRID_COLUMNS as f64 - 1.0) * 0.5) * GRID_SPACING;
            let y = hotspot_center_y + (row as f64 - 3.5) * GRID_SPACING;
            let rotation = 0.37 * index as f64;
            // Select a geometry that passes both physical realization and the
            // exact restoration checks used by the live chemistry path.
            let physical = restorable_physical(material, &catalog, x, y, rotation);
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

#[derive(Default)]
struct BondProgressDiagnostics {
    previous: f64,
    maximum: f64,
    positive_samples: u64,
    zero_samples: u64,
    key_present_samples: u64,
    key_missing_samples: u64,
    high_exposure_samples: u64,
    decreases: u64,
    exposure_sum: f64,
    exposure_samples: u64,
    peak_exposure: f64,
    max_activation_signal: f64,
}

fn sample_bond_progress(
    simulation: &Simulation,
    diagnostics: &mut BTreeMap<u64, BondProgressDiagnostics>,
) {
    let Some(hotspot) = simulation.hotspot else {
        return;
    };
    for material in simulation.environment.field.cells.iter()
        .flat_map(|cell| cell.physical_materials.iter())
    {
        let Some(recipe) = CANDIDATE_RECIPES.iter()
            .find(|&&(a, b)| contains_recipe(&material.material, a, b))
        else {
            continue;
        };
        let Some(connection) = material.internal_connections.as_ref()
            .and_then(|connections| connections.first())
        else {
            continue;
        };
        let key = format!(
            "environment|environment:{}:{}:{:?}",
            material.environmental_identity, 0, connection
        );
        let found_progress = simulation.chemical_reaction_accumulation.get(&key).copied();
        let progress = found_progress.unwrap_or(0.0);
        let entry = diagnostics.entry(material.environmental_identity).or_default();
        if found_progress.is_some() { entry.key_present_samples += 1; }
        else { entry.key_missing_samples += 1; }
        if progress > 0.0 {
            entry.positive_samples += 1;
        } else {
            entry.zero_samples += 1;
        }
        if progress + 1e-12 < entry.previous {
            entry.decreases += 1;
        }
        entry.previous = progress;
        entry.maximum = entry.maximum.max(progress);
        let signal = crate::chemistry::normalized_chemistry_to_activation_signal(progress)
            .unwrap_or(0.0);
        entry.max_activation_signal = entry.max_activation_signal.max(signal);

        if let Some(placements) = material.placements.as_ref() {
            let intensities = placements.iter()
                .filter_map(|placement| hotspot.intensity_at(placement.x, placement.y))
                .collect::<Vec<_>>();
            if !intensities.is_empty() {
                let exposure = intensities.iter().sum::<f64>() / intensities.len() as f64;
                entry.exposure_sum += exposure;
                entry.exposure_samples += 1;
                if exposure >= hotspot.peak_energy_per_tick_per_area * 0.85 {
                    entry.high_exposure_samples += 1;
                }
                entry.peak_exposure = entry.peak_exposure.max(exposure);
            }
        }
        let _ = recipe;
    }
}

fn report_bond_progress(
    simulation: &Simulation,
    diagnostics: &BTreeMap<u64, BondProgressDiagnostics>,
) {
    for material in simulation.environment.field.cells.iter()
        .flat_map(|cell| cell.physical_materials.iter())
    {
        let Some(&(a_name, b_name)) = CANDIDATE_RECIPES.iter()
            .find(|&&(a, b)| contains_recipe(&material.material, a, b))
        else {
            continue;
        };
        let Some(connection) = material.internal_connections.as_ref()
            .and_then(|connections| connections.first())
        else {
            continue;
        };
        let Some(stats) = diagnostics.get(&material.environmental_identity) else {
            continue;
        };
        let a = simulation.environment.catalog.iter()
            .find(|resource| resource.name == a_name).unwrap().properties;
        let b = simulation.environment.catalog.iter()
            .find(|resource| resource.name == b_name).unwrap().properties;
        let strength = crate::combine::bond_strength(a, b);
        let barrier = crate::chemistry::activation_barrier_from_bond_properties(
            strength, a.reactivity, b.reactivity,
            ResourceBaselines::from_catalog(&simulation.environment.catalog).reactivity,
        ).unwrap_or(f64::NAN);
        let average_exposure = if stats.exposure_samples == 0 { 0.0 }
            else { stats.exposure_sum / stats.exposure_samples as f64 };
        println!(
            "[bond-progress] id={} recipe={}-{} bond_index=0 progress_now={:.9} progress_max={:.9} activation_signal_max={:.9} barrier={:.9} positive_samples={} zero_samples={} key_present_samples={} key_missing_samples={} decreases={} hotspot_exposure_avg={:.3} hotspot_exposure_peak={:.3} high_exposure_samples={} exposure_samples={} identity={}",
            material.environmental_identity, a_name, b_name, stats.previous, stats.maximum,
            stats.max_activation_signal, barrier, stats.positive_samples, stats.zero_samples,
            stats.key_present_samples, stats.key_missing_samples, stats.decreases,
            average_exposure, stats.peak_exposure, stats.high_exposure_samples,
            stats.exposure_samples, material.environmental_identity
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
        "NATURAL BREAK EXPERIMENT START: seed={seed} duration_seconds={} molecules={} organisms={} world={}x{} hotspot={:?} population_layout=10x8_grid grid_spacing={}",
        duration.as_secs(),
        fixture.molecule_count,
        simulation.organisms.len(),
        simulation.environment.width,
        simulation.environment.height,
        simulation.hotspot,
        1.3
    );

    let mut total_ready = 0_usize;
    let mut total_scheduled = 0_usize;
    let mut total_committed = 0_usize;
    let mut total_rejected = 0_usize;
    let mut bond_progress = BTreeMap::<u64, BondProgressDiagnostics>::new();
    while started.elapsed() < duration {
        simulation.step();
        let counters = simulation.environmental_chemistry_diagnostics;
        total_ready = total_ready.saturating_add(counters.break_ready_candidates);
        total_scheduled = total_scheduled.saturating_add(counters.break_scheduled);
        total_committed = total_committed.saturating_add(counters.break_committed);
        total_rejected = total_rejected.saturating_add(counters.break_rejected);
        sample_bond_progress(&simulation, &mut bond_progress);

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
            report_bond_progress(&simulation, &bond_progress);
        }
    }

    println!("FINAL PER-BOND PROGRESS:");
    report_bond_progress(&simulation, &bond_progress);
    println!(
        "ACCUMULATION KEY SAMPLE: total={} environmental={} sample={:?}",
        simulation.chemical_reaction_accumulation.len(),
        simulation.chemical_reaction_accumulation.keys().filter(|key| key.starts_with("environment|environment:")).count(),
        simulation.chemical_reaction_accumulation.iter().take(24).collect::<Vec<_>>()
    );
    let mut rejection_reasons = BTreeMap::<(String, String), u64>::new();
    for event in simulation
        .environmental_chemistry_events
        .iter()
        .filter(|event| event.reaction == "BREAK" && event.outcome == "rejected")
    {
        let parts = event.source_materials.first().cloned().unwrap_or_default();
        let has = |name: &str| parts.iter().any(|(part, _)| part == name);
        let recipe = if has("Hydrogen") && has("Sulfur") {
            "Hydrogen-Sulfur"
        } else if has("Carbon") && has("Hydrogen") {
            "Carbon-Hydrogen"
        } else {
            "other"
        };
        let reason = event.failure_reason.as_deref().unwrap_or("missing_reason");
        *rejection_reasons
            .entry((recipe.to_string(), reason.to_string()))
            .or_default() += 1;
    }
    println!(
        "[break-rejection-reasons] by_recipe_and_reason={:?}",
        rejection_reasons
    );

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
        assert_eq!(fixture.molecule_count, 80);
        assert!(simulation.organisms.is_empty());
        let materials = simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect::<Vec<_>>();
        assert_eq!(materials.len(), 80);
        let hotspot = simulation.hotspot.expect("fixture must retain its hotspot");
        assert!(
            materials.iter().all(|material| {
                material.placements.as_ref().is_some_and(|placements| {
                    placements.iter().all(|placement| {
                        hotspot
                            .intensity_at(placement.x, placement.y)
                            .is_some_and(|intensity| {
                                intensity >= hotspot.peak_energy_per_tick_per_area * 0.85
                            })
                    })
                })
            }),
            "every candidate constituent must start in the hotspot high-intensity core"
        );
        for material in &materials {
            let mut restored = OrganismStructure::new();
            let restored_indices = crate::material_restoration::restore_material(
                &mut restored,
                material,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                &simulation.environment.catalog,
            )
            .expect("every seeded molecule must pass the live restoration geometry checks");
            assert_eq!(restored_indices.len(), 2);
            assert_eq!(restored.bonds.len(), 1);
            assert_eq!(material.material.internal_bonds.len(), 1);
            let connection = &material.internal_connections.as_ref().unwrap()[0];
            let contact = crate::contact::connection_pair_candidates(
                &restored,
                restored_indices[connection.part_a],
                restored_indices[connection.part_b],
                &simulation.environment.catalog,
            )
            .into_iter()
            .find(|candidate| {
                (candidate.endpoint_a == connection.endpoint_a
                    && candidate.endpoint_b == connection.endpoint_b)
                    || (candidate.endpoint_a == connection.endpoint_b
                        && candidate.endpoint_b == connection.endpoint_a)
            })
            .expect("seeded bond must resolve to its stored contact endpoints");
            assert!(
                crate::chemistry::contact_factor(
                    contact.distance,
                    crate::chemistry::CHEMICAL_CONTACT_RADIUS,
                ).is_some_and(|factor| factor > 0.0),
                "seeded bond must have positive chemical contact, not merely Some(0.0)"
            );
            assert!(
                crate::chemistry::interface_engagement(contact.facing)
                    .is_some_and(|engagement| engagement > 0.0),
                "seeded bond must have positive interface engagement"
            );
            assert_eq!(connection.bond_energy, 0.0);
        }
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
