//! Browser-facing observation contracts derived from simulation truth.
//!
//! Observation types deliberately contain no simulation-owned state. They
//! describe what may be observed at a requested resolution and provide the
//! foundation for the World -> Organism -> Structure observation hierarchy.

use serde::{Deserialize, Serialize};

use crate::organism_geometry::OrganismBodyGeometry;
use crate::state::Simulation;

/// Semantic resolution of an observation.
///
/// These are observation levels, not simulation or camera zoom levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ObservationLevel {
    World,
    Organism,
    Structure,
}

/// Context describing what the observer is asking to see.
///
/// Selection and focus remain browser-owned state. This type is deliberately
/// request-oriented so the server can later use it to produce only the detail
/// required by the current observation without making selection simulation
/// state.
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

/// Common envelope for an observation produced from simulation truth.
///
/// The payload is intentionally separate from the context. Context describes
/// the requested scope; the payload is the derived result. Neither is an
/// alternate representation of an organism or environment owned by the
/// simulation.
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

/// Coarse, world-scale representation of one organism.
///
/// The organism ID is copied from simulation truth and remains stable across
/// observation levels. Physical parts come from the authoritative organism
/// geometry; no browser-side resource-name geometry is introduced here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldOrganismObservation {
    pub(crate) id: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
    pub(crate) parts: Vec<WorldOrganismPart>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldOrganismPart {
    pub(crate) form: crate::resources::Form,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rotation_radians: f64,
}

/// Environmental material stock that is spatially known only at field-cell
/// resolution. Individual shapes are not fabricated from aggregate amounts.
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

/// World-scale observation data. It intentionally excludes organism energy,
/// stress, genome, decision history, memory, and other hidden bookkeeping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WorldObservation {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) organisms: Vec<WorldOrganismObservation>,
    pub(crate) field: Vec<WorldFieldObservation>,
    pub(crate) vents: Vec<WorldPointObservation>,
    pub(crate) decomposing_bodies: Vec<WorldPointObservation>,
}

impl WorldObservation {
    pub(crate) fn from_simulation(simulation: &Simulation) -> Self {
        let catalog = &simulation.environment.catalog;
        let organisms = simulation
            .organisms
            .iter()
            .map(|organism| {
                let position = organism
                    .occupied_cells
                    .first()
                    .map(|position| (position.x, position.y))
                    .unwrap_or((0.0, 0.0));
                let geometry = OrganismBodyGeometry::from_structure(&organism.structure, catalog);
                let (min_x, max_x, min_y, max_y, parts) = match geometry {
                    Some(geometry) => (
                        geometry.min_x,
                        geometry.max_x,
                        geometry.min_y,
                        geometry.max_y,
                        geometry
                            .parts
                            .into_iter()
                            .map(|part| WorldOrganismPart {
                                form: part.form,
                                x: part.x,
                                y: part.y,
                                rotation_radians: part.rotation_radians,
                            })
                            .collect(),
                    ),
                    None => (
                        position.0,
                        position.0,
                        position.1,
                        position.1,
                        Vec::new(),
                    ),
                };

                WorldOrganismObservation {
                    id: organism.id.clone(),
                    x: position.0,
                    y: position.1,
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    parts,
                }
            })
            .collect();

        let field = simulation
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
            .collect();

        let vents = simulation
            .environment
            .vents
            .iter()
            .map(|vent| WorldPointObservation {
                x: vent.x,
                y: vent.y,
            })
            .collect();

        let decomposing_bodies = simulation
            .decomposing_bodies
            .iter()
            .map(|body| WorldPointObservation {
                x: body.position.x,
                y: body.position.y,
            })
            .collect();

        Self {
            width: simulation.environment.width,
            height: simulation.environment.height,
            organisms,
            field,
            vents,
            decomposing_bodies,
        }
    }
}

/// Organism-scale observation. It exposes identity, location, extent, and the
/// actual physical silhouette without exposing hidden biological bookkeeping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OrganismObservation {
    pub(crate) id: String,
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
pub(crate) struct OrganismSilhouettePart {
    pub(crate) form: crate::resources::Form,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rotation_radians: f64,
}

impl OrganismObservation {
    pub(crate) fn from_simulation(simulation: &Simulation, organism_id: &str) -> Option<Self> {
        let organism = simulation.organisms.iter().find(|organism| organism.id == organism_id)?;
        let position = organism.occupied_cells.first()?;
        let geometry = OrganismBodyGeometry::from_structure(
            &organism.structure,
            &simulation.environment.catalog,
        )?;
        let min_x = geometry.min_x;
        let max_x = geometry.max_x;
        let min_y = geometry.min_y;
        let max_y = geometry.max_y;
        let silhouette = geometry
            .parts
            .into_iter()
            .map(|part| OrganismSilhouettePart {
                form: part.form,
                x: part.x,
                y: part.y,
                rotation_radians: part.rotation_radians,
            })
            .collect();
        Some(Self {
            id: organism.id.clone(),
            x: position.x,
            y: position.y,
            min_x,
            max_x,
            min_y,
            max_y,
            silhouette,
            unit_count: organism.structure.units.len(),
            bond_count: organism.structure.bonds.len(),
        })
    }
}

/// Exact structural observation of one focused organism.
///
/// Internal constituent placement is intentionally not invented: a structural
/// unit supplies its authoritative external form and placement, while its
/// material retains composition and internal bonds. Bond endpoints are derived
/// from authoritative connection sites when those sites have known positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StructureObservation {
    pub(crate) id: String,
    pub(crate) units: Vec<StructureUnitObservation>,
    pub(crate) bonds: Vec<StructureBondObservation>,
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
    pub(crate) point_a: usize,
    pub(crate) unit_b: usize,
    pub(crate) point_b: usize,
    pub(crate) strength: f64,
    pub(crate) bond_energy: f64,
    pub(crate) endpoint_a: Option<WorldPointObservation>,
    pub(crate) endpoint_b: Option<WorldPointObservation>,
}

impl StructureObservation {
    pub(crate) fn from_simulation(simulation: &Simulation, organism_id: &str) -> Option<Self> {
        let organism = simulation.organisms.iter().find(|organism| organism.id == organism_id)?;
        let catalog = &simulation.environment.catalog;
        let units = organism
            .structure
            .units
            .iter()
            .enumerate()
            .map(|(unit_index, unit)| StructureUnitObservation {
                unit_index,
                material: unit.material.material().clone(),
                placement: unit.placement,
                form: unit.shape(catalog).map(|shape| shape.form.clone()),
            })
            .collect::<Vec<_>>();

        let bonds = organism
            .structure
            .bonds
            .iter()
            .map(|bond| StructureBondObservation {
                unit_a: bond.unit_a,
                point_a: bond.point_a,
                unit_b: bond.unit_b,
                point_b: bond.point_b,
                strength: bond.strength,
                bond_energy: bond.bond_energy,
                endpoint_a: connection_endpoint(&organism.structure, bond.unit_a, bond.point_a, catalog),
                endpoint_b: connection_endpoint(&organism.structure, bond.unit_b, bond.point_b, catalog),
            })
            .collect();

        Some(Self {
            id: organism.id.clone(),
            units,
            bonds,
        })
    }
}

fn connection_endpoint(
    structure: &crate::structure::OrganismStructure,
    unit_index: usize,
    point_index: usize,
    catalog: &[crate::resources::BaseResource],
) -> Option<WorldPointObservation> {
    let unit = structure.units.get(unit_index)?;
    let local = structure.connection_site(
        crate::structure::ConnectionSiteRef {
            unit_index,
            point_index,
        },
        catalog,
    )?;
    let rotation = unit.placement.rotation_radians;
    let (sin, cos) = rotation.sin_cos();
    Some(WorldPointObservation {
        x: unit.placement.x + local.x * cos - local.y * sin,
        y: unit.placement.y + local.x * sin + local.y * cos,
    })
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
        assert_ne!(ObservationLevel::Organism, ObservationLevel::Structure);
    }

    #[test]
    fn world_context_contains_no_selection() {
        let context = ObservationContext::world();
        assert_eq!(context.level, ObservationLevel::World);
        assert!(context.organism_ids.is_empty());
        assert!(context.focused_organism_id.is_none());
    }

    #[test]
    fn structure_context_preserves_stable_ids_and_focus() {
        let context = ObservationContext::structure(
            vec!["17".into(), "23".into()],
            Some("23".into()),
        );
        assert_eq!(context.level, ObservationLevel::Structure);
        assert_eq!(context.organism_ids, vec!["17", "23"]);
        assert_eq!(context.focused_organism_id.as_deref(), Some("23"));
    }

    #[test]
    fn projection_rejects_payload_at_the_wrong_level() {
        let context = ObservationContext::world();
        let simulation = Simulation::new(1, 20.0);
        let payload = OrganismObservation::from_simulation(&simulation, "1").unwrap();
        assert!(ObservationProjection::organism(context, payload).is_none());
    }

    #[test]
    fn world_observation_preserves_authoritative_organism_ids() {
        let simulation = Simulation::new(1, 20.0);
        let observation = WorldObservation::from_simulation(&simulation);
        assert_eq!(observation.organisms.len(), 1);
        assert_eq!(observation.organisms[0].id, "1");
    }

    #[test]
    fn world_observation_uses_actual_forms() {
        let simulation = Simulation::new(1, 20.0);
        let observation = WorldObservation::from_simulation(&simulation);
        assert!(!observation.organisms[0].parts.is_empty());
        assert!(observation.organisms[0]
            .parts
            .iter()
            .all(|part| part.form.is_valid()));
    }

    #[test]
    fn organism_observation_preserves_identity_and_structure_counts() {
        let simulation = Simulation::new(1, 20.0);
        let observation = OrganismObservation::from_simulation(&simulation, "1").unwrap();
        assert_eq!(observation.id, "1");
        assert_eq!(observation.unit_count, simulation.organisms[0].structure.units.len());
        assert_eq!(observation.bond_count, simulation.organisms[0].structure.bonds.len());
        assert!(!observation.silhouette.is_empty());
    }

    #[test]
    fn structure_observation_preserves_material_placement_and_bonds() {
        let simulation = Simulation::new(1, 20.0);
        let observation = StructureObservation::from_simulation(&simulation, "1").unwrap();
        assert_eq!(observation.id, "1");
        assert_eq!(
            observation.units.len(),
            simulation.organisms[0].structure.units.len()
        );
        assert_eq!(
            observation.bonds.len(),
            simulation.organisms[0].structure.bonds.len()
        );
        assert!(observation.units.iter().all(|unit| unit.form.is_some()));
    }

    #[test]
    fn observation_projection_round_trips_through_json() {
        let simulation = Simulation::new(1, 20.0);
        let projection = ObservationProjection::structure(
            ObservationContext::structure(vec!["1".into()], Some("1".into())),
            StructureObservation::from_simulation(&simulation, "1").unwrap(),
        )
        .unwrap();

        let encoded = serde_json::to_string(&projection).unwrap();
        let decoded: ObservationProjection = serde_json::from_str(&encoded).unwrap();
        let reencoded = serde_json::to_string(&decoded).unwrap();
        assert_eq!(encoded, reencoded);
    }
}
