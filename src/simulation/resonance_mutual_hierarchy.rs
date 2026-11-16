//! Test-only mutual dual-tree traversal for complete environmental interactions.
//!
//! The adaptive traversal accepts a cluster pair only when the caller supplies
//! a complete visibility certificate and independent potential, per-body x/y
//! force, and per-body torque error bounds. Missing or rejected certificates
//! subdivide to exact body-pair callbacks. The current world oracle deliberately
//! supplies no cluster evaluator, so it continues to exercise exact leaves.

use crate::resonance_visibility::VisibilityBounds;

#[derive(Clone, Debug)]
struct Node {
    bounds: VisibilityBounds,
    kind: NodeKind,
}

#[derive(Clone, Debug)]
enum NodeKind {
    Leaf(usize),
    Branch { left: Box<Node>, right: Box<Node> },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MutualErrorTolerance {
    pub(crate) potential: f64,
    pub(crate) force_x: f64,
    pub(crate) force_y: f64,
    pub(crate) torque: f64,
}

impl MutualErrorTolerance {
    fn is_valid(self) -> bool {
        [self.potential, self.force_x, self.force_y, self.torque]
            .into_iter()
            .all(|value| value.is_finite() && value >= 0.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BodyInteractionErrorBound {
    pub(crate) body_index: usize,
    pub(crate) force_x: f64,
    pub(crate) force_y: f64,
    pub(crate) torque: f64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MutualClusterErrorBounds {
    pub(crate) potential: f64,
    pub(crate) bodies: Vec<BodyInteractionErrorBound>,
}

impl MutualClusterErrorBounds {
    fn is_within(
        &self,
        bodies_a: &[usize],
        bodies_b: &[usize],
        tolerance: &MutualErrorTolerance,
    ) -> bool {
        if !self.within_numeric_tolerance(tolerance) {
            return false;
        }
        let mut expected = bodies_a
            .iter()
            .chain(bodies_b.iter())
            .copied()
            .collect::<Vec<_>>();
        expected.sort_unstable();
        if expected.windows(2).any(|pair| pair[0] == pair[1]) {
            return false;
        }
        let mut actual = Vec::with_capacity(self.bodies.len());
        for bound in &self.bodies {
            if !bound.force_x.is_finite()
                || !bound.force_y.is_finite()
                || !bound.torque.is_finite()
                || bound.force_x < 0.0
                || bound.force_y < 0.0
                || bound.torque < 0.0
                || bound.force_x > tolerance.force_x
                || bound.force_y > tolerance.force_y
                || bound.torque > tolerance.torque
            {
                return false;
            }
            actual.push(bound.body_index);
        }
        actual.sort_unstable();
        !actual.windows(2).any(|pair| pair[0] == pair[1])
            && expected
                .iter()
                .all(|body_index| actual.binary_search(body_index).is_ok())
    }

    fn within_numeric_tolerance(&self, tolerance: &MutualErrorTolerance) -> bool {
        self.potential.is_finite()
            && self.potential >= 0.0
            && self.potential <= tolerance.potential
            && self.bodies.iter().all(|bound| {
                bound.force_x.is_finite()
                    && bound.force_y.is_finite()
                    && bound.torque.is_finite()
                    && bound.force_x >= 0.0
                    && bound.force_y >= 0.0
                    && bound.torque >= 0.0
                    && bound.force_x <= tolerance.force_x
                    && bound.force_y <= tolerance.force_y
                    && bound.torque <= tolerance.torque
            })
    }
}

/// Convert a complete two-distance mutual boundary evaluation into the
/// hierarchy's independent per-body potential/force/torque certificate.
/// participating_body_indices must include source, receiver, and every
/// occluder whose generalized coordinate received a derivative.
pub(crate) fn mutual_boundary_cluster_certificate(
    visibility_certified: bool,
    participating_body_indices: &[usize],
    evaluation: crate::resonance_boundary_far_field::MutualBoundaryKernelEvaluation,
) -> Option<
    MutualClusterCertificate<crate::resonance_boundary_far_field::MutualBoundaryKernelEvaluation>,
> {
    let mut sorted_body_indices = participating_body_indices.to_vec();
    sorted_body_indices.sort_unstable();
    if !visibility_certified
        || sorted_body_indices
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        || evaluation.coordinates.len() != participating_body_indices.len().checked_mul(3)?
        || evaluation.error.coordinates.len() != evaluation.coordinates.len()
        || !evaluation.error.potential.is_finite()
        || evaluation.error.potential < 0.0
    {
        return None;
    }
    let potential_error = evaluation.error.potential;
    let mut bodies = Vec::with_capacity(participating_body_indices.len());
    for (body_offset, body_index) in participating_body_indices.iter().copied().enumerate() {
        let coordinate_offset = body_offset.checked_mul(3)?;
        let force_x = evaluation.error.coordinates[coordinate_offset];
        let force_y = evaluation.error.coordinates[coordinate_offset + 1];
        let torque = evaluation.error.coordinates[coordinate_offset + 2];
        if ![force_x, force_y, torque]
            .iter()
            .all(|error| error.is_finite() && *error >= 0.0)
        {
            return None;
        }
        bodies.push(BodyInteractionErrorBound {
            body_index,
            force_x,
            force_y,
            torque,
        });
    }
    Some(MutualClusterCertificate {
        visibility_certified,
        evaluation,
        errors: MutualClusterErrorBounds {
            potential: potential_error,
            bodies,
        },
    })
}

#[derive(Clone, Debug)]
pub(crate) struct MutualClusterCertificate<T> {
    pub(crate) visibility_certified: bool,
    pub(crate) evaluation: T,
    pub(crate) errors: MutualClusterErrorBounds,
}

#[derive(Clone, Debug)]
pub(crate) struct AcceptedClusterPair<T> {
    pub(crate) bodies_a: Vec<usize>,
    pub(crate) bodies_b: Vec<usize>,
    pub(crate) evaluation: T,
    pub(crate) errors: MutualClusterErrorBounds,
}

#[derive(Clone, Debug)]
pub(crate) struct ExactPairEvaluation<T> {
    pub(crate) pair: (usize, usize),
    pub(crate) evaluation: T,
}

#[derive(Clone, Debug)]
pub(crate) struct AdaptiveTraversalResult<T> {
    pub(crate) accepted_cluster_pairs: Vec<AcceptedClusterPair<T>>,
    pub(crate) exact_pairs: Vec<(usize, usize)>,
    pub(crate) exact_pair_evaluations: Vec<ExactPairEvaluation<T>>,
    pub(crate) accumulated_errors: MutualClusterErrorBounds,
}

impl<T> Default for AdaptiveTraversalResult<T> {
    fn default() -> Self {
        Self {
            accepted_cluster_pairs: Vec::new(),
            exact_pairs: Vec::new(),
            exact_pair_evaluations: Vec::new(),
            accumulated_errors: MutualClusterErrorBounds::default(),
        }
    }
}

impl<T> AdaptiveTraversalResult<T> {
    fn try_accumulate_errors(
        &mut self,
        errors: &MutualClusterErrorBounds,
        tolerance: &MutualErrorTolerance,
    ) -> bool {
        let mut combined = self.accumulated_errors.clone();
        combined.potential += errors.potential;
        if !combined.potential.is_finite() {
            return false;
        }
        for incoming in &errors.bodies {
            if let Some(existing) = combined
                .bodies
                .iter_mut()
                .find(|bound| bound.body_index == incoming.body_index)
            {
                existing.force_x += incoming.force_x;
                existing.force_y += incoming.force_y;
                existing.torque += incoming.torque;
            } else {
                combined.bodies.push(*incoming);
            }
        }
        combined.bodies.sort_by_key(|bound| bound.body_index);
        if !combined.within_numeric_tolerance(tolerance) {
            return false;
        }
        self.accumulated_errors = combined;
        true
    }
}

/// Immutable spatial hierarchy over the exact world-snapshot body indices.
#[derive(Clone, Debug, Default)]
pub(crate) struct MutualInteractionHierarchy {
    root: Option<Box<Node>>,
}

impl MutualInteractionHierarchy {
    /// Build a balanced hierarchy. Invalid bounds fail closed; empty worlds
    /// produce an empty hierarchy.
    pub(crate) fn new(bounds: &[VisibilityBounds]) -> Option<Self> {
        if bounds.iter().any(|bound| {
            !bound.min_x.is_finite()
                || !bound.min_y.is_finite()
                || !bound.max_x.is_finite()
                || !bound.max_y.is_finite()
                || bound.min_x > bound.max_x
                || bound.min_y > bound.max_y
        }) {
            return None;
        }
        if bounds.is_empty() {
            return Some(Self::default());
        }
        let indices = (0..bounds.len()).collect::<Vec<_>>();
        Some(Self {
            root: Some(Box::new(Self::build_node(bounds, indices)?)),
        })
    }

    fn build_node(bounds: &[VisibilityBounds], mut indices: Vec<usize>) -> Option<Node> {
        if indices.is_empty() {
            return None;
        }
        let mut union = bounds[*indices.first()?];
        for index in indices.iter().copied().skip(1) {
            let next = *bounds.get(index)?;
            union.min_x = union.min_x.min(next.min_x);
            union.min_y = union.min_y.min(next.min_y);
            union.max_x = union.max_x.max(next.max_x);
            union.max_y = union.max_y.max(next.max_y);
        }
        if indices.len() == 1 {
            return Some(Node {
                bounds: union,
                kind: NodeKind::Leaf(indices[0]),
            });
        }

        let span_x = union.max_x - union.min_x;
        let span_y = union.max_y - union.min_y;
        if !span_x.is_finite() || !span_y.is_finite() {
            return None;
        }
        let split_x = span_x >= span_y;
        indices.sort_by(|left, right| {
            let a = Self::center(bounds[*left], split_x);
            let b = Self::center(bounds[*right], split_x);
            a.total_cmp(&b).then_with(|| left.cmp(right))
        });
        let right_indices = indices.split_off(indices.len() / 2);
        let left = Self::build_node(bounds, indices)?;
        let right = Self::build_node(bounds, right_indices)?;
        Some(Node {
            bounds: union,
            kind: NodeKind::Branch {
                left: Box::new(left),
                right: Box::new(right),
            },
        })
    }

    fn maximum_extent(bounds: VisibilityBounds) -> f64 {
        (bounds.max_x - bounds.min_x).max(bounds.max_y - bounds.min_y)
    }

    fn center(bounds: VisibilityBounds, x_axis: bool) -> f64 {
        if x_axis {
            bounds.min_x * 0.5 + bounds.max_x * 0.5
        } else {
            bounds.min_y * 0.5 + bounds.max_y * 0.5
        }
    }

    /// Return every unordered pair in deterministic index order. The traversal
    /// is mutual: self-pairs split into LL, LR, RR; distinct clusters split
    /// recursively until exact body/body leaf pairs are reached.
    pub(crate) fn unordered_pairs(&self) -> Option<Vec<(usize, usize)>> {
        let mut pairs = Vec::new();
        if let Some(root) = &self.root {
            Self::visit_self(root, &mut pairs)?;
        }
        pairs.sort_unstable();
        Some(pairs)
    }

    fn visit_self(node: &Node, pairs: &mut Vec<(usize, usize)>) -> Option<()> {
        if let NodeKind::Branch { left, right } = &node.kind {
            Self::visit_self(left, pairs)?;
            Self::visit_distinct(left, right, pairs)?;
            Self::visit_self(right, pairs)?;
        }
        Some(())
    }

    fn visit_distinct(a: &Node, b: &Node, pairs: &mut Vec<(usize, usize)>) -> Option<()> {
        match (&a.kind, &b.kind) {
            (NodeKind::Leaf(index_a), NodeKind::Leaf(index_b)) => {
                if index_a == index_b {
                    return None;
                }
                pairs.push(((*index_a).min(*index_b), (*index_a).max(*index_b)));
            }
            (NodeKind::Branch { left, right }, NodeKind::Leaf(_)) => {
                Self::visit_distinct(left, b, pairs)?;
                Self::visit_distinct(right, b, pairs)?;
            }
            (NodeKind::Leaf(_), NodeKind::Branch { left, right }) => {
                Self::visit_distinct(a, left, pairs)?;
                Self::visit_distinct(a, right, pairs)?;
            }
            (NodeKind::Branch { left, right }, NodeKind::Branch { .. })
                if Self::maximum_extent(a.bounds) >= Self::maximum_extent(b.bounds) =>
            {
                Self::visit_distinct(left, b, pairs)?;
                Self::visit_distinct(right, b, pairs)?;
            }
            (
                NodeKind::Branch { .. },
                NodeKind::Branch {
                    left: other_left,
                    right: other_right,
                },
            ) => {
                Self::visit_distinct(a, other_left, pairs)?;
                Self::visit_distinct(a, other_right, pairs)?;
            }
        }
        Some(())
    }

    /// Traverse mutual cluster pairs, accepting only independently certified
    /// potential, x/y force, torque, and visibility bounds. Rejected clusters
    /// subdivide; body/body leaves always use the exact callback.
    pub(crate) fn evaluate_adaptive<T, C, E>(
        &self,
        tolerance: MutualErrorTolerance,
        mut evaluate_cluster: C,
        mut evaluate_exact: E,
    ) -> Option<AdaptiveTraversalResult<T>>
    where
        C: FnMut(&[usize], &[usize]) -> Option<MutualClusterCertificate<T>>,
        E: FnMut(usize, usize) -> Option<T>,
    {
        if !tolerance.is_valid() {
            return None;
        }
        let mut result = AdaptiveTraversalResult::default();
        if let Some(root) = &self.root {
            Self::visit_self_adaptive(
                root,
                &tolerance,
                &mut evaluate_cluster,
                &mut evaluate_exact,
                &mut result,
            )?;
        }
        result.exact_pairs.sort_unstable();
        result
            .exact_pair_evaluations
            .sort_by_key(|evaluation| evaluation.pair);
        Some(result)
    }

    /// Apply the existing conservative group-visibility certificate before
    /// asking the kernel evaluator for any cluster error bounds.
    pub(crate) fn evaluate_adaptive_with_visibility<T, C, E>(
        &self,
        visibility: &crate::resonance_visibility::ResonanceVisibilityHierarchy,
        visibility_expansion: f64,
        tolerance: MutualErrorTolerance,
        mut evaluate_kernel: C,
        evaluate_exact: E,
    ) -> Option<AdaptiveTraversalResult<T>>
    where
        C: FnMut(&[usize], &[usize]) -> Option<(T, MutualClusterErrorBounds)>,
        E: FnMut(usize, usize) -> Option<T>,
    {
        self.evaluate_adaptive(
            tolerance,
            |bodies_a, bodies_b| {
                let visibility_certified = visibility.certify_groups_unoccluded(
                    bodies_a,
                    bodies_b,
                    visibility_expansion,
                )?;
                if !visibility_certified {
                    return None;
                }
                let (evaluation, errors) = evaluate_kernel(bodies_a, bodies_b)?;
                Some(MutualClusterCertificate {
                    visibility_certified: true,
                    evaluation,
                    errors,
                })
            },
            evaluate_exact,
        )
    }

    fn visit_self_adaptive<T, C, E>(
        node: &Node,
        tolerance: &MutualErrorTolerance,
        evaluate_cluster: &mut C,
        evaluate_exact: &mut E,
        result: &mut AdaptiveTraversalResult<T>,
    ) -> Option<()>
    where
        C: FnMut(&[usize], &[usize]) -> Option<MutualClusterCertificate<T>>,
        E: FnMut(usize, usize) -> Option<T>,
    {
        if let NodeKind::Branch { left, right } = &node.kind {
            Self::visit_self_adaptive(left, tolerance, evaluate_cluster, evaluate_exact, result)?;
            Self::visit_distinct_adaptive(
                left,
                right,
                tolerance,
                evaluate_cluster,
                evaluate_exact,
                result,
            )?;
            Self::visit_self_adaptive(right, tolerance, evaluate_cluster, evaluate_exact, result)?;
        }
        Some(())
    }

    fn visit_distinct_adaptive<T, C, E>(
        a: &Node,
        b: &Node,
        tolerance: &MutualErrorTolerance,
        evaluate_cluster: &mut C,
        evaluate_exact: &mut E,
        result: &mut AdaptiveTraversalResult<T>,
    ) -> Option<()>
    where
        C: FnMut(&[usize], &[usize]) -> Option<MutualClusterCertificate<T>>,
        E: FnMut(usize, usize) -> Option<T>,
    {
        if let (NodeKind::Leaf(index_a), NodeKind::Leaf(index_b)) = (&a.kind, &b.kind) {
            let pair = ((*index_a).min(*index_b), (*index_a).max(*index_b));
            let evaluation = evaluate_exact(pair.0, pair.1)?;
            result.exact_pairs.push(pair);
            result
                .exact_pair_evaluations
                .push(ExactPairEvaluation { pair, evaluation });
            return Some(());
        }

        let bodies_a = Self::body_indices(a);
        let bodies_b = Self::body_indices(b);
        if let Some(certificate) = evaluate_cluster(&bodies_a, &bodies_b) {
            if certificate.visibility_certified
                && certificate
                    .errors
                    .is_within(&bodies_a, &bodies_b, tolerance)
                && result.try_accumulate_errors(&certificate.errors, tolerance)
            {
                result.accepted_cluster_pairs.push(AcceptedClusterPair {
                    bodies_a,
                    bodies_b,
                    evaluation: certificate.evaluation,
                    errors: certificate.errors,
                });
                return Some(());
            }
        }

        match (&a.kind, &b.kind) {
            (NodeKind::Branch { left, right }, NodeKind::Leaf(_)) => {
                Self::visit_distinct_adaptive(
                    left,
                    b,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
                Self::visit_distinct_adaptive(
                    right,
                    b,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
            }
            (NodeKind::Leaf(_), NodeKind::Branch { left, right }) => {
                Self::visit_distinct_adaptive(
                    a,
                    left,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
                Self::visit_distinct_adaptive(
                    a,
                    right,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
            }
            (NodeKind::Branch { left, right }, NodeKind::Branch { .. })
                if Self::maximum_extent(a.bounds) >= Self::maximum_extent(b.bounds) =>
            {
                Self::visit_distinct_adaptive(
                    left,
                    b,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
                Self::visit_distinct_adaptive(
                    right,
                    b,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
            }
            (
                NodeKind::Branch { .. },
                NodeKind::Branch {
                    left: other_left,
                    right: other_right,
                },
            ) => {
                Self::visit_distinct_adaptive(
                    a,
                    other_left,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
                Self::visit_distinct_adaptive(
                    a,
                    other_right,
                    tolerance,
                    evaluate_cluster,
                    evaluate_exact,
                    result,
                )?;
            }
            (NodeKind::Leaf(_), NodeKind::Leaf(_)) => unreachable!(),
        }
        Some(())
    }

    fn body_indices(node: &Node) -> Vec<usize> {
        let mut indices = Vec::new();
        Self::collect_body_indices(node, &mut indices);
        indices.sort_unstable();
        indices
    }

    fn collect_body_indices(node: &Node, indices: &mut Vec<usize>) {
        match &node.kind {
            NodeKind::Leaf(index) => indices.push(*index),
            NodeKind::Branch { left, right } => {
                Self::collect_body_indices(left, indices);
                Self::collect_body_indices(right, indices);
            }
        }
    }

    /// Aggregate bounds are exposed to tests so future acceptance can require
    /// geometric/visibility/kernel certificates on the same cluster pair.
    #[cfg(test)]
    fn root_bounds(&self) -> Option<VisibilityBounds> {
        self.root.as_ref().map(|node| node.bounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x: f64, y: f64, half_width: f64, half_height: f64) -> VisibilityBounds {
        VisibilityBounds::new(
            x - half_width,
            y - half_height,
            x + half_width,
            y + half_height,
        )
        .unwrap()
    }

    fn exact_pairs(count: usize) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for a in 0..count {
            for b in a + 1..count {
                pairs.push((a, b));
            }
        }
        pairs
    }

    #[test]
    fn mutual_tree_emits_every_unordered_pair_once_for_adversarial_layouts() {
        let layouts = [
            (0..17)
                .map(|index| bounds(index as f64, 0.0, 0.0, 0.0))
                .collect::<Vec<_>>(),
            (0..24)
                .map(|_| bounds(3.0, -2.0, 0.5, 0.5))
                .collect::<Vec<_>>(),
            (0..31)
                .map(|index| {
                    let x = ((index * 17) % 13) as f64 - 6.0;
                    let y = ((index * 29) % 19) as f64 - 9.0;
                    bounds(x, y, (index % 4) as f64 * 0.1, (index % 5) as f64 * 0.08)
                })
                .collect::<Vec<_>>(),
        ];
        for layout in layouts {
            let tree = MutualInteractionHierarchy::new(&layout)
                .expect("finite body bounds should build a mutual tree");
            let pairs = tree
                .unordered_pairs()
                .expect("mutual traversal should complete");
            assert_eq!(pairs, exact_pairs(layout.len()));
            assert_eq!(
                pairs.windows(2).filter(|pair| pair[0] == pair[1]).count(),
                0
            );
        }
    }

    #[test]
    fn mutual_tree_handles_empty_and_single_body_worlds() {
        assert!(MutualInteractionHierarchy::new(&[])
            .unwrap()
            .unordered_pairs()
            .unwrap()
            .is_empty());
        let one = [bounds(0.0, 0.0, 1.0, 1.0)];
        assert!(MutualInteractionHierarchy::new(&one)
            .unwrap()
            .unordered_pairs()
            .unwrap()
            .is_empty());
    }

    fn zero_error_certificate(
        bodies_a: &[usize],
        bodies_b: &[usize],
        visibility_certified: bool,
    ) -> MutualClusterCertificate<(Vec<usize>, Vec<usize>)> {
        let bodies = bodies_a
            .iter()
            .chain(bodies_b.iter())
            .copied()
            .map(|body_index| BodyInteractionErrorBound {
                body_index,
                force_x: 0.0,
                force_y: 0.0,
                torque: 0.0,
            })
            .collect();
        MutualClusterCertificate {
            visibility_certified,
            evaluation: (bodies_a.to_vec(), bodies_b.to_vec()),
            errors: MutualClusterErrorBounds {
                potential: 0.0,
                bodies,
            },
        }
    }

    fn strict_tolerance() -> MutualErrorTolerance {
        MutualErrorTolerance {
            potential: 0.1,
            force_x: 0.1,
            force_y: 0.1,
            torque: 0.1,
        }
    }

    #[test]
    fn adaptive_traversal_accepts_only_complete_visible_cluster_certificates() {
        let layout = [
            bounds(0.0, 0.0, 0.1, 0.1),
            bounds(1.0, 0.0, 0.1, 0.1),
            bounds(2.0, 0.0, 0.1, 0.1),
            bounds(3.0, 0.0, 0.1, 0.1),
        ];
        let tree = MutualInteractionHierarchy::new(&layout).unwrap();
        let result = tree
            .evaluate_adaptive(
                strict_tolerance(),
                |a, b| Some(zero_error_certificate(a, b, true)),
                |a, b| Some((vec![a], vec![b])),
            )
            .unwrap();

        assert_eq!(result.exact_pairs, vec![(0, 1), (2, 3)]);
        assert_eq!(result.accepted_cluster_pairs.len(), 1);
        let accepted = &result.accepted_cluster_pairs[0];
        assert_eq!(accepted.bodies_a.len(), 2);
        assert_eq!(accepted.bodies_b.len(), 2);
        assert_eq!(
            accepted.evaluation,
            (accepted.bodies_a.clone(), accepted.bodies_b.clone())
        );
        assert!(accepted.errors.is_within(
            &accepted.bodies_a,
            &accepted.bodies_b,
            &strict_tolerance()
        ));
    }

    #[test]
    fn adaptive_visibility_gate_accepts_clear_groups_and_subdivides_blocked_groups() {
        let bodies = [
            bounds(0.0, 0.0, 0.2, 0.2),
            bounds(1.0, 0.0, 0.2, 0.2),
            bounds(8.0, 0.0, 0.2, 0.2),
            bounds(9.0, 0.0, 0.2, 0.2),
        ];
        let tree = MutualInteractionHierarchy::new(&bodies).unwrap();
        let clear_visibility =
            crate::resonance_visibility::ResonanceVisibilityHierarchy::new(&bodies).unwrap();
        let clear = tree
            .evaluate_adaptive_with_visibility(
                &clear_visibility,
                0.0,
                strict_tolerance(),
                |a, b| Some(((), zero_error_certificate(a, b, true).errors)),
                |_, _| Some(()),
            )
            .unwrap();
        assert_eq!(clear.accepted_cluster_pairs.len(), 1);
        assert_eq!(clear.exact_pairs, vec![(0, 1), (2, 3)]);

        let blocked_bounds = [
            bodies[0],
            bodies[1],
            bodies[2],
            bodies[3],
            bounds(4.0, 0.0, 0.3, 0.3),
        ];
        let blocked_visibility =
            crate::resonance_visibility::ResonanceVisibilityHierarchy::new(&blocked_bounds)
                .unwrap();
        let blocked = tree
            .evaluate_adaptive_with_visibility(
                &blocked_visibility,
                0.0,
                strict_tolerance(),
                |a, b| Some(((), zero_error_certificate(a, b, true).errors)),
                |_, _| Some(()),
            )
            .unwrap();
        assert!(blocked.accepted_cluster_pairs.is_empty());
        assert_eq!(blocked.exact_pairs, exact_pairs(bodies.len()));
    }

    #[test]
    fn adaptive_traversal_subdivides_when_any_independent_error_or_visibility_fails() {
        let layout = [
            bounds(0.0, 0.0, 0.1, 0.1),
            bounds(1.0, 0.0, 0.1, 0.1),
            bounds(2.0, 0.0, 0.1, 0.1),
            bounds(3.0, 0.0, 0.1, 0.1),
        ];
        let tree = MutualInteractionHierarchy::new(&layout).unwrap();
        for failing_metric in 0..5 {
            let result = tree
                .evaluate_adaptive(
                    strict_tolerance(),
                    |a, b| {
                        let mut certificate = zero_error_certificate(a, b, failing_metric != 4);
                        match failing_metric {
                            0 => certificate.errors.potential = 0.2,
                            1 => certificate.errors.bodies[0].force_x = 0.2,
                            2 => certificate.errors.bodies[0].force_y = 0.2,
                            3 => certificate.errors.bodies[0].torque = 0.2,
                            _ => {}
                        }
                        Some(certificate)
                    },
                    |a, b| Some((vec![a], vec![b])),
                )
                .unwrap();
            let mut covered_pairs = result.exact_pairs.clone();
            for accepted in &result.accepted_cluster_pairs {
                for index_a in &accepted.bodies_a {
                    for index_b in &accepted.bodies_b {
                        covered_pairs.push(((*index_a).min(*index_b), (*index_a).max(*index_b)));
                    }
                }
            }
            covered_pairs.sort_unstable();
            assert_eq!(covered_pairs, exact_pairs(layout.len()));
            if failing_metric == 4 {
                assert!(result.accepted_cluster_pairs.is_empty());
                assert_eq!(result.exact_pairs, exact_pairs(layout.len()));
            } else {
                assert!(result.accepted_cluster_pairs.iter().all(|accepted| {
                    !accepted.bodies_a.contains(&0) && !accepted.bodies_b.contains(&0)
                }));
                assert!(result
                    .exact_pairs
                    .iter()
                    .any(|pair| pair.0 == 0 || pair.1 == 0));
            }
        }
    }

    #[test]
    fn adaptive_traversal_enforces_cumulative_world_error_budgets() {
        let layout = (0..8)
            .map(|index| bounds(index as f64, 0.0, 0.1, 0.1))
            .collect::<Vec<_>>();
        let tree = MutualInteractionHierarchy::new(&layout).unwrap();
        for charged_metric in 0..4 {
            let result = tree
                .evaluate_adaptive(
                    strict_tolerance(),
                    |a, b| {
                        let mut certificate = zero_error_certificate(a, b, true);
                        match charged_metric {
                            0 => certificate.errors.potential = 0.06,
                            1 => {
                                for body in &mut certificate.errors.bodies {
                                    body.force_x = 0.06;
                                }
                            }
                            2 => {
                                for body in &mut certificate.errors.bodies {
                                    body.force_y = 0.06;
                                }
                            }
                            _ => {
                                for body in &mut certificate.errors.bodies {
                                    body.torque = 0.06;
                                }
                            }
                        }
                        Some(certificate)
                    },
                    |a, b| Some((vec![a], vec![b])),
                )
                .unwrap();
            assert_eq!(
                result.accepted_cluster_pairs.len(),
                if charged_metric == 0 { 1 } else { 2 }
            );
            assert!(result.accumulated_errors.potential <= strict_tolerance().potential);
            assert!(result.accumulated_errors.bodies.iter().all(|bound| {
                bound.force_x <= strict_tolerance().force_x
                    && bound.force_y <= strict_tolerance().force_y
                    && bound.torque <= strict_tolerance().torque
            }));
            assert_eq!(
                result.exact_pairs.len(),
                if charged_metric == 0 { 24 } else { 20 }
            );
            let mut covered_pairs = result.exact_pairs.clone();
            for accepted in &result.accepted_cluster_pairs {
                for index_a in &accepted.bodies_a {
                    for index_b in &accepted.bodies_b {
                        covered_pairs.push(((*index_a).min(*index_b), (*index_a).max(*index_b)));
                    }
                }
            }
            covered_pairs.sort_unstable();
            assert_eq!(covered_pairs, exact_pairs(layout.len()));
        }
    }

    #[test]
    fn adaptive_traversal_rejects_missing_or_duplicate_body_error_bounds() {
        let tolerance = strict_tolerance();
        let expected_a = [0, 1];
        let expected_b = [2, 3];
        let missing = MutualClusterErrorBounds {
            potential: 0.0,
            bodies: vec![BodyInteractionErrorBound {
                body_index: 0,
                force_x: 0.0,
                force_y: 0.0,
                torque: 0.0,
            }],
        };
        assert!(!missing.is_within(&expected_a, &expected_b, &tolerance));

        let duplicate = MutualClusterErrorBounds {
            potential: 0.0,
            bodies: vec![
                BodyInteractionErrorBound {
                    body_index: 0,
                    force_x: 0.0,
                    force_y: 0.0,
                    torque: 0.0,
                },
                BodyInteractionErrorBound {
                    body_index: 0,
                    force_x: 0.0,
                    force_y: 0.0,
                    torque: 0.0,
                },
                BodyInteractionErrorBound {
                    body_index: 2,
                    force_x: 0.0,
                    force_y: 0.0,
                    torque: 0.0,
                },
                BodyInteractionErrorBound {
                    body_index: 3,
                    force_x: 0.0,
                    force_y: 0.0,
                    torque: 0.0,
                },
            ],
        };
        assert!(!duplicate.is_within(&expected_a, &expected_b, &tolerance));
    }

    #[test]
    fn adaptive_traversal_fails_closed_when_exact_leaf_evaluation_fails() {
        let layout = [bounds(0.0, 0.0, 0.1, 0.1), bounds(1.0, 0.0, 0.1, 0.1)];
        let tree = MutualInteractionHierarchy::new(&layout).unwrap();
        assert!(tree
            .evaluate_adaptive(
                strict_tolerance(),
                |_, _| -> Option<MutualClusterCertificate<()>> { None },
                |_, _| None,
            )
            .is_none());
    }

    #[test]
    fn mutual_product_kernel_errors_map_to_each_body_and_include_occluders() {
        use crate::resonance_boundary_far_field::{
            BoundaryPoseDerivativeEvaluation, MutualBoundaryKernelError,
            MutualBoundaryKernelEvaluation,
        };

        let participants = [0usize, 1, 2, 3, 99];
        let make_evaluation = |error: f64| MutualBoundaryKernelEvaluation {
            potential: 1.0,
            coordinates: vec![
                BoundaryPoseDerivativeEvaluation {
                    derivative: 0.0,
                    absolute_bound: error,
                    absolute_error: error,
                    ..BoundaryPoseDerivativeEvaluation::default()
                };
                participants.len() * 3
            ],
            error: MutualBoundaryKernelError {
                potential: error,
                coordinates: vec![error; participants.len() * 3],
            },
        };
        let certificate =
            mutual_boundary_cluster_certificate(true, &participants, make_evaluation(0.001))
                .unwrap();
        assert_eq!(certificate.errors.bodies.len(), participants.len());
        assert_eq!(
            certificate
                .errors
                .bodies
                .iter()
                .map(|body| body.body_index)
                .collect::<Vec<_>>(),
            participants
        );
        assert_eq!(certificate.errors.bodies[4].force_x, 0.001);
        assert_eq!(certificate.errors.bodies[4].force_y, 0.001);
        assert_eq!(certificate.errors.bodies[4].torque, 0.001);

        let layout = [
            bounds(0.0, 0.0, 0.2, 0.2),
            bounds(1.0, 0.0, 0.2, 0.2),
            bounds(8.0, 0.0, 0.2, 0.2),
            bounds(9.0, 0.0, 0.2, 0.2),
        ];
        let tree = MutualInteractionHierarchy::new(&layout).unwrap();
        let result = tree
            .evaluate_adaptive(
                MutualErrorTolerance {
                    potential: 0.01,
                    force_x: 0.01,
                    force_y: 0.01,
                    torque: 0.01,
                },
                |_, _| {
                    mutual_boundary_cluster_certificate(true, &participants, make_evaluation(0.001))
                },
                |_, _| Some(make_evaluation(0.0)),
            )
            .unwrap();
        assert_eq!(result.accepted_cluster_pairs.len(), 1);
        assert_eq!(result.exact_pairs, vec![(0, 1), (2, 3)]);
        assert_eq!(result.accumulated_errors.bodies.len(), participants.len());
        assert!(result
            .accumulated_errors
            .bodies
            .iter()
            .any(|body| body.body_index == 99));
    }

    #[test]
    fn adaptive_traversal_retains_exact_leaf_evaluations() {
        let layout = [bounds(0.0, 0.0, 0.1, 0.1), bounds(1.0, 0.0, 0.1, 0.1)];
        let tree = MutualInteractionHierarchy::new(&layout).unwrap();
        let result = tree
            .evaluate_adaptive(
                strict_tolerance(),
                |_, _| -> Option<MutualClusterCertificate<usize>> { None },
                |a, b| Some(a * 10 + b),
            )
            .unwrap();
        assert_eq!(result.exact_pairs, vec![(0, 1)]);
        assert_eq!(result.exact_pair_evaluations.len(), 1);
        assert_eq!(result.exact_pair_evaluations[0].pair, (0, 1));
        assert_eq!(result.exact_pair_evaluations[0].evaluation, 1);
    }

    #[test]
    fn mutual_tree_rejects_invalid_bounds_and_preserves_world_extent() {
        let invalid = [VisibilityBounds {
            min_x: 2.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        }];
        assert!(MutualInteractionHierarchy::new(&invalid).is_none());

        let layout = [bounds(-4.0, 2.0, 1.0, 3.0), bounds(8.0, -5.0, 2.0, 1.0)];
        let tree = MutualInteractionHierarchy::new(&layout).unwrap();
        let root = tree.root_bounds().unwrap();
        assert_eq!(root.min_x, -5.0);
        assert_eq!(root.min_y, -6.0);
        assert_eq!(root.max_x, 10.0);
        assert_eq!(root.max_y, 5.0);
    }
}
