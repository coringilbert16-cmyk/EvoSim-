#[cfg(test)]
mod tests {
    use crate::environment::ActiveMaterialField;
    use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::{
        BaseResource, Form, InternalBond, Material, PhysicalState, ResourceProperties, Shape,
    };
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
                },
            },
        ]
    }

    fn body() -> OrganismBodyGeometry {
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Circle { radius: 0.5 },
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: -0.5,
            max_x: 0.5,
            min_y: -0.5,
            max_y: 0.5,
        }
    }

    fn compound() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        }
    }

    #[test]
    fn logical_material_is_not_promoted_into_physical_containment() {
        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        field.deposit_at_index(0, compound());
        let contained = field.take_contained_physical_materials(&body());
        assert!(contained.is_empty());
        assert_eq!(field.cells[0].materials.len(), 1);
    }

    #[test]
    fn partially_contained_composite_remains_intact_and_accessible() {
        let catalog = catalog();
        let physical = PhysicalMaterial::realized(
            compound(),
            vec![
                Placement {
                    x: 1.8,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("test composite must have a valid physical realization");

        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        field.deposit(1.8, 0.0, physical);
        let revision_before = field.revision;
        let contained = field.take_contained_physical_materials(&body());

        assert!(contained.is_empty());
        assert_eq!(field.revision, revision_before);

        let remaining: Vec<_> = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].material, compound());
        assert_eq!(remaining[0].material.internal_bonds, compound().internal_bonds);

        let accessible = field.accessible_physical_materials(&body());
        assert_eq!(accessible.len(), 1);
        assert_eq!(accessible[0].2, vec![0]);
    }
    #[test]
    fn environmental_material_id_survives_redeposit() {
        let catalog = catalog();
        let physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("test material must be realizable");

        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        assert!(field.deposit(0.0, 0.0, physical));
        let id = field.cells[0].physical_materials[0].id;
        assert!(id > 0);

        let removed = field.remove_physical_material(id).expect("material must exist");
        assert_eq!(removed.id, id);
        assert!(field.deposit_physical(25.0, 0.0, removed));

        let found = field.find_physical_material(id).expect("stable id must survive movement");
        assert_eq!(field.cells[found.0].physical_materials[found.1].id, id);
    }

}
