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
    source_zero_shift: (f64, f64),
    source_zero_rotation: f64,
    receiver_shift: (f64, f64),
    receiver_rotation: f64,
    catalog: &[crate::resources::BaseResource],
) -> Vec<crate::physical_material::PhysicalMaterial> {
    vec![
        single_body(
            "Hydrogen",
            source_zero_shift.0,
            source_zero_shift.1,
            source_zero_rotation,
            catalog,
        ),
        single_body("Nitrogen", 0.0, 8.0, 0.0, catalog),
        asymmetric_receiver(receiver_shift, receiver_rotation, catalog),
        single_body("Oxygen", 5.0, 1.6, 0.0, catalog),
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
    let baseline_bodies = probe_world((0.0, 0.0), 0.0, (0.0, 0.0), 0.0, &catalog);
    let changed_bodies = probe_world((0.35, 0.25), 0.2, (0.0, 0.0), 0.4, &catalog);

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
fn complete_world_pose_derivatives_match_refined_central_differences() {
    let catalog = crate::resources::default_catalog();
    // Parameters are source x/y/rotation followed by receiver x/y/rotation.
    let evaluate = |pose: [f64; 6]| {
        let bodies = probe_world(
            (pose[0], pose[1]),
            pose[2],
            (pose[3], pose[4]),
            pose[5],
            &catalog,
        );
        world_potential(&record_world_pairs(&bodies, &catalog, 0))
    };

    // Recompute every ordered pair with the other bodies as exact occluders.
    // These derivatives therefore include changes to sample locations,
    // projected extents, visibility, and frequency weights—not frozen samples.
    let central_difference = |step: f64, axis: usize| {
        let mut plus = [0.0; 6];
        let mut minus = [0.0; 6];
        plus[axis] = step;
        minus[axis] = -step;
        (evaluate(plus) - evaluate(minus)) / (2.0 * step)
    };

    let mut nonzero_derivatives = 0;
    for axis in 0..6 {
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
        nonzero_derivatives >= 4,
        "the asymmetric complete-world fixture should exercise at least four independent pose derivatives; got {nonzero_derivatives}"
    );
}
