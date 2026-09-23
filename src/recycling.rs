use crate::energy_ledger::EnergyLedgerAuthority;
use crate::state::{EnergyLedger, Environment, Organism};

pub(crate) fn recycle_dead_organism(
    environment: &mut Environment,
    organism: &mut Organism,
    ledger: &mut EnergyLedger,
) -> Option<crate::decomposition::DecomposingBody> {
    let position = organism.occupied_cells.first().cloned()?;
    for entry in organism.stored_material.drain_entries() {
        match entry {
            crate::material_storage::StoredMaterial::Logical(material) => {
                environment.field.deposit(position.x, position.y, material);
            }
            crate::material_storage::StoredMaterial::Physical(material) => {
                environment.field.deposit(position.x, position.y, material);
            }
        }
    }
    if let Some(mut construction) = organism.reproductive_construction.take() {
        for entry in construction.committed_material.drain_entries() {
            match entry {
                crate::material_storage::StoredMaterial::Logical(material) => {
                    environment.field.deposit(position.x, position.y, material);
                }
                crate::material_storage::StoredMaterial::Physical(material) => {
                    environment.field.deposit(position.x, position.y, material);
                }
            }
        }
    }
    let mut body =
        crate::decomposition::DecomposingBody::new(organism.structure.clone(), 0.0, position)?;
    let energy = organism.usable_energy;
    if !ledger.transfer(&mut organism.usable_energy, &mut body.energy_budget, energy) {
        return None;
    }
    Some(body)
}
