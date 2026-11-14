//! Bounded Taylor treatment of one ordered pair's exact receiver-boundary samples.
//!
//! Each input point is one exact production interval midpoint. Its projected
//! extent and signed frequency-resolved cross-energy coefficient are folded
//! into the weight. This approximates only the smooth two-distance kernel over
//! a cluster of those already-established samples. Visibility and interval
//! generation remain exact and outside this module.
//!
//! Only scalar potential is produced. Force and torque bounds require
//! differentiating this same expansion with respect to both body poses and
//! projected-extent geometry. Until those bounds exist, this result must not
//! feed movement or be treated as an accepted mutual force.

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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryPotentialEvaluation {
    pub(crate) potential: f64,
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

        let (source_value, source_gradient) = radial_kernel_value_gradient(source_delta, source_distance);
        let (receiver_value, receiver_gradient) =
            radial_kernel_value_gradient(receiver_delta, receiver_distance);
        let kernel = source_value * receiver_value;
        let gradient = BoundaryKernelVector {
            x: source_gradient.x * receiver_value + source_value * receiver_gradient.x,
            y: source_gradient.y * receiver_value + source_value * receiver_gradient.y,
        };
        let potential = self.total_weight * kernel + gradient.dot(self.first_moment);

        // For f(r)=1/(1+r), ||grad f|| <= (1+d)^-2 and
        // ||H_f|| <= max(2(1+d)^-3, 1/(d(1+d)^2)) whenever r >= d.
        let source_value_bound = 1.0 / (1.0 + source_min);
        let receiver_value_bound = 1.0 / (1.0 + receiver_min);
        let source_gradient_bound = 1.0 / (1.0 + source_min).powi(2);
        let receiver_gradient_bound = 1.0 / (1.0 + receiver_min).powi(2);
        let source_hessian_bound = radial_hessian_bound(source_min);
        let receiver_hessian_bound = radial_hessian_bound(receiver_min);
        // Product rule: H(fg)=g Hf + f Hg + grad(f)grad(g)^T
        // + grad(g)grad(f)^T. The operator norm of the last two terms
        // is bounded by twice the product of gradient norms.
        let product_hessian_bound = receiver_value_bound * source_hessian_bound
            + source_value_bound * receiver_hessian_bound
            + 2.0 * source_gradient_bound * receiver_gradient_bound;
        let analytic_error =
            0.5 * self.absolute_weight * product_hessian_bound * self.radius * self.radius;
        let roundoff_guard =
            32.0 * f64::EPSILON * (potential.abs() + self.absolute_weight + 1.0);
        let error = analytic_error + roundoff_guard;
        if !potential.is_finite() || !error.is_finite() {
            return None;
        }

        Some(BoundaryPotentialEvaluation {
            potential,
            error: BoundaryPotentialError { potential: error },
            center,
            radius: self.radius,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

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
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 1.0, y: 0.0 },
            vec![sample(0.5, 0.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(source_touching.evaluate().is_none());

        let receiver_touching = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 9.0, y: 0.0 },
            vec![sample(9.5, 0.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(receiver_touching.evaluate().is_none());
    }

    #[test]
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
