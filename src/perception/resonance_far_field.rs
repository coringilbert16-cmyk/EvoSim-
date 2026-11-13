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
    /// Per-source force remainder bounds, in the same stable input order.
    pub(crate) source_force_error_bounds: Vec<f64>,
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
        let h_xx =
            tangential_eigenvalue + (radial_eigenvalue - tangential_eigenvalue) * unit.x * unit.x;
        let h_xy = (radial_eigenvalue - tangential_eigenvalue) * unit.x * unit.y;
        let h_yy =
            tangential_eigenvalue + (radial_eigenvalue - tangential_eigenvalue) * unit.y * unit.y;
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
        let mut source_force_error_bounds = Vec::with_capacity(self.sources.len());
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
            source_force_error_bounds.push(0.0);
        }

        let minimum = minimum_distance;
        let f_prime = -1.0 / (1.0 + minimum).powi(2);
        let f_second = 2.0 / (1.0 + minimum).powi(3);
        let f_third = -6.0 / (1.0 + minimum).powi(4);
        let hessian_bound = f_second.abs().max((f_prime / minimum).abs());
        // For a radial 2-D kernel, bound the spatial derivative of its
        // Hessian by |f'''| + |(f''/r)-(f'/r²)| + 2|f''-(f'/r)|/r.
        let third_derivative_bound = f_third.abs()
            + 2.0 * (f_second / minimum - f_prime / minimum.powi(2)).abs()
            + 2.0 * (f_second - f_prime / minimum).abs() / minimum;
        let radius_squared = self.radius * self.radius;
        let potential_error = 0.5 * self.absolute_weight * hessian_bound * radius_squared;
        let force_error = 0.5 * self.absolute_weight * third_derivative_bound * radius_squared;
        let mut torque_error = 0.0;
        for (source, source_error) in self.sources.iter().zip(&mut source_force_error_bounds) {
            *source_error = 0.5 * source.weight.abs() * third_derivative_bound * radius_squared;
            let offset_x = source.position.x - self.center.x;
            let offset_y = source.position.y - self.center.y;
            torque_error += offset_x.hypot(offset_y) * *source_error;
        }
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
            source_force_error_bounds,
            torque_about_center,
            error: FarFieldErrorBounds {
                potential: potential_error,
                force: force_error,
                torque_about_center: torque_error,
            },
        })
    }

    /// Evaluate a source set with deterministic adaptive subdivision.
    ///
    /// Each accepted cluster receives an error budget proportional to its
    /// absolute source weight. Clusters that cannot meet all three budgets are
    /// bisected along their widest spatial axis; singleton clusters are exact.
    /// The returned torque and torque error are measured about the root center.
    pub(crate) fn evaluate_adaptive(
        sources: &[WeightedFarFieldSource],
        target: FarFieldVector,
        potential_tolerance: f64,
        force_tolerance: f64,
        torque_tolerance: f64,
    ) -> Option<FarFieldEvaluation> {
        if sources.is_empty()
            || !target.x.is_finite()
            || !target.y.is_finite()
            || [potential_tolerance, force_tolerance, torque_tolerance]
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return None;
        }
        let total_absolute_weight = sources.iter().try_fold(0.0, |sum, source| {
            (source.position.x.is_finite()
                && source.position.y.is_finite()
                && source.weight.is_finite())
            .then_some(sum + source.weight.abs())
        })?;
        if !total_absolute_weight.is_finite() {
            return None;
        }
        let root_center = FarFieldVector {
            x: sources.iter().map(|source| source.position.x).sum::<f64>()
                / sources.len() as f64,
            y: sources.iter().map(|source| source.position.y).sum::<f64>()
                / sources.len() as f64,
        };
        if !root_center.x.is_finite() || !root_center.y.is_finite() {
            return None;
        }

        let mut result = FarFieldEvaluation {
            potential: 0.0,
            target_gradient: FarFieldVector::default(),
            source_forces: vec![FarFieldVector::default(); sources.len()],
            source_force_error_bounds: vec![0.0; sources.len()],
            torque_about_center: 0.0,
            error: FarFieldErrorBounds::default(),
        };
        if total_absolute_weight == 0.0 {
            return Some(result);
        }

        let indexed = sources.iter().copied().enumerate().collect::<Vec<_>>();
        Self::accumulate_adaptive_cluster(
            indexed,
            target,
            root_center,
            total_absolute_weight,
            potential_tolerance,
            force_tolerance,
            torque_tolerance,
            &mut result,
        )?;
        Some(result)
    }

    fn accumulate_adaptive_cluster(
        mut indexed_sources: Vec<(usize, WeightedFarFieldSource)>,
        target: FarFieldVector,
        root_center: FarFieldVector,
        total_absolute_weight: f64,
        potential_tolerance: f64,
        force_tolerance: f64,
        torque_tolerance: f64,
        result: &mut FarFieldEvaluation,
    ) -> Option<()> {
        let absolute_weight = indexed_sources
            .iter()
            .map(|(_, source)| source.weight.abs())
            .sum::<f64>();
        if absolute_weight == 0.0 {
            return Some(());
        }
        let center = FarFieldVector {
            x: indexed_sources
                .iter()
                .map(|(_, source)| source.position.x)
                .sum::<f64>()
                / indexed_sources.len() as f64,
            y: indexed_sources
                .iter()
                .map(|(_, source)| source.position.y)
                .sum::<f64>()
                / indexed_sources.len() as f64,
        };
        let cluster_sources = indexed_sources
            .iter()
            .map(|(_, source)| *source)
            .collect::<Vec<_>>();
        let expansion = Self::new(center, cluster_sources)?;
        if let Some(evaluation) = expansion.evaluate(target) {
            let share = absolute_weight / total_absolute_weight;
            let cluster_torque_error = indexed_sources
                .iter()
                .zip(&evaluation.source_force_error_bounds)
                .map(|((_, source), source_error)| {
                    let distance = (source.position.x - root_center.x)
                        .hypot(source.position.y - root_center.y);
                    distance * source_error
                })
                .sum::<f64>();
            let budget_met = evaluation.error.potential <= potential_tolerance * share
                && evaluation.error.force <= force_tolerance * share
                && cluster_torque_error <= torque_tolerance * share;
            if budget_met {
                result.potential += evaluation.potential;
                result.target_gradient.x += evaluation.target_gradient.x;
                result.target_gradient.y += evaluation.target_gradient.y;
                let cluster_force = evaluation
                    .source_forces
                    .iter()
                    .fold(FarFieldVector::default(), |mut sum, force| {
                        sum.x += force.x;
                        sum.y += force.y;
                        sum
                    });
                result.torque_about_center += evaluation.torque_about_center
                    + (center.x - root_center.x) * cluster_force.y
                    - (center.y - root_center.y) * cluster_force.x;
                result.error.potential += evaluation.error.potential;
                result.error.force += evaluation.error.force;
                for (((source_index, source), force), source_error) in indexed_sources
                    .drain(..)
                    .zip(evaluation.source_forces)
                    .zip(evaluation.source_force_error_bounds)
                {
                    let distance =
                        (source.position.x - root_center.x).hypot(source.position.y - root_center.y);
                    result.source_forces[source_index] = force;
                    result.source_force_error_bounds[source_index] = source_error;
                    result.error.torque_about_center += distance * source_error;
                }
                return Some(());
            }
        }

        if indexed_sources.len() == 1 {
            // A singleton has zero cluster radius and is exact unless its
            // target coincides with the source, where the radial gradient is
            // undefined and the caller must use a regularized exact policy.
            return None;
        }

        let (min_x, max_x, min_y, max_y) = indexed_sources.iter().fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |(min_x, max_x, min_y, max_y), (_, source)| {
                (
                    min_x.min(source.position.x),
                    max_x.max(source.position.x),
                    min_y.min(source.position.y),
                    max_y.max(source.position.y),
                )
            },
        );
        let split_x = max_x - min_x >= max_y - min_y;
        indexed_sources.sort_by(|(index_a, source_a), (index_b, source_b)| {
            let coordinate_a = if split_x {
                source_a.position.x
            } else {
                source_a.position.y
            };
            let coordinate_b = if split_x {
                source_b.position.x
            } else {
                source_b.position.y
            };
            coordinate_a
                .partial_cmp(&coordinate_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| index_a.cmp(index_b))
        });
        let right = indexed_sources.split_off(indexed_sources.len() / 2);
        Self::accumulate_adaptive_cluster(
            indexed_sources,
            target,
            root_center,
            total_absolute_weight,
            potential_tolerance,
            force_tolerance,
            torque_tolerance,
            result,
        )?;
        Self::accumulate_adaptive_cluster(
            right,
            target,
            root_center,
            total_absolute_weight,
            potential_tolerance,
            force_tolerance,
            torque_tolerance,
            result,
        )
    }

    /// Accept only when every requested absolute error budget is met.
    pub(crate) fn meets_error_budget(
        &self,
        evaluation: &FarFieldEvaluation,
        potential_tolerance: f64,
        force_tolerance: f64,
        torque_tolerance: f64,
    ) -> bool {
        [potential_tolerance, force_tolerance, torque_tolerance]
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

    fn exact_sum(sources: &[WeightedFarFieldSource], target: FarFieldVector) -> f64 {
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
            ResonanceFarFieldExpansion::new(FarFieldVector::default(), sources.clone()).unwrap();
        let approximation = expansion.evaluate(target).unwrap();
        let exact = exact_sum(&sources, target);
        assert!((exact - approximation.potential).abs() <= approximation.error.potential + 1e-12);
        let exact_gradient = exact_gradient(&sources, target);
        assert!(
            (exact_gradient.x - approximation.target_gradient.x)
                .hypot(exact_gradient.y - approximation.target_gradient.y)
                <= approximation.error.force + 1e-12
        );
        for ((source, approximate_force), error_bound) in sources
            .iter()
            .zip(&approximation.source_forces)
            .zip(&approximation.source_force_error_bounds)
        {
            let dx = target.x - source.position.x;
            let dy = target.y - source.position.y;
            let distance = dx.hypot(dy);
            let scale = source.weight / (distance * (1.0 + distance).powi(2));
            let exact_force = FarFieldVector {
                x: scale * dx,
                y: scale * dy,
            };
            assert!(
                (exact_force.x - approximate_force.x).hypot(exact_force.y - approximate_force.y)
                    <= *error_bound + 1e-12
            );
        }
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
        let sum =
            evaluation
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
    fn adaptive_expansion_subdivides_until_all_error_budgets_are_met() {
        let sources = vec![
            WeightedFarFieldSource { position: FarFieldVector { x: -1.2, y: 0.1 }, weight: 0.8 },
            WeightedFarFieldSource { position: FarFieldVector { x: -0.8, y: -0.1 }, weight: -0.2 },
            WeightedFarFieldSource { position: FarFieldVector { x: 0.7, y: 0.2 }, weight: 1.1 },
            WeightedFarFieldSource { position: FarFieldVector { x: 1.1, y: -0.2 }, weight: 0.4 },
        ];
        let target = FarFieldVector { x: 0.0, y: 8.0 };
        let evaluation =
            ResonanceFarFieldExpansion::evaluate_adaptive(&sources, target, 1e-4, 1e-4, 1e-4)
                .unwrap();
        let exact = exact_sum(&sources, target);
        let exact_force = exact_gradient(&sources, target);
        assert!((exact - evaluation.potential).abs() <= evaluation.error.potential + 1e-12);
        assert!(
            (exact_force.x - evaluation.target_gradient.x)
                .hypot(exact_force.y - evaluation.target_gradient.y)
                <= evaluation.error.force + 1e-12
        );
        assert!(evaluation.error.potential <= 1e-4);
        assert!(evaluation.error.force <= 1e-4);
        assert!(evaluation.error.torque_about_center <= 1e-4);
        for (index, (source, force)) in sources.iter().zip(&evaluation.source_forces).enumerate() {
            let dx = target.x - source.position.x;
            let dy = target.y - source.position.y;
            let distance = dx.hypot(dy);
            let scale = source.weight / (distance * (1.0 + distance).powi(2));
            let exact_source_force = FarFieldVector {
                x: scale * dx,
                y: scale * dy,
            };
            assert!(
                (exact_source_force.x - force.x).hypot(exact_source_force.y - force.y)
                    <= evaluation.source_force_error_bounds[index] + 1e-12
            );
        }
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
