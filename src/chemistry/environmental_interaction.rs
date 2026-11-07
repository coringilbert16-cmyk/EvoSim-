#![expect(
    dead_code,
    reason = "Stage 1 contract is intentionally not wired into production yet"
)]
//! Shared, read-only environmental interaction inventory contract.
//!
//! This module defines the common record shape for environmental chemistry.
//! It does not discover contacts, advance activation, schedule reactions, or
//! commit topology/energy changes. Existing reaction paths remain authoritative
//! until reference-parity tests authorize a later cutover.

use std::collections::BTreeMap;

use crate::contact::{connection_pair_candidate_matches_endpoints, ConnectionPairCandidate};
use crate::structure::ConnectionEndpoint;

/// Stable identity of one physical material object in the active environment.
/// This is never a cell index, vector slot, coordinate, or constituent index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MaterialIdentity(pub(crate) u64);

/// Current field location. This is a mutable locator, never an identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaterialLocator {
    pub(crate) cell_index: usize,
    pub(crate) material_index: usize,
}

/// Versioned reference to one constituent within one material topology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ConstituentRef {
    pub(crate) material: MaterialIdentity,
    pub(crate) topology_generation: u64,
    pub(crate) local_index: usize,
}

/// Snapshot of a material needed to interpret its interaction records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MaterialRecord {
    pub(crate) identity: MaterialIdentity,
    pub(crate) locator: MaterialLocator,
    pub(crate) topology_generation: u64,
    pub(crate) state_version: u64,
    pub(crate) constituent_count: usize,
}

impl MaterialRecord {
    pub(crate) fn constituent(&self, local_index: usize) -> Option<ConstituentRef> {
        (local_index < self.constituent_count).then_some(ConstituentRef {
            material: self.identity,
            topology_generation: self.topology_generation,
            local_index,
        })
    }
}

/// Existing bond inventory entry. BREAK consumes these records.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExistingBondRecord {
    pub(crate) material: MaterialIdentity,
    pub(crate) topology_generation: u64,
    pub(crate) bond_index: usize,
    pub(crate) constituent_a: ConstituentRef,
    pub(crate) endpoint_a: ConnectionEndpoint,
    pub(crate) constituent_b: ConstituentRef,
    pub(crate) endpoint_b: ConnectionEndpoint,
    pub(crate) bond_energy: f64,
}

/// A plausible physical contact. COMBINE consumes every valid interface,
/// rather than a preselected "best" contact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContactInterfaceRecord {
    pub(crate) constituent_a: ConstituentRef,
    pub(crate) endpoint_a: ConnectionEndpoint,
    pub(crate) constituent_b: ConstituentRef,
    pub(crate) endpoint_b: ConnectionEndpoint,
    /// Mutable geometry/availability snapshot, not part of interface identity.
    pub(crate) candidate: ConnectionPairCandidate,
}

impl ContactInterfaceRecord {
    fn canonicalized(mut self) -> Self {
        if self.constituent_b < self.constituent_a {
            std::mem::swap(&mut self.constituent_a, &mut self.constituent_b);
            std::mem::swap(&mut self.endpoint_a, &mut self.endpoint_b);
            self.candidate = ConnectionPairCandidate {
                endpoint_a: self.candidate.endpoint_b,
                endpoint_b: self.candidate.endpoint_a,
                ..self.candidate
            };
        }
        self
    }

    fn same_interface(&self, other: &Self) -> bool {
        self.constituent_a == other.constituent_a
            && self.constituent_b == other.constituent_b
            && connection_pair_candidate_matches_endpoints(
                &self.candidate,
                other.endpoint_a,
                other.endpoint_b,
            )
    }
}

/// Typed counters intentionally distinguish unlike units of discovery work.
/// They are observational in this stage; no existing budgets are changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiscoveryWorkCounters {
    pub(crate) broad_phase_cell_visits: u64,
    pub(crate) material_visits: u64,
    pub(crate) material_pair_visits: u64,
    pub(crate) constituent_pair_visits: u64,
    pub(crate) bond_visits: u64,
    pub(crate) raw_endpoint_candidates: u64,
    pub(crate) geometry_evaluation_operations: u64,
    pub(crate) registry_updates: u64,
}

/// Explicit coverage state prevents "not visited yet" from being confused
/// with "visited and found invalid".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiscoveryCoverage {
    pub(crate) material_pairs_known: u64,
    pub(crate) material_pairs_visited: u64,
    pub(crate) material_pairs_deferred: u64,
    pub(crate) records_invalidated: u64,
}

/// One immutable discovery-pass result shared by all reaction evaluators.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EnvironmentalInteractionInventory {
    pub(crate) pass_id: u64,
    pub(crate) materials: Vec<MaterialRecord>,
    pub(crate) existing_bonds: Vec<ExistingBondRecord>,
    pub(crate) contact_interfaces: Vec<ContactInterfaceRecord>,
    pub(crate) work: DiscoveryWorkCounters,
    pub(crate) coverage: DiscoveryCoverage,
}

/// The displacement evaluator joins an existing A-B bond with an A-C contact.
/// This is only a relationship between inventory entries; it is not a reaction
/// decision and does not split the eventual atomic replacement transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplacementJoin {
    pub(crate) bond_record_index: usize,
    pub(crate) contact_record_index: usize,
    pub(crate) anchor: ConstituentRef,
    pub(crate) incoming: ConstituentRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InventoryBuildError {
    ZeroMaterialIdentity,
    UnknownMaterial,
    StaleTopology,
    InvalidConstituentIndex,
    InvalidBondEnergy,
}

/// Mutable builder used by discovery adapters. The finished inventory is
/// immutable and can be consumed independently by each reaction evaluator.
#[derive(Default)]
pub(crate) struct EnvironmentalInteractionInventoryBuilder {
    pass_id: u64,
    materials: BTreeMap<MaterialIdentity, MaterialRecord>,
    existing_bonds: Vec<ExistingBondRecord>,
    contact_interfaces: Vec<ContactInterfaceRecord>,
    work: DiscoveryWorkCounters,
    coverage: DiscoveryCoverage,
}

impl EnvironmentalInteractionInventoryBuilder {
    pub(crate) fn new(pass_id: u64) -> Self {
        Self {
            pass_id,
            ..Self::default()
        }
    }

    /// Movement updates the locator/state snapshot without changing identity.
    /// A topology-generation change invalidates all dependent bond/contact refs.
    pub(crate) fn upsert_material(
        &mut self,
        record: MaterialRecord,
    ) -> Result<(), InventoryBuildError> {
        if record.identity.0 == 0 {
            return Err(InventoryBuildError::ZeroMaterialIdentity);
        }
        if let Some(previous) = self.materials.get(&record.identity) {
            if previous.topology_generation != record.topology_generation {
                self.existing_bonds
                    .retain(|bond| bond.material != record.identity);
                self.contact_interfaces.retain(|contact| {
                    contact.constituent_a.material != record.identity
                        && contact.constituent_b.material != record.identity
                });
                self.coverage.records_invalidated =
                    self.coverage.records_invalidated.saturating_add(1);
            }
        }
        self.materials.insert(record.identity, record);
        self.work.registry_updates = self.work.registry_updates.saturating_add(1);
        Ok(())
    }

    fn validate_constituent(&self, reference: ConstituentRef) -> Result<(), InventoryBuildError> {
        let Some(material) = self.materials.get(&reference.material) else {
            return Err(InventoryBuildError::UnknownMaterial);
        };
        if material.topology_generation != reference.topology_generation {
            return Err(InventoryBuildError::StaleTopology);
        }
        if reference.local_index >= material.constituent_count {
            return Err(InventoryBuildError::InvalidConstituentIndex);
        }
        Ok(())
    }

    pub(crate) fn add_bond(
        &mut self,
        record: ExistingBondRecord,
    ) -> Result<(), InventoryBuildError> {
        if !record.bond_energy.is_finite() || record.bond_energy < 0.0 {
            return Err(InventoryBuildError::InvalidBondEnergy);
        }
        if record.material != record.constituent_a.material
            || record.material != record.constituent_b.material
            || record.topology_generation != record.constituent_a.topology_generation
            || record.topology_generation != record.constituent_b.topology_generation
        {
            return Err(InventoryBuildError::StaleTopology);
        }
        self.validate_constituent(record.constituent_a)?;
        self.validate_constituent(record.constituent_b)?;
        self.existing_bonds.push(record);
        self.work.bond_visits = self.work.bond_visits.saturating_add(1);
        Ok(())
    }

    /// Canonicalizes participant order and deduplicates only the same physical
    /// interface. Different endpoint pairs between the same constituents remain.
    /// Returns true when a new interface was inserted.
    pub(crate) fn add_contact(
        &mut self,
        record: ContactInterfaceRecord,
    ) -> Result<bool, InventoryBuildError> {
        self.validate_constituent(record.constituent_a)?;
        self.validate_constituent(record.constituent_b)?;
        if record.constituent_a.material == record.constituent_b.material {
            return Err(InventoryBuildError::InvalidConstituentIndex);
        }
        let record = record.canonicalized();
        if let Some(existing) = self
            .contact_interfaces
            .iter_mut()
            .find(|existing| existing.same_interface(&record))
        {
            // Keep identity, refresh mutable geometry/availability facts.
            existing.candidate = record.candidate;
            self.work.registry_updates = self.work.registry_updates.saturating_add(1);
            return Ok(false);
        }
        self.contact_interfaces.push(record);
        self.work.registry_updates = self.work.registry_updates.saturating_add(1);
        Ok(true)
    }

    pub(crate) fn work_mut(&mut self) -> &mut DiscoveryWorkCounters {
        &mut self.work
    }

    pub(crate) fn coverage_mut(&mut self) -> &mut DiscoveryCoverage {
        &mut self.coverage
    }

    pub(crate) fn finish(self) -> EnvironmentalInteractionInventory {
        EnvironmentalInteractionInventory {
            pass_id: self.pass_id,
            materials: self.materials.into_values().collect(),
            existing_bonds: self.existing_bonds,
            contact_interfaces: self.contact_interfaces,
            work: self.work,
            coverage: self.coverage,
        }
    }
}

impl EnvironmentalInteractionInventory {
    /// Enumerate every bond/contact join sharing the same anchor constituent.
    /// Reaction-specific eligibility and atomic commit remain elsewhere.
    pub(crate) fn displacement_joins(&self) -> Vec<DisplacementJoin> {
        let mut joins = Vec::new();
        for (bond_index, bond) in self.existing_bonds.iter().enumerate() {
            for (contact_index, contact) in self.contact_interfaces.iter().enumerate() {
                let anchor = if contact.constituent_a == bond.constituent_a
                    || contact.constituent_a == bond.constituent_b
                {
                    Some((contact.constituent_a, contact.constituent_b))
                } else if contact.constituent_b == bond.constituent_a
                    || contact.constituent_b == bond.constituent_b
                {
                    Some((contact.constituent_b, contact.constituent_a))
                } else {
                    None
                };
                if let Some((anchor, incoming)) = anchor {
                    joins.push(DisplacementJoin {
                        bond_record_index: bond_index,
                        contact_record_index: contact_index,
                        anchor,
                        incoming,
                    });
                }
            }
        }
        joins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(identity: u64, generation: u64, count: usize) -> MaterialRecord {
        MaterialRecord {
            identity: MaterialIdentity(identity),
            locator: MaterialLocator {
                cell_index: identity as usize,
                material_index: 0,
            },
            topology_generation: generation,
            state_version: 1,
            constituent_count: count,
        }
    }

    fn candidate(
        endpoint_a: ConnectionEndpoint,
        endpoint_b: ConnectionEndpoint,
    ) -> ConnectionPairCandidate {
        ConnectionPairCandidate {
            endpoint_a,
            endpoint_b,
            distance: 0.25,
            facing: 0.8,
            load_a: 0.0,
            load_b: 0.0,
            available_a: true,
            available_b: true,
        }
    }

    fn contact(
        a: ConstituentRef,
        endpoint_a: ConnectionEndpoint,
        b: ConstituentRef,
        endpoint_b: ConnectionEndpoint,
    ) -> ContactInterfaceRecord {
        ContactInterfaceRecord {
            constituent_a: a,
            endpoint_a,
            constituent_b: b,
            endpoint_b,
            candidate: candidate(endpoint_a, endpoint_b),
        }
    }

    #[test]
    fn inventory_keeps_material_bond_and_contact_record_families_separate() {
        let mut builder = EnvironmentalInteractionInventoryBuilder::new(7);
        builder.upsert_material(material(11, 2, 2)).unwrap();
        builder.upsert_material(material(22, 4, 1)).unwrap();
        let a = ConstituentRef {
            material: MaterialIdentity(11),
            topology_generation: 2,
            local_index: 0,
        };
        let b = ConstituentRef {
            material: MaterialIdentity(11),
            topology_generation: 2,
            local_index: 1,
        };
        let c = ConstituentRef {
            material: MaterialIdentity(22),
            topology_generation: 4,
            local_index: 0,
        };
        builder.add_bond(ExistingBondRecord {
            material: MaterialIdentity(11),
            topology_generation: 2,
            bond_index: 0,
            constituent_a: a,
            endpoint_a: ConnectionEndpoint::Corner { point_index: 0 },
            constituent_b: b,
            endpoint_b: ConnectionEndpoint::Corner { point_index: 1 },
            bond_energy: 1.5,
        })
        .unwrap();
        builder
            .add_contact(contact(
                a,
                ConnectionEndpoint::Corner { point_index: 0 },
                c,
                ConnectionEndpoint::Corner { point_index: 0 },
            ))
            .unwrap();
        let inventory = builder.finish();
        assert_eq!(inventory.pass_id, 7);
        assert_eq!(inventory.materials.len(), 2);
        assert_eq!(inventory.existing_bonds.len(), 1);
        assert_eq!(inventory.contact_interfaces.len(), 1);
        assert_eq!(inventory.displacement_joins().len(), 1);
    }

    #[test]
    fn contact_identity_is_canonical_and_endpoint_tolerance_aware() {
        let mut builder = EnvironmentalInteractionInventoryBuilder::new(1);
        builder.upsert_material(material(1, 0, 1)).unwrap();
        builder.upsert_material(material(2, 0, 1)).unwrap();
        let a = ConstituentRef {
            material: MaterialIdentity(1),
            topology_generation: 0,
            local_index: 0,
        };
        let b = ConstituentRef {
            material: MaterialIdentity(2),
            topology_generation: 0,
            local_index: 0,
        };
        let first = contact(
            a,
            ConnectionEndpoint::Boundary { angle_radians: 0.0 },
            b,
            ConnectionEndpoint::Fluid { x: 3.0, y: 4.0 },
        );
        assert!(builder.add_contact(first).unwrap());
        let reversed = contact(
            b,
            ConnectionEndpoint::Fluid {
                x: 3.0 + 5e-9,
                y: 4.0,
            },
            a,
            ConnectionEndpoint::Boundary { angle_radians: std::f64::consts::TAU },
        );
        assert!(!builder.add_contact(reversed).unwrap());
        assert_eq!(builder.contact_interfaces.len(), 1);
    }

    #[test]
    fn distinct_interfaces_between_same_constituents_are_preserved() {
        let mut builder = EnvironmentalInteractionInventoryBuilder::new(1);
        builder.upsert_material(material(1, 0, 1)).unwrap();
        builder.upsert_material(material(2, 0, 1)).unwrap();
        let a = ConstituentRef {
            material: MaterialIdentity(1),
            topology_generation: 0,
            local_index: 0,
        };
        let b = ConstituentRef {
            material: MaterialIdentity(2),
            topology_generation: 0,
            local_index: 0,
        };
        builder
            .add_contact(contact(
                a,
                ConnectionEndpoint::Corner { point_index: 0 },
                b,
                ConnectionEndpoint::Corner { point_index: 0 },
            ))
            .unwrap();
        builder
            .add_contact(contact(
                a,
                ConnectionEndpoint::Corner { point_index: 1 },
                b,
                ConnectionEndpoint::Corner { point_index: 0 },
            ))
            .unwrap();
        assert_eq!(builder.contact_interfaces.len(), 2);
    }

    #[test]
    fn topology_generation_change_invalidates_dependent_records_but_movement_does_not() {
        let mut builder = EnvironmentalInteractionInventoryBuilder::new(1);
        builder.upsert_material(material(1, 5, 1)).unwrap();
        builder.upsert_material(material(2, 3, 1)).unwrap();
        let a = ConstituentRef {
            material: MaterialIdentity(1),
            topology_generation: 5,
            local_index: 0,
        };
        let b = ConstituentRef {
            material: MaterialIdentity(2),
            topology_generation: 3,
            local_index: 0,
        };
        builder
            .add_contact(contact(
                a,
                ConnectionEndpoint::Corner { point_index: 0 },
                b,
                ConnectionEndpoint::Corner { point_index: 0 },
            ))
            .unwrap();
        let mut moved = material(1, 5, 1);
        moved.locator.cell_index = 99;
        moved.state_version = 2;
        builder.upsert_material(moved).unwrap();
        assert_eq!(builder.contact_interfaces.len(), 1);
        builder.upsert_material(material(1, 6, 1)).unwrap();
        assert!(builder.contact_interfaces.is_empty());
        assert_eq!(builder.coverage.records_invalidated, 1);
    }

    #[test]
    fn stale_or_unknown_constituent_references_are_rejected() {
        let mut builder = EnvironmentalInteractionInventoryBuilder::new(1);
        builder.upsert_material(material(1, 2, 1)).unwrap();
        let stale = ConstituentRef {
            material: MaterialIdentity(1),
            topology_generation: 1,
            local_index: 0,
        };
        let unknown = ConstituentRef {
            material: MaterialIdentity(9),
            topology_generation: 1,
            local_index: 0,
        };
        let endpoint = ConnectionEndpoint::Corner { point_index: 0 };
        assert_eq!(
            builder.add_contact(contact(stale, endpoint, unknown, endpoint)),
            Err(InventoryBuildError::StaleTopology)
        );
    }
}
