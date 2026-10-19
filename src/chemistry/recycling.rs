use crate::energy_ledger::EnergyLedgerAuthority;
use crate::state::{EnergyLedger, Environment, Organism};

pub(crate) fn recycle_dead_organism(
    environment: &mut Environment,
    organism: &mut Organism,
    ledger: &mut EnergyLedger,
) -> Option<crate::decomposition::DecomposingBody> {
    let position = organism.occupied_cells.first().cloned()?;
    let developing_energy = organism
        .reproductive_construction
        .as_ref()
        .map_or(0.0, |construction| construction.developing_energy);
    let energy = organism.usable_energy;
    let total_energy = energy + developing_energy;
    // Validate both stores before moving material or energy, so malformed
    // state cannot leave a partially recycled organism.
    if !energy.is_finite()
        || energy < 0.0
        || !developing_energy.is_finite()
        || developing_energy < 0.0
        || !total_energy.is_finite()
    {
        return None;
    }

    for entry in organism.stored_material.drain_entries() {
        let crate::material_storage::StoredMaterial::Physical(material) = entry;
        environment.field.deposit(position.x, position.y, material);
    }
    let mut construction = organism.reproductive_construction.take();
    if let Some(construction) = construction.as_mut() {
        for entry in construction.committed_material.drain_entries() {
            let crate::material_storage::StoredMaterial::Physical(material) = entry;
            environment.field.deposit(position.x, position.y, material);
        }
    }

    let mut body =
        crate::decomposition::DecomposingBody::new(organism.structure.clone(), 0.0, position)?;
    if !ledger.transfer(&mut organism.usable_energy, &mut body.energy_budget, energy) {
        return None;
    }
    if let Some(construction) = construction.as_mut() {
        if !ledger.transfer(
            &mut construction.developing_energy,
            &mut body.energy_budget,
            developing_energy,
        ) {
            return None;
        }
    }
    Some(body)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Material;
    use crate::state::Simulation;

    #[test]
    fn recycling_preserves_parent_and_developing_offspring_energy() {
        let mut simulation = Simulation::new(71, 20.0);
        let mut organism = simulation.organisms.remove(0);
        assert!(organism.store_material(Material::free_base("Carbon", 1.0)));
        let mut ledger = EnergyLedger::default();
        assert!(crate::reproduction::begin_reproduction(
            &mut organism,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        organism.usable_energy = 5.0;
        organism
            .reproductive_construction
            .as_mut()
            .expect("construction started")
            .developing_energy = 3.0;

        let body = recycle_dead_organism(
            &mut simulation.environment,
            &mut organism,
            &mut ledger,
        )
        .expect("organism can be recycled");

        assert_eq!(body.energy_budget, 8.0);
        assert_eq!(organism.usable_energy, 0.0);
        assert!(organism.reproductive_construction.is_none());
    }
}
