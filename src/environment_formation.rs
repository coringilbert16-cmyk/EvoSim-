//! Deterministic coarse-grained physical formation realization.
//!
//! A formation is not an authored molecule or terrain type. Its local
//! composition is realized as a small physical pattern using the existing
//! resource geometry, placement, contact, and bond-validation rules. The
//! pattern can then be repeated by a formation representation without
//! instantiating its entire bulk.

use serde::{Deserialize, Serialize};

use crate::combine::bond_strength;
use crate::contact::connection_pair_candidates;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{BaseResource, InternalBond, Material};
use crate::state::Organism;
use crate::structure::{Bond, BondEndpoint, OrganismStructure, Placement, StructuralUnit};

/// Initial experimental pattern period. This is a representation parameter,
/// not a biological constant or a resource property.
pub(crate) const PATTERN_SIDE: usize = 4;
pub(crate) const PATTERN_SIZE: usize = PATTERN_SIDE * PATTERN_SIDE;

/// Resolved formation depth is twice the world's largest realized organism extent.
pub(crate) const FORMATION_RESOLUTION_EXTENT_MULTIPLIER: f64 = 2.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct FormationBulk {
    pub(crate) composition: Vec<(String, f64)>,
    pub(crate) resolved_depth: f64,
}

impl FormationBulk {
    pub(crate) fn new(composition: Vec<(String, f64)>, resolved_depth: f64) -> Option<Self> {
        if !resolved_depth.is_finite() || resolved_depth <= 0.0 || composition.is_empty() {
            return None;
        }
        if composition
            .iter()
            .any(|(_, amount)| !amount.is_finite() || *amount <= 0.0)
        {
            return None;
        }
        Some(Self {
            composition,
            resolved_depth,
        })
    }

    pub(crate) fn total_amount(&self) -> f64 {
        self.composition.iter().map(|(_, amount)| *amount).sum()
    }

    pub(crate) fn remove_composition(&mut self, removed: &[(String, f64)]) -> bool {
        let mut next = self.composition.clone();
        for (name, amount) in removed {
            let Some((_, available)) = next.iter_mut().find(|(candidate, _)| candidate == name)
            else {
                return false;
            };
            if !amount.is_finite() || *amount <= 0.0 || *available + f64::EPSILON < *amount {
                return false;
            }
            *available -= *amount;
        }
        next.retain(|(_, amount)| *amount > f64::EPSILON);
        self.composition = next;
        true
    }
}

/// A deterministic local pattern that can be repeated through a continuous
/// formation. Coordinates are pattern-local; the owning formation supplies
/// world position and repetition.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Formation {
    pub(crate) bulk: FormationBulk,
    pub(crate) pattern: FormationPattern,
    /// Already-resolved physical material belonging to this same formation.
    /// This is a frontier representation, not additional material.
    #[allow(dead_code)]
    pub(crate) resolved_frontier: Vec<PhysicalMaterial>,
    pub(crate) origin: (f64, f64),
    /// Maximum radial distance currently represented by the resolved frontier.
    #[serde(default)]
    pub(crate) resolved_radius: f64,
}

impl Formation {
    pub(crate) fn new(
        composition: Vec<(String, f64)>,
        resolved_depth: f64,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        let bulk = FormationBulk::new(composition, resolved_depth)?;
        let pattern = realize_repeating_pattern(&bulk.composition, catalog)?;
        Some(Self {
            bulk,
            pattern,
            resolved_frontier: Vec::new(),
            origin: (0.0, 0.0),
            resolved_radius: 0.0,
        })
    }

    pub(crate) fn total_amount(&self) -> f64 {
        self.bulk.total_amount()
    }

    pub(crate) fn resolved_depth(&self) -> f64 {
        self.bulk.resolved_depth
    }

    /// Removes material from the formation's backing quantity. The resolved
    /// frontier is a representation of this same quantity, so its physical
    /// separation must be performed before the backing quantity is reduced.
    pub(crate) fn consume(&mut self, removed: &[(String, f64)]) -> bool {
        self.bulk.remove_composition(removed)
    }

    pub(crate) fn resolved_amount(&self) -> f64 {
        self.resolved_frontier
            .iter()
            .map(|material| material.material.total_amount())
            .sum()
    }

    pub(crate) fn set_origin(&mut self, origin: (f64, f64)) {
        self.origin = origin;
    }

    pub(crate) fn resolve_pattern_instance(
        &self,
        pattern_x: i64,
        pattern_y: i64,
    ) -> Option<PhysicalMaterial> {
        let mut material = self
            .pattern
            .repeated_local_placement(pattern_x, pattern_y)?;
        let placements = material.placements.as_mut()?;
        for placement in placements {
            placement.x += self.origin.0;
            placement.y += self.origin.1;
        }
        Some(material)
    }

    pub(crate) fn resolve_frontier(&mut self) {
        if !self.resolved_frontier.is_empty()
            || self.pattern.width <= 0.0
            || self.pattern.height <= 0.0
        {
            return;
        }
        let depth = self.bulk.resolved_depth;
        let radius = depth * 0.5;
        self.resolve_blob_layer(radius);
        self.resolved_radius = radius;
    }

    fn resolve_blob_layer(&mut self, radius: f64) {
        let inner_radius = self.resolved_radius;
        let span_x = (radius / self.pattern.width).ceil() as i64;
        let span_y = (radius / self.pattern.height).ceil() as i64;
        for pattern_y in -span_y..=span_y {
            for pattern_x in -span_x..=span_x {
                let center_x = pattern_x as f64 * self.pattern.width;
                let center_y = pattern_y as f64 * self.pattern.height;
                let distance = (center_x * center_x + center_y * center_y).sqrt();
                let boundary = radius * blob_radius_factor(pattern_x, pattern_y);
                if distance > boundary || distance <= inner_radius {
                    continue;
                }
                if let Some(material) = self.resolve_pattern_instance(pattern_x, pattern_y) {
                    self.resolved_frontier.push(material);
                }
            }
        }
    }

    /// Promotes the next deterministic layer from the aggregate side of the
    /// same formation. No material is spawned at the consumed location.
    pub(crate) fn promote_frontier(&mut self) {
        if self.resolved_frontier.is_empty()
            || self.pattern.width <= 0.0
            || self.pattern.height <= 0.0
        {
            return;
        }
        let next_radius = self.resolved_radius + self.pattern.width.max(self.pattern.height);
        self.resolve_blob_layer(next_radius);
        self.resolved_radius = next_radius;
    }

    pub(crate) fn add_resolved_instance(&mut self, material: PhysicalMaterial) {
        self.resolved_frontier.push(material);
    }

    pub(crate) fn remove_resolved_instance(&mut self, index: usize) -> Option<PhysicalMaterial> {
        (index < self.resolved_frontier.len()).then(|| self.resolved_frontier.remove(index))
    }

    pub(crate) fn resolved_frontier(&self) -> &[PhysicalMaterial] {
        &self.resolved_frontier
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct FormationPattern {
    pub(crate) material: PhysicalMaterial,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl FormationPattern {
    pub(crate) fn repeated_local_placement(
        &self,
        pattern_x: i64,
        pattern_y: i64,
    ) -> Option<PhysicalMaterial> {
        let placements = self.material.placements.as_ref()?;
        let dx = pattern_x as f64 * self.width;
        let dy = pattern_y as f64 * self.height;
        let placements = placements
            .iter()
            .map(|placement| Placement {
                x: placement.x + dx,
                y: placement.y + dy,
                rotation_radians: placement.rotation_radians,
            })
            .collect();
        Some(PhysicalMaterial {
            material: self.material.material.clone(),
            placements: Some(placements),
            internal_connections: self.material.internal_connections.clone(),
        })
    }
}

fn blob_radius_factor(pattern_x: i64, pattern_y: i64) -> f64 {
    let mut value = (pattern_x as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (pattern_y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58476D1CE4E5B9);
    value ^= value >> 27;
    let unit = (value as f64) / (u64::MAX as f64);
    0.88 + unit * 0.24
}

/// Returns the largest realized linear extent among the world's organisms.
pub(crate) fn largest_organism_extent(
    organisms: &[Organism],
    catalog: &[BaseResource],
) -> Option<f64> {
    organisms
        .iter()
        .filter_map(|organism| {
            crate::organism_geometry::OrganismBodyGeometry::from_structure(
                &organism.structure,
                catalog,
            )
            .map(|body| body.maximum_extent())
        })
        .filter(|extent| extent.is_finite() && *extent > 0.0)
        .max_by(f64::total_cmp)
}

/// Returns the world-relative depth for individually resolved formation material.
pub(crate) fn resolved_formation_depth(
    organisms: &[Organism],
    catalog: &[BaseResource],
) -> Option<f64> {
    largest_organism_extent(organisms, catalog)
        .map(|extent| extent * FORMATION_RESOLUTION_EXTENT_MULTIPLIER)
}

/// Deterministically converts a local composition into a finite repeated
/// physical pattern. The composition is expressed as resource quantities;
/// the pattern contains one resolved unit per slot.
pub(crate) fn realize_pattern(
    composition: &[(String, f64)],
    catalog: &[BaseResource],
) -> Option<PhysicalMaterial> {
    let composition = crate::resources::merge_parts(
        &composition
            .iter()
            .filter(|(_, amount)| amount.is_finite() && *amount > 0.0)
            .cloned()
            .collect::<Vec<_>>(),
    );
    let composition = composition.iter().collect::<Vec<_>>();
    if composition.is_empty() || catalog.is_empty() {
        return None;
    }

    let total = composition.iter().map(|(_, amount)| *amount).sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return None;
    }

    let names = balanced_pattern_names(&composition, total);
    let mut structure = OrganismStructure::new();
    let mut bonds = Vec::with_capacity(PATTERN_SIZE.saturating_sub(1));

    for (index, name) in names.iter().enumerate() {
        let resource = catalog.iter().find(|resource| resource.name == *name)?;
        let placement = if index == 0 {
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }
        } else {
            let target = index - 1;
            let anchor = structure.units[target].placement;
            let mut selected = None;

            for candidate_placement in crate::construction_runtime::candidate_placements(
                &structure,
                resource,
                anchor,
                &[target],
                catalog,
            ) {
                let mut candidate_structure = structure.clone();
                let mut unit = StructuralUnit::from_material(
                    Material::free_base(name.clone(), 1.0),
                    candidate_placement,
                )?;
                if !unit.realize_default_geometry(catalog) {
                    continue;
                }
                let candidate_index = candidate_structure.add_unit(unit);
                let Some(candidate) = connection_pair_candidates(
                    &candidate_structure,
                    target,
                    candidate_index,
                    catalog,
                )
                .into_iter()
                .find(|candidate| candidate.available_a && candidate.available_b) else {
                    continue;
                };

                let id_a = candidate_structure.physical_id(target)?;
                let id_b = candidate_structure.physical_id(candidate_index)?;
                let properties_a = candidate_structure.units[target].properties(catalog)?;
                let properties_b =
                    candidate_structure.units[candidate_index].properties(catalog)?;
                let bond = Bond {
                    endpoint_a: BondEndpoint::new(id_a, candidate.endpoint_a),
                    endpoint_b: BondEndpoint::new(id_b, candidate.endpoint_b),
                    strength: bond_strength(properties_a, properties_b),
                    bond_energy: 0.0,
                };
                if !candidate_structure.is_valid_bond(&bond, catalog) {
                    continue;
                }

                candidate_structure.push_bond_unchecked(bond);
                selected = Some((
                    candidate_structure,
                    InternalBond {
                        part_a: target,
                        part_b: candidate_index,
                    },
                ));
                break;
            }

            if let Some((candidate_structure, internal_bond)) = selected {
                structure = candidate_structure;
                bonds.push(internal_bond);
                continue;
            }

            // The local composition is not required to imply chemical bonding.
            // If the existing placement machinery can realize the unit but no
            // valid connection exists, retain the unit without inventing one.
            let fallback = crate::construction_runtime::candidate_placements(
                &structure,
                resource,
                anchor,
                &[target],
                catalog,
            )
            .into_iter()
            .find_map(|candidate_placement| {
                let mut candidate_structure = structure.clone();
                let mut unit = StructuralUnit::from_material(
                    Material::free_base(name.clone(), 1.0),
                    candidate_placement,
                )?;
                if !unit.realize_default_geometry(catalog) {
                    return None;
                }
                candidate_structure.add_unit(unit);
                Some(candidate_structure)
            })?;
            structure = fallback;
            continue;
        };

        let mut unit =
            StructuralUnit::from_material(Material::free_base(name.clone(), 1.0), placement)?;
        if !unit.realize_default_geometry(catalog) {
            return None;
        }
        structure.add_unit(unit);
    }

    let parts = names
        .into_iter()
        .map(|name| (name, 1.0))
        .collect::<Vec<_>>();
    let material = Material {
        parts,
        internal_bonds: bonds,
    };
    let placements = structure.units.iter().map(|unit| unit.placement).collect();
    PhysicalMaterial::realized(material, placements, catalog)
}

pub(crate) fn realize_repeating_pattern(
    composition: &[(String, f64)],
    catalog: &[BaseResource],
) -> Option<FormationPattern> {
    let material = realize_pattern(composition, catalog)?;
    let placements = material.placements.as_ref()?;
    let min_x = placements.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = placements
        .iter()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = placements.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = placements
        .iter()
        .map(|p| p.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let width = (max_x - min_x).max(f64::EPSILON);
    let height = (max_y - min_y).max(f64::EPSILON);
    Some(FormationPattern {
        material,
        width,
        height,
    })
}

fn balanced_pattern_names(composition: &[&(String, f64)], total: f64) -> Vec<String> {
    let mut assigned = vec![0.0; composition.len()];
    let mut names = Vec::with_capacity(PATTERN_SIZE);

    for _ in 0..PATTERN_SIZE {
        let best = (0..composition.len())
            .max_by(|&a, &b| {
                let desired_a = composition[a].1 / total;
                let desired_b = composition[b].1 / total;
                let score_a = desired_a - assigned[a] / PATTERN_SIZE as f64;
                let score_b = desired_b - assigned[b] / PATTERN_SIZE as f64;
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| composition[b].0.cmp(&composition[a].0))
            })
            .unwrap_or(0);
        assigned[best] += 1.0;
        names.push(composition[best].0.clone());
    }

    names
}

#[cfg(test)]
mod tests {
    use super::{
        largest_organism_extent, realize_pattern, realize_repeating_pattern,
        resolved_formation_depth, Formation, FormationBulk, FORMATION_RESOLUTION_EXTENT_MULTIPLIER,
        PATTERN_SIZE,
    };
    use crate::resources::default_catalog;

    #[test]
    fn largest_organism_extent_uses_realized_structure() {
        let catalog = default_catalog();
        let organism = crate::state::Simulation::create_initial_organism();
        let extent = largest_organism_extent(&[organism.clone()], &catalog).unwrap();
        assert!(extent.is_finite() && extent > 0.0);
        assert_eq!(
            resolved_formation_depth(&[organism], &catalog).unwrap(),
            extent * FORMATION_RESOLUTION_EXTENT_MULTIPLIER
        );
    }

    #[test]
    fn formation_bulk_tracks_quantity_without_creating_resource_types() {
        let mut bulk = FormationBulk::new(
            vec![("Carbon".to_string(), 60.0), ("Hydrogen".to_string(), 25.0)],
            12.0,
        )
        .unwrap();
        assert_eq!(bulk.total_amount(), 85.0);
        assert!(bulk.remove_composition(&[("Carbon".to_string(), 1.0)]));
        assert_eq!(bulk.total_amount(), 84.0);
        assert!(!bulk.remove_composition(&[("Nitrogen".to_string(), 1.0)]));
    }

    fn mixed_composition_realizes_a_deterministic_pattern() {
        let catalog = default_catalog();
        let composition = vec![
            ("Carbon".to_string(), 60.0),
            ("Hydrogen".to_string(), 25.0),
            ("Methane".to_string(), 10.0),
            ("Sulfur".to_string(), 5.0),
        ];

        let first = realize_pattern(&composition, &catalog).unwrap();
        let second = realize_pattern(&composition, &catalog).unwrap();

        assert!(first.is_realized());
        assert_eq!(first, second);
        assert_eq!(first.placements.as_ref().unwrap().len(), PATTERN_SIZE);
        assert!(!first.material.internal_bonds.is_empty());
    }

    #[test]
    fn deterministic_pattern_can_be_repeated_without_rebuilding_its_geometry() {
        let catalog = default_catalog();
        let composition = vec![
            ("Carbon".to_string(), 60.0),
            ("Hydrogen".to_string(), 25.0),
            ("Methane".to_string(), 10.0),
            ("Sulfur".to_string(), 5.0),
        ];
        let pattern = realize_repeating_pattern(&composition, &catalog).unwrap();
        let repeated = pattern.repeated_local_placement(7, -3).unwrap();
        let original = pattern.material.placements.as_ref().unwrap();
        let shifted = repeated.placements.as_ref().unwrap();
        assert_eq!(original.len(), shifted.len());
        let dx = 7.0 * pattern.width;
        let dy = -3.0 * pattern.height;
        for (a, b) in original.iter().zip(shifted.iter()) {
            assert!((b.x - a.x - dx).abs() < 1e-12);
            assert!((b.y - a.y - dy).abs() < 1e-12);
            assert_eq!(a.rotation_radians, b.rotation_radians);
        }
        assert_eq!(
            pattern.material.internal_connections,
            repeated.internal_connections
        );
    }

    #[test]
    fn duplicate_composition_entries_are_treated_as_one_local_quantity() {
        let catalog = default_catalog();
        let first = realize_pattern(
            &[("Carbon".to_string(), 2.0), ("Carbon".to_string(), 3.0)],
            &catalog,
        )
        .unwrap();
        let second = realize_pattern(&[("Carbon".to_string(), 5.0)], &catalog).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn resolved_frontier_is_blob_shaped_not_rectangular() {
        let catalog = default_catalog();
        let mut formation =
            Formation::new(vec![("Carbon".to_string(), 100.0)], 100.0, &catalog).unwrap();
        formation.set_origin((0.0, 0.0));
        formation.resolve_frontier();

        let positions = formation
            .resolved_frontier()
            .iter()
            .filter_map(|material| material.placements.as_ref()?.first().copied())
            .collect::<Vec<_>>();
        assert!(!positions.is_empty());
        assert!(positions.iter().any(|p| {
            p.x.abs() >= formation.pattern.width && p.y.abs() < formation.pattern.height
        }));
        let max_radius = positions
            .iter()
            .map(|p| (p.x * p.x + p.y * p.y).sqrt())
            .fold(0.0_f64, f64::max);
        assert!(max_radius <= formation.resolved_radius * 1.25);
        assert!(positions.iter().any(|p| {
            let distance = (p.x * p.x + p.y * p.y).sqrt();
            distance > formation.resolved_radius * 0.5
        }));
    }

    #[test]
    fn changing_composition_changes_the_pattern_without_new_resource_types() {
        let catalog = default_catalog();
        let carbon_hydrogen = vec![("Carbon".to_string(), 3.0), ("Hydrogen".to_string(), 1.0)];
        let carbon_sulfur = vec![("Carbon".to_string(), 3.0), ("Sulfur".to_string(), 1.0)];

        let first = realize_pattern(&carbon_hydrogen, &catalog).unwrap();
        let second = realize_pattern(&carbon_sulfur, &catalog).unwrap();

        assert_ne!(first.material.parts, second.material.parts);
        assert!(first
            .material
            .parts
            .iter()
            .all(|(name, _)| catalog.iter().any(|resource| &resource.name == name)));
    }
}
