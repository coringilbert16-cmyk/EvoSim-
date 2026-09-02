#[cfg(test)]
mod conservation_tests {
    use crate::environment::{
        apply_settling, apply_vents, ActiveMaterialField, DeepReservoir, Vent,
        DEFAULT_SETTLING_FRACTION, DEFAULT_SETTLING_INTERVAL_TICKS,
    };
    use crate::resources::Material;
    use crate::state::{ActiveTransformation, Snapshot, TransformationKind, Simulation};
    use crate::structure::{Bond, BondId, Placement, StructuralUnit};

    fn material_totals(
        field: &ActiveMaterialField,
        reservoir: &DeepReservoir,
    ) -> Vec<(String, f64)> {
        let mut totals = field.total_material();
        for (name, amount) in reservoir.total_material() {
            if let Some(existing) = totals.iter_mut().find(|(n, _)| n == &name) {
                existing.1 += amount;
            } else {
                totals.push((name, amount));
            }
        }
        totals.sort_by(|a, b| a.0.cmp(&b.0));
        totals
    }

    #[test]
    fn initial_seed_randomization_preserves_each_resource_total() {
        let field = ActiveMaterialField::new(1000.0, 1000.0, 25.0);
        let mut reservoir = DeepReservoir::new_matching_field(&field, 5);
        reservoir.seed_uniform("Carbon", 10_000.0);
        reservoir.seed_uniform("Methane", 5_000.0);
        reservoir.seed_uniform("Water", 20_000.0);

        let before = reservoir.total_material();
        reservoir.randomize_unbonded_distribution(12345);
        let after = reservoir.total_material();

        for (name, amount) in before {
            let actual = after
                .iter()
                .find(|(after_name, _)| after_name == &name)
                .map(|(_, value)| *value)
                .unwrap_or(0.0);
            assert!(
                (amount - actual).abs() < 1e-9,
                "resource {name} changed by {}",
                actual - amount
            );
        }
    }

    #[test]
    fn environment_transfer_conserves_each_resource_and_bonded_state() {
        let field = ActiveMaterialField::new(200.0, 200.0, 25.0);
        let mut reservoir = DeepReservoir::new_matching_field(&field, 2);
        let mut field = field;
        let source = field.index_for_position(50.0, 50.0).unwrap();
        let reservoir_index = reservoir.reservoir_index_for_field_index(&field, source);

        reservoir.cells[reservoir_index].add(false, "Carbon", 500.0);
        reservoir.cells[reservoir_index].add(true, "Carbon", 300.0);
        reservoir.cells[reservoir_index].add(false, "Methane", 200.0);
        reservoir.cells[reservoir_index].add(true, "Methane", 100.0);

        let before = material_totals(&field, &reservoir);
        let mut vents = vec![Vent {
            x: 50.0,
            y: 50.0,
            composition: vec![("Carbon".into(), 0.5), ("Methane".into(), 0.5)],
            emission_amount: 120.0,
            emission_interval: 0,
            emission_timer: 0,
        }];

        for tick in 0..200u64 {
            apply_vents(&mut field, &mut reservoir, &mut vents);
            field.diffuse_step(0.1);
            if tick % DEFAULT_SETTLING_INTERVAL_TICKS == 0 {
                apply_settling(&mut field, &mut reservoir, DEFAULT_SETTLING_FRACTION);
            }
        }

        let after = material_totals(&field, &reservoir);
        assert_eq!(before.len(), after.len());
        for (name, amount) in before {
            let actual = after
                .iter()
                .find(|(after_name, _)| after_name == &name)
                .map(|(_, value)| *value)
                .unwrap_or(0.0);
            assert!(
                (amount - actual).abs() < 1e-6,
                "resource {name} changed by {}",
                actual - amount
            );
        }

        let bonded = field
            .cells
            .iter()
            .map(|cell| cell.bonded.total_amount())
            .sum::<f64>()
            + reservoir
                .cells
                .iter()
                .map(|cell| {
                    cell.bonded_entries
                        .iter()
                        .map(|(_, amount)| *amount)
                        .sum::<f64>()
                })
                .sum::<f64>();
        let unbonded = field
            .cells
            .iter()
            .map(|cell| cell.unbonded.total_amount())
            .sum::<f64>()
            + reservoir
                .cells
                .iter()
                .map(|cell| {
                    cell.unbonded_entries
                        .iter()
                        .map(|(_, amount)| *amount)
                        .sum::<f64>()
                })
                .sum::<f64>();
        assert!((bonded - 400.0).abs() < 1e-6);
        assert!((unbonded - 700.0).abs() < 1e-6);
    }

    #[test]
    fn simulation_material_conservation_includes_organism_storage_and_structure() {
        let mut sim = Simulation::new(42, 10.0);
        let before = sim.total_material_in_system();
        for _ in 0..1000 {
            sim.step();
        }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }

    #[test]
    fn raw_material_take_and_store_preserve_amount() {
        let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        field.deposit_at_index(5, Material::free_base("Carbon", 25.0));
        let taken = field.take_at_index(5, false, 7.5).expect("material exists");
        assert!((taken.total_amount() - 7.5).abs() < 1e-12);
        assert!((field.total_amount() - 17.5).abs() < 1e-12);
    }

    #[test]
    fn snapshot_round_trip_preserves_stable_bond_and_transformation_identity() {
        let mut sim = Simulation::new(7, 10.0);
        let organism = &mut sim.organisms[0];
        organism
            .structure
            .add_bond(Bond {
                id: BondId(41),
                unit_a: 0,
                point_a: 0,
                unit_b: 1,
                point_b: 0,
                strength: 0.75,
                bond_energy: 2.5,
            });
        let bond_id = organism.structure.bonds[0].id;
        sim.active_transformations.push(ActiveTransformation {
            id: 9,
            organism_id: "1".into(),
            kind: TransformationKind::Break,
            material: Material::free_base("Carbon", 1.0),
            bond_id: Some(bond_id),
            legacy_bond: None,
            complexity: 0.5,
            duration_ticks: 3,
            remaining_ticks: 2,
            decision_context_key: Some("bond:41".into()),
        });

        let encoded = serde_json::to_string(&sim.snapshot()).expect("snapshot serializes");
        let decoded: Snapshot = serde_json::from_str(&encoded).expect("snapshot deserializes");

        let decoded_bond = decoded.organisms[0]
            .structure
            .bond_by_id(bond_id)
            .expect("bond id survives snapshot round trip");
        assert_eq!(decoded_bond.id, bond_id);
        assert_eq!(decoded_bond.bond_energy, 2.5);
        assert_eq!(decoded.active_transformations[0].bond_id, Some(bond_id));
        assert_eq!(decoded.active_transformations[0].decision_context_key.as_deref(), Some("bond:41"));

        let mut restored_structure = decoded.organisms[0].structure.clone();
        let new_index = restored_structure.add_bond(Bond {
            id: BondId(0),
            unit_a: 2,
            point_a: 0,
            unit_b: 3,
            point_b: 0,
            strength: 0.5,
            bond_energy: 1.0,
        });
        assert_eq!(restored_structure.bonds[new_index].id, BondId(42));
    }

    #[test]
    fn legacy_zero_bond_ids_are_repaired_without_losing_bond_state() {
        let mut structure = crate::structure::OrganismStructure::new();
        for i in 0..2 {
            structure.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: i as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        structure.add_bond(Bond {
            id: BondId(0),
            unit_a: 0,
            point_a: 0,
            unit_b: 1,
            point_b: 0,
            strength: 0.25,
            bond_energy: 3.0,
        });

        let mut value = serde_json::to_value(&structure).expect("structure serializes");
        value["bonds"][0].as_object_mut().unwrap().remove("id");
        let mut decoded: crate::structure::OrganismStructure =
            serde_json::from_value(value).expect("legacy structure deserializes");
        assert_eq!(decoded.bonds[0].id, BondId(0));

        decoded.ensure_bond_ids();
        assert_ne!(decoded.bonds[0].id, BondId(0));
        assert_eq!(decoded.bonds[0].strength, 0.25);
        assert_eq!(decoded.bonds[0].bond_energy, 3.0);
    }
}
