#[cfg(test)]
mod tests {
    use crate::environment::ActiveMaterialField;
    use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::{
        BaseResource, Form, InternalBond, Material, PhysicalState, ResourceProperties, Shape,
    };
    use crate::structure::Placement;
    use crate::interior_geometry::EnclosedRegion;

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

    fn region(half_extent: f64) -> EnclosedRegion {
        EnclosedRegion {
            area: (half_extent * 2.0).powi(2),
            boundary_units: vec![],
            sample_point: (0.0, 0.0),
            boundary: vec![
                (-half_extent, -half_extent),
                (half_extent, -half_extent),
                (half_extent, half_extent),
                (-half_extent, half_extent),
            ],
        }
    }

    fn large_body() -> OrganismBodyGeometry {
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Circle { radius: 2.0 },
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: -2.0,
            max_x: 2.0,
            min_y: -2.0,
            max_y: 2.0,
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
        let contained = field.take_contained_physical_materials_in_regions(&body(), &[], &catalog());
        assert!(contained.is_empty());
        assert_eq!(field.cells[0].materials.len(), 1);
    }

    #[test]
    fn composite_straddling_boundary_remains_intact() {
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
        let contained = field.take_contained_physical_materials_in_regions(&body(), &[region(1.0)], &catalog);

        assert!(contained.is_empty());

        let remaining: Vec<_> = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].material, compound());
        assert_eq!(remaining[0].material.internal_bonds.len(), 1);
    }

    #[test]
    fn fully_enclosed_composite_transfers_as_one_physical_component() {
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
                    x: 1.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("test composite must have a valid physical realization");

        let mut field = ActiveMaterialField::new(50.0, 50.0, 25.0);
        field.deposit(0.0, 0.0, physical);
        let contained = field.take_contained_physical_materials_in_regions(&large_body(), &[region(2.0)], &catalog);

        assert_eq!(contained.len(), 1);
        assert_eq!(contained[0].material, compound());
        assert_eq!(contained[0].material.internal_bonds.len(), 1);
        assert!(field
            .cells
            .iter()
            .all(|cell| cell.physical_materials.is_empty()));
    }
}
