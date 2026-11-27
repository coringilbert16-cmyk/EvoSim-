//! Boundary-based incoming energy flux for exact exposed straight intervals.
//!
//! Geometry/visibility owns interval discovery. This module only measures the
//! inward normal component of the synchronized field flux over those intervals.
//! It does not discover geometry, infer a receiver footprint, or settle energy.

use crate::closed_boundary_wave_solver::{ClosedBoundaryWaveField, ClosedWaveError};
use crate::source_exposure::ExposedBoundaryInterval;

const GAUSS_NODES: [f64; 3] = [-0.774_596_669_241_483_4, 0.0, 0.774_596_669_241_483_4];
const GAUSS_WEIGHTS: [f64; 3] = [
    0.555_555_555_555_555_6,
    0.888_888_888_888_888_8,
    0.555_555_555_555_555_6,
];

/// Instantaneous incoming power across a set of already-exposed intervals.
/// Positive inward flux is max(0, -J·n); outward flux is not counted.
/// Three-point Gauss-Legendre quadrature is applied independently to each
/// straight interval. Unsupported/out-of-domain samples fail closed.
pub(crate) fn incoming_boundary_power(
    field: &ClosedBoundaryWaveField,
    intervals: &[ExposedBoundaryInterval],
) -> Result<f64, ClosedWaveError> {
    validate_intervals(intervals)?;
    let mut total = 0.0;
    for interval in intervals {
        let dx = interval.end.0 - interval.start.0;
        let dy = interval.end.1 - interval.start.1;
        let length = dx.hypot(dy);
        for (node, weight) in GAUSS_NODES.iter().zip(GAUSS_WEIGHTS) {
            let fraction = 0.5 * (node + 1.0);
            let x = interval.start.0 + fraction * dx;
            let y = interval.start.1 + fraction * dy;
            let sample = field.query_world_at_current_time(x, y)?;
            let inward_flux = -(sample.flux_x * interval.outward_normal.0
                + sample.flux_y * interval.outward_normal.1);
            let contribution = 0.5 * length * weight * inward_flux.max(0.0);
            total += contribution;
            if !total.is_finite() {
                return Err(ClosedWaveError::InvalidState);
            }
        }
    }
    Ok(total)
}

/// Derive exposed polygonal receiver intervals from the geometry visibility
/// authority, then measure incoming field power over those exact intervals.
/// The geometry layer retains all bodies as possible occluders while restricting
/// measurement to the requested receiver indices. Unsupported geometry fails
/// closed; curved receiver boundaries require a separate exact arc adapter.
pub(crate) fn incoming_power_by_visible_polygonal_target(
    field: &ClosedBoundaryWaveField,
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Result<Vec<(usize, f64)>, ClosedWaveError> {
    if target_body_indices.is_empty()
        || target_body_indices
            .iter()
            .enumerate()
            .any(|(index, body_index)| target_body_indices[..index].contains(body_index))
    {
        return Err(ClosedWaveError::InvalidState);
    }
    let intervals = crate::source_exposure::visible_polygonal_intervals_for_targets(
        source,
        bodies,
        target_body_indices,
    )
    .ok_or(ClosedWaveError::InvalidState)?;

    // Visibility is evaluated once for the complete arrangement. Every body
    // remains an occluder, including other requested receivers; only the final
    // exposed intervals are separated into per-body accounting totals.
    let mut powers = Vec::with_capacity(target_body_indices.len());
    for &body_index in target_body_indices {
        let body_intervals: Vec<_> = intervals
            .iter()
            .copied()
            .filter(|interval| interval.body_index == body_index)
            .collect();
        let power = incoming_boundary_power(field, &body_intervals)?;
        powers.push((body_index, power));
    }
    Ok(powers)
}

pub(crate) fn incoming_power_for_visible_polygonal_targets(
    field: &ClosedBoundaryWaveField,
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Result<f64, ClosedWaveError> {
    incoming_power_by_visible_polygonal_target(field, source, bodies, target_body_indices)?
        .into_iter()
        .try_fold(0.0, |total, (_, power)| {
            let next = total + power;
            next.is_finite()
                .then_some(next)
                .ok_or(ClosedWaveError::InvalidState)
        })
}

/// Measure a complete polygonal structure perimeter against the continuous
/// field's local flux vector. Unlike point-source visibility, this has no
/// invented source position or preferred direction: every exterior perimeter
/// segment is evaluated using its own outward normal, and only inward local
/// flux contributes. The geometry authority removes internal constituent
/// boundaries before measurement. This single-body entry point does not yet
/// resolve overlap/occlusion between distinct bodies; callers must not pass
/// overlapping bodies as independent receivers.
pub(crate) fn incoming_power_for_polygonal_body(
    field: &ClosedBoundaryWaveField,
    body: &crate::organism_geometry::OrganismBodyGeometry,
    body_index: usize,
) -> Result<f64, ClosedWaveError> {
    let perimeter = crate::phase3_geometry::disjoint_polygonal_material_perimeter(body, body_index)
        .ok_or(ClosedWaveError::InvalidState)?;
    incoming_boundary_power(field, &perimeter)
}

/// A time-integrated, geometry-backed incident-energy measurement for one
/// receiver. Only this module's integration path constructs these values.
#[derive(Clone, Debug)]
pub(crate) struct BoundaryFluxEnergyMeasurement {
    pub(crate) sites: Vec<crate::closed_wave_field_funding::BoundaryFundingSite>,
}

/// Integrate incoming flux at each spatial quadrature site across synchronized
/// field snapshots, using the exact exposed intervals from the shared
/// multi-body visibility arrangement. The resulting site energies are suitable
/// inputs to staged field settlement; no receiver can claim hidden intervals.
pub(crate) fn integrated_visible_boundary_flux_by_target(
    field_history: &[(f64, ClosedBoundaryWaveField)],
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Result<Vec<(usize, BoundaryFluxEnergyMeasurement)>, ClosedWaveError> {
    if field_history.len() < 2
        || field_history.iter().any(|(time, _)| !time.is_finite())
        || field_history.windows(2).any(|pair| pair[1].0 <= pair[0].0)
        || target_body_indices.is_empty()
        || target_body_indices
            .iter()
            .enumerate()
            .any(|(index, body_index)| target_body_indices[..index].contains(body_index))
    {
        return Err(ClosedWaveError::InvalidState);
    }
    let exposed = crate::source_exposure::visible_polygonal_intervals_for_targets(
        source,
        bodies,
        target_body_indices,
    )
    .ok_or(ClosedWaveError::InvalidState)?;
    let mut result = Vec::with_capacity(target_body_indices.len());

    for &body_index in target_body_indices {
        let intervals: Vec<_> = exposed
            .iter()
            .copied()
            .filter(|interval| interval.body_index == body_index)
            .collect();
        validate_intervals(&intervals)?;
        let mut sites: Vec<crate::closed_wave_field_funding::BoundaryFundingSite> =
            Vec::with_capacity(intervals.len() * GAUSS_NODES.len());

        for interval in &intervals {
            let dx = interval.end.0 - interval.start.0;
            let dy = interval.end.1 - interval.start.1;
            let length = dx.hypot(dy);
            for (node, weight) in GAUSS_NODES.iter().zip(GAUSS_WEIGHTS) {
                let fraction = 0.5 * (node + 1.0);
                let x = interval.start.0 + fraction * dx;
                let y = interval.start.1 + fraction * dy;
                let spatial_weight = 0.5 * length * weight;
                let mut powers = Vec::with_capacity(field_history.len());

                for (time, field) in field_history {
                    let sample = field.query_world_at_current_time(x, y)?;
                    let inward_flux = -(sample.flux_x * interval.outward_normal.0
                        + sample.flux_y * interval.outward_normal.1);
                    let power = spatial_weight * inward_flux.max(0.0);
                    if !power.is_finite() {
                        return Err(ClosedWaveError::InvalidState);
                    }
                    powers.push((*time, power));
                }
                let energy = integrate_incoming_power_samples(&powers)
                    .ok_or(ClosedWaveError::InvalidState)?;

                if let Some(existing) = sites
                    .iter_mut()
                    .find(|site| site.position_x_l == x && site.position_y_l == y)
                {
                    existing.incident_energy += energy;
                    if !existing.incident_energy.is_finite() {
                        return Err(ClosedWaveError::InvalidState);
                    }
                } else {
                    sites.push(crate::closed_wave_field_funding::BoundaryFundingSite {
                        position_x_l: x,
                        position_y_l: y,
                        incident_energy: energy,
                    });
                }
            }
        }
        result.push((body_index, BoundaryFluxEnergyMeasurement { sites }));
    }
    Ok(result)
}

/// Integrate sampled incoming power over strictly increasing flux-sample times
/// using the trapezoidal rule. Samples must be taken at each solver substep;
/// timestamps should be the samples' leapfrog velocity_time_ticks.
pub(crate) fn integrate_incoming_power_samples(samples: &[(f64, f64)]) -> Option<f64> {
    if samples
        .iter()
        .any(|(time, power)| !time.is_finite() || !power.is_finite() || *power < 0.0)
    {
        return None;
    }
    let mut energy = 0.0;
    for pair in samples.windows(2) {
        let dt = pair[1].0 - pair[0].0;
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        energy += 0.5 * (pair[0].1 + pair[1].1) * dt;
        if !energy.is_finite() {
            return None;
        }
    }
    Some(energy)
}

fn validate_intervals(intervals: &[ExposedBoundaryInterval]) -> Result<(), ClosedWaveError> {
    for interval in intervals {
        let values = [
            interval.start.0,
            interval.start.1,
            interval.end.0,
            interval.end.1,
            interval.outward_normal.0,
            interval.outward_normal.1,
        ];
        let length = (interval.end.0 - interval.start.0).hypot(interval.end.1 - interval.start.1);
        let normal_length = interval.outward_normal.0.hypot(interval.outward_normal.1);
        let tangent_normal = ((interval.end.0 - interval.start.0) * interval.outward_normal.0
            + (interval.end.1 - interval.start.1) * interval.outward_normal.1)
            / length.max(1.0e-300);
        if values.iter().any(|value| !value.is_finite())
            || !length.is_finite()
            || length <= 1.0e-12
            || (normal_length - 1.0).abs() > 1.0e-9
            || !tangent_normal.is_finite()
            || tangent_normal.abs() > 1.0e-9
        {
            return Err(ClosedWaveError::InvalidState);
        }
    }

    // Visibility promises disjoint intervals. Reject collinear overlap here
    // as a defensive duplicate-counting guard; endpoint contact is permitted.
    for (index, first) in intervals.iter().enumerate() {
        let first_dx = first.end.0 - first.start.0;
        let first_dy = first.end.1 - first.start.1;
        let first_length = first_dx.hypot(first_dy);
        let tangent = (first_dx / first_length, first_dy / first_length);
        for second in intervals.iter().skip(index + 1) {
            let second_dx = second.end.0 - second.start.0;
            let second_dy = second.end.1 - second.start.1;
            let cross_offset = (second.start.0 - first.start.0) * tangent.1
                - (second.start.1 - first.start.1) * tangent.0;
            let cross_direction = second_dx * tangent.1 - second_dy * tangent.0;
            let scale = first_length.max(second_dx.hypot(second_dy)).max(1.0);
            if cross_offset.abs() <= 1.0e-10 * scale && cross_direction.abs() <= 1.0e-10 * scale {
                let second_start = (second.start.0 - first.start.0) * tangent.0
                    + (second.start.1 - first.start.1) * tangent.1;
                let second_end = (second.end.0 - first.start.0) * tangent.0
                    + (second.end.1 - first.start.1) * tangent.1;
                let overlap = first_length.min(second_start.max(second_end))
                    - 0.0_f64.max(second_start.min(second_end));
                if overlap > 1.0e-10 * scale {
                    return Err(ClosedWaveError::InvalidState);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(start: (f64, f64), end: (f64, f64), normal: (f64, f64)) -> ExposedBoundaryInterval {
        ExposedBoundaryInterval {
            body_index: 0,
            unit_index: 0,
            start,
            end,
            outward_normal: normal,
        }
    }

    #[test]
    fn trapezoidal_time_integral_is_exact_for_linear_power() {
        let samples = [(0.0, 0.0), (0.5, 1.0), (1.0, 2.0)];
        assert_eq!(integrate_incoming_power_samples(&samples), Some(1.0));
    }

    #[test]
    fn time_integral_rejects_nonmonotonic_or_invalid_samples() {
        assert_eq!(
            integrate_incoming_power_samples(&[(0.0, 1.0), (0.0, 1.0)]),
            None
        );
        assert_eq!(
            integrate_incoming_power_samples(&[(0.0, -1.0), (1.0, 1.0)]),
            None
        );
    }

    #[test]
    fn interval_validation_rejects_duplicate_and_degenerate_edges() {
        let duplicate = [
            interval((1.0, 1.0), (3.0, 1.0), (0.0, 1.0)),
            interval((1.0, 1.0), (3.0, 1.0), (0.0, 1.0)),
        ];
        assert_eq!(
            validate_intervals(&duplicate),
            Err(ClosedWaveError::InvalidState)
        );
        assert_eq!(
            validate_intervals(&[interval((0.0, 0.0), (0.0, 0.0), (0.0, 1.0))]),
            Err(ClosedWaveError::InvalidState)
        );
    }

    #[test]
    fn interval_validation_allows_disjoint_collinear_edges() {
        let disjoint = [
            interval((0.0, 0.0), (1.0, 0.0), (0.0, 1.0)),
            interval((2.0, 0.0), (3.0, 0.0), (0.0, 1.0)),
        ];
        assert!(validate_intervals(&disjoint).is_ok());
    }

    #[test]
    fn geometry_visibility_intervals_feed_the_field_boundary_flux_measurement() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let mut field = ClosedBoundaryWaveField::new(8, 8, 1.0, 1.0, 2, 1.0).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                field.set_initial_state(x, y, 0.0, x as f64).unwrap();
            }
        }
        field.advance_substeps(1).unwrap();

        let radius = 2.0_f64.sqrt();
        let receiver = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x: 4.0,
                y: 4.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0 - radius,
            max_x: 4.0 + radius,
            min_y: 4.0 - radius,
            max_y: 4.0 + radius,
        };

        // The source lies to the right, so visibility should expose the
        // receiver's right edge only. Its +x outward normal sees inward flux
        // 15 across a length of 2, giving an independently predictable power.
        let measured =
            incoming_power_for_visible_polygonal_targets(&field, (7.0, 4.0), &[receiver], &[0])
                .unwrap();
        assert!((measured - 30.0).abs() < 1.0e-10);
    }

    #[test]
    fn continuous_field_flux_is_integrated_over_the_whole_structure_perimeter() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let mut field = ClosedBoundaryWaveField::new(8, 8, 1.0, 1.0, 2, 1.0).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                field.set_initial_state(x, y, 0.0, x as f64).unwrap();
            }
        }
        field.advance_substeps(1).unwrap();

        let radius = 2.0_f64.sqrt();
        let receiver = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x: 4.0,
                y: 4.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0 - radius,
            max_x: 4.0 + radius,
            min_y: 4.0 - radius,
            max_y: 4.0 + radius,
        };

        let perimeter =
            crate::phase3_geometry::disjoint_polygonal_material_perimeter(&receiver, 0).unwrap();
        assert_eq!(
            perimeter.len(),
            4,
            "a rectangle must contribute its four exterior edges"
        );

        // The field carries flux in the -x direction. Only the right-hand
        // face has inward flux; the rear and side faces contribute zero.
        // This verifies a full perimeter query without choosing a point source.
        let measured = incoming_power_for_polygonal_body(&field, &receiver, 0).unwrap();
        assert!(
            (measured - 30.0).abs() < 1.0e-9,
            "unexpected perimeter flux: {measured}"
        );
    }

    #[test]
    fn incoming_power_integrates_the_inward_flux_along_the_exposed_interval() {
        let mut field = ClosedBoundaryWaveField::new(8, 8, 1.0, 1.0, 2, 1.0).unwrap();
        // Linear displacement has unit spatial gradient in the interior. The
        // first leapfrog step gives u_t=2x and midpoint gradient 1.5, hence
        // inward flux 3x on the +x-facing boundary.
        for y in 0..8 {
            for x in 0..8 {
                field.set_initial_state(x, y, 0.0, x as f64).unwrap();
            }
        }
        field.advance_substeps(1).unwrap();
        // A vertical segment has a +x outward normal. At x=3 the inward
        // flux is constant at 9, so the length-3 interval carries power 27.
        let edge = [interval((3.0, 2.0), (3.0, 5.0), (1.0, 0.0))];
        let measured = incoming_boundary_power(&field, &edge).unwrap();
        let expected = 27.0;
        assert!((measured - expected).abs() < 1.0e-10);
    }

    #[test]
    fn boundary_flux_quadrature_converges_under_collinear_interval_subdivision() {
        let mut field = ClosedBoundaryWaveField::new(10, 10, 1.0, 1.0, 2, 1.0).unwrap();
        for y in 0..10 {
            for x in 0..10 {
                // Smooth spatial variation exercises quadrature accuracy while
                // keeping the incoming-flux profile away from a sign-change kink.
                let displacement = 0.2 * (0.31 * x as f64).sin() * (0.23 * y as f64).cos();
                let velocity = 0.2 * (0.17 * x as f64).cos() * (0.29 * y as f64).sin();
                field
                    .set_initial_state(x, y, displacement, velocity)
                    .unwrap();
            }
        }
        field.advance_substeps(2).unwrap();

        // The field query is piecewise bilinear and max(0, inward_flux)
        // introduces kinks where the flux changes sign. A single 3-point
        // interval estimate is therefore not a valid reference for a finely
        // partitioned estimate. Check convergence between two refined
        // composite-Gauss evaluations instead.
        let integrate_partition = |parts: usize| {
            let intervals: Vec<_> = (0..parts)
                .map(|part| {
                    let step = 5.0 / parts as f64;
                    let y0 = 2.0 + step * part as f64;
                    interval((6.0, y0), (6.0, y0 + step), (1.0, 0.0))
                })
                .collect();
            incoming_boundary_power(&field, &intervals).unwrap()
        };

        let refined_power = integrate_partition(100);
        let finer_power = integrate_partition(200);
        assert!(
            (refined_power - finer_power).abs() < 1.0e-5,
            "composite boundary-flux quadrature did not converge under refinement: 100={refined_power}, 200={finer_power}"
        );
    }

    #[test]
    fn partial_occlusion_removes_blocked_receiver_boundary_from_flux_measurement() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let mut field = ClosedBoundaryWaveField::new(8, 8, 1.0, 1.0, 2, 1.0).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                field.set_initial_state(x, y, 0.0, x as f64).unwrap();
            }
        }
        field.advance_substeps(1).unwrap();

        let receiver = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x: 4.0,
                y: 4.0,
                rotation_radians: 0.0,
            }],
            min_x: 3.0,
            max_x: 5.0,
            min_y: 3.0,
            max_y: 5.0,
        };
        // This narrow blocker sits between source and receiver, close enough
        // to the receiver to shadow only a central slice of its front edge.
        let blocker = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 0.2,
                    height: 0.4,
                },
                x: 5.5,
                y: 4.0,
                rotation_radians: 0.0,
            }],
            min_x: 5.4,
            max_x: 5.6,
            min_y: 3.8,
            max_y: 4.2,
        };
        let bodies = [receiver, blocker];
        let exposed = crate::source_exposure::visible_polygonal_intervals_for_targets(
            (7.0, 4.0),
            &bodies,
            &[0],
        )
        .expect("supported polygon visibility should return exposed receiver sections");

        let front_length: f64 = exposed
            .iter()
            .filter(|edge| {
                edge.body_index == 0
                    && (edge.outward_normal.0 - 1.0).abs() < 1e-12
                    && edge.outward_normal.1.abs() < 1e-12
            })
            .map(|edge| (edge.end.0 - edge.start.0).hypot(edge.end.1 - edge.start.1))
            .sum();
        assert!(
            front_length > 0.0 && front_length < 2.0,
            "the occluder should block only part of the front edge, got {front_length}"
        );

        let incoming =
            incoming_power_for_visible_polygonal_targets(&field, (7.0, 4.0), &bodies, &[0])
                .unwrap();
        assert!(
            incoming > 0.0 && incoming < 30.0,
            "blocked portions must not contribute; measured power was {incoming}"
        );
        assert!(
            (incoming - front_length * 15.0).abs() < 1e-9,
            "only the exposed front-edge length contributes inward flux"
        );
    }
    #[test]
    fn multiple_receivers_share_one_occlusion_pass_and_keep_separate_energy_totals() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let mut field = ClosedBoundaryWaveField::new(12, 8, 1.0, 1.0, 2, 1.0).unwrap();
        for y in 0..8 {
            for x in 0..12 {
                field.set_initial_state(x, y, 0.0, x as f64).unwrap();
            }
        }
        field.advance_substeps(1).unwrap();

        let rectangle = |x: f64| OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x,
                y: 4.0,
                rotation_radians: 0.0,
            }],
            min_x: x - 1.0,
            max_x: x + 1.0,
            min_y: 3.0,
            max_y: 5.0,
        };
        // Body 1 is nearer the source and therefore occludes body 0. Both
        // are requested receivers, but neither may be counted twice.
        let bodies = [rectangle(4.0), rectangle(7.0)];
        let by_body =
            incoming_power_by_visible_polygonal_target(&field, (11.0, 4.0), &bodies, &[0, 1])
                .unwrap();
        assert_eq!(by_body.len(), 2);
        assert_eq!(by_body[0].0, 0);
        assert_eq!(by_body[1].0, 1);
        assert!(by_body[0].1 >= 0.0 && by_body[1].1 > 0.0);

        assert_eq!(
            incoming_power_by_visible_polygonal_target(&field, (11.0, 4.0), &bodies, &[0, 0]),
            Err(ClosedWaveError::InvalidState),
            "duplicate receiver indices must not duplicate energy claims"
        );
        let total =
            incoming_power_for_visible_polygonal_targets(&field, (11.0, 4.0), &bodies, &[0, 1])
                .unwrap();
        let separated_total: f64 = by_body.iter().map(|(_, power)| power).sum();
        assert!((total - separated_total).abs() < 1.0e-12);

        // The nearer receiver must block the farther receiver even when both
        // are included in the target set.
        let farther_with_near_body = by_body[0].1;
        let farther_without_near_body = incoming_power_for_visible_polygonal_targets(
            &field,
            (11.0, 4.0),
            std::slice::from_ref(&bodies[0]),
            &[0],
        )
        .unwrap();
        assert!(farther_with_near_body < farther_without_near_body);
    }
    #[test]
    fn measured_boundary_site_energy_comes_from_shared_exposure_and_flux_history() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let mut field = ClosedBoundaryWaveField::new(12, 8, 1.0, 1.0, 2, 1.0).unwrap();
        for y in 0..8 {
            for x in 0..12 {
                field.set_initial_state(x, y, 0.0, x as f64).unwrap();
            }
        }
        field.advance_substeps(1).unwrap();
        let rectangle = |x: f64| OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x,
                y: 4.0,
                rotation_radians: 0.0,
            }],
            min_x: x - 1.0,
            max_x: x + 1.0,
            min_y: 3.0,
            max_y: 5.0,
        };
        let bodies = [rectangle(4.0), rectangle(7.0)];
        let history = [(0.0, field.clone()), (1.0, field.clone())];

        let measured =
            integrated_visible_boundary_flux_by_target(&history, (11.0, 4.0), &bodies, &[0, 1])
                .unwrap();
        assert_eq!(measured.len(), 2);
        assert!(
            !measured[1].1.sites.is_empty(),
            "the nearer receiver must have exposed boundary sites"
        );
        // The farther receiver is fully shadowed by the aligned nearer
        // rectangle, so an empty site list is the correct visibility result.

        // Carry the measured receiver boundary directly into the existing
        // staged field-settlement contract; no hand-authored site-energy list.
        use crate::energy_ledger::{EnvironmentalEnergyProvider, EnvironmentalEnergyRequest};
        let mut funding =
            crate::closed_wave_field_funding::ClosedWaveBoundaryFundingSource::from_measured_flux(
                field.clone(),
                measured[1].1.clone(),
            )
            .unwrap();
        let before = funding.field().energy();
        let staged = funding
            .stage_funding(EnvironmentalEnergyRequest::new(0.001).unwrap())
            .expect("measured boundary should support a small conservative staged transfer");
        funding.commit_funding(staged.staged_source);
        assert!((before - funding.field().energy() - 0.001).abs() < 2.0e-10);
        assert!(
            measured[1]
                .1
                .sites
                .iter()
                .map(|site| site.incident_energy)
                .sum::<f64>()
                > 0.0,
            "the exposed nearer structure should receive a positive measured energy"
        );
        assert!(
            measured[0]
                .1
                .sites
                .iter()
                .map(|site| site.incident_energy)
                .sum::<f64>()
                < measured[1]
                    .1
                    .sites
                    .iter()
                    .map(|site| site.incident_energy)
                    .sum::<f64>(),
            "the nearer receiver should occlude part of the farther receiver's measured boundary"
        );
    }
}
