use crate::core_geometry::CoreGeometry;
use crate::resources::BaseResource;

/// Geometric description of the membrane as the physical boundary outside
/// the core.  The membrane does not have a magic permeability value: this
/// module only establishes where membrane material can physically exist.
#[derive(Clone, Debug, PartialEq)]
pub struct MembraneGeometry {
    /// Inner boundary of the membrane.  This is exactly the outer boundary
    /// of the core, so the membrane cannot occupy the hollow core cavity.
    pub inner_radius: f64,
    /// Radial thickness represented by one membrane material diameter.
    /// It is derived from the selected membrane material's immutable geometry.
    pub thickness: f64,
    /// Outer cell boundary supplied by the membrane geometry.
    pub outer_radius: f64,
    /// Minimum number of identical membrane material envelopes required to
    /// form a closed ring without gaps, using tangent bounding envelopes.
    pub minimum_unit_count: usize,
    /// Radius at which those membrane material envelopes are placed.
    pub ring_radius: f64,
}

/// Build the minimum closed membrane boundary around the already-defined
/// core.  No arbitrary thickness constant is introduced: thickness is the
/// diameter of the actual selected membrane material's conservative
/// bounding envelope.
///
/// This is deliberately a geometry checkpoint only.  It does not choose a
/// biological membrane material, alter resource properties, or assign a
/// permeability stat.  The caller supplies the resource type when a concrete
/// membrane material is eventually selected.
pub fn build_membrane_geometry(
    core: &CoreGeometry,
    catalog: &[BaseResource],
    membrane_resource_name: &str,
) -> Option<MembraneGeometry> {
    let membrane_radius = catalog
        .iter()
        .find(|resource| resource.name == membrane_resource_name)?
        .shape
        .form
        .bounding_radius();

    if !membrane_radius.is_finite() || membrane_radius <= 0.0 {
        return None;
    }

    let inner_radius = core.outer_radius;
    if !inner_radius.is_finite() || inner_radius <= 0.0 {
        return None;
    }

    // A single membrane unit spans one conservative diameter radially.
    // Its center therefore lies one radius beyond the core boundary.
    let ring_radius = inner_radius + membrane_radius;
    let thickness = 2.0 * membrane_radius;

    // For identical circular bounding envelopes of radius r on a ring of
    // radius R, neighboring envelopes are tangent when their center angle
    // is 2*asin(r/R).  Taking the ceiling gives the smallest integer count
    // whose angular spacing is no larger than that tangent angle, hence a
    // closed envelope with no radial gap.
    let tangent_angle = 2.0 * (membrane_radius / ring_radius).asin();
    if !tangent_angle.is_finite() || tangent_angle <= 0.0 {
        return None;
    }
    let minimum_unit_count = (std::f64::consts::TAU / tangent_angle).ceil() as usize;
    if minimum_unit_count < 3 {
        return None;
    }

    Some(MembraneGeometry {
        inner_radius,
        thickness,
        outer_radius: inner_radius + thickness,
        minimum_unit_count,
        ring_radius,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_geometry::build_f_core;
    use crate::resources::default_catalog;

    #[test]
    fn membrane_starts_at_core_outer_boundary() {
        let catalog = default_catalog();
        let core = build_f_core(&catalog).unwrap();
        let membrane = build_membrane_geometry(&core, &catalog, "Carbon").unwrap();

        assert_eq!(membrane.inner_radius, core.outer_radius);
        assert!(membrane.outer_radius > membrane.inner_radius);
    }

    #[test]
    fn membrane_thickness_is_derived_from_selected_resource_geometry() {
        let catalog = default_catalog();
        let core = build_f_core(&catalog).unwrap();
        let membrane = build_membrane_geometry(&core, &catalog, "Carbon").unwrap();
        let radius = catalog
            .iter()
            .find(|r| r.name == "Carbon")
            .unwrap()
            .shape
            .form
            .bounding_radius();

        assert!((membrane.thickness - 2.0 * radius).abs() < 1e-12);
        assert!((membrane.ring_radius - (core.outer_radius + radius)).abs() < 1e-12);
    }

    #[test]
    fn membrane_ring_is_closed_by_conservative_envelopes() {
        let catalog = default_catalog();
        let core = build_f_core(&catalog).unwrap();
        let membrane = build_membrane_geometry(&core, &catalog, "Carbon").unwrap();
        let r = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap()
            .shape
            .form
            .bounding_radius();
        let spacing = std::f64::consts::TAU / membrane.minimum_unit_count as f64;
        let center_distance = 2.0 * membrane.ring_radius * (spacing / 2.0).sin();

        assert!(center_distance + 1e-10 >= 2.0 * r);
    }

    #[test]
    fn invalid_membrane_resource_is_rejected() {
        let catalog = default_catalog();
        let core = build_f_core(&catalog).unwrap();
        assert!(build_membrane_geometry(&core, &catalog, "NotAResource").is_none());
    }
}
