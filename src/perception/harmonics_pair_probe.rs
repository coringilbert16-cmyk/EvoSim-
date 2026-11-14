//! Test-only differential coverage for source-dependent receiver quadrature.
//!
//! These probes deliberately retain each ordered pair's exact production
//! samples. They do not introduce shared quadrature, aggregate spectra, or an
//! approximate evaluator.

use super::*;

#[derive(Clone, Debug)]
struct BoundarySampleRecord {
    midpoint: (f64, f64),
    normal: (f64, f64),
    projected_extent: f64,
    boundary_geometry: Vec<f64>,
    incident_components: Vec<(f64, f64, f64)>,
}

#[derive(Clone, Debug)]
struct OrderedPairRecord {
    source_index: usize,
    receiver_index: usize,
    samples: Vec<BoundarySampleRecord>,
    potential: Vec<(f64, f64)>,
}

fn single_body(
    name: &str,
    x: f64,
    y: f64,
    rotation_radians: f64,
    catalog: &[crate::resources::BaseResource],
) -> crate::physical_material::PhysicalMaterial {
    crate::physical_material::PhysicalMaterial::realized(
        crate::resources::Material::free_base(name, 1.0),
        vec![crate::structure::Placement {
            x,
            y,
            rotation_radians,
        }],
        catalog,
    )
    .expect("probe body should have a valid realized physical shape")
}

fn asymmetric_receiver(
    shift: (f64, f64),
    rotation_radians: f64,
    catalog: &[crate::resources::BaseResource],
) -> crate::physical_material::PhysicalMaterial {
    let center = (10.75 + shift.0, 3.35 + shift.1);
    let rotate = |x: f64, y: f64| {
        let dx = x - center.0;
        let dy = y - center.1;
        (
            center.0 + dx * rotation_radians.cos() - dy * rotation_radians.sin(),
            center.1 + dx * rotation_radians.sin() + dy * rotation_radians.cos(),
        )
    };
    let carbon = rotate(10.0, 3.0);
    let hydrogen = rotate(11.5, 3.7);
    crate::physical_material::PhysicalMaterial::realized(
        crate::resources::Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: Vec::new(),
        },
        vec![
            crate::structure::Placement {
                x: carbon.0,
                y: carbon.1,
                rotation_radians,
            },
            crate::structure::Placement {
                x: hydrogen.0,
                y: hydrogen.1,
                rotation_radians,
            },
        ],
        catalog,
    )
    .expect("asymmetric receiver should have a valid realized physical shape")
}

fn probe_world(
    pose: [f64; 12],
    catalog: &[crate::resources::BaseResource],
) -> Vec<crate::physical_material::PhysicalMaterial> {
    vec![
        single_body("Hydrogen", pose[0], pose[1], pose[2], catalog),
        single_body("Nitrogen", pose[3], 8.0 + pose[4], pose[5], catalog),
        asymmetric_receiver((pose[6], pose[7]), pose[8], catalog),
        single_body("Oxygen", 5.0 + pose[9], 1.6 + pose[10], pose[11], catalog),
    ]
}

fn boundary_geometry(boundary: MaterialAuraBoundary) -> Vec<f64> {
    match boundary {
        MaterialAuraBoundary::Straight { start, end, normal } => {
            vec![0.0, start.0, start.1, end.0, end.1, normal.0, normal.1]
        }
        MaterialAuraBoundary::Arc(arc) => vec![
            1.0,
            arc.center_x,
            arc.center_y,
            arc.radius,
            arc.start_angle_radians,
            arc.sweep_radians,
        ],
    }
}

fn record_world_pairs(
    bodies: &[crate::physical_material::PhysicalMaterial],
    catalog: &[crate::resources::BaseResource],
    tick: u64,
) -> Vec<OrderedPairRecord> {
    let mut records = Vec::new();
    for source_index in 0..bodies.len() {
        for receiver_index in 0..bodies.len() {
            if source_index == receiver_index {
                continue;
            }
            let occluders = bodies
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != source_index && *index != receiver_index)
                .map(|(_, body)| body.clone())
                .collect::<Vec<_>>();
            let source = &bodies[source_index];
            let receiver = &bodies[receiver_index];
            let receptions = material_aura_receptions_with_occluders(
                source, receiver, &occluders, catalog, tick,
            )
            .expect("every valid ordered pair should produce an exact reception result");
            let potential = directional_resonance_pair_potential_components_with_occluders(
                source, receiver, &occluders, catalog, tick,
            )
            .expect("every valid ordered pair should produce an exact potential result");

            let samples = receptions
                .into_iter()
                .map(|sample| BoundarySampleRecord {
                    midpoint: (sample.receiver_x, sample.receiver_y),
                    normal: (sample.receiver_normal_x, sample.receiver_normal_y),
                    projected_extent: sample.projected_extent,
                    boundary_geometry: boundary_geometry(sample.boundary),
                    incident_components: sample
                        .incident_spectrum
                        .components
                        .iter()
                        .map(|component| {
                            (
                                component.frequency_cycles_per_day,
                                component.amplitude,
                                component.phase_radians,
                            )
                        })
                        .collect(),
                })
                .collect();
            let potential = potential
                .into_iter()
                .map(|component| (component.frequency_cycles_per_day, component.energy))
                .collect();
            records.push(OrderedPairRecord {
                source_index,
                receiver_index,
                samples,
                potential,
            });
        }
    }
    records
}

fn pair_record(
    records: &[OrderedPairRecord],
    source_index: usize,
    receiver_index: usize,
) -> &OrderedPairRecord {
    records
        .iter()
        .find(|record| {
            record.source_index == source_index && record.receiver_index == receiver_index
        })
        .expect("ordered pair should be present")
}

fn sample_geometry_differs(left: &[BoundarySampleRecord], right: &[BoundarySampleRecord]) -> bool {
    if left.len() != right.len() {
        return true;
    }
    left.iter().zip(right).any(|(a, b)| {
        let scalar_pairs = [
            (a.midpoint.0, b.midpoint.0),
            (a.midpoint.1, b.midpoint.1),
            (a.normal.0, b.normal.0),
            (a.normal.1, b.normal.1),
            (a.projected_extent, b.projected_extent),
        ];
        scalar_pairs.iter().any(|(x, y)| (x - y).abs() > 1.0e-12)
            || a.boundary_geometry.len() != b.boundary_geometry.len()
            || a.boundary_geometry
                .iter()
                .zip(&b.boundary_geometry)
                .any(|(x, y)| (x - y).abs() > 1.0e-12)
            || a.incident_components.len() != b.incident_components.len()
            || a.incident_components
                .iter()
                .zip(&b.incident_components)
                .any(|(x, y)| {
                    (x.0 - y.0).abs() > 1.0e-9
                        || (x.1 - y.1).abs() > 1.0e-12
                        || (x.2 - y.2).abs() > 1.0e-12
                })
    })
}

fn potential_differs(left: &[(f64, f64)], right: &[(f64, f64)]) -> bool {
    left.len() != right.len()
        || left
            .iter()
            .zip(right)
            .any(|(a, b)| (a.0 - b.0).abs() > 1.0e-9 || (a.1 - b.1).abs() > 1.0e-12)
}

#[test]
fn multi_pair_probe_records_source_specific_receiver_quadrature() {
    let catalog = crate::resources::default_catalog();
    let baseline_bodies = probe_world([0.0; 12], &catalog);
    let mut changed_pose = [0.0; 12];
    changed_pose[0] = 0.35;
    changed_pose[1] = 0.25;
    changed_pose[2] = 0.2;
    changed_pose[8] = 0.4;
    let changed_bodies = probe_world(changed_pose, &catalog);

    let baseline = record_world_pairs(&baseline_bodies, &catalog, 0);
    let changed = record_world_pairs(&changed_bodies, &catalog, 0);

    // Four bodies produce all twelve directed interactions. Every record
    // retains its own source-specific visible boundary intervals, spectra,
    // projected extents, and frequency-resolved directional potential.
    assert_eq!(baseline.len(), 12);
    assert_eq!(changed.len(), 12);
    assert!(
        baseline.iter().any(|record| !record.samples.is_empty()),
        "the deterministic world must expose at least one receiver interval"
    );
    assert!(
        baseline.iter().any(|record| !record.potential.is_empty()),
        "the deterministic world must produce frequency-resolved pair potential"
    );

    let source_zero_to_receiver = pair_record(&baseline, 0, 2);
    let source_one_to_receiver = pair_record(&baseline, 1, 2);
    assert!(
        !source_zero_to_receiver.samples.is_empty() && !source_one_to_receiver.samples.is_empty(),
        "both independent sources must expose the asymmetric receiver"
    );
    assert!(
        sample_geometry_differs(
            &source_zero_to_receiver.samples,
            &source_one_to_receiver.samples
        ),
        "different sources aimed at one receiver must retain distinct exact quadrature"
    );

    let changed_source_pair = pair_record(&changed, 0, 2);
    assert!(
        sample_geometry_differs(
            &source_zero_to_receiver.samples,
            &changed_source_pair.samples
        ),
        "source translation/rotation and receiver rotation must be reflected in sample geometry"
    );
    assert!(
        potential_differs(
            &source_zero_to_receiver.potential,
            &changed_source_pair.potential
        ),
        "pose changes must be reflected in the frequency-resolved directional potential"
    );

    // The blocker is part of every pair's occluder set except when it is
    // itself an endpoint. At least one source/receiver pair must therefore
    // retain a distinct visibility result rather than sharing samples by
    // receiver identity alone.
    let unblocked = record_world_pairs(&baseline_bodies[..3], &catalog, 0);
    let blocked_pair = pair_record(&baseline, 0, 2);
    let unblocked_pair = pair_record(&unblocked, 0, 2);
    assert!(
        sample_geometry_differs(&blocked_pair.samples, &unblocked_pair.samples)
            || potential_differs(&blocked_pair.potential, &unblocked_pair.potential),
        "the third-body blocker must affect exact visibility or pair potential"
    );
}

fn world_potential(records: &[OrderedPairRecord]) -> f64 {
    0.5 * records
        .iter()
        .flat_map(|record| record.potential.iter())
        .map(|(_, energy)| energy)
        .sum::<f64>()
}

#[test]
fn smooth_pose_sweep_records_boundary_and_extent_derivatives() {
    let catalog = crate::resources::default_catalog();
    let baseline_pose = [0.0; 12];
    let baseline_bodies = probe_world(baseline_pose, &catalog);
    let baseline = record_world_pairs(&baseline_bodies, &catalog, 0);
    let base_pair = pair_record(&baseline, 0, 2);
    assert!(!base_pair.samples.is_empty());

    let step = 1.0e-4;
    let mut midpoint_derivative = 0.0;
    let mut normal_derivative = 0.0;
    let mut extent_derivative = 0.0;
    let mut boundary_derivative = 0.0;

    // Perturb the source and receiver's x/y/rotation coordinates independently.
    // The fixture is deliberately kept inside one visibility-topology region;
    // endpoint creation/removal is handled by exact fallback, not derivatives.
    for axis in [0, 1, 2, 6, 7, 8] {
        let mut plus_pose = baseline_pose;
        let mut minus_pose = baseline_pose;
        plus_pose[axis] = step;
        minus_pose[axis] = -step;
        let plus_bodies = probe_world(plus_pose, &catalog);
        let minus_bodies = probe_world(minus_pose, &catalog);
        let plus_records = record_world_pairs(&plus_bodies, &catalog, 0);
        let minus_records = record_world_pairs(&minus_bodies, &catalog, 0);
        let plus = pair_record(&plus_records, 0, 2);
        let minus = pair_record(&minus_records, 0, 2);

        assert_eq!(
            plus.samples.len(),
            base_pair.samples.len(),
            "positive perturbation changed visibility topology on axis {axis}"
        );
        assert_eq!(
            minus.samples.len(),
            base_pair.samples.len(),
            "negative perturbation changed visibility topology on axis {axis}"
        );

        for ((base, positive), negative) in base_pair
            .samples
            .iter()
            .zip(&plus.samples)
            .zip(&minus.samples)
        {
            assert_eq!(
                positive.boundary_geometry.first(),
                base.boundary_geometry.first()
            );
            assert_eq!(
                negative.boundary_geometry.first(),
                base.boundary_geometry.first()
            );
            assert_eq!(
                positive.boundary_geometry.len(),
                base.boundary_geometry.len()
            );
            assert_eq!(
                negative.boundary_geometry.len(),
                base.boundary_geometry.len()
            );

            midpoint_derivative += (positive.midpoint.0 - negative.midpoint.0).abs() / (2.0 * step)
                + (positive.midpoint.1 - negative.midpoint.1).abs() / (2.0 * step);
            normal_derivative += (positive.normal.0 - negative.normal.0).abs() / (2.0 * step)
                + (positive.normal.1 - negative.normal.1).abs() / (2.0 * step);
            extent_derivative +=
                (positive.projected_extent - negative.projected_extent).abs() / (2.0 * step);
            for ((positive_value, negative_value), _) in positive
                .boundary_geometry
                .iter()
                .zip(&negative.boundary_geometry)
                .zip(&base.boundary_geometry)
            {
                boundary_derivative += (positive_value - negative_value).abs() / (2.0 * step);
            }
        }
    }

    assert!(
        midpoint_derivative.is_finite()
            && normal_derivative.is_finite()
            && extent_derivative.is_finite()
            && boundary_derivative.is_finite()
    );
    assert!(
        midpoint_derivative > 1.0e-8,
        "source/receiver pose changes must move exact boundary sample locations"
    );
    assert!(
        extent_derivative > 1.0e-8,
        "source/receiver pose changes must change projected boundary extent"
    );
    assert!(
        boundary_derivative > 1.0e-8,
        "source/receiver pose changes must alter exact boundary interval geometry"
    );
}

#[test]
fn complete_world_pose_derivatives_match_refined_central_differences() {
    let catalog = crate::resources::default_catalog();
    // Each body contributes independent x/y/rotation coordinates in world order.
    // This includes the third-body blocker and its occlusion reactions.
    let evaluate = |pose: [f64; 12]| {
        let bodies = probe_world(pose, &catalog);
        world_potential(&record_world_pairs(&bodies, &catalog, 0))
    };

    // Recompute every ordered pair with the other bodies as exact occluders.
    // These derivatives therefore include changes to sample locations,
    // projected extents, visibility, and frequency weights—not frozen samples.
    let central_difference = |step: f64, axis: usize| {
        let mut plus = [0.0; 12];
        let mut minus = [0.0; 12];
        plus[axis] = step;
        minus[axis] = -step;
        (evaluate(plus) - evaluate(minus)) / (2.0 * step)
    };

    let mut nonzero_derivatives = 0;
    for axis in 0..12 {
        let coarse = central_difference(2.0e-4, axis);
        let fine = central_difference(1.0e-4, axis);
        assert!(coarse.is_finite() && fine.is_finite());
        if fine.abs() > 1.0e-12 {
            nonzero_derivatives += 1;
        }
        let scale = coarse.abs().max(fine.abs()).max(1.0e-9);
        assert!(
            (fine - coarse).abs() <= 2.0e-2 * scale,
            "central-difference derivative should converge for pose axis {axis}: coarse={coarse} fine={fine}"
        );
    }
    assert!(
        nonzero_derivatives >= 5,
        "the asymmetric complete-world fixture should exercise at least five independent pose derivatives; got {nonzero_derivatives}"
    );
}


#[test]
fn smooth_boundary_geometry_derivatives_match_refined_differences() {
    let catalog = crate::resources::default_catalog();
    let baseline_pose = [0.0; 12];
    let baseline = record_world_pairs(&probe_world(baseline_pose, &catalog), &catalog, 0);
    let base_pair = pair_record(&baseline, 0, 2);

    let flatten_geometry = |sample: &BoundarySampleRecord| {
        let mut values = vec![
            sample.midpoint.0,
            sample.midpoint.1,
            sample.normal.0,
            sample.normal.1,
            sample.projected_extent,
        ];
        values.extend(sample.boundary_geometry.iter().skip(1).copied());
        values
    };
    let derivative = |axis: usize, step: f64| {
        let mut plus_pose = baseline_pose;
        let mut minus_pose = baseline_pose;
        plus_pose[axis] = step;
        minus_pose[axis] = -step;
        let plus_world = probe_world(plus_pose, &catalog);
        let minus_world = probe_world(minus_pose, &catalog);
        let plus_records = record_world_pairs(&plus_world, &catalog, 0);
        let minus_records = record_world_pairs(&minus_world, &catalog, 0);
        let plus = pair_record(&plus_records, 0, 2);
        let minus = pair_record(&minus_records, 0, 2);
        assert_eq!(plus.samples.len(), base_pair.samples.len());
        assert_eq!(minus.samples.len(), base_pair.samples.len());

        let mut values = Vec::new();
        for ((base, positive), negative) in base_pair
            .samples
            .iter()
            .zip(&plus.samples)
            .zip(&minus.samples)
        {
            assert_eq!(
                positive.boundary_geometry.first(),
                base.boundary_geometry.first()
            );
            assert_eq!(
                negative.boundary_geometry.first(),
                base.boundary_geometry.first()
            );
            assert_eq!(
                positive.boundary_geometry.len(),
                base.boundary_geometry.len()
            );
            assert_eq!(
                negative.boundary_geometry.len(),
                base.boundary_geometry.len()
            );
            let positive_values = flatten_geometry(positive);
            let negative_values = flatten_geometry(negative);
            assert_eq!(positive_values.len(), negative_values.len());
            values.extend(
                positive_values
                    .iter()
                    .zip(&negative_values)
                    .map(|(p, n)| (p - n) / (2.0 * step)),
            );
        }
        values
    };

    for axis in [0, 1, 2, 6, 7, 8] {
        let coarse = derivative(axis, 2.0e-4);
        let fine = derivative(axis, 1.0e-4);
        assert_eq!(coarse.len(), fine.len());
        for (component, (coarse_value, fine_value)) in
            coarse.iter().zip(&fine).enumerate()
        {
            assert!(
                coarse_value.is_finite() && fine_value.is_finite(),
                "non-finite geometry derivative at axis {axis}, component {component}"
            );
            let scale = coarse_value.abs().max(fine_value.abs()).max(1.0e-6);
            assert!(
                (coarse_value - fine_value).abs() <= 5.0e-2 * scale + 1.0e-7,
                "geometry derivative did not refine at axis {axis}, component {component}: coarse={coarse_value} fine={fine_value}"
            );
        }
    }
}
