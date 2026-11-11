//! Reproducible, organism-free starting state for environmental chemistry validation.

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::environment::{ActiveMaterialField, DEFAULT_CELL_SIZE};
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::state::{EnergyLedger, Simulation};
use crate::structure::{OrganismStructure, Placement, StructuralUnit};

const ENVIRONMENT_OBJECT_COUNT: usize = 1_280;
const ENVIRONMENT_SINGLE_RESOURCE_COUNT: usize = 1_200;
const ENVIRONMENT_MOLECULE_COUNT: usize = 80;

/// Build the seeded molecule using the same eligibility and bond-energy path as
/// an ordinary COMBINE operation, so environmental BREAK sees realistic bonds.
pub(crate) fn realize_natural_molecule(
    parts: &[String],
    placement: Placement,
    catalog: &[crate::resources::BaseResource],
) -> PhysicalMaterial {
    assert_eq!(
        parts.len(),
        2,
        "seed molecules must contain two constituents"
    );
    let mut structure = OrganismStructure::new();
    let unit_a = structure.add_unit(StructuralUnit::new(parts[0].clone(), placement));
    let unit_b = structure.add_unit(StructuralUnit::new(parts[1].clone(), placement));
    assert!(structure
        .units
        .iter_mut()
        .all(|unit| unit.realize_default_geometry(catalog)));

    let mut selected_contact = None;
    'poses: for direction_step in 0..16 {
        let direction =
            placement.rotation_radians + std::f64::consts::TAU * f64::from(direction_step) / 16.0;
        let (sin, cos) = direction.sin_cos();
        for distance_step in 20..=100 {
            let separation = f64::from(distance_step) * 0.025;
            for rotation_step in 0..16 {
                structure.units[unit_b].placement = Placement {
                    x: placement.x + separation * cos,
                    y: placement.y + separation * sin,
                    rotation_radians: placement.rotation_radians
                        + std::f64::consts::TAU * f64::from(rotation_step) / 16.0,
                };
                let incoming = structure.units[unit_b].clone();
                if crate::construction_runtime::placed_unit_overlaps(
                    &structure,
                    &incoming,
                    &[unit_b],
                    catalog,
                ) {
                    continue;
                }
                let mut cache = crate::contact::ConnectionCompatibilityCache::new();
                for candidate in crate::contact::connection_pair_candidates_cached(
                    &structure, unit_a, unit_b, catalog, &mut cache,
                ) {
                    if candidate.distance > crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                        || !candidate.available_a
                        || !candidate.available_b
                        || !candidate.facing.is_finite()
                        || candidate.facing <= 0.0
                    {
                        continue;
                    }
                    let Some((_, _, investment, required_energy)) =
                        crate::combine_runtime::selected_candidate_evaluation(
                            &structure, unit_a, unit_b, candidate, catalog,
                        )
                    else {
                        continue;
                    };
                    selected_contact = Some((candidate, investment, required_energy));
                    break 'poses;
                }
            }
        }
    }
    let (candidate, investment, required_energy) =
        selected_contact.expect("seeded molecule pair must admit an ordinary COMBINE contact");
    let mut usable_energy = required_energy;
    let mut ledger = EnergyLedger {
        total_usable_energy_held: usable_energy,
        initial_usable_energy_held: usable_energy,
        total_structural_energy_held: crate::energy_ledger::structural_potential(&structure)
            .expect("unbonded seed structure must have valid structural energy"),
        ..EnergyLedger::default()
    };
    ledger.initial_structural_energy_held = ledger.total_structural_energy_held;
    crate::combine_runtime::form_selected_bond(
        &mut structure,
        unit_a,
        unit_b,
        candidate,
        investment,
        catalog,
        &mut crate::contact::ConnectionCompatibilityCache::new(),
        &mut ledger,
        &mut usable_energy,
    )
    .expect("seeded molecule must form through ordinary COMBINE");
    assert_eq!(structure.bonds.len(), 1);
    assert!(structure.bonds[0].bond_energy.is_finite() && structure.bonds[0].bond_energy > 0.0);
    let mut products = crate::environmental_combine::physical_materials_from_structure_components(
        &structure, catalog,
    )
    .expect("ordinary COMBINE product must export as a physical molecule");
    assert_eq!(products.len(), 1);
    let molecule = products
        .pop()
        .expect("one bonded component must be exported");
    let connection = molecule
        .internal_connections
        .as_ref()
        .and_then(|bonds| bonds.first())
        .expect("exported molecule must retain its physical bond");
    assert!(connection.bond_energy.is_finite() && connection.bond_energy > 0.0);
    let mut restored = OrganismStructure::new();
    let restored_indices = crate::material_restoration::restore_material(
        &mut restored,
        &molecule,
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        },
        catalog,
    )
    .expect("ordinary COMBINE molecule must restore for environmental BREAK");
    assert!(
        crate::contact::connection_pair_candidate_for_endpoints(
            &restored,
            restored_indices[connection.part_a],
            restored_indices[connection.part_b],
            catalog,
            connection.endpoint_a,
            connection.endpoint_b,
        )
        .is_some(),
        "ordinary COMBINE molecule must resolve its recorded bond endpoints after restoration"
    );
    molecule
}

/// Seed exactly 1,280 individual base resources in randomized piles. Pile sizes
/// are an unrestricted random partition, not a fixed 6–20 range.
pub(crate) fn seed_randomized_resources(
    field: &mut ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
    seed: u64,
    width: f64,
    height: f64,
) -> Vec<usize> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let resource_names = catalog
        .iter()
        .map(|resource| resource.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        resource_names.len(),
        7,
        "the default environment retains all seven canonical resources"
    );

    let mut identities = resource_names.clone();
    while identities.len() < ENVIRONMENT_OBJECT_COUNT {
        identities.push(resource_names[rng.gen_range(0..resource_names.len())].clone());
    }
    identities.shuffle(&mut rng);

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
    let mut identity_index = 0;
    for (pile_index, pile_size) in pile_sizes.iter().copied().enumerate() {
        let (center_x, center_y) = centers[pile_index];
        let columns = (pile_size as f64).sqrt().ceil() as usize;
        let rows = pile_size.div_ceil(columns);
        for local_index in 0..pile_size {
            let column = local_index % columns;
            let row = local_index / columns;
            let x = (center_x + (column as f64 - (columns - 1) as f64 * 0.5) * 1.5)
                .clamp(1.0, width - 1.0);
            let y =
                (center_y + (row as f64 - (rows - 1) as f64 * 0.5) * 1.5).clamp(1.0, height - 1.0);
            let placement = Placement {
                x,
                y,
                rotation_radians: rng.gen_range(0.0..std::f64::consts::TAU),
            };
            let material = Material::free_base(&identities[identity_index], 1.0);
            identity_index += 1;
            let physical = PhysicalMaterial::realized(material, vec![placement], catalog)
                .expect("canonical base resources must have valid physical geometry");
            assert!(
                field.deposit_physical(x, y, physical, catalog),
                "randomized pile {pile_index} resource {local_index} must remain in bounds"
            );
        }
    }
    assert_eq!(identity_index, ENVIRONMENT_OBJECT_COUNT);
    pile_sizes
}

/// Seed 1,280 physical objects for transport benchmarking: 1,200 free resources
/// plus 80 intact molecules (40 H-S and 40 C-H) in randomized piles.
fn seed_randomized_molecular_resources(
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
        6,
        "active chemistry experiments exclude Water until it is explicitly revisited"
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
        objects.push((vec!["Hydrogen".into(), "Sulfur".into()], true));
        objects.push((vec!["Carbon".into(), "Hydrogen".into()], true));
    }
    objects.shuffle(&mut rng);

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
            let x = (center_x + (column as f64 - (columns - 1) as f64 * 0.5) * 1.5)
                .clamp(1.0, width - 2.8);
            let y =
                (center_y + (row as f64 - (rows - 1) as f64 * 0.5) * 1.5).clamp(1.0, height - 1.0);
            let rotation = rng.gen_range(0.0..std::f64::consts::TAU);
            let placement = Placement {
                x,
                y,
                rotation_radians: rotation,
            };
            let (parts, is_molecule) = &objects[object_index];
            let physical = if *is_molecule {
                realize_natural_molecule(parts, placement, catalog)
            } else {
                PhysicalMaterial::realized(
                    Material::free_base(&parts[0], 1.0),
                    vec![placement],
                    catalog,
                )
                .expect("canonical base resource must have valid physical geometry")
            };
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
    simulation.record_environmental_molecular_history = true;
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
    // Keep Water in the authoritative catalog and all historical data, but
    // omit it from active chemistry experiment inventories until revisited.
    let catalog: Vec<_> = simulation
        .environment
        .catalog
        .iter()
        .filter(|resource| resource.name != "Water")
        .cloned()
        .collect();

    let pile_sizes = seed_randomized_molecular_resources(
        &mut simulation.environment.field,
        &catalog,
        seed,
        simulation.environment.width,
        simulation.environment.height,
        true,
    );

    assert!(
        simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| &cell.physical_materials)
            .all(|material| material
                .material
                .parts
                .iter()
                .all(|(name, _)| name != "Water")),
        "active environmental chemistry experiments must not seed Water"
    );

    // Establish stable IDs for the complete tick-zero inventory before any
    // diagnostics or reaction can observe the environment.
    simulation
        .environment
        .field
        .assign_missing_material_identities();

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

/// Compare recorded environmental bond endpoints before and after one ambient
/// transport step without advancing chemistry or mutating the live fixture.
pub(crate) fn audit_transport_endpoints(seed: u64) {
    let fixture = build(seed);
    let catalog = fixture.simulation.environment.catalog.clone();
    audit_endpoint_resolution("before", &fixture.simulation.environment.field, &catalog);

    let mut transported = fixture.simulation.environment.field.clone();
    let moved = crate::environment_flow::transport_field_with_singleton_center_velocity_reuse::<true>(
        &mut transported,
        &catalog,
        fixture.simulation.environment.width,
        fixture.simulation.environment.height,
    );
    println!("TRANSPORT ENDPOINT AUDIT: moved_materials={moved}");
    audit_endpoint_resolution("after", &transported, &catalog);
}

fn audit_endpoint_resolution(
    label: &str,
    field: &ActiveMaterialField,
    catalog: &[crate::resources::BaseResource],
) {
    let mut bonds = 0_u64;
    let mut restored = 0_u64;
    let mut endpoint_pairs_resolved = 0_u64;
    let mut failures = 0_u64;
    for (cell_index, cell) in field.cells.iter().enumerate() {
        for (material_index, material) in cell.physical_materials.iter().enumerate() {
            let Some(connections) = material.internal_connections.as_ref() else {
                continue;
            };
            if connections.is_empty() {
                continue;
            }
            let mut structure = OrganismStructure::new();
            let Some(indices) = crate::material_restoration::restore_material(
                &mut structure,
                material,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                catalog,
            ) else {
                continue;
            };
            restored += connections.len() as u64;
            for (bond_index, connection) in connections.iter().enumerate() {
                bonds += 1;
                let (Some(&unit_a), Some(&unit_b)) = (
                    indices.get(connection.part_a),
                    indices.get(connection.part_b),
                ) else {
                    failures += 1;
                    continue;
                };
                if crate::contact::connection_pair_candidate_for_endpoints(
                    &structure,
                    unit_a,
                    unit_b,
                    catalog,
                    connection.endpoint_a,
                    connection.endpoint_b,
                )
                .is_some()
                {
                    endpoint_pairs_resolved += 1;
                    continue;
                }

                failures += 1;
                if failures <= 6 {
                    let endpoints = [
                        crate::contact::endpoint_world_point(
                            connection.endpoint_a,
                            &structure.units[unit_a],
                            catalog,
                        ),
                        crate::contact::endpoint_world_point(
                            connection.endpoint_b,
                            &structure.units[unit_b],
                            catalog,
                        ),
                    ];
                    let all_candidates = crate::contact::connection_pair_candidates(
                        &structure, unit_a, unit_b, catalog,
                    );
                    println!(
                        "TRANSPORT ENDPOINT FAILURE: stage={label} cell={cell_index} material={material_index} identity={} bond={bond_index} parts=({}, {}) recorded=({:?}, {:?}) endpoint_world={endpoints:?} candidate_count={} placements={:?}",
                        material.environmental_identity,
                        connection.part_a,
                        connection.part_b,
                        connection.endpoint_a,
                        connection.endpoint_b,
                        all_candidates.len(),
                        material.placements
                    );
                }
            }
        }
    }
    println!(
        "TRANSPORT ENDPOINT AUDIT: stage={label} bonds={bonds} restored={restored} resolved={endpoint_pairs_resolved} unresolved={failures}"
    );
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
    fn molecular_history_recording_does_not_change_chemistry_or_energy() {
        let fixture = build(42);
        let mut recorded = fixture.simulation;
        let mut unrecorded = recorded.clone();
        unrecorded.record_environmental_molecular_history = false;
        let mut observed_lifecycle_event = false;

        for _ in 0..120 {
            recorded.step();
            unrecorded.step();
            observed_lifecycle_event |= recorded
                .environmental_chemistry_events
                .iter()
                .any(|event| !event.molecular_lifecycle.is_empty());
        }

        assert_eq!(
            recorded.environment.field.cells, unrecorded.environment.field.cells,
            "recording must not alter physical inventory, identities, or geometry"
        );
        assert_eq!(recorded.energy_ledger, unrecorded.energy_ledger);
        assert_eq!(
            recorded.environment.chemistry_usable_energy,
            unrecorded.environment.chemistry_usable_energy
        );
        assert_eq!(
            recorded.chemical_reaction_accumulation.len(),
            unrecorded.chemical_reaction_accumulation.len(),
            "history recording must preserve reaction-accumulation keys"
        );
        for (key, recorded_value) in &recorded.chemical_reaction_accumulation {
            let unrecorded_value = unrecorded
                .chemical_reaction_accumulation
                .get(key)
                .expect("both runs should retain the same reaction-accumulation keys");
            let tolerance = 1e-12 * recorded_value.abs().max(unrecorded_value.abs()).max(1.0);
            assert!(
                (recorded_value - unrecorded_value).abs() <= tolerance,
                "history recording must not materially change reaction accumulation for {key}: recorded={recorded_value}, unrecorded={unrecorded_value}"
            );
        }
        assert!(
            observed_lifecycle_event,
            "the controlled 120-tick fixture should exercise committed lifecycle recording"
        );
    }

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
        let catalog = &first.simulation.environment.catalog;
        for molecule in materials
            .iter()
            .filter(|material| material.material.parts.len() == 2)
        {
            let connection = molecule
                .internal_connections
                .as_ref()
                .and_then(|connections| connections.first())
                .expect("seeded molecules must retain their COMBINE connection");
            assert!(
                connection.bond_energy.is_finite() && connection.bond_energy > 0.0,
                "seeded molecules must not contain zero-energy bonds"
            );

            let mut restored = OrganismStructure::new();
            let indices = crate::material_restoration::restore_material(
                &mut restored,
                molecule,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                catalog,
            )
            .expect("naturally seeded molecule must restore with its physical endpoints");
            assert_eq!(restored.bonds.len(), 1);
            let unit_a = &restored.units[indices[connection.part_a]];
            let unit_b = &restored.units[indices[connection.part_b]];
            let properties_a = unit_a.properties(catalog).unwrap();
            let properties_b = unit_b.properties(catalog).unwrap();
            let resource_a = catalog
                .iter()
                .find(|resource| resource.name == unit_a.material.parts[0].0)
                .unwrap();
            let resource_b = catalog
                .iter()
                .find(|resource| resource.name == unit_b.material.parts[0].0)
                .unwrap();
            let expected_energy = crate::combine::conservative_pair_potential(
                resource_a.adhesion,
                resource_b.adhesion,
                0.0,
                crate::chemistry::CHEMICAL_CONTACT_RADIUS,
                crate::chemistry::CHEMICAL_MAX_FORCE,
            )
            .unwrap()
            .bond_energy;
            assert!(
                (connection.bond_energy - expected_energy).abs() <= 1e-9,
                "stored seed bond energy must equal the ordinary COMBINE intrinsic potential"
            );
            assert_eq!(restored.bonds[0].bond_energy, connection.bond_energy);
        }
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
        assert_eq!(represented_types.len(), 6);
        assert!(!represented_types.contains("Water"));
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
