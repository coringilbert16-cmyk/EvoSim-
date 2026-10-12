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
        let (structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog)
            .expect("confirmed seed must have a physical realization");

        assert!(!structure.units.is_empty());
        assert!(!structure.bonds.is_empty());

        let cavity = crate::cavity::analyze_genome_cavity(&structure, &catalog)
            .expect("cavity analysis must succeed")
            .expect("developmental realization must contain a qualifying physical cavity");
        assert!(cavity.qualifies());
        assert!(structure.units.iter().all(|unit| unit.geometry.is_some()));
    }

    #[test]
    fn diagnostic_swapping_sulfur_and_first_chain_element_reports_order_effect() {
        let catalog = default_catalog();
        let seed = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();

        // Keep only the six Carbon ring elements, its first chain Carbon, and
        // the first Sulfur branch. This preserves the local obstruction while
        // avoiding the full 32-element calibration's long runtime.
        let selected_old_indices = [0usize, 1, 2, 3, 4, 5, 6, 26];
        let mut new_index_for_old = vec![usize::MAX; seed.elements.len()];
        for (new_index, old_index) in selected_old_indices.iter().copied().enumerate() {
            new_index_for_old[old_index] = new_index;
        }
        let elements = selected_old_indices
            .iter()
            .map(|old_index| seed.elements[*old_index].clone())
            .collect::<Vec<_>>();
        let connections = seed
            .connections
            .iter()
            .filter(|connection| {
                new_index_for_old[connection.element_a] != usize::MAX
                    && new_index_for_old[connection.element_b] != usize::MAX
            })
            .map(|connection| BlueprintConnection {
                element_a: new_index_for_old[connection.element_a],
                element_b: new_index_for_old[connection.element_b],
            })
            .collect::<Vec<_>>();
        let baseline =
            StructuralBlueprint::with_anchor_elements(elements, connections, vec![0]);

        let mut reordered_elements = baseline.elements.clone();
        reordered_elements.swap(6, 7);
        let remapped_connections = baseline
            .connections
            .iter()
            .map(|connection| {
                let remap = |index| match index {
                    6 => 7,
                    7 => 6,
                    other => other,
                };
                BlueprintConnection {
                    element_a: remap(connection.element_a),
                    element_b: remap(connection.element_b),
                }
            })
            .collect::<Vec<_>>();
        let reordered = StructuralBlueprint::with_anchor_elements(
            reordered_elements,
            remapped_connections,
            vec![0],
        );

        let construct_once = |candidate: &StructuralBlueprint| {
            let mut ledger = crate::state::EnergyLedger::default();
            let mut energy = 1.0e6;
            crate::construction_runtime::construct_blueprint_bond_driven(
                candidate,
                &catalog,
                &mut ledger,
                &mut energy,
            )
            .map(|(structure, _)| structure.units.len())
        };
        let baseline_result = construct_once(&baseline);
        let reordered_result = construct_once(&reordered);
        assert!(
            baseline_result.is_err(),
            "baseline local obstruction unexpectedly succeeded: {baseline_result:?}"
        );
        assert!(
            reordered_result.is_ok(),
            "placing Sulfur before the first chain Carbon did not resolve the local obstruction: {reordered_result:?}"
        );
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
