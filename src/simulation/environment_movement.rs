//! One bounded, resonance-driven movement pass for environmental materials.

use crate::environment::ActiveMaterialField;
use crate::physical_material::PhysicalMaterial;
use crate::resources::BaseResource;

/// Apply one symmetric resonance movement update to free environmental objects.
///
/// Pair spectra are prepared once and every unordered pair is evaluated once.
/// Accepted translations use the field's existing atomic geometry/placement
/// transaction, which preserves each complete physical object.
pub(crate) fn apply_environmental_resonance_movement(
    field: &mut ActiveMaterialField,
    catalog: &[BaseResource],
    tick: u64,
) -> usize {
    field.assign_missing_material_identities();

    let mut identities = Vec::new();
    let mut materials: Vec<&PhysicalMaterial> = Vec::new();
    for material in field
        .cells
        .iter()
        .flat_map(|cell| cell.physical_materials.iter())
    {
        if material.environmental_identity == 0
            || material.owner_relative_origin.is_some()
            || !material.is_realized()
            || !material.material.is_valid_for_catalog(catalog)
            || material.material.is_empty()
        {
            continue;
        }
        identities.push(material.environmental_identity);
        materials.push(material);
    }

    // Opt-in phase timing keeps normal simulation runs free of profiling output.
    let profile_movement = std::env::var("EVOSIM_PROFILE_MOVEMENT")
        .is_ok_and(|value| value == "1" || value == "true");

    let resonance_started = std::time::Instant::now();
    let Some(displacements) =
        crate::harmonics::environmental_resonance_displacements(&materials, catalog, tick)
    else {
        if profile_movement {
            eprintln!(
                "MOVEMENT_PROFILE tick={tick} phase=resonance elapsed_ns={} objects={}",
                resonance_started.elapsed().as_nanos(),
                materials.len()
            );
        }
        return 0;
    };
    let resonance_elapsed = resonance_started.elapsed();
    drop(materials);

    let proposals = identities
        .into_iter()
        .zip(displacements)
        .map(|(identity, (dx, dy))| (identity, dx, dy))
        .collect::<Vec<_>>();

    let placement_started = std::time::Instant::now();
    let moved = field.move_physical_materials_by_identity(&proposals, catalog);
    let placement_elapsed = placement_started.elapsed();

    if profile_movement {
        eprintln!(
            "MOVEMENT_PROFILE tick={tick} objects={} proposals={} moved={} resonance_ns={} placement_ns={}",
            proposals.len(),
            proposals.iter().filter(|(_, dx, dy)| *dx != 0.0 || *dy != 0.0).count(),
            moved,
            resonance_elapsed.as_nanos(),
            placement_elapsed.as_nanos()
        );
    }

    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_environmental_movement_translates_whole_objects() {
        let catalog = crate::resources::default_catalog();
        let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        let make_carbon = |x| {
            PhysicalMaterial::realized(
                crate::resources::Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x,
                    y: 50.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .expect("carbon should be a valid realized material")
        };
        assert!(field.deposit_physical(40.0, 50.0, make_carbon(40.0), &catalog));
        assert!(field.deposit_physical(50.0, 50.0, make_carbon(50.0), &catalog));
        field.assign_missing_material_identities();

        let before = field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .map(|material| {
                (
                    material.environmental_identity,
                    material.material.clone(),
                    material.placements.clone().unwrap(),
                )
            })
            .collect::<Vec<_>>();

        let moved = apply_environmental_resonance_movement(&mut field, &catalog, 0);
        assert_eq!(moved, 2);

        for (identity, material, placements) in before {
            let after = field
                .cells
                .iter()
                .flat_map(|cell| cell.physical_materials.iter())
                .find(|candidate| candidate.environmental_identity == identity)
                .expect("movement must preserve the material identity");
            assert_eq!(after.material, material);
            assert_ne!(after.placements.as_ref().unwrap(), &placements);
        }
    }
}
