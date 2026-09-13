//! Emergent genome-core construction.
//!
//! This is the orchestration layer between inherited developmental preference
//! and the physical construction solver. It never manufactures geometry: all
//! committed material passes through `construction_realization`.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::construction_realization::realize_material_with_constraints;
use crate::genome_core_constructor::{
    score_candidate, select_candidate_index, CandidateAttachment, CandidateScoreInputs,
    ConstructionCandidate, MutationEffect, DEFAULT_TEMPERATURE,
};
use crate::genome_core_geometry::{measure_largest_cavity, CavityMeasurement};
use crate::genome_core_realization::{RealizedBlueprint, RealizedBlueprintElement};
use crate::resources::{BaseResource, Material};
use crate::structural_blueprint::{BlueprintElement, StructuralBlueprint};
use crate::structure::{OrganismStructure, Placement};

const CANDIDATE_DIRECTIONS: usize = 16;
const CANDIDATE_RADIAL_STEPS: usize = 3;
const POSITION_EPSILON: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq)]
pub struct GenomeCoreConstruction {
    pub structure: OrganismStructure,
    pub mapping: RealizedBlueprint,
    pub cavity: CavityMeasurement,
}

/// Construct only the inherited genome-core material until the physical cavity
/// criterion is satisfied. `core_elements` is treated as developmental intent;
/// the physical core is established exclusively by `genome_core_geometry`.
pub fn construct_genome_core(
    blueprint: &StructuralBlueprint,
    catalog: &[BaseResource],
    rng: &mut ChaCha8Rng,
) -> Result<GenomeCoreConstruction, String> {
    blueprint.validate()?;
    if blueprint.core_elements.is_empty() {
        return Err("blueprint contains no genome-core developmental elements".into());
    }

    let mut structure = OrganismStructure::new();
    let mut mapping = RealizedBlueprint::default();
    let mutation = MutationEffect::sample(rng, 1.0, crate::genome_core_constructor::MAX_MUTATION_SCORE_SHIFT);
    let threshold = measure_largest_cavity(&structure, catalog)?;

    // The first blueprint element is the mandatory developmental anchor. It is
    // always realized first, but does not itself declare that a physical core
    // exists.
    let anchor_index = blueprint.core_elements[0];
    let anchor_element = blueprint
        .elements
        .get(anchor_index)
        .ok_or_else(|| "genome-core anchor references a missing blueprint element".to_string())?;
    let anchor_ids = realize_material_with_constraints(&mut structure, anchor_element, catalog, &[])?;
    mapping.elements.push(RealizedBlueprintElement {
        blueprint_element_index: anchor_index,
        structure_unit_indices: anchor_ids,
    });

    let mut cavity = measure_largest_cavity(&structure, catalog)?;
    if cavity.qualifies {
        return Ok(GenomeCoreConstruction { structure, mapping, cavity });
    }

    let mut growth_history = Vec::<(f64, f64)>::new();
    let mut previous_step_length = None;
    let mut built_core = vec![anchor_index];

    for &element_index in blueprint.core_elements.iter().skip(1) {
        if built_core.contains(&element_index) {
            continue;
        }
        let element = blueprint
            .elements
            .get(element_index)
            .ok_or_else(|| "genome-core references a missing blueprint element".to_string())?;

        let external = connected_realized_groups(blueprint, element_index, &mapping);
        let proposals = candidate_anchor_placements(
            &structure,
            element,
            catalog,
            &external,
        );

        let mut candidates = Vec::<ConstructionCandidate>::new();
        for placement in proposals {
            let mut trial = structure.clone();
            let trial_element = BlueprintElement {
                material: element.material.clone(),
                placement: crate::structural_blueprint::BlueprintPlacement {
                    x: placement.x,
                    y: placement.y,
                    rotation_radians: placement.rotation_radians,
                },
            };

            let Ok(ids) = realize_material_with_constraints(
                &mut trial,
                &trial_element,
                catalog,
                &external,
            ) else {
                continue;
            };
            if ids.is_empty() {
                continue;
            }

            let candidate_cavity = measure_largest_cavity(&trial, catalog)?;
            let candidate_position = centroid(&trial, &ids).unwrap_or((placement.x, placement.y));
            let previous_centroid = centroid(&structure, &last_unit_indices(&structure));
            let (step_length, growth_direction) = match previous_centroid {
                Some(previous) => {
                    let dx = candidate_position.0 - previous.0;
                    let dy = candidate_position.1 - previous.1;
                    (dx.hypot(dy), (dx, dy))
                }
                None => (0.0, (0.0, 0.0)),
            };

            let attachments = collect_attachments(&trial, &ids, &mapping, catalog);
            let matched = matched_connection_count(blueprint, element_index, &mapping);
            let expected = blueprint
                .connections
                .iter()
                .filter(|connection| {
                    connection.element_a == element_index || connection.element_b == element_index
                })
                .count();
            let material = material_inputs(&element.material, catalog);
            let score_input = CandidateScoreInputs {
                candidate_position,
                candidate_orientation: placement.rotation_radians,
                blueprint_position: (element.placement.x, element.placement.y),
                blueprint_orientation: element.placement.rotation_radians,
                blueprint_radius: element.material_radius(catalog),
                expected_blueprint_connections: expected,
                matched_blueprint_connections: matched,
                attachments,
                cavity_area_before: cavity.largest_enclosed_area,
                cavity_area_after: candidate_cavity.largest_enclosed_area,
                cavity_threshold_area: candidate_cavity.threshold_area.max(threshold.threshold_area),
                has_qualifying_cavity: candidate_cavity.qualifies,
                candidate_step_length: step_length,
                reference_step_length: previous_step_length,
                candidate_growth_direction: growth_direction,
                recent_growth_directions: growth_history.clone(),
                material_cohesion: material.0,
                material_internal_bond_strengths: material.1,
            };
            let (scores, base_score, effective_score) = score_candidate(&score_input, mutation);
            candidates.push(ConstructionCandidate {
                blueprint_element_index: element_index,
                material: element.material.clone(),
                placement,
                attachments: score_input.attachments.clone(),
                score_inputs: score_input,
                scores,
                mutation,
                base_score,
                effective_score,
            });
        }

        let Some(selected_index) = select_candidate_index(&candidates, DEFAULT_TEMPERATURE, rng) else {
            return Err(format!(
                "genome-core construction could not produce a physically feasible candidate for blueprint element {element_index}"
            ));
        };
        let selected = &candidates[selected_index];
        let selected_element = BlueprintElement {
            material: selected.material.clone(),
            placement: crate::structural_blueprint::BlueprintPlacement {
                x: selected.placement.x,
                y: selected.placement.y,
                rotation_radians: selected.placement.rotation_radians,
            },
        };
        let ids = realize_material_with_constraints(
            &mut structure,
            &selected_element,
            catalog,
            &external,
        )?;
        let selected_centroid = centroid(&structure, &ids);
        if let Some((x, y)) = selected_centroid {
            if let Some(previous) = centroid(&structure, &last_unit_indices(&structure)) {
                let dx = x - previous.0;
                let dy = y - previous.1;
                let len = dx.hypot(dy);
                if len > POSITION_EPSILON {
                    previous_step_length = Some(len);
                    growth_history.push((dx, dy));
                    if growth_history.len() > 4 {
                        growth_history.remove(0);
                    }
                }
            }
        }
        mapping.elements.push(RealizedBlueprintElement {
            blueprint_element_index: element_index,
            structure_unit_indices: ids,
        });
        built_core.push(element_index);
        cavity = measure_largest_cavity(&structure, catalog)?;
        if cavity.qualifies {
            return Ok(GenomeCoreConstruction { structure, mapping, cavity });
        }
    }

    Err(format!(
        "genome-core construction exhausted its inherited core elements without producing a qualifying cavity (largest {:.6}, threshold {:.6})",
        cavity.largest_enclosed_area, cavity.threshold_area
    ))
}

fn connected_realized_groups(
    blueprint: &StructuralBlueprint,
    element_index: usize,
    mapping: &RealizedBlueprint,
) -> Vec<Vec<usize>> {
    blueprint
        .connections
        .iter()
        .filter_map(|connection| {
            let neighbor = if connection.element_a == element_index {
                connection.element_b
            } else if connection.element_b == element_index {
                connection.element_a
            } else {
                return None;
            };
            mapping.units_for(neighbor).map(|ids| ids.to_vec())
        })
        .collect()
}

fn candidate_anchor_placements(
    structure: &OrganismStructure,
    element: &BlueprintElement,
    catalog: &[BaseResource],
    external: &[Vec<usize>],
) -> Vec<Placement> {
    let mut out = Vec::new();
    push_unique(&mut out, Placement {
        x: element.placement.x,
        y: element.placement.y,
        rotation_radians: element.placement.rotation_radians,
    });

    let candidate_radius = element.material_radius(catalog).max(POSITION_EPSILON);
    for group in external {
        for &index in group {
            let Some(unit) = structure.units.get(index) else { continue };
            let Some(shape) = unit.shape(catalog) else { continue };
            let distance = (candidate_radius + shape.form.bounding_radius()).max(POSITION_EPSILON);
            for step in 0..CANDIDATE_RADIAL_STEPS {
                let radial = distance * (0.95 + step as f64 * 0.05);
                for direction in 0..CANDIDATE_DIRECTIONS {
                    let angle = std::f64::consts::TAU * direction as f64 / CANDIDATE_DIRECTIONS as f64;
                    push_unique(&mut out, Placement {
                        x: unit.placement.x + radial * angle.cos(),
                        y: unit.placement.y + radial * angle.sin(),
                        rotation_radians: element.placement.rotation_radians,
                    });
                }
            }
        }
    }
    out
}

fn push_unique(out: &mut Vec<Placement>, placement: Placement) {
    if !placement.x.is_finite() || !placement.y.is_finite() || !placement.rotation_radians.is_finite() {
        return;
    }
    if out.iter().any(|existing| {
        (existing.x - placement.x).abs() <= POSITION_EPSILON
            && (existing.y - placement.y).abs() <= POSITION_EPSILON
            && (existing.rotation_radians - placement.rotation_radians).abs() <= POSITION_EPSILON
    }) {
        return;
    }
    out.push(placement);
}

fn centroid(structure: &OrganismStructure, ids: &[usize]) -> Option<(f64, f64)> {
    if ids.is_empty() { return None; }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut count = 0.0;
    for &id in ids {
        let unit = structure.units.get(id)?;
        x += unit.placement.x;
        y += unit.placement.y;
        count += 1.0;
    }
    Some((x / count, y / count))
}

fn last_unit_indices(structure: &OrganismStructure) -> Vec<usize> {
    if structure.units.is_empty() { return Vec::new(); }
    vec![structure.units.len() - 1]
}

fn matched_connection_count(
    blueprint: &StructuralBlueprint,
    element_index: usize,
    mapping: &RealizedBlueprint,
) -> usize {
    blueprint
        .connections
        .iter()
        .filter(|connection| {
            let neighbor = if connection.element_a == element_index {
                connection.element_b
            } else if connection.element_b == element_index {
                connection.element_a
            } else {
                return false;
            };
            mapping.units_for(neighbor).is_some()
        })
        .count()
}

fn collect_attachments(
    structure: &OrganismStructure,
    ids: &[usize],
    mapping: &RealizedBlueprint,
    catalog: &[BaseResource],
) -> Vec<CandidateAttachment> {
    let mut out = Vec::new();
    for &new_id in ids {
        for element in &mapping.elements {
            for &old_id in &element.structure_unit_indices {
                for candidate in crate::contact::connection_pair_candidates(structure, new_id, old_id, catalog) {
                    if candidate.distance <= 1.0e-9 {
                        out.push(CandidateAttachment {
                            existing_unit_index: old_id,
                            existing_endpoint_index: endpoint_index(candidate.endpoint_b),
                            candidate_endpoint_index: endpoint_index(candidate.endpoint_a),
                            contact_distance: candidate.distance,
                            normal_alignment: candidate.facing,
                            bond_strength: structure
                                .units
                                .get(new_id)
                                .and_then(|a| a.properties(catalog))
                                .zip(structure.units.get(old_id).and_then(|b| b.properties(catalog)))
                                .map(|(a, b)| crate::combine::bond_strength(a, b))
                                .unwrap_or(0.0),
                        });
                    }
                }
            }
        }
    }
    out
}

fn endpoint_index(endpoint: crate::structure::ConnectionEndpoint) -> usize {
    match endpoint {
        crate::structure::ConnectionEndpoint::Corner { point_index }
        | crate::structure::ConnectionEndpoint::LineEndpoint { point_index } => point_index,
        crate::structure::ConnectionEndpoint::Boundary { .. }
        | crate::structure::ConnectionEndpoint::Fluid { .. } => 0,
    }
}

fn material_inputs(material: &Material, catalog: &[BaseResource]) -> (f64, Vec<f64>) {
    if !material.internal_bonds.is_empty() {
        let strengths = material
            .internal_bonds
            .iter()
            .filter_map(|bond| {
                let a = material.parts.get(bond.part_a)?.0.as_str();
                let b = material.parts.get(bond.part_b)?.0.as_str();
                let pa = catalog.iter().find(|r| r.name == a)?.properties;
                let pb = catalog.iter().find(|r| r.name == b)?.properties;
                Some(crate::combine::bond_strength(&pa, &pb))
            })
            .collect::<Vec<_>>();
        let cohesion = material.weighted_properties(catalog).cohesion;
        return (cohesion, strengths);
    }
    (material.weighted_properties(catalog).cohesion, Vec::new())
}

trait BlueprintMaterialRadius {
    fn material_radius(&self, catalog: &[BaseResource]) -> f64;
}

impl BlueprintMaterialRadius for BlueprintElement {
    fn material_radius(&self, catalog: &[BaseResource]) -> f64 {
        self.material
            .parts
            .iter()
            .filter_map(|(name, _)| catalog.iter().find(|resource| resource.name == *name))
            .map(|resource| resource.shape.form.bounding_radius())
            .fold(0.0, f64::max)
            .max(POSITION_EPSILON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structural_blueprint::BlueprintPlacement;

    #[test]
    fn candidate_generation_includes_inherited_target() {
        let catalog = default_catalog();
        let element = BlueprintElement {
            material: Material::free_base("Carbon", 1.0),
            placement: BlueprintPlacement { x: 3.0, y: 4.0, rotation_radians: 0.0 },
        };
        let placements = candidate_anchor_placements(&OrganismStructure::new(), &element, &catalog, &[]);
        assert!(placements.iter().any(|p| (p.x - 3.0).abs() < POSITION_EPSILON && (p.y - 4.0).abs() < POSITION_EPSILON));
    }
}
