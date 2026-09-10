//! Differential diagnostics for blueprint realization.
//!
//! These blueprints deliberately span three regimes:
//! - a trivial chain that should be fully realizable,
//! - a closed triangle that exercises multi-constraint placement,
//! - a K4 contact graph that is physically impossible for equal non-overlapping
//!   circular constituents in the current 2-D geometry model.
//!
//! The purpose is not to bless the seed blueprint. It separates solver failure
//! from physical/geometric infeasibility before changing the seed genome.

#[cfg(test)]
mod tests {
    use crate::resources::{default_catalog, Material};
    use crate::structural_blueprint::{BlueprintElement, BlueprintPlacement, StructuralBlueprint};

    fn water_element(x: f64, y: f64) -> BlueprintElement {
        BlueprintElement {
            material: Material::free_base("Water", 1.0),
            placement: BlueprintPlacement { x, y },
        }
    }

    fn blueprint(element_count: usize, edges: &[(usize, usize)], positions: &[(f64, f64)]) -> StructuralBlueprint {
        assert_eq!(element_count, positions.len());
        StructuralBlueprint::new(
            positions.iter().map(|&(x, y)| water_element(x, y)).collect(),
            edges.iter().map(|&(a, b)| crate::structural_blueprint::BlueprintConnection { element_a: a, element_b: b }).collect(),
        )
    }

    fn external_bond_count(b: &StructuralBlueprint, realized: usize) -> usize {
        // Each diagnostic constituent is a one-part material, so every physical
        // bond belongs to a blueprint connection. Missing elements are therefore
        // counted as zero realized external connections.
        let _ = realized;
        b.connections.len()
    }

    fn realized_result(b: &StructuralBlueprint) -> (usize, usize) {
        let catalog = default_catalog();
        let structure = b.realize(&catalog).expect("diagnostic blueprint must be structurally valid");
        (structure.units.len(), structure.bonds.len())
    }

    #[test]
    fn easy_chain_is_fully_realized() {
        let r = (0.5 / std::f64::consts::PI).sqrt();
        let d = 2.0 * r;
        let b = blueprint(4, &[(0, 1), (1, 2), (2, 3)], &[(0.0, 0.0), (d, 0.0), (2.0 * d, 0.0), (3.0 * d, 0.0)]);
        let (units, bonds) = realized_result(&b);
        println!("easy_chain: elements={} units={} intended_bonds={} realized_bonds={}", b.elements.len(), units, external_bond_count(&b, units), bonds);
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
        println!("triangle: elements={} units={} intended_bonds={} realized_bonds={}", b.elements.len(), units, b.connections.len(), bonds);
        assert_eq!(units, 3, "triangle blueprint must realize every constituent");
        assert_eq!(bonds, 3, "triangle must realize all three constraints");
    }

    #[test]
    fn physically_impossible_k4_does_not_claim_full_realization() {
        let r = (0.5 / std::f64::consts::PI).sqrt();
        let d = 2.0 * r;
        let h = d * (3.0_f64).sqrt() / 2.0;
        // K4 is not a contact graph of four equal, non-overlapping disks in 2-D.
        // The solver may still realize all four constituents; what must not happen
        // is silently claiming all six physical connections were formed.
        let b = blueprint(
            4,
            &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
            &[(0.0, 0.0), (d, 0.0), (d / 2.0, h), (d / 2.0, h / 3.0)],
        );
        let (units, bonds) = realized_result(&b);
        println!("k4_impossible: elements={} units={} intended_bonds={} realized_bonds={}", b.elements.len(), units, b.connections.len(), bonds);
        assert_eq!(units, 4, "K4 diagnostic should still realize the four constituents");
        assert!(bonds < 6, "physically impossible K4 must not realize all six contacts; got {bonds}");
    }
}
