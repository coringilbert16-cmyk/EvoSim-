#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::decision::{DecisionHistory, DecisionParameters};
use crate::decomposition::DecomposingBody;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::environment::ActiveMaterialField;
use crate::genome::Genome;
use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource, Material};
use crate::structure::{Bond, OrganismStructure, Placement};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) simulation: Arc<tokio::sync::Mutex<crate::runtime::SimulationProcess>>,
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
pub(crate) struct ActiveMovement {
    /// One movement decision is a persistent operation. Physical progress is
    /// always exactly one world unit per movement step.
    pub(crate) direction_x: f64,
    pub(crate) direction_y: f64,
    pub(crate) remaining_steps: u32,
    /// Number of ticks between physical one-unit steps.
    pub(crate) step_interval: u64,
    #[serde(default)]
    pub(crate) ticks_until_step: u64,
    /// Distance selected when the operation began; retained for learning context.
    pub(crate) decision_distance: f64,
    /// Baseline state captured when the movement operation begins.
    pub(crate) before_energy: f64,
    pub(crate) before_stress: f64,
    pub(crate) before_developmental_realization: f64,
}
pub(crate) const MEMORY_DECAY_PER_TICK: f64 = 0.995;
pub(crate) const COMBINE_PROCESSING_RATE: usize = 1;
pub(crate) const BREAK_PROCESSING_RATE: usize = 1;
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub(crate) enum TransformationKind {
    Break,
    Combine,
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
    /// Original owner-storage index captured when a stored BREAK reserves its source.
    /// Kept separately from mutable decision metadata so rollback preserves exact order.
    #[serde(default)]
    pub(crate) stored_source_index: Option<usize>,
    /// Original environmental cell/material indices captured when BREAK reserves a source.
    #[serde(default)]
    pub(crate) environmental_source_location: Option<(usize, usize)>,
    /// True when the transformed physical material came directly from the
    /// environment rather than organism storage.
    #[serde(default)]
    pub(crate) environmental_source: bool,
    pub(crate) complexity: f64,
    pub(crate) duration_ticks: u64,
    pub(crate) remaining_ticks: u64,
    /// Energy transaction prepared on the completion tick before atomic commit.
    #[serde(default)]
    pub(crate) prepared_energy: Option<(f64, f64, f64)>,
    /// Perception and needs captured when a voluntary BREAK begins.
    #[serde(default)]
    pub(crate) pending_experience: Option<crate::memory::PendingTransformationExperience>,
    pub(crate) decision_context_key: Option<String>,
    /// Deferred COMBINE state. Selection occurs on the first tick;
    /// energy preparation and physical mutation commit together on completion.
    #[serde(default)]
    pub(crate) prepared_structure: Option<OrganismStructure>,
    #[serde(default)]
    pub(crate) prepared_stored_material: Option<MaterialStorage>,
    #[serde(default)]
    pub(crate) prepared_usable_energy: Option<f64>,
    #[serde(default)]
    pub(crate) prepared_stress: Option<f64>,
    #[serde(default)]
    pub(crate) prepared_ledger: Option<EnergyLedger>,
    /// Exact ledger state before staged COMBINE settlement. Absent on legacy persisted transactions.
    #[serde(default)]
    pub(crate) prepared_ledger_before: Option<EnergyLedger>,
    /// Exact usable-energy value before staged COMBINE settlement; absent on legacy persisted transactions.
    #[serde(default)]
    pub(crate) prepared_usable_energy_before: Option<f64>,
    /// Environmental material removed when COMBINE begins, held until resolve.
    #[serde(default)]
    pub(crate) combine_environmental_source:
        Option<(usize, crate::physical_material::PhysicalMaterial)>,
    /// Original index within the environmental cell, for exact COMBINE rollback.
    /// Optional for compatibility with persisted transformations created before
    /// indexed environmental-source escrow was recorded.
    #[serde(default)]
    pub(crate) combine_environmental_source_index: Option<usize>,
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ReproductiveConstruction {
    pub(crate) committed_material: MaterialStorage,
    pub(crate) developing_structure: OrganismStructure,
    /// Stable physical IDs owned by the developing member within the aggregate migration.
    /// This metadata is intentionally ID-based; unit indices are not durable across staging.
    #[serde(default)]
    pub(crate) developing_member_ids: Vec<crate::structure::PhysicalConstituentId>,
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
    /// Stable identity of the reproductive anchor; index is retained for legacy serialized state.
    #[serde(default)]
    pub(crate) anchor_physical_id: Option<crate::structure::PhysicalConstituentId>,
    /// Usable energy held by the physically separate developing offspring.
    /// It is transferred from the parent while construction remains active.
    #[serde(default)]
    pub(crate) developing_energy: f64,
    /// Maintenance historically unpaid by the developing offspring.
    #[serde(default)]
    pub(crate) developing_maintenance_debt: f64,
    /// The parent needs to reorganize its own structure to make room for the developing offspring.
    #[serde(default)]
    pub(crate) needs_space: bool,
}

impl ReproductiveConstruction {
    /// Refresh the developing graph from stable IDs when those constituents are
    /// already owned by the parent's aggregate. The aggregate is authoritative:
    /// stale cached poses/bonds must never make an attached member look detached.
    pub(crate) fn refresh_aggregate_view(&mut self, parent: &OrganismStructure) -> bool {
        if self.developing_member_ids.is_empty() {
            return false;
        }
        let mut seen = std::collections::HashSet::new();
        if self.developing_member_ids.iter().any(|id| !seen.insert(*id)) {
            return false;
        }
        let Some(anchor_id) = self.anchor_physical_id else {
            return false;
        };
        if !seen.contains(&anchor_id)
            || self
                .developing_member_ids
                .iter()
                .any(|id| parent.unit_index(*id).is_none())
        {
            return false;
        }
        let Some(view) = parent.subgraph_for_physical_ids(
            &self.developing_member_ids,
            &[anchor_id],
        ) else {
            return false;
        };
        let Some(anchor_index) = view.unit_index(anchor_id) else {
            return false;
        };
        self.developing_structure = view;
        self.anchor_unit_index = anchor_index;
        true
    }

    /// True when the developing graph is only a transient view of constituents
    /// already owned by the parent's physical aggregate. Consumers use this
    /// shared predicate to avoid counting, moving, or emitting the same body twice.
    pub(crate) fn is_aggregate_owned_by(&self, parent: &OrganismStructure) -> bool {
        let actual_ids = self.developing_structure.physical_constituent_ids();
        if actual_ids.len() != self.developing_member_ids.len() {
            return false;
        }
        let actual: std::collections::HashSet<_> = actual_ids.iter().copied().collect();
        let recorded: std::collections::HashSet<_> =
            self.developing_member_ids.iter().copied().collect();
        actual.len() == actual_ids.len()
            && recorded.len() == self.developing_member_ids.len()
            && actual == recorded
            && parent.contains_exact_physical_subgraph(&self.developing_structure)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EnergyLedger {
    /// Explicit non-structural energy sources; stored bond energy release is
    /// represented by the structural-store delta instead.
    pub(crate) total_potential_energy_released: f64,
    /// Net change in energy stored in realized structure (positive stores, negative releases).
    #[serde(default)]
    pub(crate) total_structural_energy_delta: f64,
    /// Energy received from explicit external sources, distinct from bond release.
    #[serde(default)]
    pub(crate) total_external_energy_input: f64,
    /// Guards the simulation's one-time initial-condition energy grant.
    #[serde(default)]
    pub(crate) initial_condition_energy_settled: bool,
    /// Cumulative world-tone energy that was incident on exposed material but
    /// not absorbed. This is throughput telemetry, not stored energy or an
    /// inventory term; the remainder still needs explicit propagation/escape handling.
    #[serde(default)]
    pub(crate) total_world_tone_energy_unabsorbed: f64,
    /// Residual world-tone energy that leaves the modeled world after propagation.
    #[serde(default)]
    pub(crate) total_world_tone_energy_escaped: f64,
    pub(crate) total_usable_energy_gained: f64,
    #[serde(default)]
    pub(crate) total_work_expended: f64,
    pub(crate) total_heat_dissipated: f64,
    pub(crate) total_usable_energy_held: f64,
    /// Current retained resonant energy across organism reservoirs.
    #[serde(default)]
    pub(crate) total_resonant_energy_held: f64,
    /// Tick-zero retained-resonance baseline.
    #[serde(default)]
    pub(crate) initial_resonant_energy_held: f64,
    /// Usable-energy baseline captured at simulation tick zero.
    #[serde(default)]
    pub(crate) initial_usable_energy_held: f64,
    /// Current stored bond energy across realized live and decomposing structures.
    #[serde(default)]
    pub(crate) total_structural_energy_held: f64,
    /// Stored bond-energy baseline captured at simulation tick zero.
    #[serde(default)]
    pub(crate) initial_structural_energy_held: f64,
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
    /// Energy retained across ticks, distinct from the currently received signal.
    #[serde(default)]
    pub(crate) resonant_energy: crate::harmonics::ResonantEnergyReservoir,
    /// Associative memory learned from physically received resonance and its consequences.
    #[serde(default)]
    pub(crate) experience_memory: crate::memory::ExperienceMemory,
    #[serde(default)]
    pub(crate) pending_movement_experience: Option<crate::memory::PendingMovementExperience>,
    /// Persistent multi-tick movement operation, analogous to BREAK/COMBINE.
    #[serde(default)]
    pub(crate) active_movement: Option<ActiveMovement>,
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
    /// Once this organism has established a genome cavity, an empty set of
    /// surviving boundary IDs means its core is gone, not that another cavity
    /// in a shared aggregate may silently become its genome.
    #[serde(default)]
    pub(crate) genome_core_identity_established: bool,
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
    pub(crate) cached_harmonic_key: Option<(u64, u64, u64, u64)>,
    #[serde(skip)]
    pub(crate) last_movement_attempt: Option<MovementAttemptDiagnostic>,
}
pub(crate) const STRESS_DECAY_PER_TICK: f64 = 0.98;
pub(crate) const INITIAL_STRESS_THRESHOLD: f64 = 100.0;
pub(crate) const MAINTENANCE_ENERGY_PER_MASS: f64 = 0.001;
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

    pub(crate) fn genome_cavity_cached_ref(
        &mut self,
        catalog: &[BaseResource],
    ) -> Option<&crate::cavity::GenomeCavity> {
        if self.cached_cavity_revision != Some(self.structure_revision) {
            let known_core_ids = self.structure.genome_constituent_ids().to_vec();
            // Existing serialized organisms with core IDs predate the explicit
            // flag, so infer an established identity from those IDs as well.
            let identity_established =
                self.genome_core_identity_established || !known_core_ids.is_empty();
            let cavity = if !identity_established {
                crate::cavity::analyze_genome_cavity(&self.structure, catalog)
            } else if known_core_ids.is_empty() {
                Ok(None)
            } else {
                crate::cavity::analyze_genome_cavity_for_core(
                    &self.structure,
                    catalog,
                    &known_core_ids,
                )
            }
            .ok()
            .flatten()
            .filter(|cavity| cavity.qualifies());
            // Once a physical cavity establishes this organism's core, keep
            // analyzing that core. A breached core must not silently switch to
            // another qualifying cavity in the same aggregate.
            let ids = cavity
                .as_ref()
                .map(|cavity| {
                    cavity
                        .boundary_units
                        .iter()
                        .filter_map(|&index| self.structure.physical_id(index))
                        .collect::<Vec<_>>()
                })
                .unwrap_or(known_core_ids);
            self.structure.set_genome_constituent_ids(ids);
            self.genome_core_identity_established |= cavity.is_some() || identity_established;
            self.cached_cavity = Some(cavity);
            self.cached_cavity_revision = Some(self.structure_revision);
        }
        self.cached_cavity.as_ref().and_then(Option::as_ref)
    }

    pub(crate) fn genome_cavity_cached(
        &mut self,
        catalog: &[BaseResource],
    ) -> Option<crate::cavity::GenomeCavity> {
        self.genome_cavity_cached_ref(catalog).cloned()
    }

    pub(crate) fn developmental_realization_cached(
        &mut self,
        catalog: &[BaseResource],
    ) -> Option<crate::developmental_blueprint::DevelopmentalRealization> {
        let reference = crate::juvenile::confirmed_seed_scale_reference(catalog).ok()?;
        self.developmental_realization_cached_for_reference(catalog, reference)
    }

    pub(crate) fn developmental_realization_cached_for_reference(
        &mut self,
        catalog: &[BaseResource],
        (seed_mass, seed_length): (f64, f64),
    ) -> Option<crate::developmental_blueprint::DevelopmentalRealization> {
        if self.cached_developmental_revision != Some(self.structure_revision) {
            let preferred_length = self
                .genome
                .developmental_blueprint
                .preferred_developmental_length(
                    self.genome.preferred_mass(),
                    seed_mass,
                    seed_length,
                );
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

    /// Test/construction convenience that immediately realizes the supplied
    /// material before placing it in organism storage.
    ///
    /// Unstructured aggregate quantities are expanded into individual physical
    /// objects. Structured material is realized as one intact physical object.
    /// No logical material is ever inserted into storage.
    pub(crate) fn store_material(&mut self, material: Material) -> bool {
        let origin = self
            .occupied_cells
            .first()
            .cloned()
            .unwrap_or(Position { x: 0.0, y: 0.0 });
        let placement = Placement {
            x: origin.x,
            y: origin.y,
            rotation_radians: 0.0,
        };
        let catalog = crate::resources::default_catalog();

        if material.has_internal_structure() {
            if material
                .parts
                .iter()
                .any(|(_, amount)| (*amount - 1.0).abs() > 1e-9)
            {
                return false;
            }
            let part_count = material.parts.len();
            let Some(instance) = crate::physical_material::PhysicalMaterial::realized(
                material,
                vec![placement; part_count],
                &catalog,
            ) else {
                return false;
            };
            return self.stored_material.store_physical_instance(instance);
        }

        let mut instances = Vec::new();
        for (name, amount) in material.parts {
            if !amount.is_finite() || amount < 0.0 || amount.fract().abs() > 1e-9 {
                return false;
            }
            for _ in 0..amount as usize {
                let Some(instance) = crate::physical_material::PhysicalMaterial::realized(
                    Material::free_base(name.clone(), 1.0),
                    vec![placement],
                    &catalog,
                ) else {
                    return false;
                };
                instances.push(instance);
            }
        }

        if instances.is_empty() {
            return false;
        }
        for instance in instances {
            debug_assert!(self.stored_material.store_physical_instance(instance));
        }
        true
    }
    pub(crate) fn structural_mass(&self, catalog: &[BaseResource]) -> f64 {
        self.structure
            .structural_unit_indices(catalog)
            .into_iter()
            .filter_map(|index| self.structure.units.get(index))
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
        let available = self.usable_energy.max(0.0);
        let attempted_payment = available.min(demand).max(0.0);
        let settled_payment = if attempted_payment > 0.0 {
            let tx = EnergyTransaction {
                reason: EnergyReason::Maintenance,
                potential_released: 0.0,
                usable_delta: -attempted_payment,
                structural_delta: 0.0,
                work_expended: 0.0,
                heat_dissipated: attempted_payment,
            };
            if ledger.settle_transaction(&mut self.usable_energy, tx) {
                self.add_transaction_stress(attempted_payment);
                attempted_payment
            } else {
                0.0
            }
        } else {
            0.0
        };

        let deficit = demand - settled_payment;
        if deficit > 0.0 {
            self.maintenance_debt += deficit;

            // Historical debt only becomes active starvation pressure while
            // current maintenance is also going unpaid. Recovering energy
            // stops further starvation damage without retroactively repairing
            // past consequences.
            self.stress += deficit + self.maintenance_debt;
        }
    }
    /// Validate stress bookkeeping without granting stress an independent
    /// authority to break physical bonds. Structural damage is resolved only
    /// through the shared resonance/chemistry mechanism.
    pub(crate) fn validate_stress_state(&self) -> bool {
        !self.stress.is_finite() || !self.stress_threshold.is_finite()
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Environment {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) catalog: Vec<BaseResource>,
    pub(crate) field: ActiveMaterialField,
}
#[derive(Clone)]
pub(crate) struct Simulation {
    pub(crate) tick: u64,
    pub(crate) ticks_per_second: f64,
    pub(crate) running: bool,
    pub(crate) organisms: Vec<Organism>,
    pub(crate) environment: Environment,
    pub(crate) active_transformations: Vec<ActiveTransformation>,
    pub(crate) chemical_reaction_accumulation: std::collections::BTreeMap<String, f64>,
    pub(crate) chemistry_library: crate::chemistry_library::ChemistryLibrary,
    pub(crate) decomposing_bodies: Vec<DecomposingBody>,
    pub(crate) energy_ledger: EnergyLedger,
    pub(crate) next_organism_id: u64,
    pub(crate) next_transformation_id: u64,
    pub(crate) rng: ChaCha8Rng,
    pub(crate) decision_parameters: DecisionParameters,
    /// Deterministic calibration derived from the immutable resource catalog.
    /// Cached once at simulation creation so the tick hot path never rebuilds
    /// the confirmed seed structure.
    pub(crate) seed_scale_reference: Option<(f64, f64)>,
    /// Optional rotating source; absent by default until world geometry and signal bounds are configured.
    pub(crate) world_tone: Option<crate::world_tone::WorldToneRuntimeConfig>,
    /// Precomputed deterministic source samples for short cycles; long cycles fall back to direct sampling.
    pub(crate) world_tone_cycle: Option<crate::world_tone::WorldToneCycle>,
}
pub(crate) const DESIRABILITY_AMOUNT_HALF_SATURATION: f64 = 100.0;
pub(crate) const DESIRABILITY_MAX: f64 = 1.0;

#[cfg(test)]
mod genome_core_identity_tests {
    use super::*;

    #[test]
    fn established_primary_core_does_not_fall_back_to_an_additional_core() {
        let mut simulation = Simulation::new(31, 20.0);
        let catalog = simulation.environment.catalog.clone();
        let organism = &mut simulation.organisms[0];
        let surviving_id = organism
            .structure
            .physical_id(0)
            .expect("initial organism has a physical constituent");

        organism.structure.clear_genome_constituent_ids();
        organism
            .structure
            .add_genome_constituent_group([surviving_id]);
        organism.genome_core_identity_established = true;
        organism.mark_structure_changed();

        assert!(
            organism.genome_cavity_cached_ref(&catalog).is_none(),
            "loss of the primary core must not silently select another core"
        );
        assert_eq!(
            organism.structure.additional_genome_constituent_groups(),
            &[vec![surviving_id]],
            "cache refresh must not erase the independent additional core"
        );
    }
}
