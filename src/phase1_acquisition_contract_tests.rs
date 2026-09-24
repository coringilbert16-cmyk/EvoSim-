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
        let contained = field.take_contained_physical_materials(&body(), None);
        assert!(contained.is_empty());
        assert_eq!(field.cells[0].materials.len(), 1);
    }

    #[test]
    fn composite_is_partitioned_at_constituent_boundary() {
        let catalog = catalog();
        let physical = PhysicalMaterial::realized(
            compound(),
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 1.8,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("test composite must have a valid physical realization");

        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        field.deposit(0.0, 0.0, physical);
        let contained = field.take_contained_physical_materials(&body());

        assert_eq!(contained.len(), 1);
        assert_eq!(contained[0].material.parts, vec![("Carbon".into(), 1.0)]);
        assert!(contained[0].material.internal_bonds.is_empty());

        let remaining: Vec<_> = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].material.parts, vec![("Hydrogen".into(), 1.0)]);
        assert!(remaining[0].material.internal_bonds.is_empty());
    }
}
