use crate::state::Simulation;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};

/// Event-oriented diagnostic recorder for exploratory simulation runs.
///
/// This is deliberately separate from the normal headless output and is only
/// active when --diagnostics is supplied. It records enough state around
/// structural changes to explain what transformations actually did without
/// changing simulation behavior.
pub(crate) struct DiagnosticsRecorder {
    writer: BufWriter<File>,
    interval: u64,
    before: Option<SimulationSnapshot>,
}

#[derive(Clone)]
struct SimulationSnapshot {
    tick: u64,
    organisms: HashMap<String, OrganismSnapshot>,
    transformations: HashMap<u64, TransformationSnapshot>,
}

#[derive(Clone)]
struct OrganismSnapshot {
    stage: Value,
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
    memory: Value,
    decision_history: Value,
    occupied_cells: Value,
    active_transformation_id: Option<u64>,
    reproductive_construction: Value,
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
        simulation: &mut Simulation,
    ) -> std::io::Result<Self> {
        let file = File::create(path)?;
        let mut recorder = Self {
            writer: BufWriter::new(file),
            interval: interval.max(1),
            before: None,
        };
        recorder.write_event(json!({
            "event": "diagnostic_header",
            "version": 1,
            "seed": 42,
            "max_detail_interval": recorder.interval,
            "purpose": "Explain transformations, structural changes, developmental progress, energy/stress, memory, harmonics, storage, and environmental changes."
        }))?;
        recorder.write_snapshot(simulation, "initial")?;
        Ok(recorder)
    }

    pub(crate) fn observe_before(&mut self, simulation: &mut Simulation) {
        self.before = Some(self.capture(simulation));
    }

    pub(crate) fn observe_after(&mut self, simulation: &mut Simulation) -> std::io::Result<()> {
        let Some(before) = self.before.take() else {
            return Ok(());
        };
        let after = self.capture(simulation);

        self.record_transformation_starts(&before, &after)?;
        self.record_transformation_completions(&before, &after)?;
        self.record_structure_changes(&before, &after)?;
        self.record_lifecycle_changes(&before, &after)?;

        if after.tick % self.interval == 0 {
            self.write_snapshot(simulation, "interval")?;
        }

        self.flush_if_needed(after.tick)
    }

    pub(crate) fn finish(&mut self, simulation: &mut Simulation) -> std::io::Result<()> {
        self.write_snapshot(simulation, "final")?;
        self.write_event(json!({
            "event": "diagnostic_end",
            "tick": simulation.tick,
            "population": simulation.organisms.len(),
            "completed_births": simulation.next_organism_id.saturating_sub(2),
            "active_transformations": simulation.active_transformations.len(),
            "decomposing_bodies": simulation.decomposing_bodies.len(),
            "field_revision": simulation.environment.field.revision
        }))?;
        self.writer.flush()
    }

    fn capture(&self, simulation: &mut Simulation) -> SimulationSnapshot {
        let organisms = simulation
            .organisms
            .iter_mut()
            .map(|organism| {
                let growth = crate::developmental_decision::growth_fraction(
                    organism,
                    &simulation.environment,
                );
                (
                    organism.id.clone(),
                    OrganismSnapshot {
                        stage: serde_json::to_value(&organism.development_stage)
                            .unwrap_or(Value::Null),
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
                        structure: serde_json::to_value(&organism.structure)
                            .unwrap_or(Value::Null),
                        genome: serde_json::to_value(&organism.genome).unwrap_or(Value::Null),
                        harmonic_spectrum: serde_json::to_value(&organism.harmonic_spectrum)
                            .unwrap_or(Value::Null),
                        memory: serde_json::to_value(&organism.memory).unwrap_or(Value::Null),
                        decision_history: serde_json::to_value(&organism.decision_history)
                            .unwrap_or(Value::Null),
                        occupied_cells: serde_json::to_value(&organism.occupied_cells)
                            .unwrap_or(Value::Null),
                        active_transformation_id: organism.active_transformation_id,
                        reproductive_construction: serde_json::to_value(
                            &organism.reproductive_construction,
                        )
                        .unwrap_or(Value::Null),
                    },
                )
            })
            .collect();

        let transformations = simulation
            .active_transformations
            .iter()
            .map(|transformation| {
                (
                    transformation.id,
                    TransformationSnapshot {
                        id: transformation.id,
                        organism_id: transformation.organism_id.clone(),
                        kind: serde_json::to_value(transformation.kind)
                            .unwrap_or(Value::Null),
                        bond: serde_json::to_value(transformation.bond)
                            .unwrap_or(Value::Null),
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
                "energy_delta": scalar_delta(before_organism.map(|o| o.energy), after_organism.map(|o| o.energy)),
                "stress_delta": scalar_delta(before_organism.map(|o| o.stress), after_organism.map(|o| o.stress)),
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

    fn record_lifecycle_changes(
        &mut self,
        before: &SimulationSnapshot,
        after: &SimulationSnapshot,
    ) -> std::io::Result<()> {
        let ids: HashSet<_> = before
            .organisms
            .keys()
            .chain(after.organisms.keys())
            .cloned()
            .collect();

        for id in ids {
            let old = before.organisms.get(&id).map(|o| o.stage.clone());
            let new = after.organisms.get(&id).map(|o| o.stage.clone());
            if old != new {
                self.write_event(json!({
                    "event": "lifecycle_change",
                    "tick": after.tick,
                    "organism_id": id,
                    "from": old,
                    "to": new,
                    "growth_fraction": after.organisms.get(&id)
                        .and_then(|o| o.developmental_growth_fraction),
                    "structural_delta": structural_delta(
                        before.organisms.get(&id),
                        after.organisms.get(&id),
                    ),
                }))?;
            }
        }
        Ok(())
    }

    fn write_snapshot(&mut self, simulation: &mut Simulation, reason: &str) -> std::io::Result<()> {
        let snapshot = self.capture(simulation);
        let total_field_mass: f64 = simulation
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.materials.iter())
            .map(|material| material.mass)
            .sum();
        let total_stored_mass: f64 = simulation
            .organisms
            .iter()
            .flat_map(|organism| organism.stored_material.entries.iter())
            .map(|entry| match entry {
                crate::material_storage::StoredMaterial::Logical(material)
                | crate::material_storage::StoredMaterial::Physical(material) => material.mass(),
            })
            .sum();
        self.write_event(json!({
            "event": "snapshot",
            "reason": reason,
            "tick": simulation.tick,
            "population": simulation.organisms.len(),
            "active_transformations": simulation.active_transformations.len(),
            "decomposing_bodies": simulation.decomposing_bodies.len(),
            "field_revision": simulation.environment.field.revision,
            "field_total_mass": total_field_mass,
            "stored_total_mass": total_stored_mass,
            "energy_ledger": simulation.energy_ledger,
            "organisms": snapshot.organisms.into_iter().map(|(id, organism)| {
                (id, self.state_json(&organism))
            }).collect::<HashMap<_, _>>()
        }))
    }

    fn state_json(&self, state: &OrganismSnapshot) -> Value {
        json!({
            "stage": state.stage,
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
            "memory": state.memory,
            "decision_history": state.decision_history,
            "occupied_cells": state.occupied_cells,
            "active_transformation_id": state.active_transformation_id,
            "reproductive_construction": state.reproductive_construction,
        })
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

fn structural_delta(
    before: Option<&OrganismSnapshot>,
    after: Option<&OrganismSnapshot>,
) -> Value {
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

    bonds.iter().find(|candidate| {
        candidate.get("endpoint_a") == Some(endpoint_a)
            && candidate.get("endpoint_b") == Some(endpoint_b)
            || candidate.get("endpoint_a") == Some(endpoint_b)
                && candidate.get("endpoint_b") == Some(endpoint_a)
    }).cloned()
}

