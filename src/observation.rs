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
pub(crate) struct WorldFieldObservation {
    pub(crate) cell_index: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) materials: Vec<(String, f64)>,
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
                let (min_x, max_x, min_y, max_y, silhouette) = match geometry {
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
                        (
                            g.min_x + position.0,
                            g.max_x + position.0,
                            g.min_y + position.1,
                            g.max_y + position.1,
                            silhouette,
                        )
                    }
                    None => (
                        position.0,
                        position.0,
                        position.1,
                        position.1,
                        Vec::new(),
                    ),
                };
                Some(WorldOrganismObservation {
                    id: organism.id.clone(),
                    x: position.0,
                    y: position.1,
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    silhouette,
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
                .min(field.height_cells.saturating_sub(1) as f64) as usize;
            if min_col > max_col || min_row > max_row {
                Vec::new()
            } else {
                (min_row..=max_row)
                    .flat_map(|row| {
                        (min_col..=max_col).map(move |col| row * field.width_cells + col)
                    })
                    .filter_map(|cell_index| {
                        let cell = &field.cells[cell_index];
                        let materials = cell.total_material();
                        if materials.is_empty() {
                            return None;
                        }
                        let (x, y) = field.cell_center(cell_index);
                        Some(WorldFieldObservation {
                            cell_index,
                            x,
                            y,
                            materials,
                        })
                    })
                    .collect()
            }
        } else {
            simulation
                .environment
                .field
                .cells
                .iter()
                .enumerate()
                .filter_map(|(cell_index, cell)| {
                    let materials = cell.total_material();
                    if materials.is_empty() {
                        return None;
                    }
                    let (x, y) = simulation.environment.field.cell_center(cell_index);
                    Some(WorldFieldObservation {
                        cell_index,
                        x,
                        y,
                        materials,
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
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OrganismObservation {
    pub(crate) id: String,
    pub(crate) physical: Option<OrganismPhysicalObservation>,
    pub(crate) development_stage: crate::state::DevelopmentStage,
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
            development_stage: organism.development_stage.clone(),
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
        assert!(!observation.bonds.is_empty());
        assert!(observation
            .bonds
            .iter()
            .all(|b| b.endpoint_a.is_some() && b.endpoint_b.is_some()))
    }
}
