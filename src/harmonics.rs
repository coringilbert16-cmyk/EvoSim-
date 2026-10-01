fn environmental_material_resonance(
    physical: &crate::physical_material::PhysicalMaterial,
    catalog: &[crate::resources::BaseResource],
) -> Option<(f64, f64, f64, ToneSpectrum)> {
    if !physical.material.is_valid() || physical.material.is_empty() {
        return None;
    }
    let placements = physical.placements.as_ref()?;
    if placements.len() != physical.material.parts.len() || placements.is_empty() {
        return None;
    }

    let baselines = ResourceBaselines::from_catalog(catalog);
    let mut source_x = 0.0;
    let mut source_y = 0.0;
    let mut total_mass = 0.0;
    for ((name, amount), placement) in physical.material.parts.iter().zip(placements.iter()) {
        let part_mass = catalog
            .iter()
            .find(|resource| resource.name == *name)
            .map(|resource| resource.mass.max(0.0) * amount.max(0.0))
            .unwrap_or(0.0);
        source_x += placement.x * part_mass;
        source_y += placement.y * part_mass;
        total_mass += part_mass;
    }
    if total_mass > f64::EPSILON {
        source_x /= total_mass;
        source_y /= total_mass;
    } else {
        let count = placements.len() as f64;
        source_x = placements.iter().map(|placement| placement.x).sum::<f64>() / count;
        source_y = placements.iter().map(|placement| placement.y).sum::<f64>() / count;
    }

    let mass = physical.material.mass(catalog);
    let spectrum = material_response(
        physical.material.weighted_properties(catalog),
        baselines,
        0.0,
    );
    let extent = placements
        .iter()
        .map(|placement| (placement.x - source_x).hypot(placement.y - source_y))
        .fold(0.0_f64, f64::max);
    Some((
        source_x,
        source_y,
        extent,
        spectrum,
    ))
}

/// Calculate one directional environmental contribution at one realized