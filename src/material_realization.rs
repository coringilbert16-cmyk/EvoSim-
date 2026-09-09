//! Physical realization of constituent attachment relationships.
//!
//! Genetic/material structure identifies physical attachment features; this
//! module derives relative constituent placement from those features. It does
//! not store world-space coordinates in the material definition.

use crate::attachment::{AttachmentFeature, ConstituentAttachment};
use crate::resources::{BaseResource, ConnectionSites, Form};
use crate::structure::Placement;

/// Resolve a rigid discrete attachment without storing an authored placement.
///
/// The first constituent is treated only as the local reference frame for this
/// solve; it has no special scaffold meaning. The second constituent is
/// rotated so its attachment direction faces the first, then translated so
/// the two physical features coincide.
///
/// Continuous Boundary/Fluid features intentionally return `None` here. They
/// require an assembly/contact resolver to select the actual point on the
/// continuous region rather than inventing a socket or coordinate.
pub fn resolve_rigid_attachment(
    resource_a: &BaseResource,
    attachment_a: &ConstituentAttachment,
    resource_b: &BaseResource,
    attachment_b: &ConstituentAttachment,
) -> Option<(Placement, Placement)> {
    let AttachmentFeature::Discrete(feature_a) = attachment_a.feature else {
        return None;
    };
    let AttachmentFeature::Discrete(feature_b) = attachment_b.feature else {
        return None;
    };

    resolve_rigid_discrete_features(resource_a, feature_a, resource_b, feature_b)
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

/// Resolve a pair of immutable resource geometry features into relative
/// placement. No world-space coordinates are stored in the attachment itself.
pub fn resolve_rigid_discrete_features(
    resource_a: &BaseResource,
    feature_a: u32,
    resource_b: &BaseResource,
    feature_b: u32,
) -> Option<(Placement, Placement)> {
    let (ax, ay, adir) = discrete_connection_point(resource_a, feature_a)?;
    let (bx, by, bdir) = discrete_connection_point(resource_b, feature_b)?;

    // Put A in the canonical reference frame. B's attachment direction must
    // face back toward A at the shared contact.
    let rotation_a = 0.0;
    let rotation_b = normalize_angle(adir + std::f64::consts::PI - bdir);

    let (bsin, bcos) = rotation_b.sin_cos();
    let rotated_bx = bx * bcos - by * bsin;
    let rotated_by = bx * bsin + by * bcos;

    // The attachment features occupy the same world-space contact point.
    let placement_b = Placement {
        x: ax - rotated_bx,
        y: ay - rotated_by,
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

fn transform_point(point: (f64, f64), placement: Placement) -> (f64, f64) {
    let (x, y) = point;
    let (sin, cos) = placement.rotation_radians.sin_cos();
    (
        placement.x + x * cos - y * sin,
        placement.y + x * sin + y * cos,
    )
}

fn normalize_angle(angle: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    (angle + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI
}

/// Rigid forms have meaningful local geometry. Fluid deliberately does not:
/// its nominal area is a quantity, not a circular boundary.
pub fn has_rigid_geometry(form: &Form) -> bool {
    !matches!(form, Form::Fluid { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::{AttachmentFeature, ConstituentId};
    use crate::resources::default_catalog;

    fn resource<'a>(catalog: &'a [BaseResource], name: &str) -> &'a BaseResource {
        catalog.iter().find(|resource| resource.name == name).unwrap()
    }

    fn point(resource: &BaseResource, feature: u32) -> (f64, f64) {
        let ConnectionSites::Corners(points) = resource.shape.connection_sites() else {
            panic!("test resource does not expose a discrete connection feature")
        };
        let point = points.get(feature as usize).unwrap();
        (point.x, point.y)
    }

    #[test]
    fn hydrogen_terminal_attachment_derives_contacting_placement() {
        let catalog = default_catalog();
        let carbon = resource(&catalog, "Carbon");
        let hydrogen = resource(&catalog, "Hydrogen");
        let carbon_attachment = ConstituentAttachment {
            constituent: ConstituentId(0),
            feature: AttachmentFeature::Discrete(0),
        };
        let hydrogen_attachment = ConstituentAttachment {
            constituent: ConstituentId(1),
            feature: AttachmentFeature::Discrete(0),
        };

        let (carbon_placement, hydrogen_placement) = resolve_rigid_attachment(
            carbon,
            &carbon_attachment,
            hydrogen,
            &hydrogen_attachment,
        )
        .unwrap();

        let carbon_contact = transform_point(point(carbon, 0), carbon_placement);
        let hydrogen_contact = transform_point(point(hydrogen, 0), hydrogen_placement);
        assert!((carbon_contact.0 - hydrogen_contact.0).abs() < 1e-12);
        assert!((carbon_contact.1 - hydrogen_contact.1).abs() < 1e-12);
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
    fn invalid_discrete_feature_is_rejected() {
        let catalog = default_catalog();
        let carbon = resource(&catalog, "Carbon");
        assert!(resolve_rigid_discrete_features(carbon, 99, carbon, 0).is_none());
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
        let carbon = resource(&catalog, "Carbon");
        let water = resource(&catalog, "Water");
        let carbon_attachment = ConstituentAttachment {
            constituent: ConstituentId(0),
            feature: AttachmentFeature::Boundary,
        };
        let water_attachment = ConstituentAttachment {
            constituent: ConstituentId(1),
            feature: AttachmentFeature::Fluid,
        };
        assert!(resolve_rigid_attachment(
            carbon,
            &carbon_attachment,
            water,
            &water_attachment,
        )
        .is_none());
    }
}
