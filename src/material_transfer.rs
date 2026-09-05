//! Material transfer through established physical cross-organism connections.
//!
//! A connection makes transfer physically possible; it does not make transfer
//! automatic. A caller must explicitly request a directed transfer. Only
//! unbonded material may be transferred by this primitive; bonded material
//! remains subject to the separate interaction/chemistry systems.

use crate::cell_connection::{CellConnection, CellSiteRef};
use crate::resources::{merge_parts, Material};
use crate::state::Organism;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialTransferError {
    EndpointsNotConnected,
    EndpointOwnershipMismatch,
    BondedMaterialCannotTransfer,
    InvalidAmount,
    InsufficientMaterial,
}

/// Transfer unbonded material between two organisms whose physical sites are
/// joined by the supplied connection.
///
/// The organisms themselves own the material stores. This prevents a caller
/// from pairing a valid connection with an unrelated mutable `Material` and
/// thereby bypassing physical ownership. The connection remains only the
/// physical gate: direction and amount are still supplied explicitly by the
/// caller, so establishing a connection never causes material to move by
/// itself.
///
/// The transfer is atomic: if validation fails, neither organism is changed.
pub(crate) fn transfer_unbonded_material(
    connection: &CellConnection,
    sender_site: CellSiteRef,
    receiver_site: CellSiteRef,
    sender: &mut Organism,
    receiver: &mut Organism,
    amount: f64,
) -> Result<(), MaterialTransferError> {
    if !connection.connects(sender_site.clone(), receiver_site.clone()) {
        return Err(MaterialTransferError::EndpointsNotConnected);
    }
    if sender.id != sender_site.organism_id || receiver.id != receiver_site.organism_id {
        return Err(MaterialTransferError::EndpointOwnershipMismatch);
    }
    if sender.id == receiver.id {
        return Err(MaterialTransferError::EndpointOwnershipMismatch);
    }
    if sender.stored_unbonded.bonded || receiver.stored_unbonded.bonded {
        return Err(MaterialTransferError::BondedMaterialCannotTransfer);
    }
    if !amount.is_finite() || amount <= 0.0 {
        return Err(MaterialTransferError::InvalidAmount);
    }
    if sender.stored_unbonded.total_amount() + 1e-12 < amount {
        return Err(MaterialTransferError::InsufficientMaterial);
    }

    let transferred = sender
        .stored_unbonded
        .take(amount)
        .ok_or(MaterialTransferError::InsufficientMaterial)?;

    let mut parts = std::mem::take(&mut receiver.stored_unbonded.parts);
    parts.extend(transferred.parts);
    receiver.stored_unbonded.parts = merge_parts(&parts);
    receiver.stored_unbonded.bonded = false;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::state::{DevelopmentStage, Position, ResourceSense};
    use crate::structure::{Placement, OrganismStructure, StructuralUnit};

    fn site(organism_id: &str, unit_index: usize, point_index: usize) -> CellSiteRef {
        CellSiteRef { organism_id: organism_id.into(), unit_index, point_index }
    }

    fn carbon_structure(x: f64, y: f64) -> OrganismStructure {
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x, y, rotation_radians: 0.0 },
        ));
        structure
    }

    fn organism(id: &str, structure: OrganismStructure, material: Material) -> Organism {
        Organism {
            id: id.into(),
            occupied_cells: vec![Position { x: 0.0, y: 0.0 }],
            genome: crate::genome::initial_genome(),
            resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 0.0,
            stress: 0.0,
            stored_unbonded: material,
            structure,
            development_stage: DevelopmentStage::Juvenile,
            age: 0,
            reproductive_readiness: 0.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    fn connection() -> CellConnection {
        let catalog = default_catalog();
        CellConnection::try_establish(
            site("sender", 0, 0),
            site("receiver", 0, 3),
            &carbon_structure(0.0, 0.0),
            &carbon_structure(1.0, 0.0),
            &catalog,
            0.25,
            0.0,
        )
        .expect("test endpoints should be in physical contact")
    }

    #[test]
    fn connected_transfer_moves_only_requested_amount() {
        let connection = connection();
        let sender_site = site("sender", 0, 0);
        let receiver_site = site("receiver", 0, 3);
        let mut sender = organism("sender", carbon_structure(0.0, 0.0), Material::free_base("Carbon", 10.0));
        let mut receiver = organism("receiver", carbon_structure(1.0, 0.0), Material::free_base("Nitrogen", 2.0));

        transfer_unbonded_material(
            &connection,
            sender_site,
            receiver_site,
            &mut sender,
            &mut receiver,
            3.0,
        )
        .expect("connected unbonded transfer should succeed");

        assert!((sender.stored_unbonded.total_amount() - 7.0).abs() < 1e-9);
        assert!((receiver.stored_unbonded.total_amount() - 5.0).abs() < 1e-9);
        assert_eq!(receiver.stored_unbonded.parts.len(), 2);
    }

    #[test]
    fn mismatched_material_owner_cannot_transfer() {
        let connection = connection();
        let mut sender = organism("not-the-sender", carbon_structure(0.0, 0.0), Material::free_base("Carbon", 10.0));
        let mut receiver = organism("receiver", carbon_structure(1.0, 0.0), Material::free_base("Nitrogen", 0.0));
        let sender_before = sender.stored_unbonded.clone();
        let receiver_before = receiver.stored_unbonded.clone();

        let result = transfer_unbonded_material(
            &connection,
            site("sender", 0, 0),
            site("receiver", 0, 3),
            &mut sender,
            &mut receiver,
            3.0,
        );

        assert_eq!(result, Err(MaterialTransferError::EndpointOwnershipMismatch));
        assert_eq!(sender.stored_unbonded, sender_before);
        assert_eq!(receiver.stored_unbonded, receiver_before);
    }

    #[test]
    fn reversed_direction_must_match_organism_ownership() {
        let connection = connection();
        let mut sender = organism("sender", carbon_structure(0.0, 0.0), Material::free_base("Carbon", 10.0));
        let mut receiver = organism("receiver", carbon_structure(1.0, 0.0), Material::free_base("Nitrogen", 0.0));

        let result = transfer_unbonded_material(
            &connection,
            site("receiver", 0, 3),
            site("sender", 0, 0),
            &mut sender,
            &mut receiver,
            3.0,
        );

        assert_eq!(result, Err(MaterialTransferError::EndpointOwnershipMismatch));
        assert!((sender.stored_unbonded.total_amount() - 10.0).abs() < 1e-9);
        assert!(receiver.stored_unbonded.is_empty());
    }

    #[test]
    fn disconnected_endpoints_cannot_transfer() {
        let connection = connection();
        let mut sender = organism("sender", carbon_structure(0.0, 0.0), Material::free_base("Carbon", 10.0));
        let mut receiver = organism("receiver", carbon_structure(1.0, 0.0), Material::free_base("Nitrogen", 0.0));
        let sender_before = sender.stored_unbonded.clone();
        let receiver_before = receiver.stored_unbonded.clone();

        let result = transfer_unbonded_material(
            &connection,
            site("sender", 0, 1),
            site("receiver", 0, 3),
            &mut sender,
            &mut receiver,
            3.0,
        );

        assert_eq!(result, Err(MaterialTransferError::EndpointsNotConnected));
        assert_eq!(sender.stored_unbonded, sender_before);
        assert_eq!(receiver.stored_unbonded, receiver_before);
    }

    #[test]
    fn bonded_material_cannot_transfer() {
        let connection = connection();
        let mut sender = organism("sender", carbon_structure(0.0, 0.0), Material { parts: vec![("Carbon".into(), 10.0)], bonded: true });
        let mut receiver = organism("receiver", carbon_structure(1.0, 0.0), Material::free_base("Nitrogen", 0.0));

        let result = transfer_unbonded_material(
            &connection,
            site("sender", 0, 0),
            site("receiver", 0, 3),
            &mut sender,
            &mut receiver,
            3.0,
        );

        assert_eq!(result, Err(MaterialTransferError::BondedMaterialCannotTransfer));
        assert!((sender.stored_unbonded.total_amount() - 10.0).abs() < 1e-9);
        assert!(receiver.stored_unbonded.is_empty());
    }

    #[test]
    fn insufficient_transfer_is_atomic() {
        let connection = connection();
        let mut sender = organism("sender", carbon_structure(0.0, 0.0), Material::free_base("Carbon", 2.0));
        let mut receiver = organism("receiver", carbon_structure(1.0, 0.0), Material::free_base("Nitrogen", 1.0));
        let sender_before = sender.stored_unbonded.clone();
        let receiver_before = receiver.stored_unbonded.clone();

        let result = transfer_unbonded_material(
            &connection,
            site("sender", 0, 0),
            site("receiver", 0, 3),
            &mut sender,
            &mut receiver,
            3.0,
        );

        assert_eq!(result, Err(MaterialTransferError::InsufficientMaterial));
        assert_eq!(sender.stored_unbonded, sender_before);
        assert_eq!(receiver.stored_unbonded, receiver_before);
    }
}
