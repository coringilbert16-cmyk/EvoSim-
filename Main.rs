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
 
mod genome;
mod math;
mod resources;
 
use genome::{initial_genome, Genome};
use math::exponential_influence;
use resources::{
    property_ranges, BaseResource, Material, ResourceBaselines, ResourceProperties,
};
 
 
// ============================================================
// APPLICATION STATE
// ============================================================
 
#[derive(Clone)]
struct AppState {
    simulation: Arc<Mutex<Simulation>>,
    broadcaster: broadcast::Sender<String>,
}
 
 
// ============================================================
// FUNDAMENTAL RESOURCE MODEL
// ============================================================
//
// Resource properties are immutable.
// Amount is environmental quantity and is NOT a property of the
// resource itself.
//
// The four universal properties are:
//   mass
//   potential_energy
//   reactivity
//   cohesion
//
// Energy is NOT a fundamental resource. It is produced as a
// consequence of resource transformations (see TRANSFORMATIONS
// below).
// ============================================================
 
// ResourceProperties, BaseResource (the catalog entry), ResourceBaselines,
// and Material now come from `resources.rs` (the canonical implementation -
// see PRIMARY OBJECTIVE of this integration). Main.rs no longer keeps its
// own duplicate copies of these types.
//
// Environmental *amount* bookkeeping (how much raw material exists) is not
// part of the canonical catalog (which only models immutable per-type
// properties), so it's tracked here as a separate, minimal reservoir.
 
#[derive(Serialize, Deserialize, Clone)]
struct ReservoirEntry {
    name: String,
    amount: f64,
}
 
 
// Genome (mutable organism traits) now comes from `genome.rs` (the
// canonical implementation). Main.rs no longer keeps its own copy.
 
 
// ============================================================
// RESOURCE PERCEPTION
// ============================================================
//
// A ResourceObservation is what the organism currently perceives.
// It is NOT itself a stored environmental resource.
//
// Desirability is explicitly represented here so the frontend can
// inspect the organism's actual evaluation rather than inferring
// desire from raw quantity.
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
    // blob). No longer used for lookup - see try_start_transformation,
    // which now locates the source cloud by position, not by name,
    // since a Material stack can span multiple resource types.
    name: String,
    properties: ResourceProperties,
 
    // Whether the source Material is bonded. Raw/unbonded material
    // cannot BREAK (hard rule, §4) - this lets initiation filter out
    // non-bondable sources before picking a target, rather than
    // picking the "best" target and only then discovering it's
    // ineligible.
    bonded: bool,
 
    perceived_amount: f64,
 
    deviations: PropertyDeviations,
    affinity_responses: AffinityResponses,
 
    base_desirability: f64,
    amount_factor: f64,
    potential_energy_need_factor: f64,
 
    desirability: f64,
 
    // Distance from the organism to this specific cloud, retained
    // so the processing-initiation step does not need to re-walk
    // the environment a second time to find "is this in reach".
    distance: f64,
 
    // Coordinates of the cloud this observation came from. Needed
    // so a transformation can be started against the correct
    // physical source without re-deriving it from direction alone.
    source_x: f64,
    source_y: f64,
 
    // Index of the specific Material stack within that cloud's
    // `materials` list. A cloud can (in principle) hold more than one
    // Material stack, and a Material can span several resource types,
    // so lookup by name is no longer sufficient - see
    // try_start_transformation.
    material_index: usize,
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
// Only BREAK is implemented at this stage. COMBINE is deferred
// until physical body construction (programming step 8) exists,
// since combining resources into constructed material without a
// body to attach it to has nothing to justify it under §0.
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
// Resource properties stay immutable as required; what is not yet
// modeled is the fact that this particular mass has already been
// "spent" once. This should be replaced with a proper derived-material
// representation once §16 is built.
// (Integration note: this waste is now ejected into the environment at
// the organism's location - see resolve_transformation - rather than
// being dissolved back into the abstract global reservoir, which was a
// rule violation fixed during the resources.rs integration.)
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
    // full duration of the transformation (§18.1). Reactivity, cohesion,
    // and potential energy are derived from this via the catalog at
    // resolution time rather than being snapshotted into loose fields -
    // resource properties are immutable per-type, so there's nothing to
    // snapshot that the catalog can't already give us.
    material: Material,
 
    complexity: f64,
    duration_ticks: u64,
    remaining_ticks: u64,
}
 
// complexity() now comes from math.rs (the shared canonical
// implementation) - see PRIMARY OBJECTIVE of this integration.
 
 
// ============================================================
// ENERGY LEDGER  (Master Spec §21, §101)
// ============================================================
//
// A running diagnostic ledger, not part of the causal simulation
// itself. It exists so conservation can be observed/tested (§103:
// "energy never appears, energy never disappears").
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
 
    // Energy is an organism state, not a fundamental environmental
    // resource. It changes only through resolved transformations.
    usable_energy: f64,
 
    // Accumulated physiological consequence of processing heat (see
    // resolve_transformation). This is a stand-in for real physical
    // damage (§24) until the damage system (programming step 15)
    // exists to consume it. A death-on-threshold consequence is
    // deliberately not implemented yet - out of scope for the
    // resources.rs integration.
    stress: f64,
 
    stored_resources: Vec<Material>,
 
    development_stage: DevelopmentStage,
 
    age: u64,
 
    // At most one active transformation per organism at this stage
    // (minimum-information simplification; nothing in the spec
    // requires supporting concurrent transformations yet).
    active_transformation_id: Option<u64>,
}
 
 
// ============================================================
// ENVIRONMENTAL SOURCES
// ============================================================
 
#[derive(Serialize, Deserialize, Clone)]
struct Vent {
    x: f64,
    y: f64,
 
    composition: Vec<(String, f64)>,
 
    emission_amount: f64,
    emission_interval: u64,
    emission_timer: u64,
}
 
#[derive(Serialize, Deserialize, Clone)]
struct ResourceCloud {
    x: f64,
    y: f64,
 
    radius: f64,
    maximum_radius: f64,
    expansion_rate: f64,
 
    // Canonical Material stacks (see resources.rs), not raw named
    // Resource entries. A cloud normally holds a single Material - one
    // vent emission or one BREAK's ejected waste - but the Vec allows
    // more than one to accumulate without special-casing.
    materials: Vec<Material>,
}
 
#[derive(Serialize, Deserialize, Clone)]
struct Environment {
    width: f64,
    height: f64,
 
    // Immutable per-type properties for every resource that exists in
    // this world. Never mutated after construction - see resources.rs.
    catalog: Vec<BaseResource>,
 
    // How much raw material of each catalog type currently exists in
    // the abstract global pool (§7: finite in implementation, large
    // enough to behave as inexhaustible). This is NOT part of the
    // canonical resources.rs types, since the catalog only models
    // immutable properties, not quantity - amount is deliberately kept
    // separate here.
    reservoir: Vec<ReservoirEntry>,
 
    vents: Vec<Vent>,
 
    resource_clouds: Vec<ResourceCloud>,
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
        // The catalog (immutable per-type properties) is the canonical
        // one from resources.rs, not a duplicate hand-rolled list.
        let catalog = resources::default_catalog();
 
        // Starting abundance per type. This is bookkeeping the catalog
        // itself deliberately doesn't carry (see Environment::reservoir
        // doc comment), so it's declared here alongside catalog
        // construction rather than embedded in a duplicate resource type.
        let starting_amounts: [(&str, f64); 7] = [
            ("Carbon", 10_000.0),
            ("Methane", 5_000.0),
            ("Hydrogen", 5_000.0),
            ("Sulfur", 5_000.0),
            ("Nitrogen", 5_000.0),
            ("Phosphorus", 5_000.0),
            ("Water", 20_000.0),
        ];
 
        let reservoir = starting_amounts
            .iter()
            .map(|(name, amount)| ReservoirEntry {
                name: (*name).into(),
                amount: *amount,
            })
            .collect();
 
        Environment {
            width: 1000.0,
            height: 1000.0,
 
            catalog,
            reservoir,
 
            vents: vec![
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
            ],
 
            resource_clouds: Vec::new(),
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
 
            // The canonical genome constructor from genome.rs - this also
            // picks up adult_mass, which Main's hand-rolled trait list was
            // missing (genome.rs's Genome::adult_mass() would otherwise
            // have silently fallen back to its default every time).
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
 
            stored_resources: Vec::new(),
 
            development_stage: DevelopmentStage::Juvenile,
 
            age: 0,
 
            active_transformation_id: None,
        }
    }
 
 
    // property_ranges() now comes from resources.rs (canonical) - called
    // as `property_ranges(&environment.catalog)` at call sites below.
 
 
    // ========================================================
    // PROPERTY DEVIATION
    // ========================================================
 
    fn calculate_property_deviations(
        properties: &ResourceProperties,
        baselines: &ResourceBaselines,
        ranges: &ResourceProperties,
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
    //
    // Now load-bearing: usable_energy actually changes once
    // transformations resolve, so this genuinely modulates
    // potential-energy affinity as energy state changes, per §38.
    // ========================================================
 
    fn energy_need_factor(usable_energy: f64) -> f64 {
        1.0 / (1.0 + usable_energy.max(0.0))
    }
 
 
    // ========================================================
    // RESOURCE DESIRABILITY
    // ========================================================
 
    fn calculate_desirability(
        organism: &Organism,
        properties: &ResourceProperties,
        perceived_amount: f64,
        baselines: &ResourceBaselines,
        ranges: &ResourceProperties,
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
    // CLOUD EMISSION
    // ========================================================
 
    fn emit_resource_clouds(&mut self) {
        let mut new_clouds = Vec::new();
 
        for vent in &mut self.environment.vents {
            if vent.emission_timer > 0 {
                vent.emission_timer -= 1;
                continue;
            }
 
            let mut parts = Vec::new();
 
            for (resource_name, proportion) in &vent.composition {
                if let Some(entry) = self
                    .environment
                    .reservoir
                    .iter_mut()
                    .find(|r| &r.name == resource_name)
                {
                    let requested = vent.emission_amount * proportion;
 
                    let amount = requested.min(entry.amount);
 
                    entry.amount -= amount;
 
                    if amount > 0.0 {
                        parts.push((entry.name.clone(), amount));
                    }
                }
            }
 
            if !parts.is_empty() {
                // §4 / Correction #5: raw/unbonded material cannot BREAK,
                // so vents supply their emission pre-bonded - otherwise
                // there is no viable initial energy pathway for the first
                // organism(s). This is a data decision explicitly
                // permitted by the spec ("some material emitted by
                // environmental vents may be pre-bonded when necessary"),
                // not new bonding mechanics.
                let material = Material {
                    parts,
                    bonded: true,
                };
 
                new_clouds.push(ResourceCloud {
                    x: vent.x,
                    y: vent.y,
 
                    radius: 8.0,
                    maximum_radius: 100.0,
                    expansion_rate: 1.0,
 
                    materials: vec![material],
                });
            }
 
            vent.emission_timer = vent.emission_interval;
        }
 
        self.environment.resource_clouds.extend(new_clouds);
    }
 
 
    // ========================================================
    // CLOUD DISPERSION
    // ========================================================
 
    fn update_resource_clouds(&mut self) {
        for cloud in &mut self.environment.resource_clouds {
            cloud.radius += cloud.expansion_rate;
        }
 
        let (still_active, expired): (Vec<_>, Vec<_>) = self
            .environment
            .resource_clouds
            .drain(..)
            .partition(|cloud| cloud.radius < cloud.maximum_radius);
 
        self.environment.resource_clouds = still_active;
 
        for expired_cloud in expired {
            for material in expired_cloud.materials {
                for (name, amount) in material.parts {
                    if let Some(entry) = self
                        .environment
                        .reservoir
                        .iter_mut()
                        .find(|r| r.name == name)
                    {
                        entry.amount += amount;
                    }
                }
            }
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
 
        let baselines = ResourceBaselines::from_catalog(
            &environment.catalog,
        );
 
        let ranges = property_ranges(
            &environment.catalog,
        );
 
        for cloud in &environment.resource_clouds {
            let dx = cloud.x - px;
            let dy = cloud.y - py;
 
            let distance = (dx * dx + dy * dy).sqrt();
 
            if distance > perception_radius + cloud.radius {
                continue;
            }
 
            let direction_x = if distance > 0.0 {
                dx / distance
            } else {
                0.0
            };
 
            let direction_y = if distance > 0.0 {
                dy / distance
            } else {
                0.0
            };
 
            for (material_index, material) in cloud.materials.iter().enumerate() {
                let perceived_amount =
                    material.total_amount() * sensory_resolution;
 
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
 
                        bonded: material.bonded,
 
                        perceived_amount,
 
                        deviations,
 
                        affinity_responses: responses,
 
                        base_desirability,
 
                        amount_factor,
 
                        potential_energy_need_factor:
                            energy_need_factor,
 
                        desirability,
 
                        distance,
 
                        source_x: cloud.x,
                        source_y: cloud.y,
 
                        material_index,
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
    //
    // Location + strength derived from the strongest currently
    // perceived positive-desirability source. This intentionally
    // does not yet distinguish "saw something good" from
    // "processing it actually succeeded" - that refinement belongs
    // to §65 interaction/consequence history and is layered on by
    // reinforce_memory_from_transformation() below once a
    // transformation resolves.
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
            ResourceBaselines::from_catalog(
                &environment.catalog,
            );
 
        let ranges =
            property_ranges(
                &environment.catalog,
            );
 
        let mut strongest_source:
            Option<(f64, f64, f64)> = None;
 
        for cloud in &environment.resource_clouds {
            let dx = cloud.x - px;
            let dy = cloud.y - py;
 
            let distance =
                (dx * dx + dy * dy).sqrt();
 
            if distance > perception_radius + cloud.radius {
                continue;
            }
 
            for material in &cloud.materials {
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
                        cloud.x,
                        cloud.y,
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
 
        // An organism mid-transformation holds physical position
        // rather than drifting away from committed resources. This
        // is a movement-physics consequence of an active process,
        // not a special-cased rule (§18.1 resource commitment still
        // implies physical presence at the source).
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
    // PROCESSING_REACH of a positively-desirable resource cloud,
    // commits a bounded amount of that resource and begins a BREAK.
    // ========================================================
 
    fn try_start_transformation(
        organism: &mut Organism,
        environment: &mut Environment,
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
                    // Filtered here, before picking a target, so an
                    // organism never "chooses" a source it's physically
                    // unable to process.
                    && r.bonded
            })
            .max_by(|a, b| {
                a.desirability
                    .partial_cmp(&b.desirability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?
            .clone();
 
        let cloud = environment
            .resource_clouds
            .iter_mut()
            .find(|c| {
                (c.x - target.source_x).abs() < f64::EPSILON
                    && (c.y - target.source_y).abs() < f64::EPSILON
            })?;
 
        let material = cloud.materials.get_mut(target.material_index)?;
 
        // Defensive re-check: can_break() requires bonded && total >= 2.0,
        // matching the hard rule at the mechanical level too, not just at
        // the perception-filter level above.
        if !material.can_break() {
            return None;
        }
 
        let committed_amount =
            PROCESSING_RATE.min(material.total_amount());
 
        if committed_amount <= 0.0 {
            return None;
        }
 
        // §18.1 resource commitment: remove from the environment now,
        // the transformation owns it until it resolves.
        let committed = material.take(committed_amount)?;
 
        if material.is_empty() {
            cloud.materials.remove(target.material_index);
        }
 
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
 
        // Reactivity's influence is exponential (§11) but bounded so
        // it can never extract more potential energy than the input
        // actually contains (§21 - no formula may create energy).
        // Uses the shared canonical helper from math.rs rather than a
        // locally duplicated formula.
        let yield_fraction = exponential_influence(props.reactivity);
 
        let gross_extracted =
            input_potential_energy * yield_fraction;
 
        // Cohesion resists breaking apart (§12): high-cohesion
        // material taxes the extraction as an overhead cost that is
        // dissipated as heat rather than becoming usable energy.
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
 
        // Heat generated by processing is internal physiological stress
        // (per this integration's rules), not the old "excess held
        // energy" trigger apply_energy_capacity() used to check. Stress
        // still decays over time (see apply_energy_capacity) and a
        // death-on-threshold consequence is deliberately NOT implemented
        // yet - that formula is out of scope for this task.
        organism.stress += heat;
 
        // §5 / Correction #3: spent material does not simply disappear
        // and must NOT be dissolved back into the abstract global
        // reservoir. BREAK breaks the bonds, so the leftover mass becomes
        // waste (unbonded) and is physically ejected at the organism's
        // current location as a new environmental Material, discoverable
        // by other organisms (§5, §6 ecological-opportunity intent).
        // "Build self" (retaining spent mass as organism structure) is
        // NOT implemented here - that requires a body/structure system
        // that hasn't been integrated yet, so 100% of spent mass
        // currently becomes waste.
        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };
 
        if !transformation.material.is_empty() {
            let waste = Material {
                parts: transformation.material.parts.clone(),
                bonded: false,
            };
 
            environment.resource_clouds.push(ResourceCloud {
                x: px,
                y: py,
 
                radius: 8.0,
                maximum_radius: 100.0,
                expansion_rate: 1.0,
 
                materials: vec![waste],
            });
        }
 
        ledger.total_potential_energy_released += gross_extracted;
        ledger.total_usable_energy_gained += usable_gained;
        ledger.total_heat_dissipated += heat;
 
        organism.active_transformation_id = None;
 
        // Outcome-based reinforcement (§65): a transformation that
        // actually completed here is stronger evidence than merely
        // perceiving desirability, so it reinforces memory again.
        // (px, py) was already computed above for waste ejection.
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
        // Stress now accumulates from processing heat at the moment a
        // transformation resolves (see resolve_transformation) rather
        // than from held usable_energy exceeding a capacity placeholder.
        // This function's remaining job is just the decay side (§24).
        // A death consequence once stress exceeds some threshold is
        // explicitly deferred - see integration notes.
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
        // ENVIRONMENT / RESOURCES
        // ----------------------------------------------------
 
        self.emit_resource_clouds();
        self.update_resource_clouds();
 
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
                &mut self.environment,
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
