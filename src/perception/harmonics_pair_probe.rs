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
        for (component, (coarse_value, fine_value)) in coarse.iter().zip(&fine).enumerate() {
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

#[test]
fn occlusion_sweep_resolves_partial_and_near_silhouette_quadrature() {
    let catalog = crate::resources::default_catalog();
    let source = single_body("Carbon", 0.0, 0.0, 0.0, &catalog);
    let receiver = single_body("Carbon", 8.0, 0.0, 0.0, &catalog);
    let mut observations = Vec::new();

    // The dense offset sweep resolves the transition region instead of
    // treating one arbitrary offset as representative of every partial view.
    for blocker_y in [0.0, 0.2, 0.35, 0.5, 0.65, 0.8, 1.0, 5.0] {
        let blocker = single_body("Nitrogen", 4.0, blocker_y, 0.0, &catalog);
        let occluders = std::slice::from_ref(&blocker);
        let receptions =
            material_aura_receptions_with_occluders(&source, &receiver, occluders, &catalog, 0)
                .expect("blocker pose should produce exact receiver samples");
        let potential = directional_resonance_pair_potential_components_with_occluders(
            &source, &receiver, occluders, &catalog, 0,
        )
        .expect("blocker pose should produce exact frequency-resolved potential");

        let visible_extent = receptions
            .iter()
            .map(|sample| sample.projected_extent)
            .sum::<f64>();
        let samples = receptions
            .iter()
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
            .collect::<Vec<_>>();
        let potential = potential
            .into_iter()
            .map(|component| (component.frequency_cycles_per_day, component.energy))
            .collect::<Vec<_>>();
        observations.push((blocker_y, visible_extent, samples, potential));
    }

    let centered = &observations[0];
    let clear = observations.last().expect("clear pose should be recorded");
    assert!(
        centered.1 < clear.1,
        "centered occlusion must reduce visible boundary extent"
    );
    assert!(potential_differs(&centered.3, &clear.3));

    // Identify the actual partial-visibility samples from the exact production
    // output. The closest partial sample to the clear exposure is the
    // near-silhouette observation; no assumed blocker radius or analytic
    // silhouette approximation is substituted for production geometry.
    let partial = observations
        .iter()
        .filter(|observation| {
            observation.1 > centered.1 + 1.0e-12 && observation.1 < clear.1 - 1.0e-12
        })
        .collect::<Vec<_>>();
    assert!(
        !partial.is_empty(),
        "the offset sweep must contain a partially exposed receiver"
    );
    let near_silhouette = partial
        .iter()
        .min_by(|left, right| (clear.1 - left.1).total_cmp(&(clear.1 - right.1)))
        .expect("partial observations should identify a near-silhouette case");

    assert!(
        sample_geometry_differs(&centered.2, &partial[0].2),
        "partial visibility must change exact receiver interval geometry"
    );
    assert!(
        sample_geometry_differs(&near_silhouette.2, &clear.2),
        "near-silhouette visibility must retain a distinct exact boundary representation"
    );
    assert!(
        potential_differs(&near_silhouette.3, &clear.3),
        "near-silhouette visibility must change frequency-resolved potential"
    );

    // Keep the full sweep observable: every retained pose has its own exact
    // quadrature and frequency-resolved potential record.
    assert!(observations.iter().all(|observation| {
        observation.1.is_finite()
            && observation.2.iter().all(|sample| {
                sample.midpoint.0.is_finite()
                    && sample.midpoint.1.is_finite()
                    && sample.projected_extent.is_finite()
            })
            && observation
                .3
                .iter()
                .all(|(frequency, energy)| frequency.is_finite() && energy.is_finite())
    }));
}

#[test]
fn complete_world_pair_force_and_torque_match_independent_potential_differences() {
    let catalog = crate::resources::default_catalog();
    let baseline_pose = [0.0; 12];
    let baseline_bodies = probe_world(baseline_pose, &catalog);
    let mut accumulated =
        vec![crate::harmonics::ResonanceBodyForce::default(); baseline_bodies.len()];

    // Build the complete exact gradient by summing each unordered pair once.
    // Every other body remains an occluder and receives its own derivative.
    for index_a in 0..baseline_bodies.len() {
        for index_b in index_a + 1..baseline_bodies.len() {
            let occluders = baseline_bodies
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != index_a && *index != index_b)
                .map(|(_, body)| body.clone())
                .collect::<Vec<_>>();
            let pair_forces = super::resonance_multi_body_forces_with_occluders(
                &baseline_bodies[index_a],
                &baseline_bodies[index_b],
                &occluders,
                &catalog,
                0,
            )
            .expect("every exact unordered pair should yield its complete multi-body gradient");
            assert_eq!(pair_forces.len(), baseline_bodies.len());

            accumulated[index_a].x += pair_forces[0].x;
            accumulated[index_a].y += pair_forces[0].y;
            accumulated[index_a].torque += pair_forces[0].torque;
            accumulated[index_b].x += pair_forces[1].x;
            accumulated[index_b].y += pair_forces[1].y;
            accumulated[index_b].torque += pair_forces[1].torque;

            let mut occluder_index = 0;
            for (world_index, accumulated_force) in accumulated.iter_mut().enumerate() {
                if world_index == index_a || world_index == index_b {
                    continue;
                }
                let force = pair_forces[occluder_index + 2];
                accumulated_force.x += force.x;
                accumulated_force.y += force.y;
                accumulated_force.torque += force.torque;
                occluder_index += 1;
            }
        }
    }

    // The oracle independently rebuilds every ordered directional reception
    // and sums them into the complete unordered-pair world potential.
    let evaluate = |body_index: usize, component: usize, displacement: f64| {
        let mut bodies = baseline_bodies.clone();
        let body = &baseline_bodies[body_index];
        bodies[body_index] = match component {
            0 => super::translated_physical_material(body, displacement, 0.0)?,
            1 => super::translated_physical_material(body, 0.0, displacement)?,
            2 => super::rotated_physical_material(body, displacement, &catalog)?,
            _ => return None,
        };
        Some(world_potential(&record_world_pairs(&bodies, &catalog, 0)))
    };
    let step = 1.0e-4;

    for (body_index, accumulated_force) in accumulated.iter().enumerate() {
        for component in 0..3 {
            let plus = evaluate(body_index, component, step)
                .expect("positive rigid-pose perturbation should be valid");
            let minus = evaluate(body_index, component, -step)
                .expect("negative rigid-pose perturbation should be valid");
            let derivative = (plus - minus) / (2.0 * step);
            let expected_force = -derivative;
            let actual_force = match component {
                0 => accumulated_force.x,
                1 => accumulated_force.y,
                2 => accumulated_force.torque,
                _ => unreachable!(),
            };
            let scale = expected_force.abs().max(actual_force.abs()).max(1.0e-7);
            assert!(
                (actual_force - expected_force).abs() <= 5.0e-2 * scale + 1.0e-7,
                "complete-world gradient mismatch for body {body_index}, component {component}: pair sum={actual_force}, independent potential derivative={expected_force}"
            );
        }
    }

    assert!(accumulated
        .iter()
        .all(|force| { force.x.is_finite() && force.y.is_finite() && force.torque.is_finite() }));

    // The complete scalar world potential is invariant under a common
    // translation and rotation. Its summed per-body gradient must therefore
    // conserve linear and angular momentum, including reactions on occluders.
    let centers = [
        (baseline_pose[0], baseline_pose[1]),
        (baseline_pose[3], 8.0 + baseline_pose[4]),
        (10.75 + baseline_pose[6], 3.35 + baseline_pose[7]),
        (5.0 + baseline_pose[9], 1.6 + baseline_pose[10]),
    ];
    let net_force_x = accumulated.iter().map(|force| force.x).sum::<f64>();
    let net_force_y = accumulated.iter().map(|force| force.y).sum::<f64>();
    let force_scale = accumulated
        .iter()
        .map(|force| force.x.abs() + force.y.abs())
        .sum::<f64>();
    assert!(
        net_force_x.abs() + net_force_y.abs() <= 5.0e-2 * force_scale + 1.0e-7,
        "complete-world pair gradients must balance linear momentum: net=({net_force_x}, {net_force_y}), scale={force_scale}"
    );

    let net_angular_momentum = accumulated
        .iter()
        .zip(centers)
        .map(|(force, (x, y))| x * force.y - y * force.x + force.torque)
        .sum::<f64>();
    let angular_scale = accumulated
        .iter()
        .zip(centers)
        .map(|(force, (x, y))| (x * force.y).abs() + (y * force.x).abs() + force.torque.abs())
        .sum::<f64>();
    assert!(
        net_angular_momentum.abs() <= 5.0e-2 * angular_scale + 1.0e-7,
        "complete-world pair gradients must balance angular momentum: net={net_angular_momentum}, scale={angular_scale}"
    );
}

#[test]
fn frequency_resolved_boundary_expansion_is_bounded_by_exact_sample_potential() {
    let catalog = crate::resources::default_catalog();
    let source = single_body("Carbon", 0.0, 0.0, 0.17, &catalog);
    let receiver = asymmetric_receiver((0.0, 0.0), -0.11, &catalog);
    let blockers = vec![single_body("Nitrogen", 5.1, 1.7, 0.23, &catalog)];
    let samples = super::directional_resonance_boundary_interaction_samples_with_prepared_emitters(
        &source, &receiver, &blockers, &catalog, 173, None, None,
    )
    .expect("valid occluded pair should produce exact boundary samples");
    assert!(
        !samples.is_empty(),
        "the receiver should retain visible boundary samples"
    );

    let expansions = super::expand_resonance_boundary_interaction_samples(&samples)
        .expect("exact spectral samples should feed the two-distance expansion");
    let exact = super::directional_resonance_pair_potential_components_with_occluders(
        &source, &receiver, &blockers, &catalog, 173,
    )
    .expect("the same pair should produce its exact frequency-resolved potential");

    assert_eq!(expansions.len(), exact.len());
    for expansion in expansions {
        let exact_component = exact
            .iter()
            .find(|component| {
                (component.frequency_cycles_per_day - expansion.frequency_cycles_per_day).abs()
                    <= 1e-9
            })
            .expect("every expanded frequency should exist in the exact potential");
        let error = (expansion.evaluation.potential - exact_component.energy).abs();
        assert!(
            error <= expansion.evaluation.error.potential + 1e-10,
            "frequency {} exceeded its certified potential remainder: error={}, bound={}",
            expansion.frequency_cycles_per_day,
            error,
            expansion.evaluation.error.potential,
        );
    }
}

#[test]
fn mutual_boundary_pair_evaluation_is_symmetric_and_bounds_center_forces() {
    let catalog = crate::resources::default_catalog();
    let a = single_body("Carbon", 0.0, 0.0, 0.13, &catalog);
    let b = asymmetric_receiver((0.0, 0.0), -0.09, &catalog);
    let evaluation = super::resonance_mutual_boundary_pair_evaluation(&a, &b, &catalog, 211)
        .expect("separated unoccluded bodies should have a valid mutual expansion");
    let exact = super::resonance_pair_potential(&a, &b, &catalog, 211)
        .expect("exact symmetric pair potential should be available");
    assert!(
        (evaluation.potential - exact).abs() <= evaluation.potential_error + 1e-9,
        "mutual expansion potential error exceeded its bound: approximate={}, exact={}, bound={}",
        evaluation.potential,
        exact,
        evaluation.potential_error,
    );

    let step = 1.0e-4;
    let plus_a = super::translated_physical_material(&a, step, 0.0)
        .expect("translated source should remain valid");
    let minus_a = super::translated_physical_material(&a, -step, 0.0)
        .expect("translated source should remain valid");
    let exact_force_a_x = -(super::resonance_pair_potential(&plus_a, &b, &catalog, 211)
        .expect("plus source potential")
        - super::resonance_pair_potential(&minus_a, &b, &catalog, 211)
            .expect("minus source potential"))
        / (2.0 * step);
    assert!(
        (evaluation.force_a_x - exact_force_a_x).abs() <= evaluation.force_a_error + 1e-5,
        "source force exceeded its expansion bound: approximate={}, exact={}, bound={}",
        evaluation.force_a_x,
        exact_force_a_x,
        evaluation.force_a_error,
    );

    let plus_b = super::translated_physical_material(&b, step, 0.0)
        .expect("translated receiver should remain valid");
    let minus_b = super::translated_physical_material(&b, -step, 0.0)
        .expect("translated receiver should remain valid");
    let exact_force_b_x = -(super::resonance_pair_potential(&a, &plus_b, &catalog, 211)
        .expect("plus receiver potential")
        - super::resonance_pair_potential(&a, &minus_b, &catalog, 211)
            .expect("minus receiver potential"))
        / (2.0 * step);
    assert!(
        (evaluation.force_b_x - exact_force_b_x).abs() <= evaluation.force_b_error + 1e-5,
        "receiver force exceeded its expansion bound: approximate={}, exact={}, bound={}",
        evaluation.force_b_x,
        exact_force_b_x,
        evaluation.force_b_error,
    );

    let plus_a_y = super::translated_physical_material(&a, 0.0, step)
        .expect("y-translated source should remain valid");
    let minus_a_y = super::translated_physical_material(&a, 0.0, -step)
        .expect("y-translated source should remain valid");
    let exact_force_a_y = -(super::resonance_pair_potential(&plus_a_y, &b, &catalog, 211)
        .expect("positive source-y potential")
        - super::resonance_pair_potential(&minus_a_y, &b, &catalog, 211)
            .expect("negative source-y potential"))
        / (2.0 * step);
    assert!(
        (evaluation.force_a_y - exact_force_a_y).abs() <= evaluation.force_a_error + 1e-5,
        "source y-force exceeded its expansion bound: approximate={}, exact={}, bound={}",
        evaluation.force_a_y,
        exact_force_a_y,
        evaluation.force_a_error,
    );

    let plus_b_y = super::translated_physical_material(&b, 0.0, step)
        .expect("y-translated receiver should remain valid");
    let minus_b_y = super::translated_physical_material(&b, 0.0, -step)
        .expect("y-translated receiver should remain valid");
    let exact_force_b_y = -(super::resonance_pair_potential(&a, &plus_b_y, &catalog, 211)
        .expect("positive receiver-y potential")
        - super::resonance_pair_potential(&a, &minus_b_y, &catalog, 211)
            .expect("negative receiver-y potential"))
        / (2.0 * step);
    assert!(
        (evaluation.force_b_y - exact_force_b_y).abs() <= evaluation.force_b_error + 1e-5,
        "receiver y-force exceeded its expansion bound: approximate={}, exact={}, bound={}",
        evaluation.force_b_y,
        exact_force_b_y,
        evaluation.force_b_error,
    );
    assert!(
        (evaluation.force_a_x + evaluation.force_b_x).abs() <= 1e-8
            && (evaluation.force_a_y + evaluation.force_b_y).abs() <= 1e-8,
        "unoccluded mutual center forces must be equal and opposite"
    );
}

#[test]
fn error_budgeted_boundary_expansion_refines_until_all_remainders_fit() {
    use crate::resonance_boundary_far_field::{
        BoundaryKernelVector, BoundaryPotentialError, WeightedBoundaryKernelSample,
    };

    let source = BoundaryKernelVector { x: 0.0, y: 0.0 };
    let receiver = BoundaryKernelVector { x: 8.0, y: -1.0 };
    let samples = vec![
        WeightedBoundaryKernelSample {
            position: BoundaryKernelVector { x: 2.0, y: 0.5 },
            weight: 0.7,
        },
        WeightedBoundaryKernelSample {
            position: BoundaryKernelVector { x: 2.5, y: 0.7 },
            weight: -0.2,
        },
        WeightedBoundaryKernelSample {
            position: BoundaryKernelVector { x: 3.0, y: 1.1 },
            weight: 0.4,
        },
        WeightedBoundaryKernelSample {
            position: BoundaryKernelVector { x: 3.5, y: 0.8 },
            weight: 0.3,
        },
    ];
    let exact_potential: f64 = samples
        .iter()
        .map(|sample| {
            let source_distance =
                (sample.position.x - source.x).hypot(sample.position.y - source.y);
            let receiver_distance =
                (sample.position.x - receiver.x).hypot(sample.position.y - receiver.y);
            sample.weight / ((1.0 + source_distance) * (1.0 + receiver_distance))
        })
        .sum();

    let budget = BoundaryPotentialError {
        potential: 1.0e-6,
        source_force: 1.0e-6,
        receiver_force: 1.0e-6,
    };
    let evaluation = super::evaluate_boundary_kernel_samples_adaptive_with_budget(
        source,
        receiver,
        samples.clone(),
        budget,
    )
    .expect("adaptive subdivision should satisfy the requested absolute budgets");

    assert!(evaluation.error.potential <= budget.potential);
    assert!(evaluation.error.source_force <= budget.source_force);
    assert!(evaluation.error.receiver_force <= budget.receiver_force);
    assert!(
        samples.iter().all(|sample| {
            (sample.position.x - evaluation.center.x)
                .hypot(sample.position.y - evaluation.center.y)
                <= evaluation.radius + 1.0e-12
        }),
        "combined adaptive result must retain a radius enclosing every original sample"
    );
    assert!(
        (evaluation.potential - exact_potential).abs() <= evaluation.error.potential + 1.0e-12,
        "reported potential remainder must contain the exact discrete sum"
    );

    let singleton = vec![samples[0]];
    assert!(
        super::evaluate_boundary_kernel_samples_adaptive_with_budget(
            source,
            receiver,
            singleton,
            BoundaryPotentialError::default(),
        )
        .is_none(),
        "a nonzero roundoff remainder cannot satisfy a zero error budget"
    );
}
