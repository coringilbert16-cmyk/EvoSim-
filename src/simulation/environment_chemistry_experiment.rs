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
        "the environment requires seven canonical base resources"
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
    dispersion: DispersionMode,
    starting_mixture: StartingMixture,
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
    let mut baseline_identities = resource_names.clone();
    while baseline_identities.len() < single_resource_count {
        baseline_identities.push(resource_names[rng.gen_range(0..resource_names.len())].clone());
    }
    // Keep the complete per-resource inventory identical across starting
    // mixtures. Only the allocation between bonded molecules and free
    // resources changes.
    let baseline_molecules = if include_molecules {
        StartingMixture::Baseline.molecule_parts()
    } else {
        Vec::new()
    };
    let target_molecules = if include_molecules {
        starting_mixture.molecule_parts()
    } else {
        Vec::new()
    };
    let mut total_inventory = std::collections::BTreeMap::<String, usize>::new();
    for name in &baseline_identities {
        *total_inventory.entry(name.clone()).or_default() += 1;
    }
    for parts in &baseline_molecules {
        for name in parts {
            *total_inventory.entry(name.clone()).or_default() += 1;
        }
    }
    let mut target_inventory = std::collections::BTreeMap::<String, usize>::new();
    for parts in &target_molecules {
        for name in parts {
            *target_inventory.entry(name.clone()).or_default() += 1;
        }
    }
    let mut identities = if starting_mixture == StartingMixture::Baseline {
        baseline_identities
    } else {
        let mut adjusted = Vec::with_capacity(single_resource_count);
        for name in &resource_names {
            let total = total_inventory.get(name).copied().unwrap_or_default();
            let bonded = target_inventory.get(name).copied().unwrap_or_default();
            let free = total.checked_sub(bonded).unwrap_or_else(|| {
                panic!(
                    "starting mixture {:?} exceeds total inventory for {name}",
                    starting_mixture
                )
            });
            assert!(
                free > 0,
                "every canonical resource must remain independently available"
            );
            adjusted.extend(std::iter::repeat(name.clone()).take(free));
        }
        adjusted
    };
    assert_eq!(identities.len(), single_resource_count);
    identities.shuffle(&mut rng);

    let mut objects: Vec<(Vec<String>, bool)> = identities
        .into_iter()
        .map(|name| (vec![name], false))
        .collect();
    for parts in target_molecules {
        objects.push((parts, true));
    }
    assert_eq!(objects.len(), ENVIRONMENT_OBJECT_COUNT);
    assert_eq!(
        molecule_count,
        objects.iter().filter(|(_, bonded)| *bonded).count()
    );
    objects.shuffle(&mut rng);

    let pile_sizes = match dispersion {
        DispersionMode::Randomized => {
            let pile_count = rng.gen_range(3..=10);
            let mut cuts: Vec<usize> = (1_usize..ENVIRONMENT_OBJECT_COUNT).collect();
            cuts.shuffle(&mut rng);
            cuts.truncate(pile_count - 1);
            cuts.sort_unstable();
            let mut previous = 0;
            let mut sizes = Vec::with_capacity(pile_count);
            for cut in cuts
                .into_iter()
                .chain(std::iter::once(ENVIRONMENT_OBJECT_COUNT))
            {
                sizes.push(cut - previous);
                previous = cut;
            }
            sizes
        }
        DispersionMode::Uniform => vec![1; ENVIRONMENT_OBJECT_COUNT],
        DispersionMode::Clustered => vec![40; 32],
    };

    let hotspot = crate::hotspot::RadialHotspot::right_bend_for_world(width, height)
        .expect("the configured world must support the right-bend hotspot");
    let mut positions = Vec::with_capacity(ENVIRONMENT_OBJECT_COUNT);
    match dispersion {
        DispersionMode::Randomized | DispersionMode::Clustered => {
            let centers = randomized_pile_centers(
                &pile_sizes,
                hotspot.center_x,
                hotspot.center_y,
                width,
                height,
                &mut rng,
            );
            for (group_index, (center_x, center_y)) in centers.into_iter().enumerate() {
                let count = pile_sizes[group_index];
                let columns = (count as f64).sqrt().ceil() as usize;
                let rows = count.div_ceil(columns);
                for local_index in 0..count {
                    let column = local_index % columns;
                    let row = local_index / columns;
                    let x = (center_x + (column as f64 - (columns - 1) as f64 * 0.5) * 1.5)
                        .clamp(1.0, width - 2.8);
                    let y = (center_y + (row as f64 - (rows - 1) as f64 * 0.5) * 1.5)
                        .clamp(1.0, height - 1.0);
                    positions.push((x, y));
                }
            }
        }
        DispersionMode::Uniform => {
            let columns = (ENVIRONMENT_OBJECT_COUNT as f64 * width / height)
                .sqrt()
                .ceil() as usize;
            let rows = ENVIRONMENT_OBJECT_COUNT.div_ceil(columns);
            for index in 0..ENVIRONMENT_OBJECT_COUNT {
                let column = index % columns;
                let row = index / columns;
                let x = ((column as f64 + 0.5) * width / columns as f64 + rng.gen_range(-0.3..0.3))
                    .clamp(2.8, width - 2.8);
                let y = ((row as f64 + 0.5) * height / rows as f64 + rng.gen_range(-0.3..0.3))
                    .clamp(1.0, height - 1.0);
                positions.push((x, y));
            }
        }
    }
    assert_eq!(positions.len(), ENVIRONMENT_OBJECT_COUNT);

    for (object_index, (x, y)) in positions.into_iter().enumerate() {
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
        assert!(
            field.deposit_physical(x, y, physical, catalog),
            "{:?} dispersion object {object_index} must remain in bounds",
            dispersion
        );
    }
    assert_eq!(pile_sizes.iter().sum::<usize>(), ENVIRONMENT_OBJECT_COUNT);
    pile_sizes
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DispersionMode {
    Randomized,
    Uniform,
    Clustered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartingMixture {
    Baseline,
    HighBonded,
    MixedStability,
}

impl StartingMixture {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::HighBonded => "high-bonded",
            Self::MixedStability => "mixed-stability",
        }
    }

    fn molecule_parts(self) -> Vec<Vec<String>> {
        let mut molecules = Vec::with_capacity(ENVIRONMENT_MOLECULE_COUNT);
        let mut add = |count: usize, a: &str, b: &str| {
            molecules.extend((0..count).map(|_| vec![a.to_string(), b.to_string()]));
        };
        match self {
            Self::Baseline => {
                add(40, "Hydrogen", "Sulfur");
                add(40, "Carbon", "Hydrogen");
            }
            Self::HighBonded => {
                add(27, "Carbon", "Sulfur");
                add(27, "Phosphorus", "Sulfur");
                add(26, "Nitrogen", "Sulfur");
            }
            Self::MixedStability => {
                add(20, "Carbon", "Sulfur");
                add(20, "Phosphorus", "Sulfur");
                add(20, "Nitrogen", "Sulfur");
                add(20, "Carbon", "Oxygen");
            }
        }
        assert_eq!(molecules.len(), ENVIRONMENT_MOLECULE_COUNT);
        molecules
    }
}

impl DispersionMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Randomized => "randomized",
            Self::Uniform => "uniform",
            Self::Clustered => "clustered",
        }
    }
}

pub(crate) struct EnvironmentalChemistryFixture {
    pub(crate) simulation: Simulation,
    pub(crate) pile_sizes: Vec<usize>,
}

pub(crate) fn build(seed: u64) -> EnvironmentalChemistryFixture {
    build_with_dispersion(seed, DispersionMode::Randomized)
}

pub(crate) fn build_with_dispersion(
    seed: u64,
    dispersion: DispersionMode,
) -> EnvironmentalChemistryFixture {
    build_with_dispersion_and_mixture(seed, dispersion, StartingMixture::Baseline)
}

pub(crate) fn build_with_dispersion_and_mixture(
    seed: u64,
    dispersion: DispersionMode,
    starting_mixture: StartingMixture,
) -> EnvironmentalChemistryFixture {
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
    let catalog = simulation.environment.catalog.clone();

    let pile_sizes = seed_randomized_molecular_resources(
        &mut simulation.environment.field,
        &catalog,
        seed,
        simulation.environment.width,
        simulation.environment.height,
        true,
        dispersion,
        starting_mixture,
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
    fn dispersion_layouts_are_deterministic_and_preserve_the_complete_inventory() {
        for dispersion in [
            DispersionMode::Randomized,
            DispersionMode::Uniform,
            DispersionMode::Clustered,
        ] {
            let first = build_with_dispersion(42, dispersion);
            let second = build_with_dispersion(42, dispersion);
            assert_eq!(first.pile_sizes, second.pile_sizes);
            assert_eq!(
                first.simulation.environment.field.cells, second.simulation.environment.field.cells,
                "{dispersion:?} placement must be reproducible"
            );
            assert_eq!(
                first.pile_sizes.iter().sum::<usize>(),
                ENVIRONMENT_OBJECT_COUNT
            );
            assert!(first.simulation.organisms.is_empty());
            let materials = first
                .simulation
                .environment
                .field
                .cells
                .iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .collect::<Vec<_>>();
            assert_eq!(materials.len(), ENVIRONMENT_OBJECT_COUNT);
            assert_eq!(
                materials
                    .iter()
                    .filter(|material| material.material.parts.len() == 1)
                    .count(),
                ENVIRONMENT_SINGLE_RESOURCE_COUNT
            );
            assert_eq!(
                materials
                    .iter()
                    .filter(|material| material.material.parts.len() == 2)
                    .count(),
                ENVIRONMENT_MOLECULE_COUNT
            );
            let represented = materials
                .iter()
                .flat_map(|material| material.material.parts.iter().map(|part| part.0.as_str()))
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(represented.len(), 7);
            assert_eq!(
                first.simulation.energy_inventory_snapshot().residual(),
                Some(0.0)
            );

            match dispersion {
                DispersionMode::Randomized => {
                    assert!((3..=10).contains(&first.pile_sizes.len()));
                }
                DispersionMode::Uniform => {
                    assert_eq!(first.pile_sizes.len(), ENVIRONMENT_OBJECT_COUNT);
                    assert!(first.pile_sizes.iter().all(|size| *size == 1));
                    let xs = materials.iter().map(|material| {
                        material
                            .placements
                            .as_ref()
                            .and_then(|placements| placements.first())
                            .expect("uniformly placed material must retain its placement")
                            .x
                    });
                    let min_x = xs.clone().fold(f64::INFINITY, f64::min);
                    let max_x = xs.fold(f64::NEG_INFINITY, f64::max);
                    assert!(min_x < 5.0 && max_x > 245.0);
                }
                DispersionMode::Clustered => {
                    assert_eq!(first.pile_sizes.len(), 32);
                    assert!(first.pile_sizes.iter().all(|size| *size == 40));
                }
            }
        }
    }

    #[test]
    fn starting_mixtures_preserve_the_same_total_resource_inventory() {
        let mixtures = [
            StartingMixture::Baseline,
            StartingMixture::HighBonded,
            StartingMixture::MixedStability,
        ];
        for seed in [42, 43, 44, 45, 46] {
            let mut expected = None;
            for mixture in mixtures {
                let fixture =
                    build_with_dispersion_and_mixture(seed, DispersionMode::Randomized, mixture);
                let materials = fixture
                    .simulation
                    .environment
                    .field
                    .cells
                    .iter()
                    .flat_map(|cell| cell.physical_materials.iter());
                let mut inventory = std::collections::BTreeMap::<String, usize>::new();
                let mut molecules = std::collections::BTreeMap::<String, usize>::new();
                let mut object_count = 0;
                let mut independent_count = 0;
                for material in materials {
                    object_count += 1;
                    if material.material.parts.len() == 1 {
                        independent_count += 1;
                    } else {
                        let mut names = material
                            .material
                            .parts
                            .iter()
                            .map(|part| part.0.clone())
                            .collect::<Vec<_>>();
                        names.sort();
                        *molecules.entry(names.join("-")).or_default() += 1;
                    }
                    for (name, _) in &material.material.parts {
                        *inventory.entry(name.clone()).or_default() += 1;
                    }
                }
                assert_eq!(object_count, ENVIRONMENT_OBJECT_COUNT);
                assert_eq!(independent_count, ENVIRONMENT_SINGLE_RESOURCE_COUNT);
                assert_eq!(
                    molecules.values().sum::<usize>(),
                    ENVIRONMENT_MOLECULE_COUNT
                );
                if let Some(expected_inventory) = &expected {
                    assert_eq!(
                        &inventory, expected_inventory,
                        "seed {seed}, mixture {mixture:?}"
                    );
                } else {
                    expected = Some(inventory);
                }
                let expected_molecules = match mixture {
                    StartingMixture::Baseline => [("Carbon-Hydrogen", 40_usize), ("Hydrogen-Sulfur", 40_usize)]
                        .into_iter()
                    .map(|(name, count)| (name.to_string(), count))
                    .collect::<std::collections::BTreeMap<_, _>>(),
                    StartingMixture::HighBonded => [
                        ("Carbon-Sulfur", 27_usize),
                        ("Nitrogen-Sulfur", 26_usize),
                        ("Phosphorus-Sulfur", 27_usize),
                    ]
                    .into_iter()
                    .map(|(name, count)| (name.to_string(), count))
                    .collect::<std::collections::BTreeMap<_, _>>(),
                    StartingMixture::MixedStability => [
                        ("Carbon-Oxygen", 20_usize),
                        ("Carbon-Sulfur", 20_usize),
                        ("Nitrogen-Sulfur", 20_usize),
                        ("Phosphorus-Sulfur", 20_usize),
                    ]
                    .into_iter()
                    .map(|(name, count)| (name.to_string(), count))
                    .collect::<std::collections::BTreeMap<_, _>>(),
                };
                assert_eq!(molecules, expected_molecules);
                assert_eq!(
                    fixture.simulation.energy_inventory_snapshot().residual(),
                    Some(0.0)
                );
            }
        }
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
            let strength = crate::combine::bond_strength(properties_a, properties_b);
            let expected_energy =
                crate::combine::intrinsic_bond_potential(properties_a, properties_b, strength);
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
