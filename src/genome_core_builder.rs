//! Emergent genome-core construction orchestration.
//!
//! Blueprint data supplies inherited developmental preference. Physical
//! feasibility and committed geometry remain owned by construction_realization.

use rand_chacha::ChaCha8Rng;

use crate::construction_realization::realize_material_with_constraints;
use crate::genome_core_constructor::{
    score_candidate, select_candidate_index, CandidateAttachment, CandidateScoreInputs,
    ConstructionCandidate, MutationEffect, DEFAULT_TEMPERATURE, MAX_MUTATION_SCORE_SHIFT,
};
use crate::genome_core_geometry::{measure_largest_cavity, CavityMeasurement};
use crate::genome_core_realization::{RealizedBlueprint, RealizedBlueprintElement};
use crate::resources::{BaseResource, Material};
use crate::structural_blueprint::{BlueprintElement, StructuralBlueprint};
use crate::structure::{OrganismStructure, Placement, PhysicalConstituentGraph};

const CANDIDATE_DIRECTIONS: usize = 16;
const CANDIDATE_RADIAL_STEPS: usize = 3;
const POSITION_EPSILON: f64 = 1.0e-9;

impl PartialEq for PhysicalConstituentGraph {
    fn eq(&self, other: &Self) -> bool {
        self.units == other.units && self.bonds == other.bonds
    }
}

#[derive(Clone, Debug)]
pub struct GenomeCoreConstruction {
    pub structure: OrganismStructure,
    pub mapping: RealizedBlueprint,
    pub cavity: CavityMeasurement,
}

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
    let mutation = MutationEffect::sample(rng, 1.0, MAX_MUTATION_SCORE_SHIFT);

    let anchor_index = blueprint.core_elements[0];
    let anchor = blueprint
        .elements
        .get(anchor_index)
        .ok_or_else(|| "genome-core anchor references a missing blueprint element".to_string())?;
    let anchor_ids = realize_material_with_constraints(&mut structure, anchor, catalog, &[])?;
    mapping.elements.push(RealizedBlueprintElement {
        blueprint_element_index: anchor_index,
        structure_unit_indices: anchor_ids,
    });

    let mut cavity = measure_largest_cavity(&structure, catalog)?;
    if cavity.qualifies {
        return Ok(GenomeCoreConstruction { structure, mapping, cavity });
    }

    let mut growth_history = Vec::<(f64, f64)>::new();
    let mut reference_step_length = None;
    let mut previous_centroid = mapping
        .units_for(anchor_index)
        .and_then(|ids| centroid(&structure, ids));

    for &element_index in blueprint.core_elements.iter().skip(1) {
        if mapping.units_for(element_index).is_some() {
            continue;
        }
        let element = blueprint
            .elements
            .get(element_index)
            .ok_or_else(|| "genome-core references a missing blueprint element".to_string())?;
        let external = connected_realized_groups(blueprint, element_index, &mapping);
        let proposals = candidate_anchor_placements(&structure, element, catalog, &external);
        let mut candidates = Vec::<ConstructionCandidate>::new();

        for placement in proposals {
            let mut trial = structure.clone();
            let trial_element = element_at(element, placement);
            let Ok(ids) = realize_material_with_constraints(&mut trial, &trial_element, catalog, &external) else {
                continue;
            };
            if ids.is_empty() {
                continue;
            }

            let candidate_cavity = measure_largest_cavity(&trial, catalog)?;
            let candidate_position = centroid(&trial, &ids).unwrap_or((placement.x, placement.y));
            let (step_length, growth_direction) = match previous_centroid {
                Some(previous) => {
                    let dx = candidate_position.0 - previous.0;
                    let dy = candidate_position.1 - previous.1;
                    (dx.hypot(dy), (dx, dy))
                }
                None => (0.0, (0.0, 0.0)),
            };

            let score_input = CandidateScoreInputs {
                candidate_position,
                candidate_orientation: placement.rotation_radians,
                blueprint_position: (element.placement.x, element.placement.y),
                blueprint_orientation: element.placement.rotation_radians,
                blueprint_radius: material_radius(&element.material, catalog),
                expected_blueprint_connections: connection_count(blueprint, element_index),
                matched_blueprint_connections: matched_connection_count(blueprint, element_index, &mapping),
                attachments: collect_attachments(&trial, &ids, &mapping, catalog),
                cavity_area_before: cavity.largest_enclosed_area,
                cavity_area_after: candidate_cavity.largest_enclosed_area,
                cavity_threshold_area: candidate_cavity.threshold_area,
                has_qualifying_cavity: candidate_cavity.qualifies,
                candidate_step_length: step_length,
                reference_step_length,
                candidate_growth_direction: growth_direction,
                recent_growth_directions: growth_history.clone(),
                material_cohesion: material_cohesion(&element.material, catalog),
                material_internal_bond_strengths: material_bond_strengths(&element.material, catalog),
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
        let selected_element = element_at(element, selected.placement);
        let ids = realize_material_with_constraints(&mut structure, &selected_element, catalog, &external)?;

        if let Some(new_centroid) = centroid(&structure, &ids) {
            if let Some(previous) = previous_centroid {
                let dx = new_centroid.0 - previous.0;
                let dy = new_centroid.1 - previous.1;
                let length = dx.hypot(dy);
                if length > POSITION_EPSILON {
                    reference_step_length = Some(length);
                    growth_history.push((dx, dy));
                    if growth_history.len() > 4 {
                        growth_history.remove(0);
                    }
                }
            }
            previous_centroid = Some(new_centroid);
        }

        mapping.elements.push(RealizedBlueprintElement {
            blueprint_element_index: element_index,
            structure_unit_indices: ids,
        });
        cavity = measure_largest_cavity(&structure, catalog)?;
        if cavity.qualifies {
            return Ok(GenomeCoreConstruction { structure, mapping, cavity });
        }
    }

    Err(format!(
        "genome-core construction exhausted inherited core elements without a qualifying cavity (largest {:.6}, threshold {:.6})",
        cavity.largest_enclosed_area, cavity.threshold_area
    ))
}

fn element_at(element: &BlueprintElement, placement: Placement) -> BlueprintElement {
    BlueprintElement {
        material: element.material.clone(),
        placement: crate::structural_blueprint::BlueprintPlacement {
            x: placement.x,
            y: placement.y,
            rotation_radians: placement.rotation_radians,
        },
    }
}

fn connected_realized_groups(
    blueprint: &StructuralBlueprint,
    element_index: usize,
    mapping: &RealizedBlueprint,
) -> Vec<Vec<usize>> {
    blueprint.connections.iter().filter_map(|connection| {
        let neighbor = if connection.element_a == element_index {
            connection.element_b
        } else if connection.element_b == element_index {
            connection.element_a
        } else {
            return None;
        };
        mapping.units_for(neighbor).map(|ids| ids.to_vec())
    }).collect()
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
    let candidate_radius = material_radius(&element.material, catalog).max(POSITION_EPSILON);
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
    for &id in ids {
        let unit = structure.units.get(id)?;
        x += unit.placement.x;
        y += unit.placement.y;
    }
    let count = ids.len() as f64;
    Some((x / count, y / count))
}

fn connection_count(blueprint: &StructuralBlueprint, element_index: usize) -> usize {
    blueprint.connections.iter().filter(|connection| {
        connection.element_a == element_index || connection.element_b == element_index
    }).count()
}

fn matched_connection_count(
    blueprint: &StructuralBlueprint,
    element_index: usize,
    mapping: &RealizedBlueprint,
) -> usize {
    blueprint.connections.iter().filter(|connection| {
        let neighbor = if connection.element_a == element_index {
            connection.element_b
        } else if connection.element_b == element_index {
            connection.element_a
        } else {
            return false;
        };
        mapping.units_for(neighbor).is_some()
    }).count()
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
                    if candidate.distance <= POSITION_EPSILON {
                        let strength = structure.units.get(new_id)
                            .and_then(|a| a.properties(catalog))
                            .zip(structure.units.get(old_id).and_then(|b| b.properties(catalog)))
                            .map(|(a, b)| crate::combine::bond_strength(a, b))
                            .unwrap_or(0.0);
                        out.push(CandidateAttachment {
                            existing_unit_index: old_id,
                            existing_endpoint_index: endpoint_index(candidate.endpoint_b),
                            candidate_endpoint_index: endpoint_index(candidate.endpoint_a),
                            contact_distance: candidate.distance,
                            normal_alignment: candidate.facing,
                            bond_strength: strength,
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

fn material_radius(material: &Material, catalog: &[BaseResource]) -> f64 {
    material.parts.iter()
        .filter_map(|(name, _)| catalog.iter().find(|resource| resource.name == *name))
        .map(|resource| resource.shape.form.bounding_radius())
        .fold(0.0, f64::max)
        .max(POSITION_EPSILON)
}

fn material_cohesion(material: &Material, catalog: &[BaseResource]) -> f64 {
    material.weighted_properties(catalog).cohesion
}

fn material_bond_strengths(material: &Material, catalog: &[BaseResource]) -> Vec<f64> {
    material.internal_bonds.iter().filter_map(|bond| {
        let a = material.parts.get(bond.part_a)?.0.as_str();
        let b = material.parts.get(bond.part_b)?.0.as_str();
        let pa = catalog.iter().find(|resource| resource.name == a)?.properties;
        let pb = catalog.iter().find(|resource| resource.name == b)?.properties;
        Some(crate::combine::bond_strength(pa, pb))
    }).collect()
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
