        };
        if bond_index >= bond_count {
            return None;
        }

        let removed = organism.stored_material.entries.swap_remove(storage_index);
        let stored_material = match removed {
            crate::material_storage::StoredMaterial::Physical(instance) => instance,
            _ => return None,
        };
        let material_transformed = stored_material.material.total_amount();
        let stored_bond = stored_material
            .internal_connections
            .as_ref()?
            .get(bond_index)?
            .clone();

        let complexity = crate::math::complexity(2.0);
        let duration = 1_u64.max(complexity.ceil() as u64);
        let t = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material::free_base("", 0.0),
            bond: None,
            stored_material: Some(stored_material),
            stored_bond: Some(stored_bond),
            complexity,
            duration_ticks: duration,
            remaining_ticks: duration,
            prepared_energy: None,
            pending_experience: None,
            decision_context_key: decision.context_key.clone(),
        };
        *next_id += 1;
        organism.active_transformation_id = Some(t.id);
        Some(t)
    }

    pub(crate) fn prepare_transformation(
        transformation: &mut ActiveTransformation,
        organism: &mut Organism,
        environment: &Environment,
        ledger: &mut EnergyLedger,
    ) -> bool {
        let Some(stored) = transformation.stored_material.as_ref() else {
            organism.active_transformation_id = None;
            return false;
        };
        let Some(target) = transformation.stored_bond.as_ref() else {
            organism.active_transformation_id = None;
            return false;
        };
        let Some(a) = stored
            .material
            .parts
            .get(target.part_a)
            .and_then(|(name, _)| {
                environment
                    .catalog
                    .iter()
                    .find(|resource| resource.name == *name)
                    .map(|r| r.properties)
            })
        else {
            organism.active_transformation_id = None;
            return false;
        };
        let Some(b) = stored
            .material
            .parts
            .get(target.part_b)
            .and_then(|(name, _)| {
                environment
                    .catalog
                    .iter()
                    .find(|resource| resource.name == *name)
                    .map(|r| r.properties)
            })
        else {
            organism.active_transformation_id = None;
            return false;
        };
        let Some((gross, usable, heat)) = break_energy_yield(
            a,
            b,
            water_field_amount(environment, organism),
            organism.genome.processing_efficiency(),
        ) else {
            organism.active_transformation_id = None;
            return false;
        };

        // Validate the exact bond before settling the transaction. The physical
        // structure itself is not changed until the following tick.
        if stored.break_internal_bond(target).is_none() {
            organism.active_transformation_id = None;
            return false;
        }

        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: gross,
            usable_delta: usable,
            structural_delta: 0.0,
            heat_dissipated: heat,
        };
        if !ledger.settle_transaction(&mut organism.usable_energy, tx) {
            organism.active_transformation_id = None;
            return false;
        }

        transformation.prepared_energy = Some((gross, usable, heat));
        true
    }

    pub(crate) fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        _ledger: &mut EnergyLedger,
    ) {
        let Some(stored) = transformation.stored_material.as_ref() else {
            organism.active_transformation_id = None;
            return;
        };
        let Some(target) = transformation.stored_bond.as_ref() else {
            organism.active_transformation_id = None;
            return;
        };
        if transformation.prepared_energy.is_none() {
            organism.active_transformation_id = None;
            return;
        }
        let Some(pieces) = stored.break_internal_bond(target) else {
            organism.active_transformation_id = None;
            return;
        };

        for piece in pieces {
            if !organism
                .stored_material
                .store_physical_instance(piece.clone())
            {
                if let Some(placement) = piece
                    .placements
                    .as_ref()
                    .and_then(|placements| placements.first())
                {
                    let _ = environment.field.deposit(placement.x, placement.y, piece);
                }
            }