use std::collections::BTreeSet;

use crate::contact::ConnectionCompatibilityCache;
use crate::environment::ActiveMaterialField;
use crate::environmental_broad_phase::{
    EnvironmentalMaterialSnapshot, MaterialPairRecord, PairOrientation,
};
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::structure::{OrganismStructure, Placement};

fn eligible_combine_contacts(
    field: &ActiveMaterialField,
    snapshot: &EnvironmentalMaterialSnapshot,
    pairs: &[MaterialPairRecord],
    catalog: &[crate::resources::BaseResource],
) -> BTreeSet<(u64, u64, String)> {
    let mut contacts = BTreeSet::new();
    for pair in pairs {
        let left = &snapshot.materials[pair.left];
        let right = &snapshot.materials[pair.right];
        let source_a =
            &field.cells[left.locator.cell_index].physical_materials[left.locator.material_index];
        let source_b =
            &field.cells[right.locator.cell_index].physical_materials[right.locator.material_index];

        let mut structure = OrganismStructure::new();
        let Some(indices_a) = crate::material_restoration::restore_material(
            &mut structure,
            source_a,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            catalog,
        ) else {
            continue;
        };
        let Some(indices_b) = crate::material_restoration::restore_material(
            &mut structure,
            source_b,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            catalog,
        ) else {
            continue;
        };
        if indices_b.iter().any(|index| {
            crate::construction_runtime::placed_unit_overlaps(
                &structure,
                &structure.units[*index],
                &indices_b,
                catalog,
            )
        }) {
            continue;
        }

        let mut cache = ConnectionCompatibilityCache::new();
        for &unit_a in &indices_a {
            for &unit_b in &indices_b {
                let candidates = crate::contact::connection_pair_candidates_cached(
                    &structure,
                    unit_a,
                    unit_b,
                    catalog,
                    &mut cache,
                );
                for candidate in candidates {
                    if crate::combine_runtime::selected_candidate_evaluation(
                        &structure,
                        unit_a,
                        unit_b,
                        candidate,
                        catalog,
                    )
                    .is_none()
                    {
                        continue;
                    }
                    let Some(name_a) = structure.units[unit_a]
                        .material
                        .parts
                        .first()
                        .map(|part| part.0.as_str())
                    else {
                        continue;
                    };
                    let Some(name_b) = structure.units[unit_b]
                        .material
                        .parts
                        .first()
                        .map(|part| part.0.as_str())
                    else {
                        continue;
                    };
                    if crate::geometry_reference_library::resolve_live_contact_candidate(
                        name_a,
                        &structure.units[unit_a],
                        name_b,
                        &structure.units[unit_b],
                        candidate,
                        catalog,
                    )
                    .is_some()
                    {
                        contacts.insert((
                            left.identity.0,
                            right.identity.0,
                            format!("{:?}|{:?}", candidate.endpoint_a, candidate.endpoint_b),
                        ));
                    }
                }
            }
        }
    }
    contacts
}

#[test]
fn spatial_broad_phase_preserves_exhaustive_eligible_combine_contacts() {
    let catalog = crate::resources::default_catalog();
    let mut field = ActiveMaterialField::new(30.0, 30.0, 10.0);
    let fixtures = [
        (11_u64, "Carbon", 9.5, 10.0),
        (22_u64, "Hydrogen", 11.0, 10.0),
        (33_u64, "Oxygen", 25.0, 25.0),
    ];
    for (identity, name, x, y) in fixtures {
        let mut physical = PhysicalMaterial::realized(
            Material::free_base(name.to_owned(), 1.0),
            vec![Placement {
                x,
                y,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("fixture material must be physically realizable");
        physical.environmental_identity = identity;
        let cell = field
            .index_for_position(x, y)
            .expect("fixture placement must be inside the field");
        field.cells[cell].physical_materials.push(physical);
    }

    let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 335);
    let bounds = snapshot.spatial_bounds_from_field(&field, &catalog);
    assert!(bounds.iter().all(Option::is_some));

    let (all_pairs, exhaustive_coverage) = snapshot.pair_window(0, usize::MAX);
    assert_eq!(
        exhaustive_coverage.visited_pairs,
        snapshot.known_pair_count(),
        "the reference oracle must cover every unordered pair"
    );
    let (spatial_pairs, spatial_coverage) = snapshot.spatial_pair_window(
        &field,
        &bounds,
        2.1,
        0,
        usize::MAX,
        PairOrientation::Unordered,
    );
    assert_eq!(spatial_coverage.visited_pairs, spatial_coverage.known_pairs);

    let exhaustive_contacts =
        eligible_combine_contacts(&field, &snapshot, &all_pairs, &catalog);
    let spatial_contacts =
        eligible_combine_contacts(&field, &snapshot, &spatial_pairs, &catalog);
    assert!(
        !exhaustive_contacts.is_empty(),
        "fixture must exercise at least one genuinely eligible live contact"
    );
    assert_eq!(
        spatial_contacts, exhaustive_contacts,
        "spatial discovery must preserve every eligible contact found by exhaustive pair traversal"
    );
    assert!(
        exhaustive_contacts
            .iter()
            .any(|(left, right, _)| *left == 11 && *right == 22),
        "the cross-cell Carbon/Hydrogen fixture must remain discoverable"
    );
}

#[test]
fn directed_spatial_discovery_preserves_both_displacement_participant_roles() {
    let catalog = crate::resources::default_catalog();
    let mut field = ActiveMaterialField::new(50.0, 20.0, 10.0);
    let fixtures = [
        (101_u64, "Carbon", 9.5, 10.0),
        (202_u64, "Oxygen", 11.0, 10.0),
        (303_u64, "Hydrogen", 35.0, 10.0),
    ];
    for (identity, name, x, y) in fixtures {
        let mut physical = PhysicalMaterial::realized(
            Material::free_base(name.to_owned(), 1.0),
            vec![Placement {
                x,
                y,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("fixture material must be physically realizable");
        physical.environmental_identity = identity;
        let cell = field.index_for_position(x, y).unwrap();
        field.cells[cell].physical_materials.push(physical);
    }

    let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 336);
    let bounds = snapshot.spatial_bounds_from_field(&field, &catalog);
    let mut exhaustive = BTreeSet::new();
    let mut start = 0;
    loop {
        let (page, coverage) = snapshot.directed_pair_window(start, 2);
        exhaustive.extend(
            page.into_iter()
                .map(|pair| (pair.identity_left.0, pair.identity_right.0)),
        );
        start += coverage.visited_pairs;
        if start >= coverage.known_pairs {
            break;
        }
    }
    let (spatial, coverage) = snapshot.spatial_pair_window(
        &field,
        &bounds,
        2.1,
        0,
        usize::MAX,
        PairOrientation::Directed,
    );
    let spatial = spatial
        .into_iter()
        .map(|pair| (pair.identity_left.0, pair.identity_right.0))
        .collect::<BTreeSet<_>>();
    assert_eq!(coverage.visited_pairs, coverage.known_pairs);
    assert!(
        spatial.contains(&(101, 202)) && spatial.contains(&(202, 101)),
        "nearby participants must be discoverable in both anchor/incoming roles"
    );
    assert_eq!(
        spatial,
        exhaustive
            .iter()
            .copied()
            .filter(|(anchor, incoming)| {
                let anchor_index = snapshot
                    .materials
                    .iter()
                    .position(|material| material.identity.0 == *anchor)
                    .unwrap();
                let incoming_index = snapshot
                    .materials
                    .iter()
                    .position(|material| material.identity.0 == *incoming)
                    .unwrap();
                let anchor_bounds = bounds[anchor_index].unwrap();
                let incoming_cell = snapshot.materials[incoming_index].locator.cell_index;
                field
                    .cells_intersecting_bounds(
                        anchor_bounds.min_x - 2.1,
                        anchor_bounds.max_x + 2.1,
                        anchor_bounds.min_y - 2.1,
                        anchor_bounds.max_y + 2.1,
                    )
                    .contains(&incoming_cell)
            })
            .collect()
    );
}


#[test]
fn snapshot_break_bond_inventory_matches_every_live_physical_bond() {
    use crate::physical_material::PhysicalMaterialBond;
    use crate::resources::InternalBond;

    let catalog = crate::resources::default_catalog();
    let mut field = ActiveMaterialField::new(30.0, 20.0, 10.0);
    let mut physical = PhysicalMaterial::realized(
        Material {
            parts: vec![
                ("Carbon".to_owned(), 1.0),
                ("Hydrogen".to_owned(), 1.0),
            ],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        },
        vec![
            Placement {
                x: 9.5,
                y: 10.0,
                rotation_radians: 0.0,
            },
            Placement {
                x: 11.0,
                y: 10.0,
                rotation_radians: 0.0,
            },
        ],
        &catalog,
    )
    .expect("bonded fixture must be physically realizable");
    physical.environmental_identity = 404;
    let expected = physical
        .internal_connections
        .as_ref()
        .expect("realized bond must have physical endpoints")
        .clone();
    let cell = field.index_for_position(9.5, 10.0).unwrap();
    field.cells[cell].physical_materials.push(physical);

    let snapshot = EnvironmentalMaterialSnapshot::from_field(&field, 337);
    assert_eq!(snapshot.existing_bonds.len(), expected.len());
    for (record, connection) in snapshot.existing_bonds.iter().zip(expected.iter()) {
        assert_eq!(record.material.0, 404);
        assert_eq!(record.constituent_a.local_index, connection.part_a);
        assert_eq!(record.endpoint_a, connection.endpoint_a);
        assert_eq!(record.constituent_b.local_index, connection.part_b);
        assert_eq!(record.endpoint_b, connection.endpoint_b);
        assert_eq!(record.bond_energy, connection.bond_energy);
        assert!(snapshot.bond_reference_is_current(record.reference()));
    }
    let _: Option<PhysicalMaterialBond> = None;
}
