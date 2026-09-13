#[cfg(test)]
mod tests {
    use crate::genome::initial_genome;
    use crate::resources::{default_catalog, Material};
    use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint};

    fn nitrogen_wall(x: f64, y: f64, rotation_radians: f64) -> BlueprintElement {
        BlueprintElement {
            material: Material::free_base("Nitrogen", 1.0),
            placement: BlueprintPlacement {
                x,
                y,
                rotation_radians,
            },
        }
    }

    #[test]
    fn ancestral_seed_realizes_its_inherited_spatial_targets() {
        let blueprint = initial_genome().structural_blueprint;
        let catalog = default_catalog();
        let structure = blueprint
            .realize(&catalog)
            .expect("ancestral blueprint must have a physical realization");

        assert_eq!(structure.units.len(), blueprint.elements.len());
        assert_eq!(structure.bonds.len(), blueprint.connections.len());

        for element in &blueprint.elements {
            let target = element.placement;
            let closest = structure
                .units
                .iter()
                .map(|unit| {
                    (unit.placement.x - target.x).hypot(unit.placement.y - target.y)
                })
                .fold(f64::INFINITY, f64::min);
            assert!(
                closest < 1e-6,
                "realized structure lost the inherited spatial target at ({}, {})",
                target.x,
                target.y
            );
        }
    }

    #[test]
    fn physically_valid_divergence_is_allowed_when_anchor_is_impossible() {
        let blueprint = StructuralBlueprint::new(
            vec![nitrogen_wall(0.0, 0.0, 0.0), nitrogen_wall(0.0, 0.0, 0.0)],
            vec![BlueprintConnection {
                element_a: 0,
                element_b: 1,
            }],
        );
        let structure = blueprint
            .realize(&default_catalog())
            .expect("construction should choose a physically valid alternative");

        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.bonds.len(), 1);
        let distance = (structure.units[0].placement.x - structure.units[1].placement.x)
            .hypot(structure.units[0].placement.y - structure.units[1].placement.y);
        assert!(distance > 0.0);
    }
}
