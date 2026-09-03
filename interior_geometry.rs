use crate::core_geometry::CoreGeometry;
use crate::resources::BaseResource;

/// Physical space available to internal cell material.
///
/// The interior is the core's derived cavity. Chemistry occupying this space
/// remains represented by Floor 0 material state; this type stores only the
/// spatial boundary needed by Floor 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PhysicalInterior {
    pub(crate) radius: f64,
    pub(crate) area: f64,
}

impl PhysicalInterior {
    pub(crate) fn from_core(core: &CoreGeometry) -> Option<Self> {
        let radius = core.cavity_radius;
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let area = std::f64::consts::PI * radius * radius;
        if !area.is_finite() || area <= 0.0 {
            return None;
        }
        Some(Self { radius, area })
    }

    pub(crate) fn contains(&self, x: f64, y: f64) -> bool {
        x.is_finite() && y.is_finite() && x * x + y * y <= self.radius * self.radius + 1e-12
    }

    /// Validate that the interior remains spatially inside the core boundary
    /// represented by immutable resource geometry.
    pub(crate) fn is_valid_for_core(&self, core: &CoreGeometry) -> bool {
        self.radius <= core.cavity_radius + 1e-12
            && self.radius > 0.0
            && self.area > 0.0
            && self.radius.is_finite()
            && self.area.is_finite()
    }

    /// The interior does not require a material-specific shape. The supplied
    /// catalog is accepted so callers can keep the projection pipeline
    /// catalog-driven without manufacturing an interior resource type.
    pub(crate) fn is_compatible_with_catalog(&self, catalog: &[BaseResource]) -> bool {
        self.is_valid_for_core_radius() && catalog.iter().all(|resource| resource.shape.is_valid())
    }

    fn is_valid_for_core_radius(&self) -> bool {
        self.radius.is_finite() && self.radius > 0.0 && self.area.is_finite() && self.area > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_geometry::build_f_core;
    use crate::resources::default_catalog;

    #[test]
    fn interior_is_derived_from_core_cavity() {
        let catalog = default_catalog();
        let core = build_f_core(&catalog).unwrap();
        let interior = PhysicalInterior::from_core(&core).unwrap();
        assert_eq!(interior.radius, core.cavity_radius);
        assert!(interior.area > 0.0);
        assert!(interior.is_valid_for_core(&core));
    }

    #[test]
    fn interior_contains_center_and_rejects_outside_points() {
        let core = build_f_core(&default_catalog()).unwrap();
        let interior = PhysicalInterior::from_core(&core).unwrap();
        assert!(interior.contains(0.0, 0.0));
        assert!(!interior.contains(interior.radius * 1.01, 0.0));
    }

    #[test]
    fn interior_does_not_define_its_own_chemistry() {
        let core = build_f_core(&default_catalog()).unwrap();
        let interior = PhysicalInterior::from_core(&core).unwrap();
        assert!(interior.is_compatible_with_catalog(&default_catalog()));
    }
}
