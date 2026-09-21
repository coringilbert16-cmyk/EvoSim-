//! Deterministic coarse-grained physical formation realization.
//!
//! A formation is not an authored molecule or terrain type.  Its local
//! composition is realized as a small physical pattern using the existing
//! resource geometry and contact rules.  The pattern can then be repeated by
//! a formation representation without instantiating its entire bulk.

use crate::contact::connection_pair_candidates;
use crate::physical_material::PhysicalMaterial;
use crate::resources::{BaseResource, InternalBond, Material};
use crate::structure::{OrganismStructure, Placement, StructuralUnit};

/// Initial experimental pattern period.  This is a representation parameter,
/// not a biological constant or a resource property.
pub(crate) const PATTERN_SIDE: usize = 4;
pub(crate) const PATTERN_SIZE: usize = PATTERN_SIDE * PATTERN_SIDE;

/// A deterministic local pattern that can be repeated through a continuous
/// formation. Coordinates are pattern-local; the owning formation supplies
/// world position and repetition.
#[derive(Clone, Debug, PartialEq)]
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
            .map(|placement| crate::structure::Placement {
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

/// Deterministically converts a local composition into a finite repeated
/// physical pattern.  The composition is expressed as resource quantities;
/// the pattern contains one resolved unit per slot.
pub(crate) fn realize_pattern(
    composition: &[(String, f64)],
    catalog: &[BaseResource],
) -> Option<PhysicalMaterial> {
    let composition = composition
        .iter()
        .filter(|(_, amount)| amount.is_finite() && *amount > 0.0)
        .collect::<Vec<_>>();
    if composition.is_empty() || catalog.is_empty() {
        return None;
    }

    let total = composition.iter().map(|(_, amount)| *amount).sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return None;
    }

    let names = balanced_pattern_names(&composition, total);
    let mut structure = OrganismStructure::new();
    let origin = -(PATTERN_SIDE as f64 - 1.0) * PATTERN_SPACING / 2.0;

    for (index, name) in names.iter().enumerate() {
        let row = index / PATTERN_SIDE;
        let col = index % PATTERN_SIDE;
        let placement = Placement {
            x: origin + col as f64 * PATTERN_SPACING,
            y: origin + row as f64 * PATTERN_SPACING,
            rotation_radians: 0.0,
        };
        let mut unit =
            StructuralUnit::from_material(Material::free_base(name.clone(), 1.0), placement)?;
        if !unit.realize_default_geometry(catalog) {
            return None;
        }
        structure.add_unit(unit);
    }

    let mut bonds = Vec::new();
    for row in 0..PATTERN_SIDE {
        for col in 0..PATTERN_SIDE {
            let a = row * PATTERN_SIDE + col;
            if col + 1 < PATTERN_SIDE {
                add_first_contact_bond(
                    &structure,
                    a,
                    a + 1,
                    catalog,
                    &mut bonds,
                )?;
            }
            if row + 1 < PATTERN_SIDE {
                add_first_contact_bond(
                    &structure,
                    a,
                    a + PATTERN_SIDE,
                    catalog,
                    &mut bonds,
                )?;
            }
        }
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
    let physical = PhysicalMaterial::realized(material, placements, catalog)?;
    Some(physical)
}

pub(crate) fn realize_repeating_pattern(
    composition: &[(String, f64)],
    catalog: &[BaseResource],
) -> Option<FormationPattern> {
    let material = realize_pattern(composition, catalog)?;
    let placements = material.placements.as_ref()?;
    let min_x = placements.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = placements.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = placements.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = placements.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    let pitch_x = (max_x - min_x) / PATTERN_SIDE.saturating_sub(1).max(1) as f64;
    let pitch_y = (max_y - min_y) / PATTERN_SIDE.saturating_sub(1).max(1) as f64;
    let width = (max_x - min_x + pitch_x).max(f64::EPSILON);
    let height = (max_y - min_y + pitch_y).max(f64::EPSILON);
    Some(FormationPattern {
        material,
        width,
        height,
    })
}

fn balanced_pattern_names(
    composition: &[& (String, f64)],
    total: f64,
) -> Vec<String> {
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

fn add_first_contact_bond(
    structure: &OrganismStructure,
    a: usize,
    b: usize,
    catalog: &[BaseResource],
    bonds: &mut Vec<InternalBond>,
) -> Option<()> {
    let candidate = connection_pair_candidates(structure, a, b, catalog)
        .into_iter()
        .find(|candidate| {
            candidate.available_a
                && candidate.available_b
                && candidate.distance <= CONNECTION_DISTANCE
        })?;

    let _ = candidate;
    bonds.push(InternalBond {
        part_a: a,
        part_b: b,
    });
    Some(())
}

#[cfg(test)]
mod tests {
    use super::{realize_pattern, realize_repeating_pattern, PATTERN_SIZE};
    use crate::resources::default_catalog;

    #[test]
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
        assert_eq!(pattern.material.internal_connections, repeated.internal_connections);
    }

    #[test]
    fn changing_composition_changes_the_pattern_without_new_resource_types() {
        let catalog = default_catalog();
        let carbon_hydrogen = vec![
            ("Carbon".to_string(), 3.0),
            ("Hydrogen".to_string(), 1.0),
        ];
        let carbon_sulfur = vec![
            ("Carbon".to_string(), 3.0),
            ("Sulfur".to_string(), 1.0),
        ];

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
