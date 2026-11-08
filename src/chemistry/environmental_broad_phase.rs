#![expect(
    dead_code,
    reason = "Stage 3 broad phase is being parity-tested before production cutover"
)]
//! Conservative shared environmental material snapshot and pair traversal.
//!
//! This is the Stage 3 reference broad phase: it scans field ownership once,
//! then exposes every unordered pair of distinct physical materials. It is
//! intentionally conservative (no spatial rejection yet), so it can serve as
//! a no-false-negative oracle while a bounded spatial index is developed.
//! Reaction-specific geometry, activation, scheduling, funding, and commits
//! remain in their existing subsystems.

use crate::environment::ActiveMaterialField;
use crate::environmental_interaction::BondRef;
use crate::environmental_interaction::ConstituentRef;
use crate::environmental_interaction::ExistingBondRecord;
use crate::environmental_interaction::MaterialIdentity;
use crate::environmental_interaction::MaterialLocator;
use crate::environmental_interaction::MaterialRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaterialPairRecord {
    pub(crate) left: usize,
    pub(crate) right: usize,
    pub(crate) identity_left: MaterialIdentity,
    pub(crate) identity_right: MaterialIdentity,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EnvironmentalMaterialSnapshot {
    pub(crate) pass_id: u64,
    pub(crate) materials: Vec<MaterialRecord>,
    pub(crate) existing_bonds: Vec<ExistingBondRecord>,
    pub(crate) cell_visits: u64,
    pub(crate) material_visits: u64,
    pub(crate) unassigned_materials: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PairWindowCoverage {
    pub(crate) known_pairs: u64,
    pub(crate) visited_pairs: u64,
    pub(crate) deferred_pairs: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaterialSpatialBounds {
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
}

impl MaterialSpatialBounds {
    fn expanded(self, padding: f64) -> Option<Self> {
        if !padding.is_finite()
            || padding < 0.0
            || !self.min_x.is_finite()
            || !self.max_x.is_finite()
            || !self.min_y.is_finite()
            || !self.max_y.is_finite()
            || self.min_x > self.max_x
            || self.min_y > self.max_y
        {
            return None;
        }
        Some(Self {
            min_x: self.min_x - padding,
            max_x: self.max_x + padding,
            min_y: self.min_y - padding,
            max_y: self.max_y + padding,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PairOrientation {
    /// One pair, ordered by current field locator. This is the COMBINE contract.
    Unordered,
    /// Every anchor-to-target direction. DISPLACEMENT needs both roles.
    Directed,
}

impl EnvironmentalMaterialSnapshot {
    /// Build one pass-local ownership snapshot. Topology generation is scoped
    /// to this immutable snapshot; it is not a fabricated persistent version.
    pub(crate) fn from_field(field: &ActiveMaterialField, pass_id: u64) -> Self {
        let mut snapshot = Self {
            pass_id,
            cell_visits: field.cells.len() as u64,
            ..Self::default()
        };
        for (cell_index, cell) in field.cells.iter().enumerate() {
            for (material_index, physical) in cell.physical_materials.iter().enumerate() {
                snapshot.material_visits = snapshot.material_visits.saturating_add(1);
                let identity = MaterialIdentity(physical.environmental_identity);
                if identity.0 == 0 {
                    snapshot.unassigned_materials = snapshot.unassigned_materials.saturating_add(1);
                    continue;
                }
                let topology_generation = 0;
                snapshot.materials.push(MaterialRecord {
                    identity,
                    locator: MaterialLocator {
                        cell_index,
                        material_index,
                    },
                    topology_generation,
                    state_version: 0,
                    constituent_count: physical.material.parts.len(),
                });
                if let Some(bonds) = physical.internal_connections.as_ref() {
                    for (bond_index, bond) in bonds.iter().enumerate() {
                        let (Some(constituent_a), Some(constituent_b)) = (
                            (bond.part_a < physical.material.parts.len()).then_some(bond.part_a),
                            (bond.part_b < physical.material.parts.len()).then_some(bond.part_b),
                        ) else {
                            continue;
                        };
                        snapshot.existing_bonds.push(ExistingBondRecord {
                            material: identity,
                            topology_generation,
                            bond_index,
                            constituent_a: ConstituentRef {
                                material: identity,
                                topology_generation,
                                local_index: constituent_a,
                            },
                            endpoint_a: bond.endpoint_a,
                            constituent_b: ConstituentRef {
                                material: identity,
                                topology_generation,
                                local_index: constituent_b,
                            },
                            endpoint_b: bond.endpoint_b,
                            bond_energy: bond.bond_energy,
                        });
                    }
                }
            }
        }
        snapshot.materials.sort_by_key(|material| material.identity);
        snapshot.existing_bonds.sort_by_key(|bond| bond.reference());
        snapshot
    }

    pub(crate) fn known_pair_count(&self) -> u64 {
        let count = self.materials.len() as u64;
        count.saturating_mul(count.saturating_sub(1)) / 2
    }

    /// Return a deterministic window over all unordered material pairs.
    /// `start` is a pair ordinal, not a material index or cell locator.
    pub(crate) fn pair_window(
        &self,
        start: u64,
        limit: usize,
    ) -> (Vec<MaterialPairRecord>, PairWindowCoverage) {
        let known_pairs = self.known_pair_count();
        let start = start.min(known_pairs);
        let count = (known_pairs - start).min(limit as u64);
        let mut pairs = Vec::with_capacity(count as usize);
        let mut ordinal = 0u64;
        for left in 0..self.materials.len() {
            for right in left + 1..self.materials.len() {
                if ordinal >= start && pairs.len() < count as usize {
                    let a = &self.materials[left];
                    let b = &self.materials[right];
                    pairs.push(MaterialPairRecord {
                        left,
                        right,
                        identity_left: a.identity,
                        identity_right: b.identity,
                    });
                }
                ordinal = ordinal.saturating_add(1);
                if pairs.len() == count as usize {
                    break;
                }
            }
            if pairs.len() == count as usize {
                break;
            }
        }
        let visited_pairs = pairs.len() as u64;
        (
            pairs,
            PairWindowCoverage {
                known_pairs,
                visited_pairs,
                deferred_pairs: known_pairs.saturating_sub(start + visited_pairs),
            },
        )
    }

    /// Build exact broad-phase bounds by restoring each snapshot material
    /// with the same origin pose used by the live environmental reaction paths.
    /// Invalid, unrealized, or stale locators produce no spatial bounds and
    /// therefore cannot create a false candidate.
    pub(crate) fn spatial_bounds_from_field(
        &self,
        field: &ActiveMaterialField,
        catalog: &[crate::resources::BaseResource],
    ) -> Vec<Option<MaterialSpatialBounds>> {
        self.materials
            .iter()
            .map(|record| {
                let physical = field
                    .cells
                    .get(record.locator.cell_index)?
                    .physical_materials
                    .get(record.locator.material_index)?;
                if physical.environmental_identity != record.identity.0
                    || !physical.is_realized()
                    || physical.material.is_empty()
                    || !physical.material.is_valid_for_catalog(catalog)
                {
                    return None;
                }
                let mut structure = crate::structure::OrganismStructure::new();
                crate::material_restoration::restore_material(
                    &mut structure,
                    physical,
                    crate::structure::Placement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                    catalog,
                )?;
                let body =
                    crate::organism_geometry::OrganismBodyGeometry::from_all_realized_units(
                        &structure,
                    )?;
                Some(MaterialSpatialBounds {
                    min_x: body.min_x,
                    max_x: body.max_x,
                    min_y: body.min_y,
                    max_y: body.max_y,
                })
            })
            .collect()
    }

    /// Enumerate spatially plausible material pairs using the same cell-bound
    /// query used by the live COMBINE and DISPLACEMENT discovery paths.
    ///
    /// The bounds slice is aligned with self.materials and must be computed from
    /// each restored material's complete realized geometry. The cell query is
    /// only a broad-phase filter; constituent endpoint geometry remains
    /// authoritative. COMBINE requests unordered pairs, while DISPLACEMENT
    /// requests both directed anchor/incoming orientations.
    pub(crate) fn spatial_pair_window(
        &self,
        field: &ActiveMaterialField,
        bounds: &[Option<MaterialSpatialBounds>],
        padding: f64,
        start: u64,
        limit: usize,
        orientation: PairOrientation,
    ) -> (Vec<MaterialPairRecord>, PairWindowCoverage) {
        if bounds.len() != self.materials.len() {
            return (
                Vec::new(),
                PairWindowCoverage {
                    known_pairs: 0,
                    visited_pairs: 0,
                    deferred_pairs: 0,
                },
            );
        }
        let mut locator_to_material = std::collections::BTreeMap::new();
        for (index, material) in self.materials.iter().enumerate() {
            locator_to_material.insert(
                (material.locator.cell_index, material.locator.material_index),
                index,
            );
        }
        let mut unordered_candidates = std::collections::BTreeMap::new();
        let mut directed_candidates = Vec::new();
        for (anchor_index, anchor) in self.materials.iter().enumerate() {
            let Some(expanded) = bounds[anchor_index].and_then(|value| value.expanded(padding))
            else {
                continue;
            };
            let target_cells = field.cells_intersecting_bounds(
                expanded.min_x,
                expanded.max_x,
                expanded.min_y,
                expanded.max_y,
            );
            for cell_index in target_cells {
                let Some(cell) = field.cells.get(cell_index) else {
                    continue;
                };
                for target_slot in 0..cell.physical_materials.len() {
                    let Some(&target_index) =
                        locator_to_material.get(&(cell_index, target_slot))
                    else {
                        continue;
                    };
                    if target_index == anchor_index {
                        continue;
                    }
                    let target = &self.materials[target_index];
                    if orientation == PairOrientation::Unordered {
                        // Query both material bounds before canonicalizing. A
                        // physically overlapping target may occupy a cell that
                        // is reached only by the target's expanded bounds.
                        // Keeping the union avoids a direction-dependent miss.
                        let (left, right) = if (
                            anchor.locator.cell_index,
                            anchor.locator.material_index,
                        ) < (target.locator.cell_index, target.locator.material_index)
                        {
                            (anchor_index, target_index)
                        } else {
                            (target_index, anchor_index)
                        };
                        unordered_candidates.entry((left, right)).or_insert_with(|| {
                            let left_material = &self.materials[left];
                            let right_material = &self.materials[right];
                            MaterialPairRecord {
                                left,
                                right,
                                identity_left: left_material.identity,
                                identity_right: right_material.identity,
                            }
                        });
                    } else {
                        directed_candidates.push(MaterialPairRecord {
                            left: anchor_index,
                            right: target_index,
                            identity_left: anchor.identity,
                            identity_right: target.identity,
                        });
                    }
                }
            }
        }
        let candidates = match orientation {
            PairOrientation::Unordered => unordered_candidates.into_values().collect::<Vec<_>>(),
            PairOrientation::Directed => directed_candidates,
        };
        let known_pairs = candidates.len() as u64;
        let start = start.min(known_pairs);
        let count = (known_pairs - start).min(limit as u64) as usize;
        let pairs = candidates
            .into_iter()
            .skip(start as usize)
            .take(count)
            .collect::<Vec<_>>();
        let visited_pairs = pairs.len() as u64;
        (
            pairs,
            PairWindowCoverage {
                known_pairs,
                visited_pairs,
                deferred_pairs: known_pairs.saturating_sub(start + visited_pairs),
            },
        )
    }

    pub(crate) fn bond_reference_is_current(&self, reference: BondRef) -> bool {
        self.existing_bonds
            .iter()
            .any(|bond| bond.reference() == reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::Material;

    fn physical(identity: u64, name: &str) -> PhysicalMaterial {
        PhysicalMaterial {
            material: Material::free_base(name.to_owned(), 1.0),
            environmental_identity: identity,
            placements: None,
            internal_connections: None,
            owner_relative_origin: None,
            realized_shapes: None,
        }
    }

    #[test]
    fn snapshot_visits_field_once_and_retains_stable_identity_across_locators() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        field.cells[1]
            .physical_materials
            .push(physical(20, "Carbon"));
        field.cells[1]
            .physical_materials
            .push(physical(10, "Oxygen"));
        field.cells[3]
            .physical_materials
            .push(physical(30, "Hydrogen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 77);
        assert_eq!(snapshot.pass_id, 77);
        assert_eq!(snapshot.cell_visits, 8);
        assert_eq!(snapshot.material_visits, 3);
        assert_eq!(snapshot.materials.len(), 3);
        assert_eq!(snapshot.materials[0].identity, MaterialIdentity(10));
        assert_eq!(snapshot.materials[0].locator.cell_index, 1);
        assert_eq!(snapshot.materials[1].identity, MaterialIdentity(20));
        assert_eq!(snapshot.materials[2].identity, MaterialIdentity(30));
    }

    #[test]
    fn pair_windows_cover_every_unordered_pair_without_duplicates_or_gaps() {
        let mut field = ActiveMaterialField::new(50.0, 10.0, 10.0);
        for identity in 1..=5 {
            field.cells[0]
                .physical_materials
                .push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 1);
        assert_eq!(snapshot.known_pair_count(), 10);
        let (first, first_coverage) = snapshot.pair_window(0, 4);
        let (second, second_coverage) = snapshot.pair_window(4, 4);
        let (third, third_coverage) = snapshot.pair_window(8, 4);
        assert_eq!(first.len(), 4);
        assert_eq!(second.len(), 4);
        assert_eq!(third.len(), 2);
        assert_eq!(first_coverage.deferred_pairs, 6);
        assert_eq!(second_coverage.deferred_pairs, 2);
        assert_eq!(third_coverage.deferred_pairs, 0);
        let mut identities = first
            .into_iter()
            .chain(second)
            .chain(third)
            .map(|pair| (pair.identity_left, pair.identity_right))
            .collect::<Vec<_>>();
        identities.sort();
        identities.dedup();
        assert_eq!(identities.len(), 10);
        assert!(identities.iter().all(|(a, b)| a < b));
    }

    #[test]
    fn spatial_pairs_prune_distant_materials_and_keep_displacement_directions() {
        let mut field = ActiveMaterialField::new(100.0, 100.0, 10.0);
        field.cells[0].physical_materials.push(physical(10, "Carbon"));
        field.cells[11].physical_materials.push(physical(20, "Oxygen"));
        field.cells[99].physical_materials.push(physical(30, "Hydrogen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 3);
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 9.0, max_x: 11.0, min_y: 9.0, max_y: 11.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 9.0, max_x: 11.0, min_y: 9.0, max_y: 11.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 89.0, max_x: 91.0, min_y: 89.0, max_y: 91.0,
            }),
        ];
        let (unordered, coverage) = snapshot.spatial_pair_window(
            &field, &bounds, 0.0, 0, usize::MAX, PairOrientation::Unordered,
        );
        assert_eq!(coverage.known_pairs, 1);
        assert_eq!(unordered.len(), 1);
        assert_eq!(
            (unordered[0].identity_left, unordered[0].identity_right),
            (MaterialIdentity(10), MaterialIdentity(20))
        );

        let (directed, directed_coverage) = snapshot.spatial_pair_window(
            &field, &bounds, 0.0, 0, usize::MAX, PairOrientation::Directed,
        );
        assert_eq!(directed_coverage.known_pairs, 2);
        assert_eq!(directed.len(), 2);
        assert!(directed.iter().any(|pair| {
            pair.identity_left == MaterialIdentity(10)
                && pair.identity_right == MaterialIdentity(20)
        }));
        assert!(directed.iter().any(|pair| {
            pair.identity_left == MaterialIdentity(20)
                && pair.identity_right == MaterialIdentity(10)
        }));
    }

    #[test]
    fn unordered_spatial_pair_uses_union_of_both_material_queries() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        field.cells[0].physical_materials.push(physical(10, "Carbon"));
        field.cells[1].physical_materials.push(physical(20, "Oxygen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 6);
        // Only the higher-locator material's expanded bounds reach the
        // lower-locator material's cell. Unordered discovery must not lose it.
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: 0.0, max_x: 1.0, min_y: 0.0, max_y: 1.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 0.0, max_x: 11.0, min_y: 0.0, max_y: 1.0,
            }),
        ];
        let (pairs, coverage) = snapshot.spatial_pair_window(
            &field, &bounds, 0.0, 0, usize::MAX, PairOrientation::Unordered,
        );
        assert_eq!(coverage.known_pairs, 1);
        assert_eq!(pairs.len(), 1);
        assert_eq!(
            (pairs[0].identity_left, pairs[0].identity_right),
            (MaterialIdentity(10), MaterialIdentity(20))
        );
    }

    #[test]
    fn spatial_pairs_preserve_existing_vertical_wrap_and_horizontal_bounds() {
        let mut field = ActiveMaterialField::new(40.0, 40.0, 10.0);
        field.cells[0].physical_materials.push(physical(10, "Carbon"));
        field.cells[12].physical_materials.push(physical(20, "Oxygen"));
        field.cells[3].physical_materials.push(physical(30, "Hydrogen"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 5);
        let bounds = vec![
            Some(MaterialSpatialBounds {
                min_x: -1.0, max_x: 1.0, min_y: -1.0, max_y: 1.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 0.0, max_x: 10.0, min_y: 29.0, max_y: 31.0,
            }),
            Some(MaterialSpatialBounds {
                min_x: 29.0, max_x: 31.0, min_y: 0.0, max_y: 10.0,
            }),
        ];
        let (pairs, coverage) = snapshot.spatial_pair_window(
            &field, &bounds, 0.0, 0, usize::MAX, PairOrientation::Unordered,
        );
        assert_eq!(coverage.known_pairs, 1);
        assert_eq!(pairs.len(), 1);
        assert_eq!(
            (pairs[0].identity_left, pairs[0].identity_right),
            (MaterialIdentity(10), MaterialIdentity(20))
        );
    }

    #[test]
    fn spatial_pair_windows_report_deferred_work_without_losing_candidates() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        for identity in 1..=3 {
            field.cells[0].physical_materials.push(physical(identity, "Carbon"));
        }
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 4);
        let bounds = vec![Some(MaterialSpatialBounds {
            min_x: 0.0, max_x: 10.0, min_y: 0.0, max_y: 10.0,
        }); 3];
        let (_, first) = snapshot.spatial_pair_window(
            &field, &bounds, 0.0, 0, 1, PairOrientation::Unordered,
        );
        let (_, second) = snapshot.spatial_pair_window(
            &field, &bounds, 0.0, 1, 8, PairOrientation::Unordered,
        );
        assert_eq!(first.known_pairs, 3);
        assert_eq!(first.visited_pairs, 1);
        assert_eq!(first.deferred_pairs, 2);
        assert_eq!(second.visited_pairs, 2);
        assert_eq!(second.deferred_pairs, 0);
    }

    #[test]
    fn unassigned_materials_are_reported_not_given_fabricated_identity() {
        let mut field = ActiveMaterialField::new(10.0, 10.0, 10.0);
        field.cells[0]
            .physical_materials
            .push(physical(0, "Carbon"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 2);
        assert_eq!(snapshot.unassigned_materials, 1);
        assert!(snapshot.materials.is_empty());
        assert_eq!(snapshot.known_pair_count(), 0);
    }
}
