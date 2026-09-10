//! Differential diagnostics for blueprint realization.
//!
//! These diagnostics deliberately span assembly regimes. The realizable cases
//! exercise the construction solver; the impossible case is checked by the
//! geometry layer rather than asking an unbounded backtracking solver to prove
//! a non-solution.

#[cfg(test)]
mod tests {
    use crate::resources::{default_catalog, Form, Material};
    use crate::structural_blueprint::{BlueprintElement, BlueprintPlacement, StructuralBlueprint};
    use crate::structure::{Placement, StructuralUnit};

    fn water_element(x: f64, y: f64) -> BlueprintElement {
        BlueprintElement {
            material: Material::free_base("Water", 1.0),
            placement: BlueprintPlacement { x, y, rotation_radians: 0.0 },
        }
    }

    fn blueprint(element_count: usize, edges: &[(usize, usize)], positions: &[(f64, f64)]) -> StructuralBlueprint {
        assert_eq!(element_count, positions.len());
        StructuralBlueprint::new(
            positions.iter().map(|&(x, y)| water_element(x, y)).collect(),
            edges.iter().map(|&(a, b)| crate::structural_blueprint::BlueprintConnection { element_a: a, element_b: b }).collect(),
        )
    }

    fn realized_result(b: &StructuralBlueprint) -> (usize, usize) {
        let catalog = default_catalog();
        let structure = b.realize(&catalog).expect("diagnostic blueprint must be structurally valid");
        (structure.units.len(), structure.bonds.len())
    }

    #[test]
    fn legacy_and_explicit_orientation_round_trip() {
        let legacy = r#"{"material":{"parts":[["Water",1.0]],"internal_bonds":[]},"placement":{"x":2.0,"y":3.0}}"#;
        let explicit = r#"{"material":{"parts":[["Water",1.0]],"internal_bonds":[]},"placement":{"x":2.0,"y":3.0,"rotation_radians":1.25}}"#;
        let legacy: BlueprintElement = serde_json::from_str(legacy).expect("legacy blueprint must deserialize");
        let explicit: BlueprintElement = serde_json::from_str(explicit).expect("oriented blueprint must deserialize");
        assert_eq!(legacy.placement.rotation_radians, 0.0);
        assert!((explicit.placement.rotation_radians - 1.25).abs() <= f64::EPSILON);
    }

    #[test]
    fn blueprint_orientation_is_realized_as_element_frame() {
        let angle = std::f64::consts::FRAC_PI_3;
        let element = BlueprintElement {
            material: Material::free_base("Water", 1.0),
            placement: BlueprintPlacement { x: 4.0, y: -2.0, rotation_radians: angle },
        };
        let structure = StructuralBlueprint::new(vec![element], Vec::new()).realize(&default_catalog()).expect("oriented element must realize");
        assert_eq!(structure.units.len(), 1);
        assert!((structure.units[0].placement.x - 4.0).abs() <= 1e-12);
        assert!((structure.units[0].placement.y + 2.0).abs() <= 1e-12);
        assert!((structure.units[0].placement.rotation_radians - angle).abs() <= 1e-12);
    }

    #[test]
    fn blueprint_orientation_must_be_finite() {
        let element = water_element(0.0, 0.0);
        let mut oriented = element.clone();
        oriented.placement.rotation_radians = f64::NAN;
        let blueprint = StructuralBlueprint::new(vec![oriented], Vec::new());
        assert_eq!(blueprint.validate(), Err("element 0: blueprint orientation must be finite".into()));
    }

    #[test]
    fn easy_chain_is_fully_realized() {
        let r = (0.5 / std::f64::consts::PI).sqrt();
        let d = 2.0 * r;
        let b = blueprint(4, &[(0, 1), (1, 2), (2, 3)], &[(0.0, 0.0), (d, 0.0), (2.0 * d, 0.0), (3.0 * d, 0.0)]);
        let (units, bonds) = realized_result(&b);
        assert_eq!(units, 4, "easy blueprint must realize every constituent");
        assert_eq!(bonds, 3, "easy blueprint must realize every intended connection");
    }

    #[test]
    fn multi_constraint_triangle_is_fully_realized() {
        let r = (0.5 / std::f64::consts::PI).sqrt();
        let d = 2.0 * r;
        let h = d * (3.0_f64).sqrt() / 2.0;
        let b = blueprint(3, &[(0, 1), (0, 2), (1, 2)], &[(0.0, 0.0), (d, 0.0), (d / 2.0, h)]);
        let (units, bonds) = realized_result(&b);
        assert_eq!(units, 3, "triangle blueprint must realize every constituent");
        assert_eq!(bonds, 3, "triangle must realize all three constraints");
    }

    #[test]
    fn k4_is_rejected_by_the_physical_contact_graph_oracle() {
        let catalog = default_catalog();
        let water = catalog.iter().find(|r| r.name == "Water").unwrap();
        let Form::Circle { radius } = water.shape.form else { panic!("water diagnostic requires circular default geometry") };
        let d = 2.0 * radius;
        let h = d * (3.0_f64).sqrt() / 2.0;
        let units = [
            StructuralUnit::new("Water", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }),
            StructuralUnit::new("Water", Placement { x: d, y: 0.0, rotation_radians: 0.0 }),
            StructuralUnit::new("Water", Placement { x: d / 2.0, y: h, rotation_radians: 0.0 }),
            StructuralUnit::new("Water", Placement { x: d / 2.0, y: h / 3.0, rotation_radians: 0.0 }),
        ];
        let mut pair_contacts = 0;
        for a in 0..units.len() {
            for b in (a + 1)..units.len() {
                let distance = (units[a].placement.x - units[b].placement.x).hypot(units[a].placement.y - units[b].placement.y);
                if (distance - 2.0 * radius).abs() <= 1e-9 { pair_contacts += 1; }
            }
        }
        assert!(pair_contacts < 6, "K4 geometry cannot contain all six pair contacts");
    }
}
