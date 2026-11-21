#![expect(
    dead_code,
    reason = "Staged diagnostic API retained for exploratory and UI integration"
)]

use crate::state::Simulation;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Event-oriented diagnostic recorder for exploratory simulation runs.
///
/// This is deliberately separate from the normal headless output and is only
/// active when --diagnostics is supplied. It records enough state around
/// structural changes to explain what transformations actually did without
/// changing simulation behavior.
pub(crate) struct DiagnosticsRecorder {
    writer: BufWriter<File>,
    interval: u64,
    organism_id: String,
    before: Option<SimulationSnapshot>,
    stopped: bool,
    summary_path: String,
    summary: DiagnosticSummary,
}

#[derive(Default)]
struct DiagnosticSummary {
    transformation_starts: u64,
    transformation_completions: u64,
    transformation_types: HashMap<String, u64>,
    structure_changes: u64,
    min_growth_fraction: Option<f64>,
    max_growth_fraction: Option<f64>,
    min_energy: Option<f64>,
    max_energy: Option<f64>,
    min_stress: Option<f64>,
    max_stress: Option<f64>,
    first_transformation_tick: Option<u64>,
    last_transformation_tick: Option<u64>,
}

#[derive(Clone)]
struct SimulationSnapshot {
    tick: u64,
    organisms: HashMap<String, OrganismSnapshot>,
    transformations: HashMap<u64, TransformationSnapshot>,
}

#[derive(Clone)]
struct OrganismSnapshot {
    energy: f64,
    stress: f64,
    stress_threshold: f64,
    structure_revision: u64,
    position_revision: u64,
    developmental_growth_fraction: Option<f64>,
    structural_mass: f64,
    unit_count: usize,
    bond_count: usize,
    component_count: usize,
    stored_material_count: usize,
    stored_material: Value,
    structure: Value,
    genome: Value,
    harmonic_spectrum: Value,
    experience_memory: Value,
    decision_history: Value,
    occupied_cells: Value,
    active_transformation_id: Option<u64>,
    reproductive_construction: Value,
    movement_attempt: Option<crate::state::MovementAttemptDiagnostic>,
}

#[derive(Clone)]
struct TransformationSnapshot {
    id: u64,
    organism_id: String,
    kind: Value,
    bond: Value,
    complexity: f64,
    duration_ticks: u64,
    remaining_ticks: u64,
    decision_context_key: Option<String>,
}

impl DiagnosticsRecorder {
    pub(crate) fn new(
        path: &str,
        interval: u64,
        organism_id: &str,
        _simulation: &mut Simulation,
    ) -> std::io::Result<Self> {
        let file = File::create(path)?;
        let mut recorder = Self {
            writer: BufWriter::new(file),
            summary_path: format!("{path}.summary.txt"),
            interval: interval.max(1),
            organism_id: organism_id.to_string(),
            before: None,
            stopped: false,
            summary: DiagnosticSummary::default(),
        };
        recorder.write_event(json!({
            "event": "diagnostic_header",
            "version": 1,
            "seed": 42,
            "max_detail_interval": recorder.interval,
            "purpose": "Trace the complete life of the initial organism, tick by tick, until death."
        }))?;
        // The first pre-tick state is captured by observe_before.
        Ok(recorder)
    }

    pub(crate) fn observe_before(&mut self, simulation: &mut Simulation) {
        if self.stopped {
            return;
        }
        self.before = Some(self.capture(simulation));
    }

    pub(crate) fn observe_after(&mut self, simulation: &mut Simulation) -> std::io::Result<()> {
        if self.stopped {
            return Ok(());
        }
        let Some(before) = self.before.take() else {
            return Ok(());
        };
        let after = self.capture(simulation);

        self.record_transformation_starts(&before, &after)?;
        self.record_transformation_completions(&before, &after)?;
        self.record_structure_changes(&before, &after)?;
        self.record_movement_attempts(&after)?;

        self.write_tick_snapshot(&before, &after)?;

        if !after.organisms.contains_key(&self.organism_id) {
            self.write_event(json!({
                "event": "organism_died",
                "tick": after.tick,
                "organism_id": self.organism_id,
                "last_state": before.organisms.get(&self.organism_id).map(|o| self.state_json(o))
            }))?;
            self.stopped = true;
        }

        self.flush_if_needed(after.tick)
    }

    pub(crate) fn finish(&mut self, _simulation: &mut Simulation) -> std::io::Result<()> {
        if !self.stopped {
            self.write_event(json!({
                "event": "diagnostic_end",
                "organism_id": self.organism_id,
                "reason": "simulation_finished_before_target_death"
            }))?;
        }
        self.writer.flush()
    }

    fn capture(&self, simulation: &mut Simulation) -> SimulationSnapshot {
        let organisms = simulation
            .organisms
            .iter_mut()
            .filter(|organism| organism.id == self.organism_id)
            .map(|organism| {
                let growth = crate::developmental_decision::growth_fraction(
                    organism,
                    &simulation.environment,
                );
                (
                    organism.id.clone(),
                    OrganismSnapshot {
                        energy: organism.usable_energy,
                        stress: organism.stress,
                        stress_threshold: organism.stress_threshold,
                        structure_revision: organism.structure_revision,
                        position_revision: organism.position_revision,
                        developmental_growth_fraction: growth.is_finite().then_some(growth),
                        structural_mass: organism.structural_mass(&simulation.environment.catalog),
                        unit_count: organism.structure.units.len(),
                        bond_count: organism.structure.bonds.len(),
                        component_count: organism.structure.connected_components().len(),
                        stored_material_count: organism.stored_material.physical_count(),
                        stored_material: serde_json::to_value(&organism.stored_material)
                            .unwrap_or(Value::Null),
                        structure: serde_json::to_value(&organism.structure).unwrap_or(Value::Null),
                        genome: serde_json::to_value(&organism.genome).unwrap_or(Value::Null),
                        harmonic_spectrum: serde_json::to_value(&organism.harmonic_spectrum)
                            .unwrap_or(Value::Null),
                        experience_memory: serde_json::to_value(&organism.experience_memory)
                            .unwrap_or(Value::Null),
                        decision_history: serde_json::to_value(&organism.decision_history)
                            .unwrap_or(Value::Null),
                        occupied_cells: serde_json::to_value(&organism.occupied_cells)
                            .unwrap_or(Value::Null),
                        active_transformation_id: organism.active_transformation_id,
                        reproductive_construction: serde_json::to_value(
                            &organism.reproductive_construction,
                        )
                        .unwrap_or(Value::Null),
                        movement_attempt: organism.last_movement_attempt.clone(),
                    },
                )
            })
            .collect();

        let transformations = simulation
            .active_transformations
            .iter()
            .filter(|transformation| transformation.organism_id == self.organism_id)
            .map(|transformation| {
                (
                    transformation.id,
                    TransformationSnapshot {
                        id: transformation.id,
                        organism_id: transformation.organism_id.clone(),
                        kind: serde_json::to_value(transformation.kind).unwrap_or(Value::Null),
                        bond: serde_json::to_value(transformation.bond).unwrap_or(Value::Null),
                        complexity: transformation.complexity,
                        duration_ticks: transformation.duration_ticks,
                        remaining_ticks: transformation.remaining_ticks,
                        decision_context_key: transformation.decision_context_key.clone(),
                    },
                )
            })
            .collect();

        SimulationSnapshot {
            tick: simulation.tick,
            organisms,
            transformations,
        }
    }

    fn record_transformation_starts(
        &mut self,
        before: &SimulationSnapshot,
        after: &SimulationSnapshot,
    ) -> std::io::Result<()> {
        for transformation in after.transformations.values() {
            if !before.transformations.contains_key(&transformation.id) {
                self.summary.transformation_starts += 1;
                self.summary
                    .first_transformation_tick
                    .get_or_insert(after.tick);
                self.summary.last_transformation_tick = Some(after.tick);
                let kind = serde_json::to_string(&transformation.kind)
                    .unwrap_or_else(|_| "unknown".to_string());
                *self.summary.transformation_types.entry(kind).or_default() += 1;
                let organism = after.organisms.get(&transformation.organism_id);
                self.write_event(json!({
                    "event": "transformation_started",
                    "tick": after.tick,
                    "transformation": {
                        "id": transformation.id,
                        "organism_id": transformation.organism_id,
                        "kind": transformation.kind,
                        "bond": transformation.bond,
                        "complexity": transformation.complexity,
                        "duration_ticks": transformation.duration_ticks,
                        "remaining_ticks": transformation.remaining_ticks,
                        "decision_context_key": transformation.decision_context_key,
                    },
                    "organism_state_at_start": organism.map(|o| self.state_json(o))
                }))?;
            }
        }
        Ok(())
    }

    fn record_transformation_completions(
        &mut self,
        before: &SimulationSnapshot,
        after: &SimulationSnapshot,
    ) -> std::io::Result<()> {
        for transformation in before.transformations.values() {
            if after.transformations.contains_key(&transformation.id) {
                continue;
            }
            self.summary.transformation_completions += 1;
            let before_organism = before.organisms.get(&transformation.organism_id);
            let after_organism = after.organisms.get(&transformation.organism_id);
            let before_bond = before_organism
                .and_then(|o| bond_json_for_transformation(&o.structure, &transformation.bond));

            self.write_event(json!({
                "event": "transformation_completed",
                "tick": after.tick,
                "transformation": {
                    "id": transformation.id,
                    "organism_id": transformation.organism_id,
                    "kind": transformation.kind,
                    "bond": transformation.bond,
                    "complexity": transformation.complexity,
                    "duration_ticks": transformation.duration_ticks,
                    "decision_context_key": transformation.decision_context_key,
                },
                "bond_material_before": before_bond,
                "organism_before": before_organism.map(|o| self.state_json(o)),
                "organism_after": after_organism.map(|o| self.state_json(o)),
                "structural_delta": structural_delta(before_organism, after_organism),
                "energy_delta": scalar_delta(
                    before_organism.map(|o| o.energy),
                    after_organism.map(|o| o.energy),
                ),
                "stress_delta": scalar_delta(
                    before_organism.map(|o| o.stress),
                    after_organism.map(|o| o.stress),
                ),
                "growth_fraction_delta": scalar_delta(
                    before_organism.and_then(|o| o.developmental_growth_fraction),
                    after_organism.and_then(|o| o.developmental_growth_fraction),
                ),
            }))?;
        }
        Ok(())
    }

    fn record_structure_changes(
        &mut self,
        before: &SimulationSnapshot,
        after: &SimulationSnapshot,
    ) -> std::io::Result<()> {
        for (id, after_organism) in &after.organisms {
            let Some(before_organism) = before.organisms.get(id) else {
                continue;
            };
            if before_organism.structure_revision == after_organism.structure_revision {
                continue;
            }
            self.summary.structure_changes += 1;
            self.write_event(json!({
                "event": "structure_changed",
                "tick": after.tick,
                "organism_id": id,
                "structural_delta": structural_delta(Some(before_organism), Some(after_organism)),
                "before": self.state_json(before_organism),
                "after": self.state_json(after_organism),
            }))?;
        }
        Ok(())
    }

    fn record_movement_attempts(&mut self, snapshot: &SimulationSnapshot) -> std::io::Result<()> {
        for (organism_id, organism) in &snapshot.organisms {
            let Some(attempt) = &organism.movement_attempt else {
                continue;
            };
            if attempt.tick != snapshot.tick {
                continue;
            }
            self.write_event(json!({
                "event": "movement_attempt",
                "tick": attempt.tick,
                "organism_id": organism_id,
                "direction": {
                    "x": attempt.direction_x,
                    "y": attempt.direction_y,
                },
                "step": attempt.step,
                "usable_energy": attempt.usable_energy,
                "active_transformation_id": attempt.active_transformation_id,
                "result": attempt.result,
                "old_position": attempt.old_position,
                "new_position": attempt.new_position,
            }))?;
        }
        Ok(())
    }

    fn write_tick_snapshot(
        &mut self,
        before: &SimulationSnapshot,
        after: &SimulationSnapshot,
    ) -> std::io::Result<()> {
        let state = after.organisms.get(&self.organism_id);
        let before_state = before.organisms.get(&self.organism_id);
        self.write_event(json!({
            "event": "tick",
            "tick": after.tick,
            "organism_id": self.organism_id,
            "energy_delta": scalar_delta(
                before_state.map(|o| o.energy),
                state.map(|o| o.energy),
            ),
            "stress_delta": scalar_delta(
                before_state.map(|o| o.stress),
                state.map(|o| o.stress),
            ),
            "growth_fraction_delta": scalar_delta(
                before_state.and_then(|o| o.developmental_growth_fraction),
                state.and_then(|o| o.developmental_growth_fraction),
            ),
            "state": state.map(|o| self.state_json(o)),
            "active_transformations": after.transformations.values().map(|t| json!({
                "id": t.id,
                "organism_id": t.organism_id,
                "kind": t.kind,
                "bond": t.bond,
                "complexity": t.complexity,
                "duration_ticks": t.duration_ticks,
                "remaining_ticks": t.remaining_ticks,
                "decision_context_key": t.decision_context_key,
            })).collect::<Vec<_>>(),
        }))
    }

    fn state_json(&self, state: &OrganismSnapshot) -> Value {
        json!({
            "energy": state.energy,
            "stress": state.stress,
            "stress_threshold": state.stress_threshold,
            "structure_revision": state.structure_revision,
            "position_revision": state.position_revision,
            "developmental_growth_fraction": state.developmental_growth_fraction,
            "structural_mass": state.structural_mass,
            "unit_count": state.unit_count,
            "bond_count": state.bond_count,
            "component_count": state.component_count,
            "stored_material_count": state.stored_material_count,
            "stored_material": state.stored_material,
            "structure": state.structure,
            "genome": state.genome,
            "harmonic_spectrum": state.harmonic_spectrum,
            "experience_memory": state.experience_memory,
            "decision_history": state.decision_history,
            "occupied_cells": state.occupied_cells,
            "active_transformation_id": state.active_transformation_id,
            "reproductive_construction": state.reproductive_construction,
            "movement_attempt": state.movement_attempt,
        })
    }

    fn write_summary_file(&self, _simulation: &mut Simulation) -> std::io::Result<()> {
        let mut file = File::create(&self.summary_path)?;
        writeln!(file, "EVOSIM LIFE DIAGNOSTIC SUMMARY")?;
        writeln!(file, "==============================")?;
        writeln!(file, "organism_id: {}", self.organism_id)?;
        writeln!(file, "stopped_at_death: {}", self.stopped)?;
        writeln!(
            file,
            "transformation_starts: {}",
            self.summary.transformation_starts
        )?;
        writeln!(
            file,
            "transformation_completions: {}",
            self.summary.transformation_completions
        )?;
        writeln!(
            file,
            "first_transformation_tick: {:?}",
            self.summary.first_transformation_tick
        )?;
        writeln!(
            file,
            "last_transformation_tick: {:?}",
            self.summary.last_transformation_tick
        )?;
        writeln!(
            file,
            "structure_changes: {}",
            self.summary.structure_changes
        )?;
        write_range(
            &mut file,
            "growth_fraction",
            self.summary.min_growth_fraction,
            self.summary.max_growth_fraction,
        )?;
        write_range(
            &mut file,
            "energy",
            self.summary.min_energy,
            self.summary.max_energy,
        )?;
        write_range(
            &mut file,
            "stress",
            self.summary.min_stress,
            self.summary.max_stress,
        )?;
        Ok(())
    }

    fn write_event(&mut self, event: Value) -> std::io::Result<()> {
        serde_json::to_writer(&mut self.writer, &event)?;
        self.writer.write_all(b"\n")
    }

    fn flush_if_needed(&mut self, tick: u64) -> std::io::Result<()> {
        if tick % self.interval == 0 {
            self.writer.flush()?;
        }
        Ok(())
    }
}

fn scalar_delta(before: Option<f64>, after: Option<f64>) -> Option<f64> {
    match (before, after) {
        (Some(a), Some(b)) if a.is_finite() && b.is_finite() => Some(b - a),
        _ => None,
    }
}

fn structural_delta(before: Option<&OrganismSnapshot>, after: Option<&OrganismSnapshot>) -> Value {
    json!({
        "unit_count": scalar_usize_delta(
            before.map(|o| o.unit_count),
            after.map(|o| o.unit_count),
        ),
        "bond_count": scalar_usize_delta(
            before.map(|o| o.bond_count),
            after.map(|o| o.bond_count),
        ),
        "component_count": scalar_usize_delta(
            before.map(|o| o.component_count),
            after.map(|o| o.component_count),
        ),
        "structural_mass": scalar_delta(
            before.map(|o| o.structural_mass),
            after.map(|o| o.structural_mass),
        ),
        "structure_revision": match (
            before.map(|o| o.structure_revision),
            after.map(|o| o.structure_revision),
        ) {
            (Some(a), Some(b)) => json!(b as i128 - a as i128),
            _ => Value::Null,
        },
    })
}

fn scalar_usize_delta(before: Option<usize>, after: Option<usize>) -> Option<i64> {
    match (before, after) {
        (Some(a), Some(b)) => Some(b as i64 - a as i64),
        _ => None,
    }
}

fn bond_json_for_transformation(structure: &Value, transformation_bond: &Value) -> Option<Value> {
    let bond = transformation_bond.as_object()?;
    let endpoint_a = bond.get("endpoint_a")?;
    let endpoint_b = bond.get("endpoint_b")?;
    let bonds = structure.get("bonds")?.as_array()?;

    bonds
        .iter()
        .find(|candidate| {
            candidate.get("endpoint_a") == Some(endpoint_a)
                && candidate.get("endpoint_b") == Some(endpoint_b)
                || candidate.get("endpoint_a") == Some(endpoint_b)
                    && candidate.get("endpoint_b") == Some(endpoint_a)
        })
        .cloned()
}

fn update_min_max(min: &mut Option<f64>, max: &mut Option<f64>, value: Option<f64>) {
    let Some(value) = value.filter(|v| v.is_finite()) else {
        return;
    };
    *min = Some(min.map_or(value, |current| current.min(value)));
    *max = Some(max.map_or(value, |current| current.max(value)));
}

fn write_range(
    file: &mut File,
    name: &str,
    min: Option<f64>,
    max: Option<f64>,
) -> std::io::Result<()> {
    writeln!(file, "{name}: {:?} .. {:?}", min, max)
}

/// Machine-readable recorder for the organism-free environmental chemistry run.
/// It records every tick with reaction activity and periodic full inventory
/// snapshots, so the experiment can be evaluated without watching the canvas.
pub(crate) struct EnvironmentalChemistryDiagnosticsRecorder {
    writer: BufWriter<File>,
    interval: u64,
    totals: EnvironmentalChemistryDiagnosticTotals,
    diagnostics_path: String,
    manifest_path: String,
    started_at: Instant,
    started_at_unix_ms: u128,
    manifest_context: Value,
}

#[derive(Default)]
struct EnvironmentalChemistryDiagnosticTotals {
    ticks_observed: u64,
    break_ready_candidates: u64,
    break_scheduled: u64,
    break_committed: u64,
    break_rejected: u64,
    break_failure_reasons: BTreeMap<String, u64>,
    combine_ready_candidates: u64,
    combine_scheduled: u64,
    combine_committed: u64,
    combine_rejected: u64,
    break_discovery: crate::state::EnvironmentalBreakDiscoveryDiagnostics,
    displacement_ready_candidates: u64,
    displacement_scheduled: u64,
    displacement_committed: u64,
    displacement_rejected: u64,
    displacement_participant_sets: BTreeMap<String, u64>,
    displacement_failure_reasons: BTreeMap<String, u64>,
    displacement_discovery: crate::state::EnvironmentalDisplacementDiscoveryDiagnostics,
    displacement_discovery_work_remaining_total: u64,
    displacement_discovery_budget_exhausted_ticks: u64,
}

impl EnvironmentalChemistryDiagnosticsRecorder {
    pub(crate) fn new(
        path: &str,
        interval: u64,
        seed: u64,
        duration_seconds: Option<u64>,
        planned_ticks: Option<u64>,
        simulation: &Simulation,
        pile_sizes: &[usize],
    ) -> std::io::Result<Self> {
        let file = File::create(path)?;
        let initial_material_inventory = environmental_material_inventory(simulation);
        let initial_physical_material_count = physical_material_count(simulation);
        let started_at = Instant::now();
        let started_at_unix_ms = unix_timestamp_ms();
        let mut recorder = Self {
            writer: BufWriter::new(file),
            interval: interval.max(1),
            totals: EnvironmentalChemistryDiagnosticTotals::default(),
            diagnostics_path: path.to_string(),
            manifest_path: format!("{path}.manifest.json"),
            started_at,
            started_at_unix_ms,
            manifest_context: Value::Null,
        };
        let (initial_independent_resource_count, initial_molecule_count, initial_constituent_count) =
            simulation
                .environment
                .field
                .cells
                .iter()
                .flat_map(|cell| &cell.physical_materials)
                .fold(
                    (0_usize, 0_usize, 0_usize),
                    |(resources, molecules, constituents), material| {
                        let part_count = material.material.parts.len();
                        (
                            resources + usize::from(part_count == 1),
                            molecules + usize::from(part_count > 1),
                            constituents + part_count,
                        )
                    },
                );
        recorder.manifest_context = json!({
            "seed": seed,
            "requested_duration_seconds": duration_seconds,
            "planned_ticks": planned_ticks,
            "diagnostic_interval_ticks": recorder.interval,
            "deep_diagnostics_enabled": simulation.record_local_resonance_diagnostics,
            "profile_movement_enabled": std::env::var("EVOSIM_PROFILE_MOVEMENT")
                .map(|value| value.eq_ignore_ascii_case("true")).unwrap_or(false),
            "profile_tick_enabled": std::env::var("EVOSIM_PROFILE_TICK")
                .map(|value| value.eq_ignore_ascii_case("true")).unwrap_or(false),
            "biology_enabled": false,
            "initial_physical_material_count": initial_physical_material_count,
            "initial_independent_resource_count": initial_independent_resource_count,
            "initial_molecule_count": initial_molecule_count,
            "initial_constituent_count": initial_constituent_count,
            "initial_material_inventory": initial_material_inventory,
            "pile_sizes": pile_sizes,
            "hotspot": simulation.hotspot
        });
        recorder.write_event(json!({
            "event": "environmental_chemistry_header",
            "version": 2,
            "seed": seed,
            "tick_zero": 0,
            "requested_duration_seconds": duration_seconds,
            "planned_ticks": planned_ticks,
            "biology_enabled": false,
            "dormant_organism_holder_count": simulation.organisms.len(),
            "initial_physical_material_count": initial_physical_material_count,
            "initial_independent_resource_count": initial_independent_resource_count,
            "initial_molecule_count": initial_molecule_count,
            "initial_constituent_count": initial_constituent_count,
            "initial_material_inventory": initial_material_inventory,
            "excluded_resources": [],
            "pile_sizes": pile_sizes,
            "hotspot": simulation.hotspot,
            "diagnostic_interval": recorder.interval,
            "experiment_goal": "validate chemistry building blocks; no organism emergence criterion"
        }))?;
        for material in simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
        {
            if material.environmental_identity == 0 {
                return Err(std::io::Error::other(
                    "tick-zero environmental material is missing a stable identity",
                ));
            }
            recorder.write_event(json!({
                "event": "molecular_lifecycle",
                "action": "created",
                "tick": 0,
                "molecule_id": material.environmental_identity,
                "parent_ids": [],
                "reaction_id": "initial",
                "reaction": "INITIAL",
                "molecule": material
            }))?;
        }
        recorder.write_snapshot(simulation, "initial")?;
        recorder.writer.flush()?;
        recorder.write_run_manifest(simulation, "running")?;
        Ok(recorder)
    }

    pub(crate) fn mark_started(&mut self) {
        self.started_at = Instant::now();
        self.started_at_unix_ms = unix_timestamp_ms();
    }

    pub(crate) fn observe(&mut self, simulation: &Simulation) -> std::io::Result<()> {
        for sample in &simulation.environmental_local_resonance_samples {
            self.write_event(sample.clone())?;
        }
        for event in &simulation.environmental_chemistry_events {
            for lifecycle in &event.molecular_lifecycle {
                self.write_event(json!(lifecycle))?;
            }
            if event.reaction == "BREAK" {
                if let Some(reason) = &event.failure_reason {
                    *self
                        .totals
                        .break_failure_reasons
                        .entry(reason.clone())
                        .or_default() += 1;
                }
            }
            if event.reaction == "DISPLACEMENT" {
                let participant_key = serde_json::to_string(&event.source_materials)
                    .unwrap_or_else(|_| "unserializable_participants".to_string());
                *self
                    .totals
                    .displacement_participant_sets
                    .entry(participant_key)
                    .or_default() += 1;
                let reason = event
                    .failure_reason
                    .clone()
                    .unwrap_or_else(|| "none".to_string());
                *self
                    .totals
                    .displacement_failure_reasons
                    .entry(reason)
                    .or_default() += 1;
            }
            self.write_event(json!({
                "event": "environmental_reaction",
                "reaction": event
            }))?;
        }
        let diagnostics = simulation.environmental_chemistry_diagnostics;
        self.totals.ticks_observed = self.totals.ticks_observed.saturating_add(1);
        self.totals.break_ready_candidates = self
            .totals
            .break_ready_candidates
            .saturating_add(diagnostics.break_ready_candidates as u64);
        self.totals.break_scheduled = self
            .totals
            .break_scheduled
            .saturating_add(diagnostics.break_scheduled as u64);
        self.totals.break_committed = self
            .totals
            .break_committed
            .saturating_add(diagnostics.break_committed as u64);
        self.totals.break_rejected = self
            .totals
            .break_rejected
            .saturating_add(diagnostics.break_rejected as u64);
        self.totals
            .break_discovery
            .accumulate(diagnostics.break_discovery);
        self.totals.combine_ready_candidates = self
            .totals
            .combine_ready_candidates
            .saturating_add(diagnostics.combine_ready_candidates as u64);
        self.totals.combine_scheduled = self
            .totals
            .combine_scheduled
            .saturating_add(diagnostics.combine_scheduled as u64);
        self.totals.combine_committed = self
            .totals
            .combine_committed
            .saturating_add(diagnostics.combine_committed as u64);
        self.totals.combine_rejected = self
            .totals
            .combine_rejected
            .saturating_add(diagnostics.combine_rejected as u64);
        self.totals.displacement_ready_candidates = self
            .totals
            .displacement_ready_candidates
            .saturating_add(diagnostics.displacement_ready_candidates as u64);
        self.totals.displacement_scheduled = self
            .totals
            .displacement_scheduled
            .saturating_add(diagnostics.displacement_scheduled as u64);
        self.totals.displacement_committed = self
            .totals
            .displacement_committed
            .saturating_add(diagnostics.displacement_committed as u64);
        self.totals.displacement_rejected = self
            .totals
            .displacement_rejected
            .saturating_add(diagnostics.displacement_rejected as u64);
        self.totals
            .displacement_discovery
            .accumulate(diagnostics.displacement_discovery);
        self.totals.displacement_discovery_work_remaining_total = self
            .totals
            .displacement_discovery_work_remaining_total
            .saturating_add(diagnostics.displacement_discovery_work_remaining as u64);
        if diagnostics.displacement_discovery_work_remaining == 0 {
            self.totals.displacement_discovery_budget_exhausted_ticks = self
                .totals
                .displacement_discovery_budget_exhausted_ticks
                .saturating_add(1);
        }

        if diagnostics.break_ready_candidates > 0
            || diagnostics.break_scheduled > 0
            || diagnostics.break_committed > 0
            || diagnostics.break_rejected > 0
            || diagnostics.break_discovery.scan_entries_inspected > 0
            || diagnostics.break_discovery.bond_candidates_inspected > 0
            || diagnostics.break_discovery.activation_updates > 0
            || diagnostics.break_discovery.progress_preserved_unsearched > 0
            || diagnostics.break_discovery.progress_dissipated_checked > 0
            || diagnostics.combine_ready_candidates > 0
            || diagnostics.combine_scheduled > 0
            || diagnostics.combine_committed > 0
            || diagnostics.combine_rejected > 0
            || diagnostics.combine_search
                != crate::state::EnvironmentalCombineSearchDiagnostics::default()
            || diagnostics.displacement_ready_candidates > 0
            || diagnostics.displacement_scheduled > 0
            || diagnostics.displacement_committed > 0
            || diagnostics.displacement_rejected > 0
            || diagnostics.displacement_discovery
                != crate::state::EnvironmentalDisplacementDiscoveryDiagnostics::default()
            || diagnostics.hotspot_settlement_succeeded == Some(false)
        {
            self.write_event(json!({
                "event": "environmental_chemistry_tick",
                "tick": diagnostics.tick,
                "reaction_work": diagnostics,
                "accumulation_entries": diagnostics.accumulation_entries
            }))?;
        }

        if simulation.tick % self.interval == 0 {
            self.write_snapshot(simulation, "periodic")?;
            self.writer.flush()?;
        }
        Ok(())
    }

    pub(crate) fn finish(&mut self, simulation: &Simulation) -> std::io::Result<()> {
        self.write_snapshot(simulation, "final")?;
        self.write_event(json!({
            "event": "environmental_chemistry_summary",
            "ticks_observed": self.totals.ticks_observed,
            "break_ready_candidates": self.totals.break_ready_candidates,
            "break_scheduled": self.totals.break_scheduled,
            "break_committed": self.totals.break_committed,
            "break_rejected": self.totals.break_rejected,
            "break_failure_reasons": self.totals.break_failure_reasons,
            "break_discovery": self.totals.break_discovery,
            "combine_ready_candidates": self.totals.combine_ready_candidates,
            "combine_scheduled": self.totals.combine_scheduled,
            "combine_committed": self.totals.combine_committed,
            "combine_rejected": self.totals.combine_rejected,
            "displacement_ready_candidates": self.totals.displacement_ready_candidates,
            "displacement_scheduled": self.totals.displacement_scheduled,
            "displacement_committed": self.totals.displacement_committed,
            "displacement_rejected": self.totals.displacement_rejected,
            "displacement_participant_sets": self.totals.displacement_participant_sets,
            "displacement_failure_reasons": self.totals.displacement_failure_reasons,
            "displacement_discovery": self.totals.displacement_discovery,
            "displacement_discovery_work_remaining_total": self.totals.displacement_discovery_work_remaining_total,
            "displacement_discovery_budget_exhausted_ticks": self.totals.displacement_discovery_budget_exhausted_ticks,
            "final_material_inventory": environmental_material_inventory(simulation),
            "final_physical_material_count": physical_material_count(simulation),
            "energy_balance_residual": simulation.energy_inventory_snapshot().residual()
        }))?;
        self.writer.flush()?;
        self.write_run_manifest(simulation, "completed")
    }

    fn write_run_manifest(&self, simulation: &Simulation, status: &str) -> std::io::Result<()> {
        let artifact_metadata = std::fs::metadata(&self.diagnostics_path)?;
        let artifact_reference = Path::new(&self.diagnostics_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.diagnostics_path);
        let artifact_sha256 = if status == "completed" {
            let mut artifact = File::open(&self.diagnostics_path)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let bytes_read = artifact.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Some(format!("{:x}", hasher.finalize()))
        } else {
            None
        };
        let configuration_canonical_json = serde_json::to_string(&self.manifest_context)?;
        let configuration_sha256 = format!(
            "{:x}",
            Sha256::digest(configuration_canonical_json.as_bytes())
        );
        let completed_at_unix_ms = unix_timestamp_ms();
        let commit_sha = std::env::var("GITHUB_SHA")
            .or_else(|_| std::env::var("EVOSIM_COMMIT_SHA"))
            .ok();
        let repository = std::env::var("GITHUB_REPOSITORY").ok();
        let workflow_run_id = std::env::var("GITHUB_RUN_ID").ok();
        let workflow_run_attempt = std::env::var("GITHUB_RUN_ATTEMPT").ok();
        let workflow_name = std::env::var("GITHUB_WORKFLOW").ok();
        let mut warnings = Vec::new();
        if commit_sha.is_none() {
            warnings.push(
                "repository commit SHA was not supplied by the execution environment".to_string(),
            );
        }
        if workflow_run_id.is_none() {
            warnings.push(
                "workflow run identity is unavailable outside a configured CI environment"
                    .to_string(),
            );
        }
        if status != "completed" {
            warnings.push("observation did not reach the recorder's completed state".to_string());
        }
        if std::env::var("EVOSIM_EXPERIMENT_ID").is_err() {
            warnings.push("experiment registry identity was not supplied to the run".to_string());
        }
        let workflow_run_url =
            repository
                .as_ref()
                .zip(workflow_run_id.as_ref())
                .map(|(repo, run_id)| {
                    format!(
                        "{}/{}/actions/runs/{}",
                        std::env::var("GITHUB_SERVER_URL")
                            .unwrap_or_else(|_| "https://github.com".to_string()),
                        repo,
                        run_id
                    )
                });
        let enabled_tiers = if simulation.record_local_resonance_diagnostics {
            vec!["core", "standard", "deep"]
        } else {
            vec!["core", "standard"]
        };
        let manifest = json!({
            "schema": "evosim.experiment_manifest",
            "schema_version": 1,
            "experiment": {
                "experiment_id": std::env::var("EVOSIM_EXPERIMENT_ID").ok(),
                "research_plan": std::env::var("EVOSIM_RESEARCH_PLAN").ok(),
                "condition_id": std::env::var("EVOSIM_CONDITION_ID").ok(),
                "seed": self.manifest_context["seed"]
            },
            "run": {
                "run_id": workflow_run_id,
                "run_attempt": workflow_run_attempt,
                "workflow": workflow_name,
                "workflow_url": workflow_run_url,
                "status": status,
                "started_at_unix_ms": self.started_at_unix_ms,
                "completed_at_unix_ms": (status == "completed").then_some(completed_at_unix_ms),
                "requested_duration_seconds": self.manifest_context["requested_duration_seconds"],
                "actual_elapsed_wall_seconds": (status == "completed")
                    .then_some(self.started_at.elapsed().as_secs_f64()),
                "start_tick": 0,
                "end_tick": simulation.tick
            },
            "build": {
                "repository": repository,
                "commit_sha": commit_sha,
                "executable_version": env!("CARGO_PKG_VERSION"),
                "dirty": std::env::var("EVOSIM_BUILD_DIRTY").ok()
                    .and_then(|value| value.parse::<bool>().ok())
            },
            "conditions": {
                "configuration_schema_version": 1,
                "configuration_sha256": configuration_sha256,
                "configuration_canonical_json": configuration_canonical_json,
                "configuration_completeness": "captured_fields_only",
                "configuration": self.manifest_context
            },
            "diagnostics": {
                "raw_schema_versions": { "environmental_chemistry_header": 2 },
                "diagnostic_interval_ticks": self.manifest_context["diagnostic_interval_ticks"],
                "enabled_tiers": enabled_tiers
            },
            "analyzers": {
                "source_commit_sha": commit_sha,
                "version_basis": "same repository commit as the simulation build"
            },
            "artifacts": [{
                "role": "raw_observation",
                "path": artifact_reference,
                "sha256": artifact_sha256,
                "size_bytes": artifact_metadata.len()
            }],
            "quality": {
                "observation_complete": status == "completed",
                "diagnostic_coverage": "reaction_specific",
                "warnings": warnings
            }
        });
        let manifest_file = File::create(&self.manifest_path)?;
        serde_json::to_writer_pretty(manifest_file, &manifest)?;
        Ok(())
    }

    fn write_snapshot(&mut self, simulation: &Simulation, label: &str) -> std::io::Result<()> {
        let inventory = simulation.energy_inventory_snapshot();
        self.write_event(json!({
            "event": "environmental_chemistry_snapshot",
            "label": label,
            "tick": simulation.tick,
            "physical_material_count": physical_material_count(simulation),
            "constituent_count": environmental_constituent_count(simulation),
            "internal_bond_count": environmental_internal_bond_count(simulation),
            "material_inventory": environmental_material_inventory(simulation),
            "chemistry_usable_energy": simulation.environment.chemistry_usable_energy,
            "environment_resonant_energy": simulation.environment.resonant_energy.total_energy(),
            "energy": {
                "initial_usable": inventory.initial_usable_energy,
                "current_usable": inventory.current_usable_energy,
                "initial_resonant": inventory.initial_resonant_energy,
                "current_resonant": inventory.current_resonant_energy,
                "initial_structural": inventory.initial_structural_energy,
                "current_structural": inventory.current_structural_energy,
                "external_input": inventory.external_energy_input,
                "escaped_signal_energy": inventory.world_tone_energy_escaped,
                "work_expended": inventory.work_expended,
                "heat_dissipated": inventory.heat_dissipated,
                "balance_residual": inventory.residual()
            },
            "ledger": {
                "total_external_energy_input": simulation.energy_ledger.total_external_energy_input,
                "total_resonant_energy_held": simulation.energy_ledger.total_resonant_energy_held,
                "total_usable_energy_held": simulation.energy_ledger.total_usable_energy_held,
                "total_structural_energy_held": simulation
                    .energy_ledger
                    .total_structural_energy_held,
                "total_work_expended": simulation.energy_ledger.total_work_expended,
                "total_heat_dissipated": simulation.energy_ledger.total_heat_dissipated,
                "escaped_signal_energy": simulation.energy_ledger.total_world_tone_energy_escaped
            },
            "reaction_work": simulation.environmental_chemistry_diagnostics,
            "hotspot": simulation.hotspot
        }))
    }

    fn write_event(&mut self, event: Value) -> std::io::Result<()> {
        serde_json::to_writer(&mut self.writer, &event)?;
        self.writer.write_all(b"\n")
    }
}

fn unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn physical_material_count(simulation: &Simulation) -> usize {
    simulation
        .environment
        .field
        .cells
        .iter()
        .map(|cell| cell.physical_materials.len())
        .sum()
}

fn environmental_constituent_count(simulation: &Simulation) -> usize {
    simulation
        .environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .map(|material| material.material.parts.len())
        .sum()
}

fn environmental_internal_bond_count(simulation: &Simulation) -> usize {
    simulation
        .environment
        .field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
        .map(|material| material.internal_connections.as_ref().map_or(0, Vec::len))
        .sum()
}

fn environmental_material_inventory(simulation: &Simulation) -> BTreeMap<String, f64> {
    let mut inventory = BTreeMap::new();
    for cell in &simulation.environment.field.cells {
        for material in &cell.materials {
            for (name, amount) in &material.parts {
                *inventory.entry(name.clone()).or_insert(0.0) += *amount;
            }
        }
        for material in &cell.physical_materials {
            for (name, amount) in &material.material.parts {
                *inventory.entry(name.clone()).or_insert(0.0) += *amount;
            }
        }
    }
    inventory
}
