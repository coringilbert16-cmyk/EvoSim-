use crate::energy_ledger::EnergyLedgerAuthority;
use crate::state::{EnergyLedger, Environment, Organism};

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Kept disconnected until canonical post-death chemistry replaces legacy carriers"
    )
)]
pub(crate) fn recycle_dead_organism(
    environment: &mut Environment,
    organism: &mut Organism,
    ledger: &mut EnergyLedger,
) -> Option<crate::decomposition::DecomposingBody> {
    let position = organism.occupied_cells.first().cloned()?;
    let energy = organism.usable_energy;
    // Construction membership must be either fully parent-owned or fully
    // detached. Partial overlap would make recycling drop unowned members or
    // count physical state twice, so reject it before staging any transfers.
    if organism
        .reproductive_construction
        .as_ref()
        .is_some_and(|construction| {
            construction.has_any_member_in(&organism.structure)
                && !construction.is_aggregate_owned_by(&organism.structure)
        })
    {
        return None;
    }
    // Once the construction members are part of the parent aggregate, the
    // construction record is only a view: its legacy energy field is not a
    // second physical energy holder and must not be recycled twice.
    let construction_has_separate_energy_owner = organism
        .reproductive_construction
        .as_ref()
        .is_some_and(|construction| {
            !construction.is_aggregate_owned_by(&organism.structure)
                && !construction.has_any_member_in(&organism.structure)
        });
    if organism
        .reproductive_construction
        .as_ref()
        .is_some_and(|construction| {
            !construction_has_separate_energy_owner && construction.developing_energy != 0.0
        })
    {
        // Reject ambiguous legacy energy rather than clearing a second scalar
        // holder that conflicts with the parent's authoritative energy balance.
        return None;
    }
    let developing_energy = organism
        .reproductive_construction
        .as_ref()
        .filter(|_| construction_has_separate_energy_owner)
        .map_or(0.0, |construction| construction.developing_energy);
    let total_energy = energy + developing_energy;
    if !energy.is_finite()
        || energy < 0.0
        || !developing_energy.is_finite()
        || developing_energy < 0.0
        || !total_energy.is_finite()
    {
        return None;
    }

    // Stage every ownership change against local copies. No material is
    // removed from its live holder and no energy counter changes until all
    // destinations and both energy transfers have passed validation.
    let mut staged_storage = organism.stored_material.clone();
    let mut staged_construction = organism.reproductive_construction.clone();
    let mut deposits = Vec::new();
    for entry in staged_storage.drain_entries() {
        let crate::material_storage::StoredMaterial::Physical(material) = entry;
        deposits.push((position.x, position.y, material));
    }
    if let Some(construction) = staged_construction.as_mut() {
        for entry in construction.committed_material.drain_entries() {
            let crate::material_storage::StoredMaterial::Physical(material) = entry;
            deposits.push((position.x, position.y, material));
        }
    }

    let mut body =
        crate::decomposition::DecomposingBody::new(organism.structure.clone(), 0.0, position)?;
    let mut staged_ledger = *ledger;
    let mut staged_parent_energy = energy;
    let mut staged_developing_energy = developing_energy;
    if !staged_ledger.transfer(&mut staged_parent_energy, &mut body.energy_budget, energy) {
        return None;
    }
    if let Some(construction) = staged_construction.as_mut() {
        if !staged_ledger.transfer(
            &mut staged_developing_energy,
            &mut body.energy_budget,
            developing_energy,
        ) {
            return None;
        }
        construction.developing_energy = staged_developing_energy;
    }

    // Batch deposit validates every destination and object before mutating the
    // field. After this succeeds, the remaining holder/ledger assignments are
    // infallible and commit the same ownership transition.
    if !environment.field.deposit_physical_batch(deposits) {
        return None;
    }
    organism.usable_energy = staged_parent_energy;
    organism.stored_material = staged_storage;
    organism.reproductive_construction = None;
    *ledger = staged_ledger;
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Material;
    use crate::state::Simulation;

    #[test]
    fn aggregate_owned_recycling_does_not_count_a_second_developing_energy_holder() {
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
        assert_eq!(
            organism
                .reproductive_construction
                .as_ref()
                .expect("construction started")
                .developing_energy,
            0.0
        );

        let body = recycle_dead_organism(&mut simulation.environment, &mut organism, &mut ledger)
            .expect("organism can be recycled");

        assert_eq!(body.energy_budget, 5.0);
        assert_eq!(organism.usable_energy, 0.0);
        assert!(organism.reproductive_construction.is_none());
    }

    #[test]
    fn aggregate_owned_construction_energy_is_recycled_only_once() {
        let mut simulation = Simulation::new(31, 20.0);
        let mut organism = simulation.organisms.remove(0);
        organism.usable_energy = 5.0;
        let aggregate = organism.structure.clone();
        let member_ids = aggregate.physical_constituent_ids();
        organism.reproductive_construction = Some(crate::state::ReproductiveConstruction {
            committed_material: crate::material_storage::MaterialStorage::default(),
            developing_structure: aggregate,
            developing_member_ids: member_ids,
            child_genome: organism.genome.clone(),
            developmental_origin: organism.developmental_origin.clone(),
            developmental_orientation_radians: organism.developmental_orientation_radians,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: organism.structure.physical_id(0),
            developing_energy: 0.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        });
        let mut ledger = EnergyLedger::default();

        let body = recycle_dead_organism(&mut simulation.environment, &mut organism, &mut ledger)
            .expect("aggregate-owned organism can be recycled");

        assert_eq!(body.energy_budget, 5.0);
        assert_eq!(organism.usable_energy, 0.0);
        assert!(organism.reproductive_construction.is_none());
    }

    #[test]
    fn partial_construction_membership_blocks_recycling_without_losing_state() {
        let mut simulation = Simulation::new(41, 20.0);
        let mut organism = simulation.organisms.remove(0);
        assert!(organism.store_material(Material::free_base("Carbon", 1.0)));
        let mut ledger = EnergyLedger::default();
        assert!(crate::reproduction::begin_reproduction(
            &mut organism,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut ledger,
        ));
        let construction = organism.reproductive_construction.as_mut().unwrap();
        let first_member = construction.developing_member_ids[0];
        construction.developing_member_ids.push(first_member);
        let original_structure = organism.structure.clone();
        let original_storage = organism.stored_material.clone();
        let original_construction = organism.reproductive_construction.clone();
        let original_ledger = ledger;

        assert!(
            recycle_dead_organism(&mut simulation.environment, &mut organism, &mut ledger)
                .is_none()
        );
        assert_eq!(organism.structure.units, original_structure.units);
        assert_eq!(organism.structure.bonds, original_structure.bonds);
        assert_eq!(organism.stored_material, original_storage);
        assert_eq!(
            organism
                .reproductive_construction
                .as_ref()
                .unwrap()
                .developing_member_ids,
            original_construction
                .as_ref()
                .unwrap()
                .developing_member_ids
        );
        assert_eq!(ledger, original_ledger);
    }

    #[test]
    fn ambiguous_aggregate_construction_energy_blocks_recycling() {
        let mut simulation = Simulation::new(37, 20.0);
        let mut organism = simulation.organisms.remove(0);
        organism.usable_energy = 5.0;
        let aggregate = organism.structure.clone();
        let member_ids = aggregate.physical_constituent_ids();
        organism.reproductive_construction = Some(crate::state::ReproductiveConstruction {
            committed_material: crate::material_storage::MaterialStorage::default(),
            developing_structure: aggregate,
            developing_member_ids: member_ids,
            child_genome: organism.genome.clone(),
            developmental_origin: organism.developmental_origin.clone(),
            developmental_orientation_radians: organism.developmental_orientation_radians,
            developing_stress: 0.0,
            anchor_unit_index: 0,
            anchor_physical_id: organism.structure.physical_id(0),
            developing_energy: 3.0,
            developing_maintenance_debt: 0.0,
            needs_space: false,
        });
        let original_structure = organism.structure.clone();
        let mut ledger = EnergyLedger::default();
        let original_ledger = ledger;

        assert!(
            recycle_dead_organism(&mut simulation.environment, &mut organism, &mut ledger)
                .is_none()
        );
        assert_eq!(organism.usable_energy, 5.0);
        assert_eq!(organism.structure.units, original_structure.units);
        assert_eq!(organism.structure.bonds, original_structure.bonds);
        assert_eq!(
            organism
                .reproductive_construction
                .as_ref()
                .unwrap()
                .developing_energy,
            3.0
        );
        assert_eq!(ledger, original_ledger);
    }

    #[test]
    fn failed_dead_recycling_deposit_preserves_all_material_and_energy_owners() {
        let mut simulation = Simulation::new(29, 20.0);
        let mut organism = simulation.organisms.remove(0);
        assert!(organism.store_material(Material::free_base("Carbon", 1.0)));
        assert!(crate::reproduction::begin_reproduction(
            &mut organism,
            &mut simulation.rng,
            &simulation.environment.catalog,
            &mut EnergyLedger::default(),
        ));
        organism.usable_energy = 5.0;
        let construction = organism
            .reproductive_construction
            .as_mut()
            .expect("construction started");
        construction.developing_energy = 0.0;
        assert!(construction.committed_material.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &simulation.environment.catalog,
        ));
        organism.occupied_cells[0].x = 1.0e9;

        let storage_before = organism.stored_material.clone();
        let construction_before = organism
            .reproductive_construction
            .as_ref()
            .map(|construction| {
                (
                    construction.committed_material.clone(),
                    construction.developing_structure.units.clone(),
                    construction.developing_structure.bonds.clone(),
                    construction.developing_energy,
                )
            })
            .expect("construction started");
        let energy_before = organism.usable_energy;
        let mut ledger = EnergyLedger::default();
        let ledger_before = ledger;
        let field_before = simulation.environment.field.total_amount();

        assert!(
            recycle_dead_organism(&mut simulation.environment, &mut organism, &mut ledger,)
                .is_none()
        );

        assert_eq!(organism.stored_material, storage_before);
        let construction_after = organism
            .reproductive_construction
            .as_ref()
            .expect("failed recycling must retain construction");
        assert_eq!(construction_after.committed_material, construction_before.0);
        assert_eq!(
            construction_after.developing_structure.units,
            construction_before.1
        );
        assert_eq!(
            construction_after.developing_structure.bonds,
            construction_before.2
        );
        assert_eq!(construction_after.developing_energy, construction_before.3);
        assert_eq!(organism.usable_energy, energy_before);
        assert_eq!(ledger, ledger_before);
        assert_eq!(simulation.environment.field.total_amount(), field_before);
    }
}
