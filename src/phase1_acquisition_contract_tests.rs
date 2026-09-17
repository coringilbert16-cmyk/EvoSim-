#[cfg(test)]
mod tests {
    use crate::environment::ActiveMaterialField;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::{BaseResource, Form, InternalBond, Material, PhysicalState, ResourceProperties, Shape};
    use crate::structure::Placement;

    fn catalog() -> Vec<BaseResource> {
        vec![
            BaseResource {
                name: "Carbon".into(),
                properties: ResourceProperties { mass: 1.0, potential_energy: 3.0, reactivity: 1.0, cohesion: 1.0 },
                physical_state: PhysicalState::Rigid,
                shape: Shape { form: Form::Circle { radius: 1.0 }, connection_points: vec![] },
            },
            BaseResource {
                name: "Hydrogen".into(),
                properties: ResourceProperties { mass: 1.0, potential_energy: 5.0, reactivity: 1.0, cohesion: 1.0 },
                physical_state: PhysicalState::Rigid,
                shape: Shape { form: Form::Circle { radius: 1.0 }, connection_points: vec![] },
            },
        ]
    }

    #[test]
    fn acquisition_transfers_realized_composite_intact() {
        let catalog = catalog();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let placements = vec![
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
            Placement { x: 2.0, y: 0.5, rotation_radians: 0.25 },
        ];
        let physical = PhysicalMaterial::realized(material.clone(), placements.clone(), &catalog).unwrap();
        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        assert!(field.deposit_physical_at_index(0, physical));

        let acquired = field.take_physical_for_acquisition(0).unwrap();
        assert_eq!(acquired.material, material);
        assert_eq!(acquired.placements, Some(placements));
        assert_eq!(field.cells[0].physical_materials.len(), 0);
    }

    #[test]
    fn logical_structured_material_is_not_promoted_to_physical_acquisition() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        field.deposit_at_index(0, material);
        assert!(field.take_physical_for_acquisition(0).is_none());
    }
}
