//! Bounded Taylor treatment of one ordered pair's exact receiver-boundary samples.
//!
//! Each input point is one exact production interval midpoint. Its projected
//! extent and signed frequency-resolved cross-energy coefficient are folded
//! into the weight. This approximates only the smooth two-distance kernel over
//! a cluster of those already-established samples. Visibility and interval
//! generation remain exact and outside this module.
//!
//! The fixed-sample result includes scalar potential and source/receiver
//! center-gradient estimates with independent remainder bounds. These bounds
//! hold only while sample positions and weights are fixed. Pose-dependent
//! boundary geometry, projected extents, visibility, and torque remain outside
//! this primitive; it must not feed live movement.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryKernelVector {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl BoundaryKernelVector {
    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn norm(self) -> f64 {
        self.x.hypot(self.y)
    }
}

/// One exact receiver-boundary sample for one ordered pair and one frequency.
/// Weight includes the signed spectral cross coefficient, both mass scales,
/// and the exact point-source projected extent for this interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WeightedBoundaryKernelSample {
    pub(crate) position: BoundaryKernelVector,
    pub(crate) weight: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryPotentialError {
    pub(crate) potential: f64,
    pub(crate) source_force: f64,
    pub(crate) receiver_force: f64,
}

/// Certified per-sample envelope between two poses inside one smooth
/// visibility/topology region. The caller must preserve sample ordering and
/// prove these bounds for the complete pose cell.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundarySampleMotionBound {
    pub(crate) position_displacement: f64,
    pub(crate) absolute_weight_change: f64,
}

/// Change bound caused only by moving the exact boundary samples and changing
/// their signed weights. It does not include the chain-rule force/torque from
/// differentiating sample position or weight with respect to body pose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryGeometryError {
    pub(crate) potential: f64,
    pub(crate) source_force: f64,
    pub(crate) receiver_force: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryPotentialEvaluation {
    pub(crate) potential: f64,
    /// Derivatives of the same approximate scalar potential with respect to
    /// the source and receiver reference positions.
    pub(crate) source_gradient: BoundaryKernelVector,
    pub(crate) receiver_gradient: BoundaryKernelVector,
    pub(crate) error: BoundaryPotentialError,
    pub(crate) center: BoundaryKernelVector,
    pub(crate) radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoundaryKernelExpansion {
    source_center: BoundaryKernelVector,
    receiver_center: BoundaryKernelVector,
    sample_center: BoundaryKernelVector,
    samples: Vec<WeightedBoundaryKernelSample>,
    total_weight: f64,
    absolute_weight: f64,
    first_moment: BoundaryKernelVector,
    radius: f64,
}

/// Conservative displacement of a point attached to a rigid body when its
/// reference point translates by at most `translation_bound` and its
/// orientation changes by at most `rotation_bound_radians`. The point is
/// at most `maximum_radius` from the rotation pivot. This bounds only the
/// rigid transport contribution; source-ray intersection changes and
/// visibility/topology changes require separate certificates.
pub(crate) fn rigid_pose_point_displacement_bound(
    maximum_radius: f64,
    translation_bound: f64,
    rotation_bound_radians: f64,
) -> Option<f64> {
    if !maximum_radius.is_finite()
        || maximum_radius < 0.0
        || !translation_bound.is_finite()
        || translation_bound < 0.0
        || !rotation_bound_radians.is_finite()
        || rotation_bound_radians < 0.0
    {
        return None;
    }
    let angular_displacement =
        2.0 * maximum_radius * (0.5 * rotation_bound_radians.min(std::f64::consts::PI)).sin();
    let bound = translation_bound + angular_displacement;
    bound.is_finite().then_some(bound)
}

/// Pose envelope for one source ray intersecting one receiver edge. The
/// critical point is the blocker feature defining the ray; its displacement
/// bound must include its own rigid motion and any source-dependent tangent
/// construction. This helper certifies only this one intersection, not the
/// ordering of multiple cuts or the complete visibility topology.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RayEdgeIntersectionMotion {
    pub(crate) source_displacement: f64,
    pub(crate) critical_point_displacement: f64,
    pub(crate) edge_start_displacement: f64,
    pub(crate) edge_end_displacement: f64,
}

/// Bound movement of a ray/edge cut while it remains a strict interior
/// intersection and the critical point remains beyond the receiver edge.
///
/// The exact construction is s + u(v-s) = a + t(b-a). A denominator interval
/// excluding zero and strict t/u margins are required; near-parallel rays,
/// endpoint events, and cut-order transitions fail closed.
#[allow(dead_code)]
pub(crate) fn ray_edge_intersection_displacement_bound(
    source: BoundaryKernelVector,
    critical_point: BoundaryKernelVector,
    edge_start: BoundaryKernelVector,
    edge_end: BoundaryKernelVector,
    motion: RayEdgeIntersectionMotion,
) -> Option<f64> {
    let points = [source, critical_point, edge_start, edge_end];
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return None;
    }
    let bounds = [
        motion.source_displacement,
        motion.critical_point_displacement,
        motion.edge_start_displacement,
        motion.edge_end_displacement,
    ];
    if bounds
        .iter()
        .any(|bound| !bound.is_finite() || *bound < 0.0)
    {
        return None;
    }

    let q = BoundaryKernelVector {
        x: edge_start.x - source.x,
        y: edge_start.y - source.y,
    };
    let r = BoundaryKernelVector {
        x: critical_point.x - source.x,
        y: critical_point.y - source.y,
    };
    let e = BoundaryKernelVector {
        x: edge_end.x - edge_start.x,
        y: edge_end.y - edge_start.y,
    };
    let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
        left.x * right.y - left.y * right.x
    };
    let denominator = cross(e, r);
    let numerator_t = -cross(q, r);
    let numerator_u = -cross(q, e);
    if !denominator.is_finite() || !numerator_t.is_finite() || !numerator_u.is_finite() {
        return None;
    }
    let t = numerator_t / denominator;
    let u = numerator_u / denominator;
    if !t.is_finite()
        || !u.is_finite()
        || !(1.0e-10..1.0 - 1.0e-10).contains(&t)
        || u <= 1.0 + 1.0e-10
    {
        return None;
    }

    let delta_q = motion.edge_start_displacement + motion.source_displacement;
    let delta_r = motion.critical_point_displacement + motion.source_displacement;
    let delta_e = motion.edge_start_displacement + motion.edge_end_displacement;
    let q_norm = q.norm();
    let r_norm = r.norm();
    let e_norm = e.norm();
    let denominator_change = delta_e * r_norm + e_norm * delta_r + delta_e * delta_r;
    let denominator_min = denominator.abs() - denominator_change;
    if !denominator_min.is_finite() || denominator_min <= f64::EPSILON {
        return None;
    }

    let numerator_t_change = delta_q * r_norm + q_norm * delta_r + delta_q * delta_r;
    let numerator_u_change = delta_q * e_norm + q_norm * delta_e + delta_q * delta_e;
    let t_change = (numerator_t_change + t.abs() * denominator_change) / denominator_min;
    let u_change = (numerator_u_change + u.abs() * denominator_change) / denominator_min;
    if !t_change.is_finite()
        || !u_change.is_finite()
        || t_change >= t.min(1.0 - t)
        || u_change >= u - 1.0
    {
        return None;
    }

    // p = a + t e. Bound all products including simultaneous changes in t
    // and the edge vector.
    let point_change =
        motion.edge_start_displacement + t.abs() * delta_e + e_norm * t_change + delta_e * t_change;
    let roundoff_guard = 64.0 * f64::EPSILON * (point_change + q_norm + r_norm + e_norm + 1.0);
    let result = point_change + roundoff_guard;
    result.is_finite().then_some(result)
}

/// Certify that a supplied ordered list of visibility-cut parameters cannot
/// cross or reach an edge endpoint anywhere inside their individual motion
/// envelopes. This does not prove that the list contains every geometric cut.
#[allow(dead_code)]
pub(crate) fn certify_cut_parameter_order(cuts: &[(f64, f64)]) -> Option<()> {
    let mut previous_upper = 1.0e-10;
    for &(parameter, displacement) in cuts {
        if !parameter.is_finite()
            || !displacement.is_finite()
            || displacement < 0.0
            || parameter - displacement <= previous_upper
            || parameter + displacement >= 1.0 - 1.0e-10
        {
            return None;
        }
        previous_upper = parameter + displacement;
    }
    Some(())
}

/// Combined error certificate for one ordered pair over a pose cell whose
/// visibility topology and exact sample correspondence have been established
/// by the caller. The cut-order check verifies supplied cut envelopes; it does
/// not prove that every geometric cut was enumerated or blocker labels stayed
/// unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryPoseCellError {
    pub(crate) potential: f64,
    pub(crate) source_force: f64,
    pub(crate) receiver_force: f64,
}

/// Composition of the sample-cluster Taylor remainder and the pose-driven
/// sample/weight-change envelope. Forces are center-translation derivatives
/// only; pose-chain-rule terms and torque are not included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryPoseCellEvaluation {
    pub(crate) evaluation: BoundaryPotentialEvaluation,
    pub(crate) error: BoundaryPoseCellError,
}

/// Validate supplied visibility-cut ordering and compose all currently
/// available scalar-potential and center-gradient error terms for one
/// source-specific ordered pair.
///
/// The caller must additionally establish complete cut enumeration, stable
/// blocker classification on every open interval, and one-to-one sample
/// correspondence. Each sample displacement must include rigid transport,
/// moving ray/edge intersections, and interval geometry. Each weight-change
/// bound must include projected-extent and signed spectral-coefficient
/// changes. Missing or invalid local certificates fail closed.
///
/// This is not a complete rigid-pose force/torque certificate.
#[allow(dead_code)]
pub(crate) fn certify_boundary_pose_cell(
    expansion: &BoundaryKernelExpansion,
    ordered_visibility_cuts: &[(f64, f64)],
    sample_motion: &[BoundarySampleMotionBound],
) -> Option<BoundaryPoseCellEvaluation> {
    certify_cut_parameter_order(ordered_visibility_cuts)?;
    let evaluation = expansion.evaluate()?;
    let geometry = expansion.bound_sample_geometry_change(sample_motion)?;
    let error = BoundaryPoseCellError {
        potential: evaluation.error.potential + geometry.potential,
        source_force: evaluation.error.source_force + geometry.source_force,
        receiver_force: evaluation.error.receiver_force + geometry.receiver_force,
    };
    [error.potential, error.source_force, error.receiver_force]
        .iter()
        .all(|bound| bound.is_finite())
        .then_some(BoundaryPoseCellEvaluation { evaluation, error })
}

impl BoundaryKernelExpansion {
    /// Retain exact sample correspondence for this ordered pair. The
    /// expansion disk is centered at sample_center and contains every midpoint.
    pub(crate) fn new(
        source_center: BoundaryKernelVector,
        receiver_center: BoundaryKernelVector,
        sample_center: BoundaryKernelVector,
        samples: Vec<WeightedBoundaryKernelSample>,
    ) -> Option<Self> {
        let finite = |point: BoundaryKernelVector| point.x.is_finite() && point.y.is_finite();
        if !finite(source_center)
            || !finite(receiver_center)
            || !finite(sample_center)
            || samples.is_empty()
        {
            return None;
        }

        let mut total_weight = 0.0;
        let mut absolute_weight = 0.0;
        let mut first_moment = BoundaryKernelVector::default();
        let mut radius: f64 = 0.0;
        for sample in &samples {
            if !finite(sample.position) || !sample.weight.is_finite() {
                return None;
            }
            let offset = BoundaryKernelVector {
                x: sample.position.x - sample_center.x,
                y: sample.position.y - sample_center.y,
            };
            total_weight += sample.weight;
            absolute_weight += sample.weight.abs();
            first_moment.x += sample.weight * offset.x;
            first_moment.y += sample.weight * offset.y;
            radius = radius.max(offset.norm());
        }
        if !total_weight.is_finite()
            || !absolute_weight.is_finite()
            || !first_moment.x.is_finite()
            || !first_moment.y.is_finite()
            || !radius.is_finite()
        {
            return None;
        }

        Some(Self {
            source_center,
            receiver_center,
            sample_center,
            samples,
            total_weight,
            absolute_weight,
            first_moment,
            radius,
        })
    }

    /// First-order Taylor sum of each signed weight times the product of the
    /// two existing distance attenuations. The expansion disk must remain
    /// strictly separated from both body reference positions.
    pub(crate) fn evaluate(&self) -> Option<BoundaryPotentialEvaluation> {
        let center = self.sample_center;
        let source_delta = BoundaryKernelVector {
            x: center.x - self.source_center.x,
            y: center.y - self.source_center.y,
        };
        let receiver_delta = BoundaryKernelVector {
            x: center.x - self.receiver_center.x,
            y: center.y - self.receiver_center.y,
        };
        let source_distance = source_delta.norm();
        let receiver_distance = receiver_delta.norm();
        let source_min = source_distance - self.radius;
        let receiver_min = receiver_distance - self.radius;
        if !source_min.is_finite()
            || !receiver_min.is_finite()
            || source_min <= f64::EPSILON
            || receiver_min <= f64::EPSILON
        {
            return None;
        }

        let (source_value, source_gradient) =
            radial_kernel_value_gradient(source_delta, source_distance);
        let (receiver_value, receiver_gradient) =
            radial_kernel_value_gradient(receiver_delta, receiver_distance);
        let kernel = source_value * receiver_value;
        let gradient = BoundaryKernelVector {
            x: source_gradient.x * receiver_value + source_value * receiver_gradient.x,
            y: source_gradient.y * receiver_value + source_value * receiver_gradient.y,
        };
        let potential = self.total_weight * kernel + gradient.dot(self.first_moment);

        // Differentiate the same first-order sample-cluster expansion with
        // respect to each body center, holding exact sample positions/weights
        // fixed. H_f M_1 is evaluated as a radial Hessian-vector product.
        let source_hessian_moment =
            radial_hessian_apply(source_delta, source_distance, self.first_moment);
        let receiver_hessian_moment =
            radial_hessian_apply(receiver_delta, receiver_distance, self.first_moment);
        let source_gradient_moment = source_gradient.dot(self.first_moment);
        let receiver_gradient_moment = receiver_gradient.dot(self.first_moment);
        let source_center_gradient = BoundaryKernelVector {
            x: -self.total_weight * source_gradient.x * receiver_value
                - receiver_value * source_hessian_moment.x
                - source_gradient.x * receiver_gradient_moment,
            y: -self.total_weight * source_gradient.y * receiver_value
                - receiver_value * source_hessian_moment.y
                - source_gradient.y * receiver_gradient_moment,
        };
        let receiver_center_gradient = BoundaryKernelVector {
            x: -self.total_weight * receiver_gradient.x * source_value
                - source_value * receiver_hessian_moment.x
                - receiver_gradient.x * source_gradient_moment,
            y: -self.total_weight * receiver_gradient.y * source_value
                - source_value * receiver_hessian_moment.y
                - receiver_gradient.y * source_gradient_moment,
        };

        // Bounds for the fixed-sample center-gradient remainders. For
        // h_a(p)=d[g(p)]/d(c_a)=-grad(f_a) f_b, bound its spatial Hessian by
        // T_a F_b + 3 H_a G_b + 3 G_a H_b. The factor 3 deliberately
        // overbounds the mixed tensor contractions in two dimensions.
        let source_value_bound = 1.0 / (1.0 + source_min);
        let receiver_value_bound = 1.0 / (1.0 + receiver_min);
        let source_gradient_bound = 1.0 / (1.0 + source_min).powi(2);
        let receiver_gradient_bound = 1.0 / (1.0 + receiver_min).powi(2);
        let source_hessian_bound = radial_hessian_bound(source_min);
        let receiver_hessian_bound = radial_hessian_bound(receiver_min);
        let source_third_bound = radial_third_bound(source_min);
        let receiver_third_bound = radial_third_bound(receiver_min);
        let product_hessian_bound = receiver_value_bound * source_hessian_bound
            + source_value_bound * receiver_hessian_bound
            + 2.0 * source_gradient_bound * receiver_gradient_bound;
        let source_gradient_remainder = source_third_bound * receiver_value_bound
            + 3.0 * source_hessian_bound * receiver_gradient_bound
            + 3.0 * source_gradient_bound * receiver_hessian_bound;
        let receiver_gradient_remainder = receiver_third_bound * source_value_bound
            + 3.0 * receiver_hessian_bound * source_gradient_bound
            + 3.0 * receiver_gradient_bound * source_hessian_bound;
        let scale = 0.5 * self.absolute_weight * self.radius * self.radius;
        let potential_analytic_error = scale * product_hessian_bound;
        let source_analytic_error = scale * source_gradient_remainder;
        let receiver_analytic_error = scale * receiver_gradient_remainder;
        let roundoff_guard = 64.0
            * f64::EPSILON
            * (potential.abs()
                + source_center_gradient.norm()
                + receiver_center_gradient.norm()
                + self.absolute_weight
                + 1.0);
        let error = BoundaryPotentialError {
            potential: potential_analytic_error + roundoff_guard,
            source_force: source_analytic_error + roundoff_guard,
            receiver_force: receiver_analytic_error + roundoff_guard,
        };
        if !potential.is_finite()
            || !source_center_gradient.x.is_finite()
            || !source_center_gradient.y.is_finite()
            || !receiver_center_gradient.x.is_finite()
            || !receiver_center_gradient.y.is_finite()
            || !error.potential.is_finite()
            || !error.source_force.is_finite()
            || !error.receiver_force.is_finite()
        {
            return None;
        }

        Some(BoundaryPotentialEvaluation {
            potential,
            source_gradient: source_center_gradient,
            receiver_gradient: receiver_center_gradient,
            error,
            center,
            radius: self.radius,
        })
    }

    /// Bound the change in the exact discrete potential and its fixed-sample
    /// source/receiver center gradients under certified sample motion.
    ///
    /// For every sample, the caller supplies eta >= |p' - p| and
    /// omega >= |w' - w|. The closed disk of radius eta around p must remain
    /// separated from both body centers. Sample correspondence and visibility
    /// topology must be unchanged throughout the pose cell; otherwise callers
    /// must reject this path and use exact evaluation.
    pub(crate) fn bound_sample_geometry_change(
        &self,
        motion: &[BoundarySampleMotionBound],
    ) -> Option<BoundaryGeometryError> {
        if motion.len() != self.samples.len() {
            return None;
        }

        let mut potential_error = 0.0;
        let mut source_force_error = 0.0;
        let mut receiver_force_error = 0.0;
        for (sample, bound) in self.samples.iter().zip(motion) {
            let eta = bound.position_displacement;
            let omega = bound.absolute_weight_change;
            if !eta.is_finite() || eta < 0.0 || !omega.is_finite() || omega < 0.0 {
                return None;
            }
            let source_distance = BoundaryKernelVector {
                x: sample.position.x - self.source_center.x,
                y: sample.position.y - self.source_center.y,
            }
            .norm();
            let receiver_distance = BoundaryKernelVector {
                x: sample.position.x - self.receiver_center.x,
                y: sample.position.y - self.receiver_center.y,
            }
            .norm();
            let source_min = source_distance - eta;
            let receiver_min = receiver_distance - eta;
            if !source_min.is_finite()
                || !receiver_min.is_finite()
                || source_min <= f64::EPSILON
                || receiver_min <= f64::EPSILON
            {
                return None;
            }

            let source_value = 1.0 / (1.0 + source_min);
            let receiver_value = 1.0 / (1.0 + receiver_min);
            let source_gradient = 1.0 / (1.0 + source_min).powi(2);
            let receiver_gradient = 1.0 / (1.0 + receiver_min).powi(2);
            let source_hessian = radial_hessian_bound(source_min);
            let receiver_hessian = radial_hessian_bound(receiver_min);

            // g = f_a f_b; h_a = -grad(f_a) f_b and
            // h_b = -grad(f_b) f_a. Their spatial Lipschitz bounds follow
            // from the product rule and radial Hessian operator bounds.
            let kernel_bound = source_value * receiver_value;
            let kernel_lipschitz =
                source_gradient * receiver_value + source_value * receiver_gradient;
            let source_gradient_bound = source_gradient * receiver_value;
            let receiver_gradient_bound = receiver_gradient * source_value;
            let source_gradient_lipschitz =
                source_hessian * receiver_value + source_gradient * receiver_gradient;
            let receiver_gradient_lipschitz =
                receiver_hessian * source_value + receiver_gradient * source_gradient;

            let weight = sample.weight.abs();
            potential_error += omega * kernel_bound + weight * kernel_lipschitz * eta;
            source_force_error +=
                omega * source_gradient_bound + weight * source_gradient_lipschitz * eta;
            receiver_force_error +=
                omega * receiver_gradient_bound + weight * receiver_gradient_lipschitz * eta;
        }

        let roundoff_guard = 64.0
            * f64::EPSILON
            * (potential_error + source_force_error + receiver_force_error + 1.0);
        let result = BoundaryGeometryError {
            potential: potential_error + roundoff_guard,
            source_force: source_force_error + roundoff_guard,
            receiver_force: receiver_force_error + roundoff_guard,
        };
        [result.potential, result.source_force, result.receiver_force]
            .iter()
            .all(|value| value.is_finite())
            .then_some(result)
    }

    /// Exact discrete sum, retained as the differential oracle.
    #[cfg(test)]
    fn exact_potential(&self) -> f64 {
        self.samples
            .iter()
            .map(|sample| {
                let source_distance = BoundaryKernelVector {
                    x: sample.position.x - self.source_center.x,
                    y: sample.position.y - self.source_center.y,
                }
                .norm();
                let receiver_distance = BoundaryKernelVector {
                    x: sample.position.x - self.receiver_center.x,
                    y: sample.position.y - self.receiver_center.y,
                }
                .norm();
                sample.weight / ((1.0 + source_distance) * (1.0 + receiver_distance))
            })
            .sum()
    }

    /// Exact fixed-sample derivatives with respect to both body centers.
    #[cfg(test)]
    fn exact_center_gradients(&self) -> (BoundaryKernelVector, BoundaryKernelVector) {
        let mut source = BoundaryKernelVector::default();
        let mut receiver = BoundaryKernelVector::default();
        for sample in &self.samples {
            let source_delta = BoundaryKernelVector {
                x: sample.position.x - self.source_center.x,
                y: sample.position.y - self.source_center.y,
            };
            let receiver_delta = BoundaryKernelVector {
                x: sample.position.x - self.receiver_center.x,
                y: sample.position.y - self.receiver_center.y,
            };
            let source_distance = source_delta.norm();
            let receiver_distance = receiver_delta.norm();
            let (source_value, source_gradient) =
                radial_kernel_value_gradient(source_delta, source_distance);
            let (receiver_value, receiver_gradient) =
                radial_kernel_value_gradient(receiver_delta, receiver_distance);
            source.x -= sample.weight * source_gradient.x * receiver_value;
            source.y -= sample.weight * source_gradient.y * receiver_value;
            receiver.x -= sample.weight * receiver_gradient.x * source_value;
            receiver.y -= sample.weight * receiver_gradient.y * source_value;
        }
        (source, receiver)
    }
}

fn radial_kernel_value_gradient(
    delta: BoundaryKernelVector,
    distance: f64,
) -> (f64, BoundaryKernelVector) {
    let value = 1.0 / (1.0 + distance);
    let scale = -1.0 / (distance * (1.0 + distance).powi(2));
    (
        value,
        BoundaryKernelVector {
            x: scale * delta.x,
            y: scale * delta.y,
        },
    )
}

fn radial_hessian_bound(minimum_distance: f64) -> f64 {
    let first = -1.0 / (1.0 + minimum_distance).powi(2);
    let second = 2.0 / (1.0 + minimum_distance).powi(3);
    second.abs().max((first / minimum_distance).abs())
}

fn radial_third_bound(minimum_distance: f64) -> f64 {
    let d = minimum_distance;
    6.0 / (1.0 + d).powi(4)
        + 3.0 * (2.0 / (d * (1.0 + d).powi(3)) + 1.0 / (d * d * (1.0 + d).powi(2)))
}

fn radial_hessian_apply(
    delta: BoundaryKernelVector,
    distance: f64,
    vector: BoundaryKernelVector,
) -> BoundaryKernelVector {
    let radial_unit = BoundaryKernelVector {
        x: delta.x / distance,
        y: delta.y / distance,
    };
    let first = -1.0 / (1.0 + distance).powi(2);
    let second = 2.0 / (1.0 + distance).powi(3);
    let tangential = first / distance;
    let radial_component = radial_unit.dot(vector);
    BoundaryKernelVector {
        x: tangential * vector.x + (second - tangential) * radial_unit.x * radial_component,
        y: tangential * vector.y + (second - tangential) * radial_unit.y * radial_component,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rigid_pose_displacement_bound_contains_transformed_boundary_points() {
        let translation_bound = 0.35;
        let rotation_bound = 0.8;
        let maximum_radius: f64 = 3.2;
        let bound =
            rigid_pose_point_displacement_bound(maximum_radius, translation_bound, rotation_bound)
                .expect("finite rigid-pose envelope");
        for index in 0..128 {
            let radius = maximum_radius * (index as f64 / 127.0);
            let angle = index as f64 * 0.173;
            let point = (radius * angle.cos(), radius * angle.sin());
            let translation_angle = index as f64 * 0.071;
            let translation = (
                translation_bound * translation_angle.cos(),
                translation_bound * translation_angle.sin(),
            );
            let rotation = rotation_bound * (index as f64 / 127.0);
            let moved = (
                point.0 * rotation.cos() - point.1 * rotation.sin() + translation.0,
                point.0 * rotation.sin() + point.1 * rotation.cos() + translation.1,
            );
            let displacement = (moved.0 - point.0).hypot(moved.1 - point.1);
            assert!(displacement <= bound + 1e-12);
        }
        let full_turn_bound =
            rigid_pose_point_displacement_bound(maximum_radius, 0.0, 4.0 * std::f64::consts::PI)
                .expect("large angular envelope");
        assert!((full_turn_bound - 2.0 * maximum_radius).abs() < 1e-12);
    }

    #[test]
    fn rigid_pose_displacement_bound_rejects_invalid_inputs() {
        assert!(rigid_pose_point_displacement_bound(-1.0, 0.0, 0.0).is_none());
        assert!(rigid_pose_point_displacement_bound(1.0, f64::NAN, 0.0).is_none());
        assert!(rigid_pose_point_displacement_bound(1.0, 0.0, -0.1).is_none());
    }
    #[test]
    fn ray_edge_intersection_motion_bound_contains_perturbed_cuts() {
        let source = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let critical = BoundaryKernelVector { x: 1.0, y: 0.5 };
        let start = BoundaryKernelVector { x: 2.0, y: -2.0 };
        let end = BoundaryKernelVector { x: 2.0, y: 2.0 };
        let motion = RayEdgeIntersectionMotion {
            source_displacement: 0.01,
            critical_point_displacement: 0.015,
            edge_start_displacement: 0.005,
            edge_end_displacement: 0.005,
        };
        let bound = ray_edge_intersection_displacement_bound(source, critical, start, end, motion)
            .expect("strict ray-edge intersection should have a finite motion bound");

        let intersection = |s: BoundaryKernelVector,
                            v: BoundaryKernelVector,
                            a: BoundaryKernelVector,
                            b: BoundaryKernelVector| {
            let q = BoundaryKernelVector {
                x: a.x - s.x,
                y: a.y - s.y,
            };
            let r = BoundaryKernelVector {
                x: v.x - s.x,
                y: v.y - s.y,
            };
            let e = BoundaryKernelVector {
                x: b.x - a.x,
                y: b.y - a.y,
            };
            let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
                left.x * right.y - left.y * right.x
            };
            let denominator = cross(e, r);
            let t = -cross(q, r) / denominator;
            let u = -cross(q, e) / denominator;
            assert!(t > 0.0 && t < 1.0 && u > 1.0);
            BoundaryKernelVector {
                x: a.x + t * e.x,
                y: a.y + t * e.y,
            }
        };
        let original = intersection(source, critical, start, end);
        let mut state = 0x4d59_5df4_d0f3_3173_u64;
        let mut next_direction = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let angle = ((state >> 32) as u32) as f64 / u32::MAX as f64 * std::f64::consts::TAU;
            (angle.cos(), angle.sin())
        };

        for _ in 0..256 {
            let (sx, sy) = next_direction();
            let (vx, vy) = next_direction();
            let (ax, ay) = next_direction();
            let (bx, by) = next_direction();
            let moved_source = BoundaryKernelVector {
                x: source.x + sx * motion.source_displacement * 0.8,
                y: source.y + sy * motion.source_displacement * 0.8,
            };
            let moved_critical = BoundaryKernelVector {
                x: critical.x + vx * motion.critical_point_displacement * 0.8,
                y: critical.y + vy * motion.critical_point_displacement * 0.8,
            };
            let moved_start = BoundaryKernelVector {
                x: start.x + ax * motion.edge_start_displacement * 0.8,
                y: start.y + ay * motion.edge_start_displacement * 0.8,
            };
            let moved_end = BoundaryKernelVector {
                x: end.x + bx * motion.edge_end_displacement * 0.8,
                y: end.y + by * motion.edge_end_displacement * 0.8,
            };
            let moved = intersection(moved_source, moved_critical, moved_start, moved_end);
            assert!(
                (moved.x - original.x).hypot(moved.y - original.y) <= bound,
                "perturbed cut exceeded certified displacement"
            );
        }
    }

    #[test]
    fn ray_edge_intersection_motion_bound_rejects_singular_or_transitioning_cuts() {
        let source = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let critical = BoundaryKernelVector { x: 1.0, y: 0.5 };
        let start = BoundaryKernelVector { x: 2.0, y: -2.0 };
        let end = BoundaryKernelVector { x: 2.0, y: 2.0 };
        let valid = RayEdgeIntersectionMotion {
            source_displacement: 0.01,
            critical_point_displacement: 0.015,
            edge_start_displacement: 0.005,
            edge_end_displacement: 0.005,
        };
        assert!(ray_edge_intersection_displacement_bound(
            source,
            critical,
            start,
            end,
            RayEdgeIntersectionMotion {
                source_displacement: f64::NAN,
                ..valid
            },
        )
        .is_none());
        assert!(ray_edge_intersection_displacement_bound(
            source,
            critical,
            start,
            end,
            RayEdgeIntersectionMotion {
                source_displacement: 10.0,
                ..valid
            },
        )
        .is_none());
        assert!(
            ray_edge_intersection_displacement_bound(source, source, start, end, valid,).is_none()
        );
    }

    #[test]
    fn cut_parameter_order_certificate_rejects_overlap_and_edge_events() {
        assert!(certify_cut_parameter_order(&[(0.2, 0.01), (0.5, 0.02)]).is_some());
        assert!(certify_cut_parameter_order(&[(0.2, 0.1), (0.3, 0.1)]).is_none());
        assert!(certify_cut_parameter_order(&[(0.2, 0.01), (0.99, 0.02)]).is_none());
        assert!(certify_cut_parameter_order(&[(f64::NAN, 0.0)]).is_none());
        assert!(certify_cut_parameter_order(&[(0.2, -0.01)]).is_none());
    }

    fn sample(x: f64, y: f64, weight: f64) -> WeightedBoundaryKernelSample {
        WeightedBoundaryKernelSample {
            position: BoundaryKernelVector { x, y },
            weight,
        }
    }

    #[test]
    fn source_specific_boundary_cluster_bound_contains_exact_discrete_error() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples,
        )
        .expect("finite exact pair samples should form a cluster");
        let approximate = expansion
            .evaluate()
            .expect("well-separated sample cluster should admit a bounded expansion");
        let exact = expansion.exact_potential();
        assert!(
            (approximate.potential - exact).abs() <= approximate.error.potential,
            "exact={exact} approximate={} bound={}",
            approximate.potential,
            approximate.error.potential
        );
    }

    #[test]
    fn cancellation_does_not_shrink_the_absolute_remainder_bound() {
        let samples = vec![
            sample(5.0, 1.0, 1.0e6),
            sample(5.1, 1.0, -1.0e6),
            sample(5.0, 1.1, 1.0),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            samples,
        )
        .expect("finite signed samples should form a cluster");
        let approximate = expansion.evaluate().expect("cluster is separated");
        let exact = expansion.exact_potential();
        assert!((approximate.potential - exact).abs() <= approximate.error.potential);
        assert!(approximate.error.potential > 0.0);
    }

    #[test]
    fn cluster_touching_either_body_fails_closed() {
        let source_touching = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.5, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 1.0, y: 0.0 },
            vec![sample(0.5, 0.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(source_touching.evaluate().is_none());

        let receiver_touching = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 9.5, y: 0.0 },
            BoundaryKernelVector { x: 9.0, y: 0.0 },
            vec![sample(9.5, 0.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(receiver_touching.evaluate().is_none());
    }

    #[test]
    fn fixed_sample_center_gradient_bounds_contain_exact_derivatives() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples,
        )
        .expect("finite exact pair samples should form a cluster");
        let approximate = expansion.evaluate().expect("cluster is separated");
        let (exact_source, exact_receiver) = expansion.exact_center_gradients();
        let source_error = BoundaryKernelVector {
            x: approximate.source_gradient.x - exact_source.x,
            y: approximate.source_gradient.y - exact_source.y,
        }
        .norm();
        let receiver_error = BoundaryKernelVector {
            x: approximate.receiver_gradient.x - exact_receiver.x,
            y: approximate.receiver_gradient.y - exact_receiver.y,
        }
        .norm();
        assert!(
            source_error <= approximate.error.source_force,
            "source error={source_error} bound={}",
            approximate.error.source_force
        );
        assert!(
            receiver_error <= approximate.error.receiver_force,
            "receiver error={receiver_error} bound={}",
            approximate.error.receiver_force
        );
    }

    #[test]
    fn sample_geometry_change_bounds_contain_exact_potential_and_center_gradient_changes() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples.clone(),
        )
        .expect("finite exact pair samples should form a cluster");
        let motion = vec![
            BoundarySampleMotionBound {
                position_displacement: 0.03,
                absolute_weight_change: 0.02,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.02,
                absolute_weight_change: 0.01,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.04,
                absolute_weight_change: 0.015,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.01,
                absolute_weight_change: 0.005,
            },
        ];
        let bound = expansion
            .bound_sample_geometry_change(&motion)
            .expect("all sample-motion disks remain separated");

        let moved = samples
            .iter()
            .zip(&motion)
            .enumerate()
            .map(|(index, (sample, change))| {
                let direction = if index % 2 == 0 { 1.0 } else { -1.0 };
                WeightedBoundaryKernelSample {
                    position: BoundaryKernelVector {
                        x: sample.position.x + direction * change.position_displacement * 0.7,
                        y: sample.position.y - direction * change.position_displacement * 0.7,
                    },
                    weight: sample.weight + direction * change.absolute_weight_change * 0.8,
                }
            })
            .collect();
        let moved_expansion = BoundaryKernelExpansion::new(
            expansion.source_center,
            expansion.receiver_center,
            expansion.sample_center,
            moved,
        )
        .expect("moved samples remain finite");
        let potential_change =
            (moved_expansion.exact_potential() - expansion.exact_potential()).abs();
        let (base_source, base_receiver) = expansion.exact_center_gradients();
        let (moved_source, moved_receiver) = moved_expansion.exact_center_gradients();
        let source_change = BoundaryKernelVector {
            x: moved_source.x - base_source.x,
            y: moved_source.y - base_source.y,
        }
        .norm();
        let receiver_change = BoundaryKernelVector {
            x: moved_receiver.x - base_receiver.x,
            y: moved_receiver.y - base_receiver.y,
        }
        .norm();

        assert!(potential_change <= bound.potential);
        assert!(source_change <= bound.source_force);
        assert!(receiver_change <= bound.receiver_force);
    }

    #[test]
    fn sample_geometry_change_bound_rejects_missing_or_singular_envelopes() {
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            vec![sample(5.0, 1.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(expansion.bound_sample_geometry_change(&[]).is_none());
        assert!(expansion
            .bound_sample_geometry_change(&[BoundarySampleMotionBound {
                position_displacement: f64::NAN,
                absolute_weight_change: 0.0,
            }])
            .is_none());
        assert!(expansion
            .bound_sample_geometry_change(&[BoundarySampleMotionBound {
                position_displacement: 6.0,
                absolute_weight_change: 0.0,
            }])
            .is_none());
    }

    #[test]
    fn randomized_fixed_sample_center_gradient_bounds_contain_exact_derivatives() {
        let mut state = 0x9e37_79b9_u64;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 33) as u32) as f64 / u32::MAX as f64
        };
        for iteration in 0..256 {
            let source = BoundaryKernelVector { x: -2.0, y: -1.0 };
            let receiver = BoundaryKernelVector { x: 12.0, y: 2.0 };
            let center = BoundaryKernelVector {
                x: 5.0 + next(),
                y: 0.5 + next(),
            };
            let mut samples = Vec::new();
            for _ in 0..8 {
                samples.push(sample(
                    center.x + (next() - 0.5) * 0.8,
                    center.y + (next() - 0.5) * 0.8,
                    (next() - 0.5) * 4.0,
                ));
            }
            let expansion = BoundaryKernelExpansion::new(source, receiver, center, samples)
                .expect("finite randomized samples should form a cluster");
            let approximate = expansion.evaluate().expect("cluster remains separated");
            let (exact_source, exact_receiver) = expansion.exact_center_gradients();
            let source_error = BoundaryKernelVector {
                x: approximate.source_gradient.x - exact_source.x,
                y: approximate.source_gradient.y - exact_source.y,
            }
            .norm();
            let receiver_error = BoundaryKernelVector {
                x: approximate.receiver_gradient.x - exact_receiver.x,
                y: approximate.receiver_gradient.y - exact_receiver.y,
            }
            .norm();
            assert!(
                source_error <= approximate.error.source_force,
                "source error={source_error} bound={} iteration={}",
                approximate.error.source_force,
                iteration
            );
            assert!(
                receiver_error <= approximate.error.receiver_force,
                "receiver error={receiver_error} bound={} iteration={}",
                approximate.error.receiver_force,
                iteration
            );
        }
    }

    #[test]
    fn pose_cell_certificate_composes_cluster_and_geometry_errors() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples,
        )
        .expect("finite exact pair samples");
        let motion = vec![
            BoundarySampleMotionBound {
                position_displacement: 0.03,
                absolute_weight_change: 0.02,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.02,
                absolute_weight_change: 0.01,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.04,
                absolute_weight_change: 0.015,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.01,
                absolute_weight_change: 0.005,
            },
        ];
        let cluster = expansion.evaluate().unwrap();
        let geometry = expansion.bound_sample_geometry_change(&motion).unwrap();
        let certified =
            certify_boundary_pose_cell(&expansion, &[(0.2, 0.01), (0.6, 0.02)], &motion)
                .expect("stable supplied cut order and sample-motion envelope");
        assert_eq!(certified.evaluation, cluster);
        assert!(
            (certified.error.potential - cluster.error.potential - geometry.potential).abs()
                < 1e-12
        );
        assert!(
            (certified.error.source_force - cluster.error.source_force - geometry.source_force)
                .abs()
                < 1e-12
        );
        assert!(
            (certified.error.receiver_force
                - cluster.error.receiver_force
                - geometry.receiver_force)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn pose_cell_certificate_rejects_cut_or_sample_correspondence_transitions() {
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            vec![sample(5.0, 1.0, 1.0)],
        )
        .expect("valid sample input");
        let motion = [BoundarySampleMotionBound {
            position_displacement: 0.01,
            absolute_weight_change: 0.01,
        }];
        assert!(
            certify_boundary_pose_cell(&expansion, &[(0.2, 0.1), (0.3, 0.1)], &motion).is_none()
        );
        assert!(certify_boundary_pose_cell(&expansion, &[(0.2, 0.01)], &[]).is_none());
        assert!(certify_boundary_pose_cell(
            &expansion,
            &[(0.2, 0.01)],
            &[BoundarySampleMotionBound {
                position_displacement: 6.0,
                absolute_weight_change: 0.0,
            }],
        )
        .is_none());
    }

    fn zero_radius_cluster_is_exact_up_to_roundoff_guard() {
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            vec![sample(5.0, 1.0, -2.0)],
        )
        .expect("single exact sample should form a cluster");
        let approximate = expansion.evaluate().expect("sample is separated");
        assert!(
            (approximate.potential - expansion.exact_potential()).abs()
                <= approximate.error.potential
        );
        assert_eq!(approximate.radius, 0.0);
    }
}
