//! Error-controlled far-field primitives for the smooth resonance distance kernel.
//!
//! This module deliberately does not evaluate visibility or chemistry activation.
//! It expands only the source-side spatial factor 1 / (1 + distance), which is
//! the smooth part of the existing mechanical pair potential after per-body
//! spectra and receiver boundary samples have been prepared.
//!
//! A caller may use this primitive only for a group certified to have identical
//! visibility at the receiver samples. Ambiguous visibility must be subdivided
//! or evaluated by the exact path.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FarFieldVector {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl FarFieldVector {
    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    fn norm(self) -> f64 {
        self.x.hypot(self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WeightedFarFieldSource {
    pub(crate) position: FarFieldVector,
    /// Signed scalar coefficient for one receiver sample and one frequency.
    /// Spectral phase/cross-energy is represented by this coefficient.
    pub(crate) weight: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FarFieldErrorBounds {
    pub(crate) potential: f64,
    pub(crate) force: f64,
    pub(crate) torque_about_center: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FarFieldEvaluation {
    /// First-order Taylor approximation of sum(weight / (1 + distance)).
    pub(crate) potential: f64,
    /// Gradient with respect to the target position.
    pub(crate) target_gradient: FarFieldVector,
    /// Approximate force on each source, in the input source order.
    pub(crate) source_forces: Vec<FarFieldVector>,
    /// Approximate torque of source forces about the expansion center.
    pub(crate) torque_about_center: f64,
    /// Conservative absolute remainder bounds for this expansion.
    pub(crate) error: FarFieldErrorBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResonanceFarFieldExpansion {
    center: FarFieldVector,
    sources: Vec<WeightedFarFieldSource>,
    total_weight: f64,
    absolute_weight: f64,
    first_moment: FarFieldVector,
    radius: f64,
}

impl ResonanceFarFieldExpansion {
    /// Construct a source cluster around an explicit center. The radius is
    /// derived from all source positions; no source is discarded.
    pub(crate) fn new(
        center: FarFieldVector,
        sources: Vec<WeightedFarFieldSource>,
    ) -> Option<Self> {
        if !center.x.is_finite() || !center.y.is_finite() || sources.is_empty() {
            return None;
        }
        let mut total_weight = 0.0;
        let mut absolute_weight = 0.0;
        let mut first_moment = FarFieldVector::default();
        let mut radius: f64 = 0.0;
        for source in &sources {
            if !source.position.x.is_finite()
                || !source.position.y.is_finite()
                || !source.weight.is_finite()
            {
                return None;
            }
            let offset = FarFieldVector {
                x: source.position.x - center.x,
                y: source.position.y - center.y,
            };
            total_weight += source.weight;
            absolute_weight += source.weight.abs();
            first_moment.x += source.weight * offset.x;
            first_moment.y += source.weight * offset.y;
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
            center,
            sources,
            total_weight,
            absolute_weight,
            first_moment,
            radius,
        })
    }

    pub(crate) fn radius(&self) -> f64 {
        self.radius
    }

    /// Evaluate the first-order mutual source expansion at a target.
    ///
    /// The target must lie strictly outside the source cluster. Remainder
    /// bounds use the minimum possible separation (distance - radius), not an
    /// opening-angle heuristic. Potential, force, and torque are bounded
    /// independently.
    pub(crate) fn evaluate(&self, target: FarFieldVector) -> Option<FarFieldEvaluation> {
        if !target.x.is_finite() || !target.y.is_finite() {
            return None;
        }
        let separation = FarFieldVector {
            x: target.x - self.center.x,
            y: target.y - self.center.y,
        };
        let distance = separation.norm();
        let minimum_distance = distance - self.radius;
        if !distance.is_finite()
            || !minimum_distance.is_finite()
            || minimum_distance <= f64::EPSILON
        {
            return None;
        }

        let inverse_distance = 1.0 / distance;
        let unit = FarFieldVector {
            x: separation.x * inverse_distance,
            y: separation.y * inverse_distance,
        };
        let first_derivative = -1.0 / (1.0 + distance).powi(2);
        let second_derivative = 2.0 / (1.0 + distance).powi(3);
        let kernel = 1.0 / (1.0 + distance);
        let radial_eigenvalue = second_derivative;
        let tangential_eigenvalue = first_derivative / distance;

        // H = tangential * I + (radial - tangential) * u * u^T.
        let h_xx = tangential_eigenvalue
            + (radial_eigenvalue - tangential_eigenvalue) * unit.x * unit.x;
        let h_xy =
            (radial_eigenvalue - tangential_eigenvalue) * unit.x * unit.y;
        let h_yy = tangential_eigenvalue
            + (radial_eigenvalue - tangential_eigenvalue) * unit.y * unit.y;
        let gradient = FarFieldVector {
            x: first_derivative * unit.x,
            y: first_derivative * unit.y,
        };
        let hessian_moment = FarFieldVector {
            x: h_xx * self.first_moment.x + h_xy * self.first_moment.y,
            y: h_xy * self.first_moment.x + h_yy * self.first_moment.y,
        };
        let target_gradient = FarFieldVector {
            x: self.total_weight * gradient.x - hessian_moment.x,
            y: self.total_weight * gradient.y - hessian_moment.y,
        };
        let potential = self.total_weight * kernel - gradient.dot(self.first_moment);

        let mut source_forces = Vec::with_capacity(self.sources.len());
        let mut torque_about_center = 0.0;
        for source in &self.sources {
            let offset = FarFieldVector {
                x: source.position.x - self.center.x,
                y: source.position.y - self.center.y,
            };
            let hessian_offset = FarFieldVector {
                x: h_xx * offset.x + h_xy * offset.y,
                y: h_xy * offset.x + h_yy * offset.y,
            };
            let force = FarFieldVector {
                x: -source.weight * (gradient.x - hessian_offset.x),
                y: -source.weight * (gradient.y - hessian_offset.y),
            };
            if !force.x.is_finite() || !force.y.is_finite() {
                return None;
            }
            torque_about_center += offset.cross(force);
            source_forces.push(force);
        }

        let minimum = minimum_distance;
        let f_prime = -1.0 / (1.0 + minimum).powi(2);
        let f_second = 2.0 / (1.0 + minimum).powi(3);
        let f_third = -6.0 / (1.0 + minimum).powi(4);
        let hessian_bound = f_second.abs().max((f_prime / minimum).abs());
        // For a radial 2-D kernel, bound the spatial derivative of its
        // Hessian by |f'''| + |(f''/r)-(f'/r²)| + 2|f''-(f'/r)|/r.
        let third_derivative_bound = f_third.abs()
            + (f_second / minimum - f_prime / minimum.powi(2)).abs()
            + 2.0 * (f_second - f_prime / minimum).abs() / minimum;
        let radius_squared = self.radius * self.radius;
        let potential_error =
            0.5 * self.absolute_weight * hessian_bound * radius_squared;
        let force_error =
            0.5 * self.absolute_weight * third_derivative_bound * radius_squared;
        let torque_error = self.radius * force_error;
        let values = [
            potential,
            target_gradient.x,
            target_gradient.y,
            torque_about_center,
            potential_error,
            force_error,
            torque_error,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return None;
        }

        Some(FarFieldEvaluation {
            potential,
            target_gradient,
            source_forces,
            torque_about_center,
            error: FarFieldErrorBounds {
                potential: potential_error,
                force: force_error,
                torque_about_center: torque_error,
            },
        })
    }

    /// Accept only when every requested absolute error budget is met.
    pub(crate) fn meets_error_budget(
        &self,
        evaluation: &FarFieldEvaluation,
        potential_tolerance: f64,
        force_tolerance: f64,
        torque_tolerance: f64,
    ) -> bool {
        [
            potential_tolerance,
            force_tolerance,
            torque_tolerance,
        ]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
            && evaluation.error.potential <= potential_tolerance
            && evaluation.error.force <= force_tolerance
            && evaluation.error.torque_about_center <= torque_tolerance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_sum(
        sources: &[WeightedFarFieldSource],
        target: FarFieldVector,
    ) -> f64 {
        sources
            .iter()
            .map(|source| {
                let dx = target.x - source.position.x;
                let dy = target.y - source.position.y;
                source.weight / (1.0 + dx.hypot(dy))
            })
            .sum()
    }

    fn exact_gradient(
        sources: &[WeightedFarFieldSource],
        target: FarFieldVector,
    ) -> FarFieldVector {
        let mut result = FarFieldVector::default();
        for source in sources {
            let dx = target.x - source.position.x;
            let dy = target.y - source.position.y;
            let distance = dx.hypot(dy);
            let scale = source.weight / (distance * (1.0 + distance).powi(2));
            result.x -= scale * dx;
            result.y -= scale * dy;
        }
        result
    }

    #[test]
    fn first_order_expansion_matches_exact_sum_within_its_bound() {
        let sources = vec![
            WeightedFarFieldSource {
                position: FarFieldVector { x: -0.12, y: 0.04 },
                weight: 1.2,
            },
            WeightedFarFieldSource {
                position: FarFieldVector { x: 0.08, y: -0.03 },
                weight: -0.35,
            },
            WeightedFarFieldSource {
                position: FarFieldVector { x: 0.03, y: 0.11 },
                weight: 0.8,
            },
        ];
        let target = FarFieldVector { x: 8.0, y: 3.0 };
        let expansion =
            ResonanceFarFieldExpansion::new(FarFieldVector::default(), sources.clone())
                .unwrap();
        let approximation = expansion.evaluate(target).unwrap();
        let exact = exact_sum(&sources, target);
        assert!(
            (exact - approximation.potential).abs()
                <= approximation.error.potential + 1e-12
        );
        let exact_gradient = exact_gradient(&sources, target);
        assert!(
            (exact_gradient.x - approximation.target_gradient.x).hypot(
                exact_gradient.y - approximation.target_gradient.y
            ) <= approximation.error.force + 1e-12
        );
    }

    #[test]
    fn source_forces_conserve_the_approximated_pair_force() {
        let sources = vec![
            WeightedFarFieldSource {
                position: FarFieldVector { x: -0.2, y: 0.1 },
                weight: 0.7,
            },
            WeightedFarFieldSource {
                position: FarFieldVector { x: 0.15, y: -0.08 },
                weight: 1.1,
            },
        ];
        let expansion =
            ResonanceFarFieldExpansion::new(FarFieldVector::default(), sources).unwrap();
        let evaluation = expansion
            .evaluate(FarFieldVector { x: 12.0, y: -4.0 })
            .unwrap();
        let sum = evaluation
            .source_forces
            .iter()
            .fold(FarFieldVector::default(), |mut total, force| {
                total.x += force.x;
                total.y += force.y;
                total
            });
        assert!((sum.x + evaluation.target_gradient.x).abs() <= 1e-12);
        assert!((sum.y + evaluation.target_gradient.y).abs() <= 1e-12);
    }

    #[test]
    fn expansion_fails_closed_when_target_is_inside_cluster_radius() {
        let sources = vec![WeightedFarFieldSource {
            position: FarFieldVector { x: 2.0, y: 0.0 },
            weight: 1.0,
        }];
        let expansion =
            ResonanceFarFieldExpansion::new(FarFieldVector::default(), sources).unwrap();
        assert!(expansion
            .evaluate(FarFieldVector { x: 1.5, y: 0.0 })
            .is_none());
    }

    #[test]
    fn error_budget_rejects_unresolved_near_field() {
        let sources = vec![
            WeightedFarFieldSource {
                position: FarFieldVector { x: -0.5, y: 0.0 },
                weight: 1.0,
            },
            WeightedFarFieldSource {
                position: FarFieldVector { x: 0.5, y: 0.0 },
                weight: 1.0,
            },
        ];
        let expansion =
            ResonanceFarFieldExpansion::new(FarFieldVector::default(), sources).unwrap();
        let evaluation = expansion
            .evaluate(FarFieldVector { x: 3.0, y: 0.0 })
            .unwrap();
        assert!(!expansion.meets_error_budget(&evaluation, 1e-12, 1e-12, 1e-12));
    }
}
