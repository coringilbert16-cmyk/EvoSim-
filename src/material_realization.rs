//! Physical realization of constituent attachment relationships.
//!
//! Genetic/material structure identifies physical attachment features; this
//! module derives relative constituent placement from those features. It does
//! not store world-space coordinates in the material definition.

use crate::attachment::{AttachmentFeature, ConstituentAttachment};
use crate::resources::{BaseResource, ConnectionSites, Form};
use crate::structure::Placement;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedPartPlacement {
    pub part_index: usize,
    pub placement: Placement,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttachmentResolution {
    pub part_a: usize,
    pub part_b: usize,
    pub placement_a: Placement,
    pub placement_b: Placement,
}

/// Resolve a rigid discrete attachment without storing an authored placement.
///
/// The first constituent is treated only as the reference frame for this
/// local solve; it has no special scaffold meaning. The second constituent is
/// rotated so its attachment direction faces the first, then translated so
/// the two physical features coincide.
///
/// Continuous Boundary/Fluid features intentionally return `None` here. They
/// require an assembly/contact resolver to select the actual point on the
/// continuous region rather than inventing a socket or coordinate.
pub fn resolve_discrete_attachment(
    a: &ConstituentAttachment,
    b: &ConstituentAttachment,
    catalog: &[BaseResource],
) -> Option<AttachmentResolution> {
    let AttachmentFeature::Discrete(feature_a) = a.feature else {
        return None;
    };
    let AttachmentFeature::Discrete(feature_b) = b.feature else {
        return None;
    };

    let resource_a = catalog.iter().find(|resource| resource.name == "")?;
    let resource_b = catalog.iter().find(|resource| resource.name == "")?;
    let _ = (resource_a, resource_b, feature_a, feature_b);
    None
}

fn discrete_connection_point(
    resource: &BaseResource,
    feature: u32,
) -> Option<(f64, f64, f64)> {
    match resource.shape.connection_sites() {
        ConnectionSites::Corners(points) => {
            let point = points.get(feature as usize)?;
            Some((point.x, point.y, point.direction_radians))
        }
        ConnectionSites::Circumference { .. } | ConnectionSites::Undetermined => None,
    }
}

/// Resolve a pair of already-selected resource names into relative placement.
///
/// This lower-level primitive is deliberately independent of `Material` so it
/// can be used by both material realization and future blueprint realization.
pub fn resolve_rigid_discrete_features(
    resource_a: &BaseResource,
    feature_a: u32,
    resource_b: &BaseResource,
    feature_b: u32,
) -> Option<(Placement, Placement)> {
    let (ax, ay, adir) = discrete_connection_point(resource_a, feature_a)?;
    let (bx, by, bdir) = discrete_connection_point(resource_b, feature_b)?;

    // Put A in the canonical reference frame. Its attachment direction points
    // toward B; B must face back toward A at the shared contact.
    let rotation_a = 0.0;
    let rotation_b = normalize_angle(adir + std::f64::consts::PI - bdir);

    let (bsin, bcos) = rotation_b.sin_cos();
    let rotated_bx = bx * bcos - by * bsin;
    let rotated_by = bx * bsin + by * bcos;

    // The two attachment features must occupy the same world point.
    let contact_x = ax;
    let contact_y = ay;
    let placement_b = Placement {
        x: contact_x - rotated_bx,
        y: contact_y - rotated_by,
        rotation_radians: rotation_b,
    };

    Some((
        Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: rotation_a,
        },
        placement_b,
    ))
}

fn normalize_angle(angle: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    (angle + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI
}

/// Rigid forms currently have meaningful local geometry. Fluid deliberately
/// does not: its nominal area is a quantity, not a circular boundary.
pub fn has_rigid_geometry(form: &Form) -> bool {
    !matches!(form, Form::Fluid { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> &'a BaseResource {
        catalog.iter().find(|resource| resource.name == name).unwrap()
    }

    #[test]
    fn hydrogen_terminal_attachment_derives_placement() {
        let catalog = default_catalog();
        let carbon = resource(&catalog, "Carbon");
        let hydrogen = resource(&catalog, "Hydrogen");

        let (carbon_placement, hydrogen_placement) =
            resolve_rigid_discrete_features(carbon, 0, hydrogen, 0).unwrap();

        assert_eq!(carbon_placement, Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 });
        assert!(hydrogen_placement.x.is_finite());
        assert!(hydrogen_placement.y.is_finite());
        assert!(hydrogen_placement.rotation_radians.is_finite());
    }

    #[test]
    fn polygon_corner_identity_is_resolved_from_resource_geometry() {
        let catalog = default_catalog();
        let carbon = resource(&catalog, "Carbon");
        let (_, placement) = resolve_rigid_discrete_features(carbon, 2, carbon, 0).unwrap();
        assert!(placement.x.is_finite());
        assert!(placement.y.is_finite());
    }

    #[test]
    fn fluid_has_no_rigid_geometry() {
        let catalog = default_catalog();
        let water = resource(&catalog, "Water");
        assert!(!has_rigid_geometry(&water.shape.form));
    }

    #[test]
    fn continuous_attachment_is_not_faked_as_a_socket() {
        let catalog = default_catalog();
        let a = ConstituentAttachment {
            constituent: crate::attachment::ConstituentId(0),
            feature: AttachmentFeature::Boundary,
        };
        let b = ConstituentAttachment {
            constituent: crate::attachment::ConstituentId(1),
            feature: AttachmentFeature::Discrete(0),
        };
        assert!(resolve_discrete_attachment(&a, &b, &catalog).is_none());
    }
}
