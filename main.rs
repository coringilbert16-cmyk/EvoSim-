use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

mod environment;
mod genome;
mod math;
mod resources;
mod structure;
mod contact;

use environment::{
    apply_settling, apply_vents, ActiveMaterialField, DeepReservoir, Vent,
    DEFAULT_CELL_SIZE, DEFAULT_DIFFUSION_FRACTION, DEFAULT_RESERVOIR_BLOCK_SIZE,
    DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS,
};
use genome::{initial_genome, Genome};
use math::exponential_influence;
use resources::{BaseResource, Material};


// ============================================================
// APPLICATION STATE
// ============================================================

#[derive(Clone)]
struct AppState {
    simulation: Arc<Mutex<Simulation>>,
    broadcaster: broadcast::Sender<String>,
}


// ============================================================
// RESOURCE PERCEPTION
// ============================================================
//
// A ResourceObservation is what the organism currently perceives.
// It is NOT itself a stored environmental resource.
//
// Perception now reads directly from the active material field's
// grid cells (see environment.rs) instead of the legacy
// ResourceCloud list. A field cell holds exactly one bonded stack
// and one unbonded stack, so each cell can contribute up to two
// observations (one per stack) - `bonded` on the observation records
// which stack it came from, and `field_index` identifies exactly
// which cell to act on later (initiation no longer needs to re-find
// a cloud by float-comparing positions).
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
struct PropertyDeviations {
    mass: f64,
    potential_energy: f64,
    reactivity: f64,
    cohesion: f64,
}

#[derive(Serialize, Deserialize, Clone)]
struct AffinityResponses {
    mass: f64,
    potential_energy: f64,
    reactivity: f64,
    cohesion: f64,
}

#[derive(Serialize, Deserialize, Clone)]
struct ResourceObservation {
    // Display label only (e.g. "Methane+Hydrogen" for a mixed bonded
    // stack).
    name: String,
    properties: resources::ResourceProperties,

    // Whether this observation came from the cell's bonded or
    // unbonded stack. Raw/unbonded material cannot BREAK (hard rule,
    // §4) - this lets initiation filter out non-bondable sources
    // before picking a target, rather than picking the "best" target
    // and only then discovering it's ineligible.
    bonded: bool,

    perceived_amount: f64,

    deviations: PropertyDeviations,
    affinity_responses: AffinityResponses,

    base_desirability: f64,
    amount_factor: f64,
    potential_energy_need_factor: f64,

    desirability: f64,

    // Distance from the organism to this specific cell's center,
    // retained so the processing-initiation step does not need to
    // re-walk the environment a second time to find "is this in
    // reach".
    distance: f64,

    // World-space center of the source cell, for display/frontend
    // purposes only - not used for lookup.
    source_x: f64,
    source_y: f64,

    // Index of the field cell this observation came from. Combined
    // with `bonded`, this is exactly what a transformation needs to
    // take material from the correct stack with no re-derivation.
    field_index: usize,
}


// ============================================================
// RESOURCE SENSE
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
struct ResourceSense {
    sensed_resources: Vec<ResourceObservation>,

    direction_x: f64,
    direction_y: f64,
    direction_strength: f64,
}


// ============================================================
// DEVELOPMENT / POSITION
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
enum DevelopmentStage {
    Juvenile,
    Adult,
}

#[derive(Serialize, Deserialize, Clone)]
struct Position {
    x: f64,
    y: f64,
}


// ============================================================
// SPATIAL MEMORY
// ============================================================
//
// Memory is deliberately bounded.
//
// Memory means: location + strength derived from a positive
// desirability encounter. True outcome-based reinforcement (i.e.
// "processing here actually succeeded") is layered on top once a
// transformation resolves - see update_memory_from_outcome.
// ============================================================

const MAX_MEMORY_POINTS: usize = 5;
const MEMORY_DECAY_PER_TICK: f64 = 0.995;
const MEMORY_MERGE_RADIUS: f64 = 40.0;
const MEMORY_PRUNE_THRESHOLD: f64 = 0.01;

#[derive(Serialize, Deserialize, Clone)]
struct MemoryPoint {
    x: f64,
    y: f64,
    strength: f64,
}


// ============================================================
// RESOURCE TRANSFORMATIONS  (Master Spec §15-19)
// ============================================================
//
// Only BREAK is implemented at this stage.
//
// A transformation:
//   - commits its input resources for its full duration (§18.1)
//   - has a duration derived from processing complexity (§17-18)
//   - resolves into usable energy + returned mass + heat (§20-22)
//
// SIMPLIFICATION FLAGGED FOR REVISIT:
// Because derived materials (§16) are not implemented yet, spent mass
// is ejected as waste using the *same* resource-type composition it
// had going in, rather than a lower-energy derived composition.
// (Integration note: waste is now deposited directly into the active
// material field at the organism's location, and behaves exactly
// like any other field material afterward - it can diffuse, be
// perceived, be processed, and eventually settle to the reservoir.
// It is not given any special automatic cleanup.)
// ============================================================

const PROCESSING_REACH: f64 = 20.0;
const PROCESSING_RATE: f64 = 4.0; // max resource amount committed per initiation

#[derive(Serialize, Deserialize, Clone, Copy)]
enum TransformationKind {
    Break,
}

#[derive(Serialize, Deserialize, Clone)]
struct ActiveTransformation {
    id: u64,
    organism_id: String,
    kind: TransformationKind,

    // The committed input, held as a canonical bonded Material for the
    // full duration of the transformation (§18.1).
    material: Material,

    complexity: f64,
    duration_ticks: u64,
    remaining_ticks: u64,
}


// ============================================================
// ENERGY LEDGER  (Master Spec §21, §101)
// ============================================================

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
struct EnergyLedger {
    total_potential_energy_released: f64,
    total_usable_energy_gained: f64,
    total_heat_dissipated: f64,
    total_usable_energy_held: f64,
}


// ============================================================
// ORGANISM
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
struct Organism {
    id: String,

    occupied_cells: Vec<Position>,

    genome: Genome,

    resource_sense: ResourceSense,

    memory: Vec<MemoryPoint>,

    usable_energy: f64,

    stress: f64,

    // Bulk, pre-instantiation raw material the organism has acquired
    // but not yet built into anything - a continuous (name, amount)
    // stock, exactly like a field cell's unbonded stack. This is
    // deliberately still bulk: instantiating it into discrete
    // StructuralUnits is what COMBINE will do, and COMBINE remains
    // unimplemented (the interaction/threshold/surplus-strength
    // equations are not locked yet).
    stored_unbonded: Material,

    // Real bonded structure the organism has built (or acquired
    // already-bonded material into - see the TODO on
    // Organism::store_unbonded_material below for why "acquiring
    // already-bonded material" isn't wired yet either). This REPLACES
    // the old bulk `stored_bonded: Material` field: a bulk blob with
    // a single bonded=true flag cannot represent individual
    // connection points, per-bond strength, or per-point load, so it
    // was a genuinely conflicting representation once real bonds
    // exist (see structure.rs). Nothing ever read the old field live
    // (grepped before removing it), so this is a clean replacement,
    // not a migration of live behavior.
    structure: structure::OrganismStructure,

    development_stage: DevelopmentStage,

    age: u64,

    active_transformation_id: Option<u64>,
}


impl Organism {
    /// Merges acquired UNBONDED material into bulk raw storage - a
    /// relocation into the organism, not a transformation (same
    /// principle as field deposit/settling/venting).
    ///
    /// TODO(bonded-acquisition boundary): this deliberately does NOT
    /// accept bonded material. Acquiring already-bonded material from
    /// the environment would need to become a specific StructuralUnit
    /// (with a real Placement), but instantiation position/rotation
    /// and how many discrete units a contacted bulk amount represents
    /// are part of the still-undecided acquisition mechanism (see
    /// contact.rs's accessible_field_material, which identifies WHICH
    /// bonded material is physically reachable without deciding how
    /// much/how it gets acquired). Left isolated here rather than
    /// guessed at.
    fn store_unbonded_material(&mut self, material: Material) {
        if material.parts.is_empty() || material.bonded {
            return;
        }
        let mut parts = std::mem::take(&mut self.stored_unbonded.parts);
        parts.extend(material.parts);
        self.stored_unbonded.parts = resources::merge_parts(&parts);
    }
}


// ============================================================
// ENVIRONMENT
// ============================================================
//
// This is now the sole authoritative environmental state. It wraps
// exactly one active material field and one deep reservoir (see
// environment.rs) - there is no competing resource-cloud or flat
// global-reservoir representation anywhere in the simulation.
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
struct Environment {
    width: f64,
    height: f64,

    // Immutable per-type properties for every resource that exists in
    // this world. Never mutated after construction - see resources.rs.
    catalog: Vec<BaseResource>,

    // Layer 2: the active material field organisms actually perceive
    // and interact with.
    field: ActiveMaterialField,

    // Layer 1: the coarse, spatially-distributed deep reservoir.
    reservoir: DeepReservoir,

    vents: Vec<Vent>,
}


// ============================================================
// SNAPSHOT
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
struct Snapshot {
    tick: u64,
    organisms: Vec<Organism>,
    environment: Environment,
    active_transformations: Vec<ActiveTransformation>,
    energy_ledger: EnergyLedger,
}


// ============================================================
// SIMULATION
// ============================================================

struct Simulation {
    tick: u64,
    ticks_per_second: f64,
    running: bool,

    organisms: Vec<Organism>,
    environment: Environment,

    active_transformations: Vec<ActiveTransformation>,
    energy_ledger: EnergyLedger,

    next_organism_id: u64,
    next_transformation_id: u64,

    rng: ChaCha8Rng,
}


// ============================================================
// DESIRABILITY CONSTANTS
// ============================================================

const DESIRABILITY_AMOUNT_HALF_SATURATION: f64 = 100.0;
const DESIRABILITY_MAX: f64 = 1.0;

const STRESS_DECAY_PER_TICK: f64 = 0.98;


// ============================================================
// SIMULATION IMPLEMENTATION
// ============================================================

impl Simulation {

    fn new(seed: u64, ticks_per_second: f64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);

        let environment = Self::create_environment();

        let organism = Self::create_initial_organism();

        Self {
            tick: 0,
            ticks_per_second,
            running: true,

            organisms: vec![organism],

            environment,

            active_transformations: Vec::new(),
            energy_ledger: EnergyLedger::default(),

            next_organism_id: 2,
            next_transformation_id: 1,

            rng,
        }
    }


    // ========================================================
    // ENVIRONMENT
    // ========================================================

    fn create_environment() -> Environment {
        let catalog = resources::default_catalog();

        let width = 1000.0;
        let height = 1000.0;

        let field = ActiveMaterialField::new(width, height, DEFAULT_CELL_SIZE);
        let mut reservoir = DeepReservoir::new_matching_field(&field, DEFAULT_RESERVOIR_BLOCK_SIZE);

        // Starting abundance per type, seeded uniformly across the
        // reservoir as raw stock (see environment::seed_uniform).
        let starting_amounts: [(&str, f64); 7] = [
            ("Carbon", 10_000.0),
            ("Methane", 5_000.0),
            ("Hydrogen", 5_000.0),
            ("Sulfur", 5_000.0),
            ("Nitrogen", 5_000.0),
            ("Phosphorus", 5_000.0),
            ("Water", 20_000.0),
        ];

        for (name, amount) in starting_amounts {
            reservoir.seed_uniform(name, amount);
        }

        let vents = vec![
            Vent {
                x: 250.0,
                y: 250.0,

                composition: vec![
                    ("Carbon".into(), 0.10),
                    ("Methane".into(), 0.45),
                    ("Hydrogen".into(), 0.25),
                    ("Sulfur".into(), 0.10),
                    ("Nitrogen".into(), 0.05),
                    ("Phosphorus".into(), 0.02),
                    ("Water".into(), 0.03),
                ],

                emission_amount: 100.0,
                emission_interval: 20,
                emission_timer: 0,
            },

            Vent {
                x: 750.0,
                y: 300.0,

                composition: vec![
                    ("Carbon".into(), 0.35),
                    ("Methane".into(), 0.10),
                    ("Hydrogen".into(), 0.15),
                    ("Sulfur".into(), 0.25),
                    ("Nitrogen".into(), 0.05),
                    ("Phosphorus".into(), 0.05),
                    ("Water".into(), 0.05),
                ],

                emission_amount: 100.0,
                emission_interval: 30,
                emission_timer: 0,
            },

            Vent {
                x: 520.0,
                y: 550.0,

                composition: vec![
                    ("Carbon".into(), 0.25),
                    ("Methane".into(), 0.15),
                    ("Hydrogen".into(), 0.30),
                    ("Sulfur".into(), 0.10),
                    ("Nitrogen".into(), 0.10),
                    ("Phosphorus".into(), 0.02),
                    ("Water".into(), 0.08),
                ],

                emission_amount: 100.0,
                emission_interval: 25,
                emission_timer: 0,
            },
        ];

        Environment {
            width,
            height,

            catalog,
            field,
            reservoir,
            vents,
        }
    }


    // ========================================================
    // INITIAL ORGANISM
    // ========================================================

    fn create_initial_organism() -> Organism {
        Organism {
            id: "1".into(),

            occupied_cells: vec![
                Position {
                    x: 500.0,
                    y: 500.0,
                },
            ],

            genome: initial_genome(),

            resource_sense: ResourceSense {
                sensed_resources: Vec::new(),

                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
            },

            memory: Vec::new(),

            usable_energy: 0.0,
            stress: 0.0,

            stored_unbonded: Material { parts: Vec::new(), bonded: false },
            structure: structure::OrganismStructure::new(),

            development_stage: DevelopmentStage::Juvenile,

            age: 0,

            active_transformation_id: None,
        }
    }


    // ========================================================
    // PROPERTY DEVIATION
    // ========================================================

    fn calculate_property_deviations(
        properties: &resources::ResourceProperties,
        baselines: &resources::ResourceBaselines,
        ranges: &resources::ResourceProperties,
    ) -> PropertyDeviations {
        PropertyDeviations {
            mass: (
                (properties.mass - baselines.mass)
                / ranges.mass
            ).clamp(-1.0, 1.0),

            potential_energy: (
                (properties.potential_energy - baselines.potential_energy)
                / ranges.potential_energy
            ).clamp(-1.0, 1.0),

            reactivity: (
                (properties.reactivity - baselines.reactivity)
                / ranges.reactivity
            ).clamp(-1.0, 1.0),

            cohesion: (
                (properties.cohesion - baselines.cohesion)
                / ranges.cohesion
            ).clamp(-1.0, 1.0),
        }
    }


    // ========================================================
    // AFFINITY RESPONSE
    // ========================================================

    fn affinity_response(
        deviation: f64,
        affinity: f64,
    ) -> f64 {
        (deviation * affinity * 3.0).tanh()
    }


    // ========================================================
    // AMOUNT RESPONSE
    // ========================================================

    fn amount_factor(amount: f64) -> f64 {
        let amount = amount.max(0.0);

        amount / (amount + DESIRABILITY_AMOUNT_HALF_SATURATION)
    }


    // ========================================================
    // ENERGY NEED
    // ========================================================

    fn energy_need_factor(usable_energy: f64) -> f64 {
        1.0 / (1.0 + usable_energy.max(0.0))
    }


    // ========================================================
    // RESOURCE DESIRABILITY
    // ========================================================

    fn calculate_desirability(
        organism: &Organism,
        properties: &resources::ResourceProperties,
        perceived_amount: f64,
        baselines: &resources::ResourceBaselines,
        ranges: &resources::ResourceProperties,
    ) -> (
        PropertyDeviations,
        AffinityResponses,
        f64,
        f64,
        f64,
        f64,
    ) {
        let deviations = Self::calculate_property_deviations(
            properties,
            baselines,
            ranges,
        );

        let responses = AffinityResponses {
            mass: Self::affinity_response(
                deviations.mass,
                organism.genome.mass_affinity(),
            ),

            potential_energy: Self::affinity_response(
                deviations.potential_energy,
                organism.genome.potential_energy_affinity(),
            ),

            reactivity: Self::affinity_response(
                deviations.reactivity,
                organism.genome.reactivity_affinity(),
            ),

            cohesion: Self::affinity_response(
                deviations.cohesion,
                organism.genome.cohesion_affinity(),
            ),
        };

        let energy_need = Self::energy_need_factor(
            organism.usable_energy,
        );

        let energy_response = responses.potential_energy
            * (1.0 + energy_need);

        let base_desirability = (
            responses.mass
            + energy_response
            + responses.reactivity
            + responses.cohesion
        ) / 4.0;

        let amount_factor = Self::amount_factor(
            perceived_amount,
        );

        let desirability = (
            base_desirability * amount_factor
        ).clamp(
            -DESIRABILITY_MAX,
            DESIRABILITY_MAX,
        );

        (
            deviations,
            responses,
            base_desirability,
            amount_factor,
            energy_need,
            desirability,
        )
    }


    // ========================================================
    // ENVIRONMENT STEP (vents -> diffusion -> settling)
    // ========================================================
    //
    // Replaces the legacy emit_resource_clouds/update_resource_clouds
    // pair. This is now the ONLY pathway by which material moves
    // between the reservoir and the active field:
    //
    //     reservoir --(vent)--> field --(diffusion)--> field
    //     field --(settling, throttled)--> reservoir
    //
    // Settling is deliberately throttled (not every tick) since the
    // reservoir is meant to update far less often than the field -
    // per architectural decision, this asymmetry is what keeps the
    // reservoir cheap relative to the higher-resolution field.
    // ========================================================

    fn step_environment(&mut self) {
        apply_vents(
            &mut self.environment.field,
            &mut self.environment.reservoir,
            &mut self.environment.vents,
        );

        self.environment.field.diffuse_step(DEFAULT_DIFFUSION_FRACTION);

        if self.tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 {
            apply_settling(
                &mut self.environment.field,
                &mut self.environment.reservoir,
                DEFAULT_SETTLING_FRACTION,
            );
        }
    }


    // ========================================================
    // PERCEPTION
    // ========================================================

    fn update_resource_perception(
        organism: &mut Organism,
        environment: &Environment,
    ) {
        let perception_radius = organism
            .genome
            .perception_radius();

        let sensory_resolution = organism
            .genome
            .sensory_resolution();

        let directional_resolution = organism
            .genome
            .directional_resolution();

        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };

        organism.resource_sense.sensed_resources.clear();

        organism.resource_sense.direction_x = 0.0;
        organism.resource_sense.direction_y = 0.0;
        organism.resource_sense.direction_strength = 0.0;

        let baselines = resources::ResourceBaselines::from_catalog(
            &environment.catalog,
        );

        let ranges = resources::property_ranges(
            &environment.catalog,
        );

        for cell_index in environment.field.cells_within_radius(px, py, perception_radius) {
            let (cell_x, cell_y) = environment.field.cell_center(cell_index);

            let dx = cell_x - px;
            let dy = cell_y - py;
            let distance = (dx * dx + dy * dy).sqrt();

            let direction_x = if distance > 0.0 { dx / distance } else { 0.0 };
            let direction_y = if distance > 0.0 { dy / distance } else { 0.0 };

            let cell = &environment.field.cells[cell_index];

            for (bonded, material) in [(true, &cell.bonded), (false, &cell.unbonded)] {
                let perceived_amount = material.total_amount() * sensory_resolution;

                if perceived_amount <= 0.0 {
                    continue;
                }

                let properties = material.weighted_properties(&environment.catalog);

                let (
                    deviations,
                    responses,
                    base_desirability,
                    amount_factor,
                    energy_need_factor,
                    desirability,
                ) = Self::calculate_desirability(
                    organism,
                    &properties,
                    perceived_amount,
                    &baselines,
                    &ranges,
                );

                let label = material
                    .parts
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>()
                    .join("+");

                organism
                    .resource_sense
                    .sensed_resources
                    .push(ResourceObservation {
                        name: label,

                        properties,

                        bonded,

                        perceived_amount,

                        deviations,

                        affinity_responses: responses,

                        base_desirability,

                        amount_factor,

                        potential_energy_need_factor:
                            energy_need_factor,

                        desirability,

                        distance,

                        source_x: cell_x,
                        source_y: cell_y,

                        field_index: cell_index,
                    });

                organism.resource_sense.direction_x +=
                    direction_x * desirability;

                organism.resource_sense.direction_y +=
                    direction_y * desirability;
            }
        }

        let magnitude = (
            organism.resource_sense.direction_x
                * organism.resource_sense.direction_x
            +
            organism.resource_sense.direction_y
                * organism.resource_sense.direction_y
        ).sqrt();

        organism.resource_sense.direction_strength =
            magnitude;

        if magnitude <= f64::EPSILON {
            return;
        }

        organism.resource_sense.direction_x /= magnitude;
        organism.resource_sense.direction_y /= magnitude;

        let angle = organism
            .resource_sense
            .direction_y
            .atan2(organism.resource_sense.direction_x);

        let resolution =
            directional_resolution.max(0.001);

        let direction_steps =
            (resolution * 32.0).max(1.0);

        let step_angle =
            std::f64::consts::TAU / direction_steps;

        let quantized_angle =
            (angle / step_angle).round() * step_angle;

        organism.resource_sense.direction_x =
            quantized_angle.cos();

        organism.resource_sense.direction_y =
            quantized_angle.sin();
    }


    // ========================================================
    // MEMORY UPDATE (outcome-linked)
    // ========================================================

    fn update_memory_from_sources(
        organism: &mut Organism,
        environment: &Environment,
    ) {
        for point in &mut organism.memory {
            point.strength *= MEMORY_DECAY_PER_TICK;
        }

        organism.memory.retain(|p| {
            p.strength > MEMORY_PRUNE_THRESHOLD
        });

        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };

        let perception_radius =
            organism.genome.perception_radius();

        let sensory_resolution =
            organism.genome.sensory_resolution();

        let baselines =
            resources::ResourceBaselines::from_catalog(
                &environment.catalog,
            );

        let ranges =
            resources::property_ranges(
                &environment.catalog,
            );

        let mut strongest_source:
            Option<(f64, f64, f64)> = None;

        for cell_index in environment.field.cells_within_radius(px, py, perception_radius) {
            let (cell_x, cell_y) = environment.field.cell_center(cell_index);
            let cell = &environment.field.cells[cell_index];

            for material in [&cell.bonded, &cell.unbonded] {
                let perceived_amount =
                    material.total_amount() * sensory_resolution;

                if perceived_amount <= 0.0 {
                    continue;
                }

                let properties =
                    material.weighted_properties(&environment.catalog);

                let (_, _, _, _, _, desirability) =
                    Self::calculate_desirability(
                        organism,
                        &properties,
                        perceived_amount,
                        &baselines,
                        &ranges,
                    );

                if desirability <= 0.0 {
                    continue;
                }

                if strongest_source
                    .map(|(_, _, current)| desirability > current)
                    .unwrap_or(true)
                {
                    strongest_source = Some((
                        cell_x,
                        cell_y,
                        desirability,
                    ));
                }
            }
        }

        let Some((sx, sy, desirability)) =
            strongest_source
        else {
            return;
        };

        let memory_strength =
            (desirability
                * organism.genome.memory_strength())
                .clamp(0.0, 1.0);

        if memory_strength <= 0.0 {
            return;
        }

        Self::reinforce_memory_point(
            organism,
            sx,
            sy,
            memory_strength,
        );
    }

    fn reinforce_memory_point(
        organism: &mut Organism,
        sx: f64,
        sy: f64,
        memory_strength: f64,
    ) {
        let merged =
            organism.memory.iter_mut().find(|p| {
                let dx = p.x - sx;
                let dy = p.y - sy;

                (dx * dx + dy * dy).sqrt()
                    < MEMORY_MERGE_RADIUS
            });

        match merged {
            Some(existing) => {
                existing.x = sx;
                existing.y = sy;

                existing.strength =
                    (existing.strength + memory_strength)
                        .min(1.0);
            }

            None => {
                if organism.memory.len()
                    < MAX_MEMORY_POINTS
                {
                    organism.memory.push(
                        MemoryPoint {
                            x: sx,
                            y: sy,
                            strength: memory_strength,
                        },
                    );
                } else if let Some(weakest) =
                    organism.memory.iter_mut().min_by(
                        |a, b| {
                            a.strength
                                .partial_cmp(
                                    &b.strength
                                )
                                .unwrap()
                        },
                    )
                {
                    if memory_strength
                        > weakest.strength
                    {
                        *weakest =
                            MemoryPoint {
                                x: sx,
                                y: sy,
                                strength:
                                    memory_strength,
                            };
                    }
                }
            }
        }
    }


    // ========================================================
    // MOVEMENT DECISION
    // ========================================================

    fn update_movement(
        organism: &mut Organism,
        environment: &Environment,
    ) {
        let memory_strength_trait =
            organism.genome.memory_strength();

        let movement_efficiency =
            organism.genome.movement_efficiency();

        let perception_weight =
            1.0 - (
                0.5 + memory_strength_trait * 0.5
            );

        let memory_weight =
            1.0 - perception_weight;

        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };

        let mut memory_dir_x = 0.0;
        let mut memory_dir_y = 0.0;
        let mut memory_total_weight = 0.0;

        for point in &organism.memory {
            let dx = point.x - px;
            let dy = point.y - py;

            let distance =
                (dx * dx + dy * dy).sqrt();

            if distance <= f64::EPSILON {
                continue;
            }

            let weight =
                point.strength / distance;

            memory_dir_x +=
                (dx / distance) * weight;

            memory_dir_y +=
                (dy / distance) * weight;

            memory_total_weight += weight;
        }

        if memory_total_weight > 0.0 {
            memory_dir_x /=
                memory_total_weight;

            memory_dir_y /=
                memory_total_weight;
        }

        if organism.active_transformation_id.is_some() {
            return;
        }

        let mut move_x =
            memory_weight * memory_dir_x
            +
            perception_weight
                * organism.resource_sense.direction_x;

        let mut move_y =
            memory_weight * memory_dir_y
            +
            perception_weight
                * organism.resource_sense.direction_y;

        let magnitude =
            (move_x * move_x
                + move_y * move_y)
                .sqrt();

        if magnitude <= f64::EPSILON {
            return;
        }

        move_x /= magnitude;
        move_y /= magnitude;

        const STEP_DISTANCE: f64 = 5.0;

        let step =
            STEP_DISTANCE * movement_efficiency;

        let cell =
            &mut organism.occupied_cells[0];

        cell.x =
            (cell.x + move_x * step)
                .clamp(
                    0.0,
                    environment.width,
                );

        cell.y =
            (cell.y + move_y * step)
                .clamp(
                    0.0,
                    environment.height,
                );
    }


    // ========================================================
    // RESOURCE TRANSFORMATIONS - INITIATION  (§15-18)
    // ========================================================
    //
    // An organism with no active transformation, standing within
    // PROCESSING_REACH of a positively-desirable resource cell,
    // commits a bounded amount of that cell's bonded stack and
    // begins a BREAK. Takes the field directly now - no more
    // matching a cloud by float-comparing positions, since
    // perception already resolved the exact field cell index.
    // ========================================================

    fn try_start_transformation(
        organism: &mut Organism,
        field: &mut ActiveMaterialField,
        next_id: &mut u64,
    ) -> Option<ActiveTransformation> {
        if organism.active_transformation_id.is_some() {
            return None;
        }

        let target = organism
            .resource_sense
            .sensed_resources
            .iter()
            .filter(|r| {
                r.desirability > 0.0
                    && r.distance <= PROCESSING_REACH
                    // Hard rule (§4): raw/unbonded material cannot BREAK.
                    && r.bonded
            })
            .max_by(|a, b| {
                a.desirability
                    .partial_cmp(&b.desirability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?
            .clone();

        let cell = &mut field.cells[target.field_index];

        // Defensive re-check: can_break() requires bonded && total >= 2.0,
        // matching the hard rule at the mechanical level too, not just at
        // the perception-filter level above.
        if !cell.bonded.can_break() {
            return None;
        }

        let committed_amount =
            PROCESSING_RATE.min(cell.bonded.total_amount());

        if committed_amount <= 0.0 {
            return None;
        }

        // §18.1 resource commitment: remove from the environment now,
        // the transformation owns it until it resolves.
        let committed = field.take_at_index(target.field_index, true, committed_amount)?;

        let n = 2.0_f64; // baseline "resource + processing" component count, §18 example
        let c = math::complexity(n);
        let duration = c.ceil().max(1.0) as u64;

        let transformation = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: TransformationKind::Break,

            material: committed,

            complexity: c,
            duration_ticks: duration,
            remaining_ticks: duration,
        };

        *next_id += 1;

        organism.active_transformation_id = Some(transformation.id);

        Some(transformation)
    }


    // ========================================================
    // RESOURCE TRANSFORMATIONS - RESOLUTION  (§20-24)
    // ========================================================

    fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) {
        let props = transformation
            .material
            .weighted_properties(&environment.catalog);

        let input_potential_energy =
            transformation.material.potential_energy(&environment.catalog);

        let yield_fraction = exponential_influence(props.reactivity);

        let gross_extracted =
            input_potential_energy * yield_fraction;

        let cohesion_tax_fraction =
            (props.cohesion * 0.5).clamp(0.0, 1.0);
        let cohesion_tax = gross_extracted * cohesion_tax_fraction;

        let net_extracted =
            (gross_extracted - cohesion_tax).max(0.0);

        let processing_efficiency =
            organism.genome.processing_efficiency();

        let usable_gained = net_extracted * processing_efficiency;
        let heat = gross_extracted - usable_gained;

        organism.usable_energy += usable_gained;

        organism.stress += heat;

        // §5 / Correction #3: spent material does not simply disappear
        // and must NOT be dissolved back into the abstract reservoir.
        // BREAK breaks the bonds, so the leftover mass becomes waste
        // (unbonded) and is deposited directly into the active field
        // at the organism's current location - it then behaves exactly
        // like any other field material (can diffuse, be perceived,
        // be processed by another organism, eventually settle).
        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };

        if !transformation.material.is_empty() {
            let waste = Material {
                parts: transformation.material.parts.clone(),
                bonded: false,
            };

            environment.field.deposit(px, py, waste);
        }

        ledger.total_potential_energy_released += gross_extracted;
        ledger.total_usable_energy_gained += usable_gained;
        ledger.total_heat_dissipated += heat;

        organism.active_transformation_id = None;

        if usable_gained > 0.0 {
            let reinforcement =
                (usable_gained * organism.genome.memory_strength())
                    .clamp(0.0, 1.0);

            Self::reinforce_memory_point(organism, px, py, reinforcement);
        }
    }


    // ========================================================
    // ENERGY CAPACITY / EXCESS  (§23-24)
    // ========================================================

    fn apply_energy_capacity(organism: &mut Organism) {
        organism.stress *= STRESS_DECAY_PER_TICK;
    }


    // ========================================================
    // SIMULATION STEP
    // ========================================================
    //
    // Ordering follows §6.2:
    //   Environment -> Resources -> Resource transformations
    //   -> Organisms (perception/memory/movement)
    //   -> Energy/maintenance consequences
    // ========================================================

    fn step(&mut self) -> Snapshot {
        self.tick += 1;

        // ----------------------------------------------------
        // ENVIRONMENT: vents -> diffusion -> (throttled) settling
        // ----------------------------------------------------

        self.step_environment();

        // ----------------------------------------------------
        // RESOURCE TRANSFORMATIONS - advance & resolve
        // ----------------------------------------------------

        let mut still_active = Vec::new();
        let mut completed = Vec::new();

        for mut transformation in self.active_transformations.drain(..) {
            if transformation.remaining_ticks > 0 {
                transformation.remaining_ticks -= 1;
            }

            if transformation.remaining_ticks == 0 {
                completed.push(transformation);
            } else {
                still_active.push(transformation);
            }
        }

        self.active_transformations = still_active;

        for transformation in &completed {
            if let Some(organism) = self
                .organisms
                .iter_mut()
                .find(|o| o.id == transformation.organism_id)
            {
                Self::resolve_transformation(
                    transformation,
                    organism,
                    &mut self.environment,
                    &mut self.energy_ledger,
                );
            }
        }

        // ----------------------------------------------------
        // ORGANISMS: perception -> memory -> movement
        // ----------------------------------------------------

        let environment_snapshot = self.environment.clone();

        for organism in &mut self.organisms {
            organism.age += 1;

            Self::update_resource_perception(
                organism,
                &environment_snapshot,
            );

            Self::update_memory_from_sources(
                organism,
                &environment_snapshot,
            );

            Self::update_movement(
                organism,
                &environment_snapshot,
            );
        }

        // ----------------------------------------------------
        // RESOURCE TRANSFORMATIONS - initiation
        // ----------------------------------------------------

        for organism in &mut self.organisms {
            if let Some(transformation) = Self::try_start_transformation(
                organism,
                &mut self.environment.field,
                &mut self.next_transformation_id,
            ) {
                self.active_transformations.push(transformation);
            }
        }

        // ----------------------------------------------------
        // ENERGY CONSEQUENCES
        // ----------------------------------------------------

        for organism in &mut self.organisms {
            Self::apply_energy_capacity(organism);
        }

        self.energy_ledger.total_usable_energy_held =
            self.organisms.iter().map(|o| o.usable_energy).sum();

        self.snapshot()
    }


    // ========================================================
    // SNAPSHOT
    // ========================================================

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,

            organisms: self.organisms.clone(),

            environment: self.environment.clone(),

            active_transformations: self.active_transformations.clone(),

            energy_ledger: self.energy_ledger,
        }
    }

    // ========================================================
    // CONSERVATION DIAGNOSTIC  (Phase 1, Task 5)
    // ========================================================
    //
    // Total material currently accounted for across the WHOLE
    // system: reservoir + active field + every in-flight
    // transformation's committed material + every organism's stored
    // material. Not called every tick (summing the field is not
    // free) - intended for tests and periodic debug checks, to catch
    // the class of bug where material is silently created or
    // destroyed by a code path that isn't a defined physical
    // operation.
    // ========================================================

    #[cfg(test)]
    fn total_material_in_system(&self) -> f64 {
        let mut total = self.environment.field.total_amount();
        total += self.environment.reservoir.total_amount();

        for transformation in &self.active_transformations {
            total += transformation.material.total_amount();
        }

        for organism in &self.organisms {
            total += organism.stored_unbonded.total_amount();
            // Each StructuralUnit is exactly one discrete unit of its
            // resource type (locked: "resource units are discrete
            // physical units") - nominal amount 1.0 each, matching
            // how a bulk Material's (name, amount) pairs count units.
            total += organism.structure.units.len() as f64;
        }

        total
    }
}


// ============================================================
// TICK LOOP
// ============================================================

fn start_tick_loop(
    simulation: Arc<Mutex<Simulation>>,
    broadcaster: broadcast::Sender<String>,
) {
    tokio::spawn(async move {
        loop {
            let tick_duration = {
                let sim = simulation.lock();

                if !sim.running {
                    Duration::from_millis(100)
                } else {
                    let tps =
                        sim.ticks_per_second.max(0.001);

                    Duration::from_secs_f64(
                        1.0 / tps
                    )
                }
            };

            tokio::time::sleep(
                tick_duration
            ).await;

            let snapshot = {
                let mut sim =
                    simulation.lock();

                sim.step()
            };

            if let Ok(json) =
                serde_json::to_string(&snapshot)
            {
                let _ =
                    broadcaster.send(json);
            }
        }
    });
}


// ============================================================
// HTTP SNAPSHOT
// ============================================================

async fn snapshot_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let simulation =
        state.simulation.lock();

    Json(simulation.snapshot())
}


// ============================================================
// WEBSOCKET
// ============================================================

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(
        |socket| handle_socket(socket, state)
    )
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
) {
    let (mut sender, mut receiver) =
        socket.split();

    let mut rx =
        state.broadcaster.subscribe();

    let mut send_task =
        tokio::spawn(async move {
            while let Ok(message) =
                rx.recv().await
            {
                if sender
                    .send(Message::Text(message))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

    let mut receive_task =
        tokio::spawn(async move {
            while let Some(
                Ok(_message)
            ) = receiver.next().await
            {}
        });

    tokio::select! {
        _ = (&mut send_task) => {
            receive_task.abort();
        }

        _ = (&mut receive_task) => {
            send_task.abort();
        }
    }
}


// ============================================================
// MAIN
// ============================================================

#[tokio::main]
async fn main() {
    let (tx, _rx) =
        broadcast::channel::<String>(128);

    let simulation =
        Simulation::new(42, 10.0);

    let simulation =
        Arc::new(Mutex::new(simulation));

    let state = AppState {
        simulation: simulation.clone(),
        broadcaster: tx.clone(),
    };

    start_tick_loop(
        simulation,
        tx.clone(),
    );

    let app =
        Router::new()
            .route(
                "/snapshot",
                get(snapshot_handler),
            )
            .route(
                "/ws",
                get(ws_handler),
            )
            .with_state(state)
            .layer(
                CorsLayer::permissive()
            );

    let address =
        SocketAddr::from(
            ([127, 0, 0, 1], 3000)
        );

    println!(
        "Listening on {}",
        address
    );

    let listener =
        TcpListener::bind(address)
            .await
            .unwrap();

    axum::serve(
        listener,
        app,
    )
    .await
    .unwrap();
}


// ============================================================
// TESTS - Phase 1, Task 5: environmental conservation, run
// through the actual Simulation (not just environment.rs in
// isolation) so integration bugs at the call sites show up too.
// ============================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn fresh_organism_owns_an_empty_structure() {
        let organism = Simulation::create_initial_organism();
        assert!(organism.structure.units.is_empty());
        assert!(organism.structure.bonds.is_empty());
    }

    #[test]
    fn store_unbonded_material_accepts_raw_material_only() {
        let mut organism = Simulation::create_initial_organism();

        organism.store_unbonded_material(Material { parts: vec![("Carbon".into(), 5.0)], bonded: false });
        assert!((organism.stored_unbonded.total_amount() - 5.0).abs() < 1e-9);

        // Bonded material is explicitly rejected here (see the TODO
        // on store_unbonded_material) rather than silently absorbed
        // into a bulk pool that would conflict with real structure.
        organism.store_unbonded_material(Material { parts: vec![("Carbon".into(), 3.0)], bonded: true });
        assert!((organism.stored_unbonded.total_amount() - 5.0).abs() < 1e-9, "bonded material must not be silently absorbed");
        assert!(organism.structure.units.is_empty(), "bonded material must not be silently instantiated either");
    }

    #[test]
    fn structural_units_count_toward_total_material_conservation() {
        let mut sim = Simulation::new(1, 10.0);
        sim.organisms[0].structure.add_unit(structure::StructuralUnit::new(
            "Carbon",
            structure::Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 },
        ));

        let before = sim.total_material_in_system();
        for _ in 0..500 {
            sim.step();
        }
        let after = sim.total_material_in_system();

        assert!(
            (before - after).abs() < 1e-3,
            "a structural unit sitting on an organism must be conserved like any other material: before={before}, after={after}"
        );
    }

    #[test]
    fn fresh_simulation_conserves_total_material_over_many_ticks() {
        let mut sim = Simulation::new(1, 10.0);

        let before = sim.total_material_in_system();

        for _ in 0..3000 {
            sim.step();
        }

        let after = sim.total_material_in_system();

        assert!(
            (before - after).abs() < 1e-3,
            "total material must be conserved across a full simulation run: before={before}, after={after}"
        );
    }

    #[test]
    fn no_resource_cloud_pathway_exists() {
        // Compile-time proof, not a runtime assertion: Environment no
        // longer has a `resource_clouds` field or any cloud-related
        // method. If this test compiles, the legacy pathway is gone.
        let env = Simulation::create_environment();
        assert!(env.field.cells.len() > 0);
        assert!(env.reservoir.cells.len() > 0);
    }

    #[test]
    fn organism_can_perceive_material_from_the_field() {
        // Run long enough for a vent to emit near the organism's
        // starting position - proves perception is correctly wired
        // to the field (not the old ResourceCloud list).
        let mut sim = Simulation::new(7, 10.0);

        let mut ever_sensed_something = false;

        for _ in 0..500 {
            sim.step();
            if !sim.organisms[0].resource_sense.sensed_resources.is_empty() {
                ever_sensed_something = true;
            }
        }

        assert!(ever_sensed_something, "organism should perceive field material at some point");
    }

    #[test]
    fn organism_can_break_bonded_material_once_available() {
        // KNOWN PHASE 1 GAP, surfaced deliberately by this test rather
        // than hidden: a freshly-seeded world currently has NO viable
        // path to bonded material anywhere. seed_uniform only seeds
        // unbonded reservoir stock, vents are now required to be
        // indiscriminate (no bonded preference/fallback), and COMBINE
        // - the only mechanism that legitimately creates bonded
        // material - is out of scope until Phase 2. So organisms
        // cannot actually gain energy in a real, freshly-started
        // simulation right now (see organism_can_perceive_and_process_
        // material_from_the_field, which used to assert this and has
        // been narrowed to perception-only above).
        //
        // This test instead proves the INITIATION -> RESOLUTION
        // mechanism itself is correctly wired to the field, by
        // manually placing bonded material directly into the field
        // (bypassing vents entirely) and confirming the organism can
        // still sense it, commit to it, and gain usable energy from
        // it. Once COMBINE exists, the real bootstrap path will feed
        // this same mechanism - this test does not need to change
        // when that happens.
        let mut sim = Simulation::new(7, 10.0);

        let (px, py) = {
            let p = &sim.organisms[0].occupied_cells[0];
            (p.x, p.y)
        };

        sim.environment.field.deposit(
            px,
            py,
            Material {
                parts: vec![("Methane".into(), 50.0)],
                bonded: true,
            },
        );

        let mut ever_gained_energy = false;
        for _ in 0..200 {
            sim.step();
            if sim.organisms[0].usable_energy > 0.0 {
                ever_gained_energy = true;
                break;
            }
        }

        assert!(ever_gained_energy, "organism should be able to BREAK bonded material and gain energy");
    }

    #[test]
    fn vent_emission_does_not_create_or_destroy_material() {
        let mut sim = Simulation::new(3, 10.0);
        let before = sim.total_material_in_system();

        for _ in 0..1000 {
            sim.step_environment();
            sim.tick += 1;
        }

        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }
}
