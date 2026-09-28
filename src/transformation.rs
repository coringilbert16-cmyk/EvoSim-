        };
        if bond_index >= bond_count {
            return None;
        }

        let removed = organism.stored_material.entries.swap_remove(storage_index);
        let stored_material = match removed {
            crate::material_storage::StoredMaterial::Physical(instance) => instance,
            _ => return None,
        };
        let stored_bond = stored_material
            .internal_connections
            .as_ref()?
            .get(bond_index)?
            .clone();

        let complexity = crate::math::complexity(2.0);