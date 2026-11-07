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
                    snapshot.unassigned_materials =
                        snapshot.unassigned_materials.saturating_add(1);
                    continue;
                }
                let topology_generation = 0;
                snapshot.materials.push(MaterialRecord {
                    identity,
                    locator: MaterialLocator { cell_index, material_index },
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
        field.cells[1].physical_materials.push(physical(20, "Carbon"));
        field.cells[1].physical_materials.push(physical(10, "Oxygen"));
        field.cells[3].physical_materials.push(physical(30, "Hydrogen"));
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
    fn unassigned_materials_are_reported_not_given_fabricated_identity() {
        let mut field = ActiveMaterialField::new(10.0, 10.0, 10.0);
        field.cells[0].physical_materials.push(physical(0, "Carbon"));
        let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 2);
        assert_eq!(snapshot.unassigned_materials, 1);
        assert!(snapshot.materials.is_empty());
        assert_eq!(snapshot.known_pair_count(), 0);
    }
}
