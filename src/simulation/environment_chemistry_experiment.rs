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
const ENVIRONMENT_MOLECULE_COUNT: usize = 80;
const ENVIRONMENT_INDEPENDENT_RESOURCE_COUNT: usize =
    ENVIRONMENT_OBJECT_COUNT - ENVIRONMENT_MOLECULE_COUNT;

// Two-constituent structures previously observed as reaction-created products.
// Every type receives an equal starting share; the final remainder is assigned
// deterministically after shuffling so no molecule type is privileged.
const KNOWN_TWO_UNIT_MOLECULES: [(&str, &str); 18] = [
    ("Hydrogen", "Sulfur"),
    ("Hydrogen", "Oxygen"),
    ("Carbon", "Hydrogen"),
    ("Oxygen", "Phosphorus"),
    ("Carbon", "Oxygen"),
    ("Phosphorus", "Sulfur"),
    ("Nitrogen", "Oxygen"),
    ("Sulfur", "Sulfur"),
    ("Carbon", "Sulfur"),
    ("Hydrogen", "Nitrogen"),
    ("Hydrogen", "Phosphorus"),
    ("Nitrogen", "Phosphorus"),
    ("Carbon", "Phosphorus"),
    ("Carbon", "Carbon"),
    ("Oxygen", "Sulfur"),
    ("Carbon", "Nitrogen"),
    ("Nitrogen", "Sulfur"),
    ("Phosphorus", "Phosphorus"),
];

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
    let restored_indices = crate::physical_material::restore_material(
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
        6,
        "the default environment contains exactly six active elements"
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

fn place_molecule_prototype(
    mut molecule: PhysicalMaterial,
    placement: Placement,
) -> PhysicalMaterial {
    let (sin, cos) = placement.rotation_radians.sin_cos();
    for part in molecule
        .placements
        .as_mut()
        .expect("realized molecule prototypes retain constituent placements")
    {
        let local_x = part.x;
        let local_y = part.y;
        part.x = placement.x + local_x * cos - local_y * sin;
        part.y = placement.y + local_x * sin + local_y * cos;
        part.rotation_radians += placement.rotation_radians;
    }
    molecule
}

/// Seed 1,200 independent resources and 80 intact, known two-constituent molecules in randomized piles.
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
        "active chemistry experiments use exactly the six active elements"
    );

    let molecule_prototypes: Vec<PhysicalMaterial> = if include_molecules {
        KNOWN_TWO_UNIT_MOLECULES
            .iter()
            .map(|(left, right)| {
                realize_natural_molecule(
                    &[(*left).into(), (*right).into()],
                    Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                    catalog,
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    let objects: Vec<(Vec<String>, bool)> = if include_molecules {
        let mut objects = Vec::with_capacity(ENVIRONMENT_OBJECT_COUNT);
        let mut identities = resource_names.clone();
        while identities.len() < ENVIRONMENT_INDEPENDENT_RESOURCE_COUNT {
            identities.push(resource_names[rng.gen_range(0..resource_names.len())].clone());
        }
        identities.shuffle(&mut rng);
        objects.extend(identities.into_iter().map(|name| (vec![name], false)));

        let mut molecules = Vec::with_capacity(ENVIRONMENT_MOLECULE_COUNT);
        let base_share = ENVIRONMENT_MOLECULE_COUNT / KNOWN_TWO_UNIT_MOLECULES.len();
        let remainder = ENVIRONMENT_MOLECULE_COUNT % KNOWN_TWO_UNIT_MOLECULES.len();
        for (index, (left, right)) in KNOWN_TWO_UNIT_MOLECULES.iter().enumerate() {
            let count = base_share + usize::from(index < remainder);
            molecules.extend(std::iter::repeat_n(
                (vec![(*left).into(), (*right).into()], true),
                count,
            ));
        }
        molecules.shuffle(&mut rng);
        objects.extend(molecules);
        objects.shuffle(&mut rng);
        objects
    } else {
        let mut identities = resource_names.clone();
        while identities.len() < ENVIRONMENT_OBJECT_COUNT {
            identities.push(resource_names[rng.gen_range(0..resource_names.len())].clone());
        }
        identities.shuffle(&mut rng);
        identities
            .into_iter()
            .map(|name| (vec![name], false))
            .collect()
    };

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
                let prototype_index = KNOWN_TWO_UNIT_MOLECULES
                    .iter()
                    .position(|(left, right)| parts[0] == *left && parts[1] == *right)
                    .expect("every seeded molecule must belong to the known catalog");
                place_molecule_prototype(molecule_prototypes[prototype_index].clone(), placement)
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
/// Generic setup for a paired catalytic-facilitation comparison. The part
/// lists describe exact molecule compositions, not privileged reaction rules.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MatchedCatalysisConfig {
    pub(crate) candidate_parts: Vec<String>,
    pub(crate) target_reactant_a_parts: Vec<String>,
    pub(crate) target_reactant_b_parts: Vec<String>,
    #[serde(default = "default_candidate_near_distance")]
    pub(crate) near_distance: f64,
    #[serde(default = "default_max_target_pair_distance")]
    pub(crate) max_target_pair_distance: f64,
    #[serde(default = "default_minimum_remote_distance")]
    pub(crate) minimum_remote_distance: f64,
}

fn default_candidate_near_distance() -> f64 { 8.0 }
fn default_max_target_pair_distance() -> f64 { 30.0 }
fn default_minimum_remote_distance() -> f64 { 60.0 }

impl MatchedCatalysisConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        for (label, parts) in [
            ("candidate_parts", &self.candidate_parts),
            ("target_reactant_a_parts", &self.target_reactant_a_parts),
            ("target_reactant_b_parts", &self.target_reactant_b_parts),
        ] {
            if parts.len() != 2 || parts.iter().any(|part| part.trim().is_empty()) {
                return Err(format!("{label} must contain exactly two non-empty element names"));
            }
        }
        if self.near_distance <= 0.0
            || self.max_target_pair_distance <= 0.0
            || self.minimum_remote_distance <= self.near_distance
            || !self.near_distance.is_finite()
            || !self.max_target_pair_distance.is_finite()
            || !self.minimum_remote_distance.is_finite()
        {
            return Err("control distances must be finite and positive, with remote distance greater than near distance".to_string());
        }
        if canonical_part_names(&self.candidate_parts)
            == canonical_part_names(&self.target_reactant_a_parts)
            || canonical_part_names(&self.candidate_parts)
                == canonical_part_names(&self.target_reactant_b_parts)
        {
            return Err("candidate molecule must differ from each target reactant composition".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MatchedCatalysisCondition {
    CandidateNear,
    CandidateRemote,
}

impl MatchedCatalysisCondition {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "candidate_near" => Ok(Self::CandidateNear),
            "candidate_remote" => Ok(Self::CandidateRemote),
            _ => Err("matched-catalysis condition must be candidate_near or candidate_remote".to_string()),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::CandidateNear => "candidate_near",
            Self::CandidateRemote => "candidate_remote",
        }
    }
}

#[derive(Clone)]
struct MatchedMoleculeLocation {
    identity: u64,
    parts: Vec<String>,
    center: (f64, f64),
}

fn canonical_part_names(parts: &[String]) -> Vec<String> {
    let mut names = parts.to_vec();
    names.sort();
    names
}

fn material_part_names(material: &PhysicalMaterial) -> Option<Vec<String>> {
    if material.material.parts.iter().any(|(_, amount)| {
        !amount.is_finite() || (*amount - 1.0).abs() > 1e-9
    }) {
        return None;
    }
    Some(canonical_part_names(
        &material.material.parts.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>(),
    ))
}

fn physical_material_center(material: &PhysicalMaterial) -> Option<(f64, f64)> {
    let placements = material.placements.as_ref()?;
    if placements.is_empty()
        || placements.iter().any(|placement| !placement.x.is_finite() || !placement.y.is_finite())
    {
        return None;
    }
    let count = placements.len() as f64;
    Some((
        placements.iter().map(|placement| placement.x).sum::<f64>() / count,
        placements.iter().map(|placement| placement.y).sum::<f64>() / count,
    ))
}

fn matched_molecule_locations(simulation: &Simulation) -> Vec<MatchedMoleculeLocation> {
    simulation.environment.field.cells.iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .filter_map(|material| {
            Some(MatchedMoleculeLocation {
                identity: material.environmental_identity,
                parts: material_part_names(material)?,
                center: physical_material_center(material)?,
            })
        })
        .filter(|molecule| molecule.identity != 0 && molecule.parts.len() >= 2)
        .collect()
}

fn point_distance(left: (f64, f64), right: (f64, f64)) -> f64 {
    (left.0 - right.0).hypot(left.1 - right.1)
}

fn canonical_inventory(simulation: &Simulation) -> Vec<(String, f64)> {
    let mut inventory = simulation.environment.field.total_material();
    inventory.sort_by(|left, right| left.0.cmp(&right.0));
    inventory
}

fn non_candidate_layout_signature(simulation: &Simulation, candidate_id: u64) -> u64 {
    let mut rows = simulation.environment.field.cells.iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .filter(|material| material.environmental_identity != candidate_id)
        .map(|material| {
            let placements = material.placements.as_deref().unwrap_or_default();
            format!(
                "{}|{:?}|{:?}",
                material.environmental_identity,
                material.material.parts,
                placements.iter().map(|p| (p.x.to_bits(), p.y.to_bits(), p.rotation_radians.to_bits())).collect::<Vec<_>>()
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows.join("\n").bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn relocate_candidate_near_target(
    simulation: &mut Simulation,
    candidate_id: u64,
    candidate_center: (f64, f64),
    target_center: (f64, f64),
    near_distance: f64,
) -> Result<(f64, f64), String> {
    // Try deterministic angular positions at the requested separation. Each
    // movement uses the environment's atomic geometry/collision validator.
    for ring_scale in [1.0, 0.85, 1.15] {
        let radius = near_distance * ring_scale;
        if radius > near_distance + 1e-9 { continue; }
        for step in 0..64 {
            let angle = std::f64::consts::TAU * step as f64 / 64.0;
            let destination = (
                target_center.0 + radius * angle.cos(),
                target_center.1 + radius * angle.sin(),
            );
            let dx = destination.0 - candidate_center.0;
            let dy = destination.1 - candidate_center.1;
            if simulation.environment.field.move_physical_material_by_identity(
                candidate_id, dx, dy, &simulation.environment.catalog
            ) {
                return Ok(destination);
            }
        }
    }
    Err("could not place candidate molecule near the target pair without overlap or boundary violation".to_string())
}

fn relocate_candidate_remote(
    simulation: &mut Simulation,
    candidate_id: u64,
    candidate_center: (f64, f64),
    target_center: (f64, f64),
    minimum_remote_distance: f64,
) -> Result<(f64, f64), String> {
    let field = &simulation.environment.field;
    let mut destinations = (0..field.cells.len())
        .map(|index| field.cell_center(index))
        .filter(|point| point_distance(*point, target_center) >= minimum_remote_distance)
        .collect::<Vec<_>>();
    destinations.sort_by(|left, right| {
        point_distance(*right, target_center)
            .partial_cmp(&point_distance(*left, target_center))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| left.1.partial_cmp(&right.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    for destination in destinations {
        if simulation.environment.field.move_physical_material_by_identity(
            candidate_id,
            destination.0 - candidate_center.0,
            destination.1 - candidate_center.1,
            &simulation.environment.catalog,
        ) {
            return Ok(destination);
        }
    }
    Err("could not place candidate molecule at the configured remote distance without overlap or boundary violation".to_string())
}

pub(crate) struct EnvironmentalChemistryFixture {
    pub(crate) simulation: Simulation,
    pub(crate) pile_sizes: Vec<usize>,
    pub(crate) experiment_metadata: Option<serde_json::Value>,
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
    // The authoritative catalog already contains only the six active elements.
    let catalog = simulation.environment.catalog.clone();

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
        "active environmental chemistry experiments must use only the six elements"
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
        experiment_metadata: None,
    }
}

/// Build one arm of a paired facilitator-placement comparison. Both arms use
/// the exact same seed and initial inventory; only the configured candidate
/// molecule is translated after the deterministic tick-zero fixture is built.
pub(crate) fn build_matched(
    seed: u64,
    config: &MatchedCatalysisConfig,
    condition: MatchedCatalysisCondition,
) -> Result<EnvironmentalChemistryFixture, String> {
    config.validate()?;
    let mut fixture = build(seed);
    let molecules = matched_molecule_locations(&fixture.simulation);
    let candidate_parts = canonical_part_names(&config.candidate_parts);
    let target_a_parts = canonical_part_names(&config.target_reactant_a_parts);
    let target_b_parts = canonical_part_names(&config.target_reactant_b_parts);

    let target_a = molecules.iter()
        .filter(|molecule| molecule.parts == target_a_parts)
        .collect::<Vec<_>>();
    let target_b = molecules.iter()
        .filter(|molecule| molecule.parts == target_b_parts)
        .collect::<Vec<_>>();
    let mut target_pair: Option<(&MatchedMoleculeLocation, &MatchedMoleculeLocation, f64)> = None;
    for left in &target_a {
        for right in &target_b {
            if left.identity == right.identity { continue; }
            let distance = point_distance(left.center, right.center);
            if distance > config.max_target_pair_distance { continue; }
            let replace = target_pair.as_ref().is_none_or(|(_, _, current)| distance < *current);
            if replace {
                target_pair = Some((left, right, distance));
            }
        }
    }
    let (target_a, target_b, target_separation) = target_pair.ok_or_else(|| {
        format!(
            "seed {seed} has no distinct target-reactant pair within {:.3} world units",
            config.max_target_pair_distance
        )
    })?;
    let target_center = (
        0.5 * (target_a.center.0 + target_b.center.0),
        0.5 * (target_a.center.1 + target_b.center.1),
    );
    let candidate = molecules.iter()
        .filter(|molecule| {
            molecule.parts == candidate_parts
                && molecule.identity != target_a.identity
                && molecule.identity != target_b.identity
        })
        .min_by(|left, right| {
            point_distance(left.center, target_center)
                .partial_cmp(&point_distance(right.center, target_center))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.identity.cmp(&right.identity))
        })
        .ok_or_else(|| format!("seed {seed} has no molecule matching candidate_parts"))?;
    let candidate_id = candidate.identity;
    let candidate_before = candidate.center;

    let initial_inventory = canonical_inventory(&fixture.simulation);
    let initial_energy = fixture.simulation.energy_inventory_snapshot();
    let non_candidate_layout_before =
        non_candidate_layout_signature(&fixture.simulation, candidate_id);

    let candidate_after = match condition {
        MatchedCatalysisCondition::CandidateNear => relocate_candidate_near_target(
            &mut fixture.simulation,
            candidate_id,
            candidate_before,
            target_center,
            config.near_distance,
        )?,
        MatchedCatalysisCondition::CandidateRemote => relocate_candidate_remote(
            &mut fixture.simulation,
            candidate_id,
            candidate_before,
            target_center,
            config.minimum_remote_distance,
        )?,
    };
    let actual_distance = point_distance(candidate_after, target_center);
    match condition {
        MatchedCatalysisCondition::CandidateNear if actual_distance > config.near_distance + 1e-6 => {
            return Err("near candidate placement exceeded the configured distance".to_string());
        }
        MatchedCatalysisCondition::CandidateRemote if actual_distance + 1e-6 < config.minimum_remote_distance => {
            return Err("remote candidate placement fell inside the configured minimum distance".to_string());
        }
        _ => {}
    }

    let final_inventory = canonical_inventory(&fixture.simulation);
    let final_energy = fixture.simulation.energy_inventory_snapshot();
    if initial_inventory != final_inventory {
        return Err("matched control relocation changed elemental inventory".to_string());
    }
    if initial_energy.current_usable_energy != final_energy.current_usable_energy
        || initial_energy.current_resonant_energy != final_energy.current_resonant_energy
        || initial_energy.current_structural_energy != final_energy.current_structural_energy
    {
        return Err("matched control relocation changed the initial energy inventory".to_string());
    }
    if non_candidate_layout_before
        != non_candidate_layout_signature(&fixture.simulation, candidate_id)
    {
        return Err("matched control relocation changed a non-candidate molecule's geometry".to_string());
    }

    fixture.experiment_metadata = Some(serde_json::json!({
        "schema_version": 1,
        "seed": seed,
        "condition": condition.label(),
        "candidate_identity": candidate_id,
        "candidate_parts": candidate_parts,
        "candidate_position_before": {"x": candidate_before.0, "y": candidate_before.1},
        "candidate_position_after": {"x": candidate_after.0, "y": candidate_after.1},
        "candidate_distance_to_target_midpoint": actual_distance,
        "target_reactant_a_identity": target_a.identity,
        "target_reactant_a_parts": target_a_parts,
        "target_reactant_a_position": {"x": target_a.center.0, "y": target_a.center.1},
        "target_reactant_b_identity": target_b.identity,
        "target_reactant_b_parts": target_b_parts,
        "target_reactant_b_position": {"x": target_b.center.0, "y": target_b.center.1},
        "target_pair_separation": target_separation,
        "target_pair_midpoint": {"x": target_center.0, "y": target_center.1},
        "near_distance_limit": config.near_distance,
        "minimum_remote_distance": config.minimum_remote_distance,
        "max_target_pair_distance": config.max_target_pair_distance,
        "initial_elemental_inventory_unchanged": true,
        "initial_energy_inventory_unchanged": true,
        "all_non_candidate_molecule_geometry_unchanged": true,
        "interpretation": "The candidate is not established as a facilitator. Target reactants are selected by composition and proximity only; this setup does not establish an eligible endpoint contact or causal facilitation."
    }));
    Ok(fixture)
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
    fn matched_fixture_conditions_move_only_candidate_and_preserve_inventory() {
        let config = MatchedCatalysisConfig {
            candidate_parts: vec!["Hydrogen".into(), "Sulfur".into()],
            target_reactant_a_parts: vec!["Oxygen".into(), "Phosphorus".into()],
            target_reactant_b_parts: vec!["Carbon".into(), "Hydrogen".into()],
            near_distance: 20.0,
            max_target_pair_distance: 50.0,
            minimum_remote_distance: 80.0,
        };
        let near = build_matched(
            42,
            &config,
            MatchedCatalysisCondition::CandidateNear,
        )
        .expect("the standard seed should support the near control placement");
        let remote = build_matched(
            42,
            &config,
            MatchedCatalysisCondition::CandidateRemote,
        )
        .expect("the standard seed should support the remote control placement");

        assert_eq!(near.pile_sizes, remote.pile_sizes);
        assert_eq!(
            canonical_inventory(&near.simulation),
            canonical_inventory(&remote.simulation)
        );
        let near_metadata = near.experiment_metadata.expect("near metadata");
        let remote_metadata = remote.experiment_metadata.expect("remote metadata");
        assert_eq!(
            near_metadata["candidate_identity"],
            remote_metadata["candidate_identity"]
        );
        assert_eq!(
            near_metadata["target_reactant_a_identity"],
            remote_metadata["target_reactant_a_identity"]
        );
        assert_eq!(
            near_metadata["target_reactant_b_identity"],
            remote_metadata["target_reactant_b_identity"]
        );
        assert!(
            near_metadata["candidate_distance_to_target_midpoint"]
                .as_f64()
                .expect("near distance")
                <= config.near_distance
        );
        assert!(
            remote_metadata["candidate_distance_to_target_midpoint"]
                .as_f64()
                .expect("remote distance")
                >= config.minimum_remote_distance
        );
        assert_eq!(
            near_metadata["all_non_candidate_molecule_geometry_unchanged"],
            true
        );
        assert_eq!(
            remote_metadata["all_non_candidate_molecule_geometry_unchanged"],
            true
        );
    }

    #[test]
    fn matched_config_rejects_candidate_that_is_a_target_reactant() {
        let config = MatchedCatalysisConfig {
            candidate_parts: vec!["Oxygen".into(), "Phosphorus".into()],
            target_reactant_a_parts: vec!["Phosphorus".into(), "Oxygen".into()],
            target_reactant_b_parts: vec!["Carbon".into(), "Hydrogen".into()],
            near_distance: 8.0,
            max_target_pair_distance: 30.0,
            minimum_remote_distance: 60.0,
        };
        assert!(config.validate().is_err());
    }

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
    fn fixture_has_1200_independent_resources_and_80_known_molecules() {
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
            let indices = crate::physical_material::restore_material(
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
            ENVIRONMENT_INDEPENDENT_RESOURCE_COUNT
        );
        assert_eq!(
            materials
                .iter()
                .filter(|m| m.material.parts.len() == 2)
                .count(),
            ENVIRONMENT_MOLECULE_COUNT
        );
        let mut molecule_type_counts = std::collections::BTreeMap::new();
        for material in materials
            .iter()
            .filter(|material| material.material.parts.len() == 2)
        {
            let mut elements = material
                .material
                .parts
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>();
            elements.sort_unstable();
            *molecule_type_counts.entry(elements).or_insert(0_usize) += 1;
        }
        assert_eq!(molecule_type_counts.len(), KNOWN_TWO_UNIT_MOLECULES.len());
        assert_eq!(
            molecule_type_counts
                .values()
                .filter(|count| **count == 5)
                .count(),
            8
        );
        assert_eq!(
            molecule_type_counts
                .values()
                .filter(|count| **count == 4)
                .count(),
            10
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
        assert_eq!(hs, 5);
        assert_eq!(ch, 5);
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
        let expected_constituents =
            ENVIRONMENT_INDEPENDENT_RESOURCE_COUNT + 2 * ENVIRONMENT_MOLECULE_COUNT;
        assert!((total_material - expected_constituents as f64).abs() < 1e-6);
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
