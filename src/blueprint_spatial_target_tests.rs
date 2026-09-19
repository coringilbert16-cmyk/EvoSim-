#[cfg(test)]
mod tests {
    use crate::resources::{default_catalog, Material};
    use crate::structural_blueprint::{
        BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
    };

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
    fn ancestral_seed_realizes_a_physically_valid_developmental_structure() {
        let catalog = default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let structure = blueprint
            .realize(&catalog)
            .expect("developmental candidate must have a physical realization");

        assert!(!structure.units.is_empty());
        assert!(!structure.bonds.is_empty());

        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .expect("cavity analysis must succeed")
            .expect("developmental realization must contain a qualifying physical cavity");
        assert!(cavity.qualifies());
        assert!(structure.units.iter().all(|unit| unit.geometry.is_some()));
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
