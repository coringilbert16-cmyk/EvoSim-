#[cfg(test)]
mod tests {
    use crate::environment::ActiveMaterialField;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::{
        BaseResource, Form, InternalBond, Material, PhysicalState, ResourceProperties, Shape,
    };
    use crate::simulation::Simulation;
    use crate::structure::Placement;

    fn catalog() -> Vec<BaseResource> {
        vec![
            BaseResource {
                name: "Carbon".into(),
                properties: ResourceProperties {
                    mass: 1.0,
                    potential_energy: 3.0,
                    reactivity: 1.0,
                    cohesion: 1.0,
                },
                physical_state: PhysicalState::Rigid,
                shape: Shape {
                    form: Form::Circle { radius: 1.0 },
                    connection_points: vec![],
                },
            },
            BaseResource {
                name: "Hydrogen".into(),
                properties: ResourceProperties {
                    mass: 1.0,
                    potential_energy: 5.0,
                    reactivity: 1.0,
                    cohesion: 1.0,
                },
                physical_state: PhysicalState::Rigid,
                shape: Shape {
                    form: Form::Circle { radius: 1.0 },
                    connection_points: vec![],
                },
            },
        ]
    }

    fn compound() -> (Material, Vec<Placement>) {
        (
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 2.0,
                    y: 0.5,
                    rotation_radians: 0.25,
                },
            ],
        )
    }

    #[test]
    fn acquisition_transfers_realized_composite_intact() {
        let catalog = catalog();
        let (material, placements) = compound();
        let physical =
            PhysicalMaterial::realized(material.clone(), placements.clone(), &catalog).unwrap();
        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        assert!(field.deposit_physical_at_index(0, physical));
        let acquired = field.take_physical_for_acquisition(0).unwrap();
        assert_eq!(acquired.material, material);
        assert_eq!(acquired.placements, Some(placements));
        assert_eq!(field.cells[0].physical_materials.len(), 0);
    }

    #[test]
    fn logical_structured_material_is_not_promoted_to_physical_acquisition() {
        let (material, _) = compound();
        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        field.deposit_at_index(0, material);
        assert!(field.take_physical_for_acquisition(0).is_none());
    }

    #[test]
    fn organism_acquire_target_preserves_realized_composite() {
        let mut simulation = Simulation::new(7, 20.0);
        let organism_position = simulation.organisms[0].occupied_cells[0].clone();
        let field_index = simulation
            .environment
            .field
            .index_for_position(organism_position.x, organism_position.y)
            .unwrap();
        let (material, placements) = compound();
        let physical = PhysicalMaterial::realized(
            material.clone(),
            placements.clone(),
            &simulation.environment.catalog,
        )
        .unwrap();
        assert!(simulation
            .environment
            .field
            .deposit_physical_at_index(field_index, physical));
        assert!(Simulation::acquire_target(
            &mut simulation.organisms[0],
            &mut simulation.environment,
            field_index
        ));
        let acquired = simulation.organisms[0]
            .stored_material
            .peek_matching_physical(&material)
            .unwrap();
        assert_eq!(acquired.material, material);
        assert_eq!(acquired.placements, Some(placements));
        assert!(simulation.environment.field.cells[field_index]
            .physical_materials
            .is_empty());
    }
}
