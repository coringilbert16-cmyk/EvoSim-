#![expect(dead_code, reason = "Stage 1 snapshot cursor is parity-tested before production cutover")]
//! Resumable construction of the environmental material/bond snapshot.
//! Every cell, material, and bond inspection consumes one unit of caller budget.

use std::collections::BTreeMap;

use crate::environment::ActiveMaterialField;
use crate::environmental_broad_phase::EnvironmentalMaterialSnapshot;
use crate::environmental_interaction::{
    BondRef, ConstituentRef, ExistingBondRecord, MaterialIdentity, MaterialLocator, MaterialRecord,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapshotCursorStatus {
    Incomplete,
    Complete,
    Invalidated,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SnapshotCursorPage {
    pub(crate) visited_work: u64,
    pub(crate) cell_visits: u64,
    pub(crate) material_visits: u64,
    pub(crate) bond_visits: u64,
    pub(crate) status: Option<SnapshotCursorStatus>,
    pub(crate) snapshot: Option<EnvironmentalMaterialSnapshot>,
}

#[derive(Clone, Debug)]
pub(crate) struct EnvironmentalSnapshotCursor {
    pass_id: u64,
    field_revision: u64,
    next_cell: usize,
    next_material: usize,
    active_bond: Option<(usize, usize, usize)>,
    materials: BTreeMap<MaterialIdentity, MaterialRecord>,
    bonds: BTreeMap<BondRef, ExistingBondRecord>,
    cell_visits: u64,
    material_visits: u64,
    unassigned_materials: u64,
    complete: bool,
    invalidated: bool,
}

impl EnvironmentalSnapshotCursor {
    pub(crate) fn new(field: &ActiveMaterialField, pass_id: u64) -> Self {
        Self {
            pass_id,
            field_revision: field.revision,
            next_cell: 0,
            next_material: 0,
            active_bond: None,
            materials: BTreeMap::new(),
            bonds: BTreeMap::new(),
            cell_visits: 0,
            material_visits: 0,
            unassigned_materials: 0,
            complete: false,
            invalidated: false,
        }
    }

    pub(crate) fn next_page(
        &mut self,
        field: &ActiveMaterialField,
        work_limit: usize,
    ) -> SnapshotCursorPage {
        if self.invalidated || field.revision != self.field_revision {
            self.invalidated = true;
            return SnapshotCursorPage {
                status: Some(SnapshotCursorStatus::Invalidated),
                ..SnapshotCursorPage::default()
            };
        }
        if self.complete {
            return self.completed_page();
        }

        let mut page = SnapshotCursorPage::default();
        while page.visited_work < work_limit as u64 && !self.complete {
            if let Some((cell_index, material_index, bond_index)) = self.active_bond {
                let Some(physical) = field.cells.get(cell_index)
                    .and_then(|cell| cell.physical_materials.get(material_index)) else {
                    self.invalidated = true;
                    break;
                };
                let Some(bond) = physical.internal_connections.as_ref()
                    .and_then(|bonds| bonds.get(bond_index)) else {
                    self.active_bond = None;
                    continue;
                };
                let part_count = physical.material.parts.len();
                if bond.part_a < part_count && bond.part_b < part_count {
                    let identity = MaterialIdentity(physical.environmental_identity);
                    if identity.0 != 0 {
                        let generation = 0;
                        let record = ExistingBondRecord {
                            material: identity,
                            topology_generation: generation,
                            bond_index,
                            constituent_a: ConstituentRef { material: identity, topology_generation: generation, local_index: bond.part_a },
                            endpoint_a: bond.endpoint_a,
                            constituent_b: ConstituentRef { material: identity, topology_generation: generation, local_index: bond.part_b },
                            endpoint_b: bond.endpoint_b,
                            bond_energy: bond.bond_energy,
                        };
                        self.bonds.insert(record.reference(), record);
                    }
                }
                let next = bond_index + 1;
                let bond_count = physical.internal_connections.as_ref().map_or(0, Vec::len);
                self.active_bond = (next < bond_count).then_some((cell_index, material_index, next));
                page.bond_visits = page.bond_visits.saturating_add(1);
                self.consume_work(&mut page);
                continue;
            }

            let Some(cell) = field.cells.get(self.next_cell) else {
                self.complete = true;
                break;
            };
            if self.next_material >= cell.physical_materials.len() {
                self.next_cell += 1;
                self.next_material = 0;
                self.cell_visits = self.cell_visits.saturating_add(1);
                page.cell_visits = page.cell_visits.saturating_add(1);
                self.consume_work(&mut page);
                if self.next_cell >= field.cells.len() {
                    self.complete = true;
                }
                continue;
            }

            let cell_index = self.next_cell;
            let material_index = self.next_material;
            let physical = &cell.physical_materials[material_index];
            self.next_material += 1;
            self.material_visits = self.material_visits.saturating_add(1);
            page.material_visits = page.material_visits.saturating_add(1);
            let identity = MaterialIdentity(physical.environmental_identity);
            if identity.0 == 0 {
                self.unassigned_materials = self.unassigned_materials.saturating_add(1);
            } else {
                let record = MaterialRecord {
                    identity,
                    locator: MaterialLocator { cell_index, material_index },
                    topology_generation: 0,
                    state_version: 0,
                    constituent_count: physical.material.parts.len(),
                };
                self.materials.insert(identity, record);
                if physical.internal_connections.as_ref().is_some_and(|bonds| !bonds.is_empty()) {
                    self.active_bond = Some((cell_index, material_index, 0));
                }
            }
            self.consume_work(&mut page);
        }

        if self.invalidated {
            page.status = Some(SnapshotCursorStatus::Invalidated);
        } else if self.complete {
            let snapshot = EnvironmentalMaterialSnapshot {
                pass_id: self.pass_id,
                field_revision: self.field_revision,
                materials: std::mem::take(&mut self.materials).into_values().collect(),
                existing_bonds: std::mem::take(&mut self.bonds).into_values().collect(),
                cell_visits: self.cell_visits,
                material_visits: self.material_visits,
                unassigned_materials: self.unassigned_materials,
            };
            page.snapshot = Some(snapshot);
            page.status = Some(SnapshotCursorStatus::Complete);
        } else {
            page.status = Some(SnapshotCursorStatus::Incomplete);
        }
        page
    }

    fn consume_work(&self, page: &mut SnapshotCursorPage) {
        page.visited_work = page.visited_work.saturating_add(1);
    }

    fn completed_page(&self) -> SnapshotCursorPage {
        SnapshotCursorPage {
            status: Some(SnapshotCursorStatus::Complete),
            ..SnapshotCursorPage::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::ActiveMaterialField;
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
    fn pages_respect_budget_and_match_snapshot_ordering() {
        let mut field = ActiveMaterialField::new(40.0, 20.0, 10.0);
        field.cells[1].physical_materials.push(physical(20, "Carbon"));
        field.cells[1].physical_materials.push(physical(10, "Oxygen"));
        field.cells[3].physical_materials.push(physical(30, "Hydrogen"));
        let mut cursor = EnvironmentalSnapshotCursor::new(&field, 77);
        let mut total = 0;
        let snapshot = loop {
            let page = cursor.next_page(&field, 2);
            assert!(page.visited_work <= 2);
            total += page.visited_work;
            if let Some(snapshot) = page.snapshot { break snapshot; }
            assert_eq!(page.status, Some(SnapshotCursorStatus::Incomplete));
        };
        let eager = EnvironmentalMaterialSnapshot::from_field(&field, 77);
        assert_eq!(snapshot, eager);
        assert!(total > 2);
    }

    #[test]
    fn revision_change_explicitly_invalidates_cursor() {
        let mut field = ActiveMaterialField::new(20.0, 10.0, 10.0);
        field.cells[0].physical_materials.push(physical(1, "Carbon"));
        let mut cursor = EnvironmentalSnapshotCursor::new(&field, 1);
        let first = cursor.next_page(&field, 1);
        assert_eq!(first.status, Some(SnapshotCursorStatus::Incomplete));
        field.revision = field.revision.wrapping_add(1);
        let stale = cursor.next_page(&field, 1);
        assert_eq!(stale.status, Some(SnapshotCursorStatus::Invalidated));
        assert!(stale.snapshot.is_none());
    }
}
