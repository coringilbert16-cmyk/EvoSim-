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

    let mut body =
        crate::decomposition::DecomposingBody::new(organism.structure.clone(), 0.0, position)?;

    // Stage every energy transfer against local holder values and a copied
    // ledger. A rejection in either transfer must not leave material drained,
    // construction removed, or the first transfer committed on its own.
    let mut staged_ledger = *ledger;
    let mut staged_usable_energy = organism.usable_energy;
    let mut staged_developing_energy = developing_energy;
    let mut staged_body_energy = body.energy_budget;
    if !staged_ledger.transfer(
        &mut staged_usable_energy,
        &mut staged_body_energy,
        energy,
    ) {
        return None;
    }
    if !staged_ledger.transfer(
        &mut staged_developing_energy,
        &mut staged_body_energy,
        developing_energy,
    ) {
        return None;
    }

    // The full energy transaction is now known to be valid. Commit holders
    // and ledger together, then transfer stored material to the environment.
    organism.usable_energy = staged_usable_energy;
    if let Some(construction) = organism.reproductive_construction.as_mut() {
        construction.developing_energy = staged_developing_energy;
    }
    *ledger = staged_ledger;
    body.energy_budget = staged_body_energy;

    for entry in organism.stored_material.drain_entries() {
        let crate::material_storage::StoredMaterial::Physical(material) = entry;
        environment.field.deposit(position.x, position.y, material);
    }
    if let Some(mut construction) = organism.reproductive_construction.take() {
        for entry in construction.committed_material.drain_entries() {
            let crate::material_storage::StoredMaterial::Physical(material) = entry;
            environment.field.deposit(position.x, position.y, material);
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
        let mut simulation = Simulation::new(23, 20.0);
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

        let body = recycle_dead_organism(&mut simulation.environment, &mut organism, &mut ledger)
            .expect("organism can be recycled");

        assert_eq!(body.energy_budget, 8.0);
        assert_eq!(organism.usable_energy, 0.0);
        assert!(organism.reproductive_construction.is_none());
    }
    #[test]
    fn rejected_recycling_energy_settlement_preserves_all_material_owners() {
        let mut simulation = Simulation::new(24, 20.0);
        let mut organism = simulation.organisms.remove(0);
        assert!(organism.store_material(Material::free_base("Carbon", 2.0)));
        let stored_before = organism.stored_material.entries.clone();
        let usable_before = organism.usable_energy;
        let field_before = simulation.environment.field.total_amount();
        let mut ledger = EnergyLedger {
            total_heat_dissipated: f64::NAN,
            ..EnergyLedger::default()
        };

        assert!(
            recycle_dead_organism(&mut simulation.environment, &mut organism, &mut ledger).is_none(),
            "invalid ledger must reject recycling before any holder commits"
        );
        assert_eq!(organism.stored_material.entries, stored_before);
        assert_eq!(organism.usable_energy, usable_before);
        assert_eq!(simulation.environment.field.total_amount(), field_before);
        assert!(ledger.total_heat_dissipated.is_nan());
    }

}
