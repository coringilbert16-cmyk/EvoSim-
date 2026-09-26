#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::decision::{DecisionHistory, DecisionParameters};
use crate::decomposition::DecomposingBody;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::environment::ActiveMaterialField;
use crate::genome::Genome;
use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, OrganismStructure};
use parking_lot::Mutex;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) simulation: Arc<Mutex<Simulation>>,
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum DevelopmentStage {
    Offspring,
    Juvenile,
    Adult,
}
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct Position {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum MovementFailureReason {
    NoDirection,
    ActiveTransformation,
    InsufficientEnergy,
    NonFiniteDisplacement,
    NoOccupiedCell,
    ZeroDisplacement,
    BlockedByPushChain,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct MovementAttemptDiagnostic {
    pub(crate) tick: u64,
    pub(crate) direction_x: Option<f64>,
    pub(crate) direction_y: Option<f64>,
    pub(crate) step: Option<f64>,
    pub(crate) usable_energy: f64,
    pub(crate) active_transformation_id: Option<u64>,
    pub(crate) result: Result<(), MovementFailureReason>,
    pub(crate) old_position: Option<Position>,
    pub(crate) new_position: Option<Position>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct MemoryPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) strength: f64,
    /// Spectral content physically received by the genome cavity when this
    /// memory was formed or reinforced.
    #[serde(default)]
    pub(crate) spectrum: crate::harmonics::ToneSpectrum,
    /// The observed consequence associated with the remembered spectrum.
    #[serde(default)]
    pub(crate) outcome: Option<crate::decision::OutcomeKind>,
}
pub(crate) const MEMORY_DECAY_PER_TICK: f64 = 0.995;
pub(crate) const MEMORY_MERGE_RADIUS: f64 = 40.0;
pub(crate) const MEMORY_PRUNE_THRESHOLD: f64 = 0.01;
pub(crate) const COMBINE_PROCESSING_RATE: usize = 1;
pub(crate) const BREAK_PROCESSING_RATE: usize = 1;
#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) enum TransformationKind {
    Break,
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ActiveTransformation {
    pub(crate) id: u64,
    pub(crate) organism_id: String,
    pub(crate) kind: TransformationKind,
    pub(crate) material: Material,
    #[serde(default)]
    pub(crate) bond: Option<Bond>,
    /// Voluntary BREAK acts only on a bond belonging to stored physical material.
    #[serde(default)]
    pub(crate) stored_material: Option<crate::physical_material::PhysicalMaterial>,
    #[serde(default)]
    pub(crate) stored_bond: Option<crate::physical_material::PhysicalMaterialBond>,
    pub(crate) complexity: f64,
    pub(crate) duration_ticks: u64,
    pub(crate) remaining_ticks: u64,
    pub(crate) decision_context_key: Option<String>,
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ReproductiveConstruction {
    pub(crate) committed_material: MaterialStorage,
    pub(crate) developing_structure: OrganismStructure,
    pub(crate) child_genome: Genome,
    /// Persistent developmental frame for the physically separate offspring.
    #[serde(default)]
    pub(crate) developmental_origin: Position,
    #[serde(default)]
    pub(crate) developmental_orientation_radians: f64,
    /// Stress accumulated by the developing offspring before detachment.
    #[serde(default)]
    pub(crate) developing_stress: f64,
    /// Unit index of the transferred anchor. The qualifying physical genome
    /// must retain this unit as part of its realized cavity boundary.
    #[serde(default)]
    pub(crate) anchor_unit_index: usize,
    /// Usable energy held by the physically separate developing offspring.
    /// It is transferred from the parent while construction remains active.
    #[serde(default)]
    pub(crate) developing_energy: f64,
    /// The parent needs to reorganize its own structure to make room for the developing offspring.
    #[serde(default)]
    pub(crate) needs_space: bool,
}
#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub(crate) struct EnergyLedger {
    pub(crate) total_potential_energy_released: f64,
    pub(crate) total_usable_energy_gained: f64,
    pub(crate) total_heat_dissipated: f64,
    pub(crate) total_usable_energy_held: f64,
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Organism {
    pub(crate) id: String,
    /// Persistent organism-local developmental origin. This is anchored at
    /// the organism's own persistent developmental reference and moves with the
    /// organism; it is never
    /// re-centered on current geometry or center of mass.
    #[serde(default)]
    pub(crate) developmental_origin: Position,
    /// Initial developmental-frame orientation. This remains fixed unless
    /// an explicitly approved orientation mechanism is introduced.
    #[serde(default)]
    pub(crate) developmental_orientation_radians: f64,
    pub(crate) occupied_cells: Vec<Position>,
    pub(crate) genome: Genome,
    /// Spectrum currently present at the organism's physically realized genome cavity.
    #[serde(default)]
    pub(crate) harmonic_spectrum: crate::harmonics::ToneSpectrum,
    pub(crate) memory: Vec<MemoryPoint>,
    pub(crate) decision_history: DecisionHistory,
    pub(crate) usable_energy: f64,
    pub(crate) stress: f64,
    /// Maintenance that this organism has historically failed to pay.
    #[serde(default)]
    pub(crate) maintenance_debt: f64,
    #[serde(default = "default_stress_threshold")]
    pub(crate) stress_threshold: f64,
    pub(crate) stored_material: MaterialStorage,
    pub(crate) structure: OrganismStructure,
    pub(crate) development_stage: DevelopmentStage,
    pub(crate) active_transformation_id: Option<u64>,
    #[serde(default)]
    pub(crate) reproductive_construction: Option<ReproductiveConstruction>,
    #[serde(default)]
    pub(crate) structure_revision: u64,
    #[serde(default)]
    pub(crate) position_revision: u64,
    #[serde(skip)]
    pub(crate) cached_cavity_revision: Option<u64>,
    #[serde(skip)]
    pub(crate) cached_cavity: Option<Option<crate::cavity::GenomeCavity>>,
    #[serde(skip)]
    pub(crate) cached_developmental_revision: Option<u64>,
    #[serde(skip)]
    pub(crate) cached_developmental_realization:
        Option<crate::developmental_blueprint::DevelopmentalRealization>,
    /// Highest developmental realization previously reached by this organism.
    /// It is a memory of the organism's own realized physical self, not a new target.
    #[serde(default)]
    pub(crate) peak_developmental_realization: f64,
    #[serde(skip)]
    pub(crate) cached_harmonic_key: Option<(u64, u64, u64)>,
    #[serde(skip)]
    pub(crate) last_movement_attempt: Option<MovementAttemptDiagnostic>,
}
pub(crate) const STRESS_DECAY_PER_TICK: f64 = 0.98;
pub(crate) const INITIAL_STRESS_THRESHOLD: f64 = 100.0;
pub(crate) const STRESS_THRESHOLD_DECAY: f64 = 0.90;
pub(crate) const MAINTENANCE_ENERGY_PER_MASS: f64 = 0.001;
pub(crate) const MIN_STRESS_THRESHOLD: f64 = 5.0;
fn default_stress_threshold() -> f64 {
    INITIAL_STRESS_THRESHOLD
}
impl Organism {
    pub(crate) fn mark_structure_changed(&mut self) {
        self.structure_revision = self.structure_revision.wrapping_add(1);
        self.cached_cavity_revision = None;
        self.cached_cavity = None;
        self.cached_developmental_revision = None;
        self.cached_developmental_realization = None;
        self.cached_harmonic_key = None;
    }

    pub(crate) fn mark_position_changed(&mut self) {
        self.position_revision = self.position_revision.wrapping_add(1);
        self.cached_harmonic_key = None;
    }

    pub(crate) fn genome_cavity_cached(
        &mut self,
        catalog: &[BaseResource],
    ) -> Option<crate::cavity::GenomeCavity> {
        if self.cached_cavity_revision != Some(self.structure_revision) {
            let cavity = crate::cavity::analyze_genome_cavity(&self.structure, catalog)
                .ok()
                .flatten()
                .filter(|cavity| cavity.qualifies());
            self.cached_cavity = Some(cavity);
            self.cached_cavity_revision = Some(self.structure_revision);
        }
        self.cached_cavity.clone().flatten()
    }

    pub(crate) fn developmental_realization_cached(
        &mut self,
        catalog: &[BaseResource],
    ) -> Option<crate::developmental_blueprint::DevelopmentalRealization> {
        if self.cached_developmental_revision != Some(self.structure_revision) {
            let (seed_mass, seed_length) =
                crate::juvenile::confirmed_seed_scale_reference(catalog).ok()?;
            let preferred_length = self
                .genome
                .developmental_blueprint
                .preferred_developmental_length(self.genome.adult_mass(), seed_mass, seed_length);
            self.cached_developmental_realization =
                Some(self.genome.developmental_blueprint.realization_at_length(
                    &self.structure,
                    catalog,
                    (self.developmental_origin.x, self.developmental_origin.y),
                    self.developmental_orientation_radians,
                    preferred_length,
                ));
            self.cached_developmental_revision = Some(self.structure_revision);
        }
        self.cached_developmental_realization
    }

    pub(crate) fn store_material(&mut self, material: Material) -> bool {
        self.stored_material.store(material)
    }
    pub(crate) fn structural_mass(&self, catalog: &[BaseResource]) -> f64 {
        self.structure
            .units
            .iter()
            .map(|unit| unit.material.mass(catalog))
            .sum()
    }
    pub(crate) fn add_transaction_stress(&mut self, heat: f64) {
        if heat.is_finite() && heat > 0.0 {
            self.stress += heat
        }
    }
    pub(crate) fn apply_maintenance(
        &mut self,
        catalog: &[BaseResource],
        ledger: &mut EnergyLedger,
    ) {
        let demand =
            (self.structural_mass(catalog).max(0.0) * MAINTENANCE_ENERGY_PER_MASS).max(0.0);
        if !demand.is_finite() || demand <= 0.0 {
            return;
        }
        let paid = self.usable_energy.min(demand).max(0.0);
        let deficit = demand - paid;
        if deficit > 0.0 {
            self.maintenance_debt += deficit;
        }

        if paid > 0.0 {
            let tx = EnergyTransaction {
                reason: EnergyReason::Maintenance,
                potential_released: 0.0,
                usable_delta: -paid,
                structural_delta: 0.0,
                heat_dissipated: paid,
            };
            if ledger.settle_transaction(&mut self.usable_energy, tx) {
                self.add_transaction_stress(paid);
            } else {
                self.maintenance_debt += paid;
            }
        }

        // Historical debt only becomes an active stress pressure while current
        // maintenance is also going unpaid. Recovering energy stops further
        // starvation damage without retroactively repairing past consequences.
        if deficit > 0.0 {
            self.stress += deficit + self.maintenance_debt;
        }
    }
    pub(crate) fn apply_stress_damage(
        &mut self,
        environment: &Environment,
        ledger: &mut EnergyLedger,
        rng: &mut ChaCha8Rng,
    ) -> bool {
        if !self.stress.is_finite() || !self.stress_threshold.is_finite() {
            return true;
        }
        while self.stress >= self.stress_threshold.max(MIN_STRESS_THRESHOLD) {
            let threshold = self.stress_threshold.max(MIN_STRESS_THRESHOLD);
            self.stress -= threshold;
            self.stress_threshold = (threshold * STRESS_THRESHOLD_DECAY).max(MIN_STRESS_THRESHOLD);
            if self.structure.bonds.is_empty() {
                return true;
            }
            if !crate::transformation::resolve_stress_break(self, environment, ledger, rng) {
                break;
            }
        }
        false
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Environment {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) catalog: Vec<BaseResource>,
    pub(crate) field: ActiveMaterialField,
}
pub(crate) struct Simulation {
    pub(crate) tick: u64,
    pub(crate) ticks_per_second: f64,
    pub(crate) running: bool,
    pub(crate) organisms: Vec<Organism>,
    pub(crate) environment: Environment,
    pub(crate) active_transformations: Vec<ActiveTransformation>,
    pub(crate) decomposing_bodies: Vec<DecomposingBody>,
    pub(crate) energy_ledger: EnergyLedger,
    pub(crate) next_organism_id: u64,
    pub(crate) next_transformation_id: u64,
    pub(crate) rng: ChaCha8Rng,
    pub(crate) decision_parameters: DecisionParameters,
}
pub(crate) const DESIRABILITY_AMOUNT_HALF_SATURATION: f64 = 100.0;
pub(crate) const DESIRABILITY_MAX: f64 = 1.0;
