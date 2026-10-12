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
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let baseline_error = match crate::juvenile::realize_initial(&blueprint, &catalog) {
            Err(error) => error,
            Ok((structure, _, _)) => panic!(
                "baseline unexpectedly succeeded with {} physical units",
                structure.units.len()
            ),
        };
        assert!(baseline_error.contains("element 26"), "{baseline_error}");
        eprintln!("[constructor-order-diagnostic] baseline={baseline_error}");

        // Swap the first chain constituent (6) with the first Sulfur branch
        // constituent (26), preserving the graph by remapping every endpoint.
        let mut reordered = blueprint.clone();
        let mut new_index_for_old = (0..blueprint.elements.len()).collect::<Vec<_>>();
        new_index_for_old.swap(6, 26);
        for old_index in 0..blueprint.elements.len() {
            reordered.elements[new_index_for_old[old_index]] =
                blueprint.elements[old_index].clone();
        }
        for connection in &mut reordered.connections {
            connection.element_a = new_index_for_old[connection.element_a];
            connection.element_b = new_index_for_old[connection.element_b];
        }
        for anchor in &mut reordered.anchor_elements {
            *anchor = new_index_for_old[*anchor];
        }

        let outcome = match crate::juvenile::realize_initial(&reordered, &catalog) {
            Ok((structure, _, _)) => format!("success; physical_units={}", structure.units.len()),
            Err(error) => format!("failure; {error}"),
        };
        assert!(
            outcome.starts_with("success;"),
            "reordered constructor result: {outcome}"
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
