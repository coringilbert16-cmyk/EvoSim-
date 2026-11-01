#![expect(
    dead_code,
    reason = "Staged observation API retained for UI integration"
)]

//! Browser-facing observation contracts derived from simulation truth.
use crate::organism_geometry::OrganismBodyGeometry;
use crate::state::Simulation;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ObservationLevel {
    World,
    Organism,
    Structure,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ObservationContext {
    pub(crate) level: ObservationLevel,
    #[serde(default)]
    pub(crate) organism_ids: Vec<String>,
    #[serde(default)]
    pub(crate) focused_organism_id: Option<String>,
}
impl ObservationContext {
    pub(crate) fn world() -> Self {
        Self {
            level: ObservationLevel::World,
            organism_ids: Vec::new(),
            focused_organism_id: None,
        }
    }
    pub(crate) fn organism(organism_ids: Vec<String>) -> Self {
        Self {
            level: ObservationLevel::Organism,
            organism_ids,
            focused_organism_id: None,
        }
    }
    pub(crate) fn structure(
        organism_ids: Vec<String>,
        focused_organism_id: Option<String>,
    ) -> Self {
        Self {
            level: ObservationLevel::Structure,
            organism_ids,
            focused_organism_id,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ObservationProjection {
    pub(crate) context: ObservationContext,
    pub(crate) payload: ObservationPayload,
}
impl ObservationProjection {
    pub(crate) fn world(payload: WorldObservation) -> Self {
        Self {
            context: ObservationContext::world(),
            payload: ObservationPayload::World(payload),
        }
    }
    pub(crate) fn organism(
        context: ObservationContext,
        payload: OrganismObservation,
    ) -> Option<Self> {
        if context.level != ObservationLevel::Organism {
            return None;
        }
        Some(Self {
            context,
            payload: ObservationPayload::Organism(payload),
        })
    }
    pub(crate) fn structure(
        context: ObservationContext,
        payload: StructureObservation,
    ) -> Option<Self> {
        if context.level != ObservationLevel::Structure {
            return None;
        }
        Some(Self {
            context,
            payload: ObservationPayload::Structure(payload),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldOrganismObservation {
    pub(crate) id: String,
    pub(crate) structure_revision: u64,
    pub(crate) position_revision: u64,
    pub(crate) physical: Option<WorldOrganismPhysicalObservation>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldOrganismPhysicalObservation {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
    pub(crate) silhouette: Vec<OrganismSilhouettePart>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldFieldPartObservation {
    pub(crate) resource: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rotation_radians: f64,
    pub(crate) form: crate::resources::Form,
    /// Derived from the same material oscillator response used by the simulation.
    pub(crate) resonance_frequency_cycles_per_day: f64,
    pub(crate) resonance_amplitude: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldFieldObservation {
    pub(crate) cell_index: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) revision: u64,
    pub(crate) materials: Option<Vec<(String, f64)>>,
    /// Individual realized constituents retain their physical shape and position.
    /// None means unchanged in an incremental observation; Some(empty) means empty.
    #[serde(default)]
    pub(crate) physical_parts: Option<Vec<WorldFieldPartObservation>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldPointObservation {
    pub(crate) x: f64,
    pub(crate) y: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldObservation {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) organisms: Vec<WorldOrganismObservation>,
    pub(crate) field: Vec<WorldFieldObservation>,
    pub(crate) decomposing_bodies: Vec<WorldPointObservation>,
    pub(crate) hotspot: Option<crate::hotspot::RadialHotspot>,
    pub(crate) chemistry_diagnostics: crate::state::EnvironmentalChemistryTickDiagnostics,
    pub(crate) chemistry_events: Vec<crate::state::EnvironmentalChemistryEvent>,
}
fn observe_field_physical_parts(
    cell: &crate::environment::FieldCell,
    catalog: &[crate::resources::BaseResource],
) -> Vec<WorldFieldPartObservation> {
    let mut parts = Vec::new();
    let baselines = crate::resources::ResourceBaselines::from_catalog(catalog);
    for physical in &cell.physical_materials {
        let Some(placements) = physical.placements.as_ref() else {
            continue;
        };
        for (index, (resource, _)) in physical.material.parts.iter().enumerate() {
            let Some(placement) = placements.get(index) else {
                continue;
            };
            let form = physical
                .realized_shapes
                .as_ref()
                .and_then(|shapes| shapes.get(index))
                .or_else(|| {
                    catalog
                        .iter()
                        .find(|entry| entry.name == *resource)
                        .map(|entry| &entry.shape)
                })
                .map(|shape| shape.form.clone());
            let Some(form) = form else {
                continue;
            };
            let resource_definition = catalog.iter().find(|entry| entry.name == *resource);
            let Some(resource_definition) = resource_definition else {
                continue;
            };
            let response =
                crate::harmonics::material_response(resource_definition.properties, baselines, 0.0);
            let natural_frequency = crate::harmonics::natural_frequency_cycles_per_day(
                resource_definition.properties,
                baselines,
            );
            let fundamental = response.components.first();
            parts.push(WorldFieldPartObservation {
                resource: resource.clone(),
                x: placement.x,
                y: placement.y,
                rotation_radians: placement.rotation_radians,
                form,
                resonance_frequency_cycles_per_day: natural_frequency,
                resonance_amplitude: fundamental.map_or(0.0, |component| component.amplitude),
            });
        }
    }
    parts
}

impl WorldObservation {
    pub(crate) fn from_simulation(simulation: &Simulation) -> Self {
        Self::from_simulation_in_bounds(simulation, None, &[], &[])
    }

    pub(crate) fn from_simulation_in_bounds(
        simulation: &Simulation,
        bounds: Option<(f64, f64, f64, f64)>,
        known_organisms: &[(String, u64, u64)],
        known_cells: &[(usize, u64)],
    ) -> Self {
        let catalog = &simulation.environment.catalog;
        let organisms = simulation
            .organisms
            .iter()
            .filter_map(|organism| {
                let position = organism
                    .occupied_cells
                    .first()
                    .map(|p| (p.x, p.y))
                    .unwrap_or((0.0, 0.0));
                let geometry = OrganismBodyGeometry::from_structure(&organism.structure, catalog);
                if let Some((min_x, max_x, min_y, max_y)) = bounds {
                    let visible = match geometry.as_ref() {
                        Some(g) => {
                            g.max_x + position.0 >= min_x
                                && g.min_x + position.0 <= max_x
                                && g.max_y + position.1 >= min_y
                                && g.min_y + position.1 <= max_y
                        }
                        None => {
                            position.0 >= min_x
                                && position.0 <= max_x
                                && position.1 >= min_y
                                && position.1 <= max_y
                        }
                    };
                    if !visible {
                        return None;
                    }
                }
                let physical_changed = known_organisms
                    .iter()
                    .find(|(id, _, _)| id == &organism.id)
                    .map(|(_, structure_revision, position_revision)| {
                        *structure_revision != organism.structure_revision
                            || *position_revision != organism.position_revision
                    })
                    .unwrap_or(true);
                let physical = if physical_changed {
                    Some(match geometry {
                        Some(g) => {
                            let silhouette = g
                                .parts
                                .into_iter()
                                .map(|part| OrganismSilhouettePart {
                                    form: part.form,
                                    x: part.x + position.0,
                                    y: part.y + position.1,
                                    rotation_radians: part.rotation_radians,
                                })
                                .collect();
                            WorldOrganismPhysicalObservation {
                                x: position.0,
                                y: position.1,
                                min_x: g.min_x + position.0,
                                max_x: g.max_x + position.0,
                                min_y: g.min_y + position.1,
                                max_y: g.max_y + position.1,
                                silhouette,
                            }
                        }
                        None => WorldOrganismPhysicalObservation {
                            x: position.0,
                            y: position.1,
                            min_x: position.0,
                            max_x: position.0,
                            min_y: position.1,
                            max_y: position.1,
                            silhouette: Vec::new(),
                        },
                    })
                } else {
                    None
                };
                Some(WorldOrganismObservation {
                    id: organism.id.clone(),
                    structure_revision: organism.structure_revision,
                    position_revision: organism.position_revision,
                    physical,
                })
            })
            .collect();
        let field = if let Some((min_x, max_x, min_y, max_y)) = bounds {
            let field = &simulation.environment.field;
            let half = field.cell_size * 0.5;
            let min_col = ((min_x - half).max(0.0) / field.cell_size).floor() as usize;
            let max_col = ((max_x + half).max(0.0) / field.cell_size)
                .floor()
                .min(field.width_cells.saturating_sub(1) as f64) as usize;
            let min_row = ((min_y - half).max(0.0) / field.cell_size).floor() as usize;
            let max_row = ((max_y + half).max(0.0) / field.cell_size)
                .floor()
                .min(field.height_cells.saturating_sub(1) as f64)
                as usize;
            if min_col > max_col || min_row > max_row {
                Vec::new()
            } else {
                (min_row..=max_row)
                    .flat_map(|row| {
                        (min_col..=max_col).map(move |col| row * field.width_cells + col)
                    })
                    .filter_map(|cell_index| {
                        let cell = &field.cells[cell_index];
                        let revision = field.cell_revisions[cell_index];
                        let known_revision = known_cells
                            .iter()
                            .find(|(known_index, _)| *known_index == cell_index)
                            .map(|(_, revision)| *revision);
                        if known_revision == Some(revision) {
                            let (x, y) = field.cell_center(cell_index);
                            return Some(WorldFieldObservation {
                                cell_index,
                                x,
                                y,
                                revision,
                                materials: None,
                                physical_parts: None,
                            });
                        }
                        let materials = cell.total_material();
                        if materials.is_empty() && known_revision.is_none() {
                            return None;
                        }
                        let (x, y) = field.cell_center(cell_index);
                        Some(WorldFieldObservation {
                            cell_index,
                            x,
                            y,
                            revision,
                            materials: Some(materials),
                            physical_parts: Some(observe_field_physical_parts(cell, catalog)),
                        })
                    })
                    .collect()
            }
        } else {
            let field = &simulation.environment.field;
            field
                .cells
                .iter()
                .enumerate()
                .filter_map(|(cell_index, cell)| {
                    let revision = field.cell_revisions[cell_index];
                    let known_revision = known_cells
                        .iter()
                        .find(|(known_index, _)| *known_index == cell_index)
                        .map(|(_, revision)| *revision);
                    let (x, y) = field.cell_center(cell_index);
                    if known_revision == Some(revision) {
                        return Some(WorldFieldObservation {
                            cell_index,
                            x,
                            y,
                            revision,
                            materials: None,
                            physical_parts: None,
                        });
                    }
                    let materials = cell.total_material();
                    if materials.is_empty() && known_revision.is_none() {
                        return None;
                    }
                    Some(WorldFieldObservation {
                        cell_index,
                        x,
                        y,
                        revision,
                        materials: Some(materials),
                        physical_parts: Some(observe_field_physical_parts(
                            cell,
                            &simulation.environment.catalog,
                        )),
                    })
                })
                .collect()
        };
        let decomposing_bodies = simulation
            .decomposing_bodies
            .iter()
            .filter_map(|b| {
                if let Some((min_x, max_x, min_y, max_y)) = bounds {
                    if b.position.x < min_x
                        || b.position.x > max_x
                        || b.position.y < min_y
                        || b.position.y > max_y
                    {
                        return None;
                    }
                }
                Some(WorldPointObservation {
                    x: b.position.x,
                    y: b.position.y,
                })
            })
            .collect();
        Self {
            width: simulation.environment.width,
            height: simulation.environment.height,
            organisms,
            field,
            decomposing_bodies,
            hotspot: simulation.hotspot,
            chemistry_diagnostics: simulation.environmental_chemistry_diagnostics,
            chemistry_events: simulation.environmental_chemistry_events.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OrganismObservation {
    pub(crate) id: String,
    pub(crate) physical: Option<OrganismPhysicalObservation>,
    pub(crate) developmental_growth_fraction: f64,
    pub(crate) usable_energy: f64,
    pub(crate) stress: f64,
    pub(crate) active_transformation: Option<ActiveTransformationObservation>,
    pub(crate) reproductive_construction_active: bool,
    pub(crate) structure_revision: u64,
    pub(crate) position_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OrganismPhysicalObservation {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
    pub(crate) silhouette: Vec<OrganismSilhouettePart>,
    pub(crate) unit_count: usize,
    pub(crate) bond_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ActiveTransformationObservation {
    pub(crate) id: u64,
    pub(crate) kind: crate::state::TransformationKind,
    pub(crate) remaining_ticks: u64,
    pub(crate) duration_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OrganismSilhouettePart {
    pub(crate) form: crate::resources::Form,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rotation_radians: f64,
}
impl OrganismObservation {
    pub(crate) fn from_simulation(simulation: &Simulation, organism_id: &str) -> Option<Self> {
        Self::from_simulation_with_revisions(simulation, organism_id, None, None)
    }

    pub(crate) fn from_simulation_with_revisions(
        simulation: &Simulation,
        organism_id: &str,
        known_structure_revision: Option<u64>,
        known_position_revision: Option<u64>,
    ) -> Option<Self> {
        let organism = simulation.organisms.iter().find(|o| o.id == organism_id)?;
        let position = organism.occupied_cells.first()?;
        let physical_changed = known_structure_revision != Some(organism.structure_revision)
            || known_position_revision != Some(organism.position_revision);
        let physical = if physical_changed {
            let geometry = OrganismBodyGeometry::from_structure(
                &organism.structure,
                &simulation.environment.catalog,
            )?;
            let silhouette = geometry
                .parts
                .into_iter()
                .map(|part| OrganismSilhouettePart {
                    form: part.form,
                    x: part.x + position.x,
                    y: part.y + position.y,
                    rotation_radians: part.rotation_radians,
                })
                .collect();
            Some(OrganismPhysicalObservation {
                x: position.x,
                y: position.y,
                min_x: geometry.min_x + position.x,
                max_x: geometry.max_x + position.x,
                min_y: geometry.min_y + position.y,
                max_y: geometry.max_y + position.y,
                silhouette,
                unit_count: organism.structure.units.len(),
                bond_count: organism.structure.bonds.len(),
            })
        } else {
            None
        };
        Some(Self {
            id: organism.id.clone(),
            physical,
            developmental_growth_fraction: crate::developmental_decision::growth_fraction(
                organism,
                &simulation.environment,
            ),
            usable_energy: organism.usable_energy,
            stress: organism.stress,
            active_transformation: organism.active_transformation_id.and_then(|id| {
                simulation
                    .active_transformations
                    .iter()
                    .find(|transformation| transformation.id == id)
                    .map(|transformation| ActiveTransformationObservation {
                        id: transformation.id,
                        kind: transformation.kind,
                        remaining_ticks: transformation.remaining_ticks,
                        duration_ticks: transformation.duration_ticks,
                    })
            }),
            reproductive_construction_active: organism.reproductive_construction.is_some(),
            structure_revision: organism.structure_revision,
            position_revision: organism.position_revision,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StructureObservation {
    pub(crate) id: String,
    pub(crate) structure_revision: u64,
    pub(crate) units: Option<Vec<StructureUnitObservation>>,
    pub(crate) bonds: Option<Vec<StructureBondObservation>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StructureUnitObservation {
    pub(crate) unit_index: usize,
    pub(crate) material: crate::resources::Material,
    pub(crate) placement: crate::structure::Placement,
    pub(crate) form: Option<crate::resources::Form>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StructureBondObservation {
    pub(crate) unit_a: usize,
    pub(crate) unit_b: usize,
    pub(crate) strength: f64,
    pub(crate) bond_energy: f64,
    pub(crate) endpoint_a: Option<WorldPointObservation>,
    pub(crate) endpoint_b: Option<WorldPointObservation>,
}
impl StructureObservation {
    pub(crate) fn from_simulation(simulation: &Simulation, organism_id: &str) -> Option<Self> {
        Self::from_simulation_with_revision(simulation, organism_id, None)
    }

    pub(crate) fn from_simulation_with_revision(
        simulation: &Simulation,
        organism_id: &str,
        known_structure_revision: Option<u64>,
    ) -> Option<Self> {
        let organism = simulation.organisms.iter().find(|o| o.id == organism_id)?;
        let structure_changed = known_structure_revision != Some(organism.structure_revision);
        if !structure_changed {
            return Some(Self {
                id: organism.id.clone(),
                structure_revision: organism.structure_revision,
                units: None,
                bonds: None,
            });
        }
        let catalog = &simulation.environment.catalog;
        let units = organism
            .structure
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| StructureUnitObservation {
                unit_index: i,
                material: u.material.clone(),
                placement: u.placement,
                form: u.geometry.as_ref().map(|g| g.shape().form.clone()),
            })
            .collect();
        let bonds = organism
            .structure
            .bonds
            .iter()
            .map(|bond| {
                let ia = organism
                    .structure
                    .unit_index(bond.endpoint_a.constituent_id)?;
                let ib = organism
                    .structure
                    .unit_index(bond.endpoint_b.constituent_id)?;
                let endpoint_a = organism
                    .structure
                    .units
                    .get(ia)
                    .filter(|u| u.geometry.is_some())
                    .and_then(|u| bond.endpoint_a.location.world_point(u, catalog))
                    .map(|p| WorldPointObservation { x: p.x, y: p.y });
                let endpoint_b = organism
                    .structure
                    .units
                    .get(ib)
                    .filter(|u| u.geometry.is_some())
                    .and_then(|u| bond.endpoint_b.location.world_point(u, catalog))
                    .map(|p| WorldPointObservation { x: p.x, y: p.y });
                Some(StructureBondObservation {
                    unit_a: ia,
                    unit_b: ib,
                    strength: bond.strength,
                    bond_energy: bond.bond_energy,
                    endpoint_a,
                    endpoint_b,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            id: organism.id.clone(),
            structure_revision: organism.structure_revision,
            units: Some(units),
            bonds: Some(bonds),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ObservationPayload {
    World(WorldObservation),
    Organism(OrganismObservation),
    Structure(StructureObservation),
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observation_levels_are_semantic_not_zoom_values() {
        assert_ne!(ObservationLevel::World, ObservationLevel::Organism);
        assert_ne!(ObservationLevel::Organism, ObservationLevel::Structure)
    }
    #[test]
    fn environmental_world_observation_exposes_each_physical_resource_shape() {
        let fixture = crate::environment_chemistry_experiment::build(42);
        let observation = WorldObservation::from_simulation(&fixture.simulation);
        let parts = observation
            .field
            .iter()
            .flat_map(|cell| cell.physical_parts.iter().flatten())
            .collect::<Vec<_>>();

        assert_eq!(parts.len(), 1280);
        assert!(parts.iter().all(|part| {
            part.x.is_finite()
                && part.y.is_finite()
                && part.rotation_radians.is_finite()
                && part.form.is_valid()
                && part.resonance_frequency_cycles_per_day.is_finite()
                && part.resonance_frequency_cycles_per_day > 0.0
                && part.resonance_amplitude.is_finite()
                && (0.0..=1.0).contains(&part.resonance_amplitude)
        }));
        let resources = parts
            .iter()
            .map(|part| part.resource.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(resources.len(), 7);
        let resonance_frequencies = parts
            .iter()
            .map(|part| part.resonance_frequency_cycles_per_day.to_bits())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            resonance_frequencies.len() > 1,
            "resource types must expose distinct natural resonance frequencies"
        );
    }

    #[test]
    fn unchanged_full_world_cells_omit_redundant_physical_payloads() {
        let fixture = crate::environment_chemistry_experiment::build(42);
        let full = WorldObservation::from_simulation(&fixture.simulation);
        let known_cells = full
            .field
            .iter()
            .map(|cell| (cell.cell_index, cell.revision))
            .collect::<Vec<_>>();
        let incremental = WorldObservation::from_simulation_in_bounds(
            &fixture.simulation,
            None,
            &[],
            &known_cells,
        );

        assert_eq!(incremental.field.len(), full.field.len());
        assert!(incremental
            .field
            .iter()
            .all(|cell| cell.materials.is_none() && cell.physical_parts.is_none()));
    }

    #[test]
    fn environmental_transport_updates_observed_resource_positions() {
        let mut fixture = crate::environment_chemistry_experiment::build(42);
        let before_observation = WorldObservation::from_simulation(&fixture.simulation);
        let known_cells = before_observation
            .field
            .iter()
            .map(|cell| (cell.cell_index, cell.revision))
            .collect::<Vec<_>>();
        let mut before = before_observation
            .field
            .into_iter()
            .flat_map(|cell| cell.physical_parts.unwrap_or_default())
            .collect::<Vec<_>>();
        before.sort_by(|a, b| {
            a.resource
                .cmp(&b.resource)
                .then_with(|| a.x.total_cmp(&b.x))
                .then_with(|| a.y.total_cmp(&b.y))
        });
        let catalog = fixture.simulation.environment.catalog.clone();
        let moved = crate::environment_flow::transport_field(
            &mut fixture.simulation.environment.field,
            &catalog,
            fixture.simulation.environment.width,
            fixture.simulation.environment.height,
        );
        assert!(
            moved > 0,
            "the experiment's environmental current must transport resources"
        );

        let incremental = WorldObservation::from_simulation_in_bounds(
            &fixture.simulation,
            None,
            &[],
            &known_cells,
        );
        assert!(
            incremental.field.iter().any(|cell| cell
                .physical_parts
                .as_ref()
                .is_some_and(|parts| !parts.is_empty())),
            "changed cells must publish their current physical resource geometry"
        );

        let mut after = WorldObservation::from_simulation(&fixture.simulation)
            .field
            .into_iter()
            .flat_map(|cell| cell.physical_parts.unwrap_or_default())
            .collect::<Vec<_>>();
        after.sort_by(|a, b| {
            a.resource
                .cmp(&b.resource)
                .then_with(|| a.x.total_cmp(&b.x))
                .then_with(|| a.y.total_cmp(&b.y))
        });
        assert_eq!(before.len(), after.len());
        assert!(
            before.iter().zip(&after).any(|(previous, current)| {
                previous.resource == current.resource
                    && (previous.x - current.x).hypot(previous.y - current.y) > 1e-9
            }),
            "the updated world observation must expose moved resource positions"
        );
    }

    #[test]
    fn world_context_contains_no_selection() {
        let context = ObservationContext::world();
        assert_eq!(context.level, ObservationLevel::World);
        assert!(context.organism_ids.is_empty());
        assert!(context.focused_organism_id.is_none())
    }
    #[test]
    fn structure_context_preserves_stable_ids_and_focus() {
        let context =
            ObservationContext::structure(vec!["17".into(), "23".into()], Some("23".into()));
        assert_eq!(context.level, ObservationLevel::Structure);
        assert_eq!(context.organism_ids, vec!["17", "23"]);
        assert_eq!(context.focused_organism_id.as_deref(), Some("23"))
    }
    #[test]
    fn projection_rejects_payload_at_wrong_level() {
        let context = ObservationContext::world();
        let simulation = Simulation::new(1, 20.0);
        let payload = OrganismObservation::from_simulation(&simulation, "1").unwrap();
        assert!(ObservationProjection::organism(context, payload).is_none())
    }
    #[test]
    fn organism_observation_omits_unchanged_physical_state() {
        let simulation = Simulation::new(1, 20.0);
        let organism = simulation.organisms.first().unwrap();
        let observation = OrganismObservation::from_simulation_with_revisions(
            &simulation,
            &organism.id,
            Some(organism.structure_revision),
            Some(organism.position_revision),
        )
        .unwrap();
        assert!(observation.physical.is_none());
    }

    #[test]
    fn structure_observation_omits_unchanged_physical_state() {
        let simulation = Simulation::new(1, 20.0);
        let organism = simulation.organisms.first().unwrap();
        let observation = StructureObservation::from_simulation_with_revision(
            &simulation,
            &organism.id,
            Some(organism.structure_revision),
        )
        .unwrap();
        assert!(observation.units.is_none());
        assert!(observation.bonds.is_none());
    }

    #[test]
    fn structure_observation_preserves_physical_endpoints() {
        let simulation = Simulation::new(1, 20.0);
        let observation = StructureObservation::from_simulation(&simulation, "1").unwrap();
        let bonds = observation
            .bonds
            .expect("full observation must include bonds");
        assert!(!bonds.is_empty());
        assert!(bonds
            .iter()
            .all(|b| b.endpoint_a.is_some() && b.endpoint_b.is_some()))
    }
}
