//! Conservative AABB hierarchy for resonance visibility broad phases.
//!
//! This hierarchy only removes bodies whose bounds cannot intersect a
//! source-to-receiver envelope. It does not decide visibility: returned
//! candidates still go through the exact geometric visibility calculation.
//! No interaction-distance cutoff is applied.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VisibilityBounds {
    pub(crate) min_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_x: f64,
    pub(crate) max_y: f64,
}

impl VisibilityBounds {
    pub(crate) fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Option<Self> {
        let bounds = Self {
            min_x,
            min_y,
            max_x,
            max_y,
        };
        bounds.is_valid().then_some(bounds)
    }

    fn is_valid(self) -> bool {
        self.min_x.is_finite()
            && self.min_y.is_finite()
            && self.max_x.is_finite()
            && self.max_y.is_finite()
            && self.min_x <= self.max_x
            && self.min_y <= self.max_y
    }

    fn intersects(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }

    fn union(self, other: Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    fn center(self, axis_x: bool) -> f64 {
        if axis_x {
            self.min_x + (self.max_x - self.min_x) * 0.5
        } else {
            self.min_y + (self.max_y - self.min_y) * 0.5
        }
    }
}

#[derive(Clone, Debug)]
enum NodeKind {
    Leaf(Vec<usize>),
    Branch { left: usize, right: usize },
}

#[derive(Clone, Debug)]
struct Node {
    bounds: VisibilityBounds,
    kind: NodeKind,
}

/// Immutable broad-phase index over body bounds. Query results are sorted by
/// original body index so downstream force accumulation remains deterministic.
#[derive(Clone, Debug, Default)]
pub(crate) struct ResonanceVisibilityHierarchy {
    nodes: Vec<Node>,
    body_bounds: Vec<VisibilityBounds>,
    root: Option<usize>,
}

impl ResonanceVisibilityHierarchy {
    const LEAF_CAPACITY: usize = 8;

    pub(crate) fn new(bounds: &[VisibilityBounds]) -> Option<Self> {
        if bounds.iter().any(|bound| !bound.is_valid()) {
            return None;
        }
        let mut hierarchy = Self {
            nodes: Vec::new(),
            body_bounds: bounds.to_vec(),
            root: None,
        };
        if bounds.is_empty() {
            return Some(hierarchy);
        }
        let indices = (0..bounds.len()).collect::<Vec<_>>();
        hierarchy.root = Some(hierarchy.build_node(indices, bounds)?);
        Some(hierarchy)
    }

    fn build_node(
        &mut self,
        mut indices: Vec<usize>,
        bounds: &[VisibilityBounds],
    ) -> Option<usize> {
        let aggregate = indices
            .iter()
            .map(|index| bounds.get(*index).copied())
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .reduce(VisibilityBounds::union)?;
        if indices.len() <= Self::LEAF_CAPACITY {
            let node_index = self.nodes.len();
            self.nodes.push(Node {
                bounds: aggregate,
                kind: NodeKind::Leaf(indices),
            });
            return Some(node_index);
        }

        let split_x = aggregate.max_x - aggregate.min_x >= aggregate.max_y - aggregate.min_y;
        indices.sort_by(|left, right| {
            bounds[*left]
                .center(split_x)
                .total_cmp(&bounds[*right].center(split_x))
                .then_with(|| left.cmp(right))
        });
        let right_indices = indices.split_off(indices.len() / 2);
        let left = self.build_node(indices, bounds)?;
        let right = self.build_node(right_indices, bounds)?;
        let node_index = self.nodes.len();
        self.nodes.push(Node {
            bounds: aggregate,
            kind: NodeKind::Branch { left, right },
        });
        Some(node_index)
    }

    /// Return every indexed bound intersecting the query envelope. A false
    /// positive is permitted; a false negative is not.
    pub(crate) fn query(&self, envelope: VisibilityBounds) -> Option<Vec<usize>> {
        if !envelope.is_valid() {
            return None;
        }
        let Some(root) = self.root else {
            return Some(Vec::new());
        };
        let mut pending = vec![root];
        let mut matches = Vec::new();
        while let Some(node_index) = pending.pop() {
            let node = self.nodes.get(node_index)?;
            if !node.bounds.intersects(envelope) {
                continue;
            }
            match &node.kind {
                NodeKind::Leaf(indices) => {
                    matches.extend(indices.iter().copied().filter(|index| {
                        self.body_bounds[*index].intersects(envelope)
                    }));
                }
                NodeKind::Branch { left, right } => {
                    pending.push(*right);
                    pending.push(*left);
                }
            }
        }
        matches.sort_unstable();
        Some(matches)
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchy_query_matches_brute_force_without_false_negatives() {
        let bounds = (0..257)
            .map(|index| {
                let x = (index as f64 * 17.0).rem_euclid(113.0) - 56.0;
                let y = (index as f64 * 29.0).rem_euclid(97.0) - 48.0;
                VisibilityBounds::new(x, y, x + 0.7, y + 1.3).unwrap()
            })
            .collect::<Vec<_>>();
        let hierarchy = ResonanceVisibilityHierarchy::new(&bounds).unwrap();
        for query_index in 0..71 {
            let x = query_index as f64 * 1.9 - 60.0;
            let y = query_index as f64 * -1.1 + 35.0;
            let query = VisibilityBounds::new(x, y, x + 12.0, y + 9.0).unwrap();
            let expected = bounds
                .iter()
                .enumerate()
                .filter_map(|(index, bound)| bound.intersects(query).then_some(index))
                .collect::<Vec<_>>();
            assert_eq!(hierarchy.query_with_bounds(&bounds, query), Some(expected));
        }
    }

    #[test]
    fn hierarchy_handles_empty_and_degenerate_bounds() {
        let empty = ResonanceVisibilityHierarchy::new(&[]).unwrap();
        let query = VisibilityBounds::new(0.0, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(empty.query(query), Some(Vec::new()));

        let bounds = [VisibilityBounds::new(1.0, 1.0, 1.0, 1.0).unwrap()];
        let hierarchy = ResonanceVisibilityHierarchy::new(&bounds).unwrap();
        assert_eq!(hierarchy.query_with_bounds(&bounds, query), Some(Vec::new()));
        assert_eq!(
            hierarchy.query_with_bounds(
                &bounds,
                VisibilityBounds::new(1.0, 1.0, 1.0, 1.0).unwrap()
            ),
            Some(vec![0])
        );
    }

    #[test]
    fn invalid_bounds_fail_closed() {
        assert!(VisibilityBounds::new(2.0, 0.0, 1.0, 1.0).is_none());
        assert!(ResonanceVisibilityHierarchy::new(&[
            VisibilityBounds {
                min_x: f64::NAN,
                min_y: 0.0,
                max_x: 1.0,
                max_y: 1.0,
            }
        ])
        .is_none());
    }
}
