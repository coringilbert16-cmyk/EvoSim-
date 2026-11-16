//! Test-only mutual dual-tree traversal for complete environmental interactions.
//!
//! This stage deliberately performs no multipole acceptance. Every unordered
//! body pair is emitted exactly once as an exact leaf interaction. The spatial
//! hierarchy establishes deterministic mutual traversal and a safe integration
//! point for later kernel/visibility certificates without dropping long-range
//! interactions or changing runtime behavior.

use crate::resonance_visibility::VisibilityBounds;

#[derive(Clone, Debug)]
struct Node {
    bounds: VisibilityBounds,
    count: usize,
    kind: NodeKind,
}

#[derive(Clone, Debug)]
enum NodeKind {
    Leaf(usize),
    Branch { left: Box<Node>, right: Box<Node> },
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
                count: 1,
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
            count: left.count.checked_add(right.count)?,
            kind: NodeKind::Branch {
                left: Box::new(left),
                right: Box::new(right),
            },
        })
    }

    fn maximum_extent(self) -> f64 {
        (self.max_x - self.min_x).max(self.max_y - self.min_y)
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
            (
                NodeKind::Branch { left, right },
                NodeKind::Branch {
                    left: other_left,
                    right: other_right,
                },
            ) if a.bounds.maximum_extent() >= b.bounds.maximum_extent() => {
                Self::visit_distinct(left, b, pairs)?;
                Self::visit_distinct(right, b, pairs)?;
                let _ = (other_left, other_right);
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
                .map(|index| bounds(3.0, -2.0, 0.5, 0.5))
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
            assert_eq!(pairs.windows(2).filter(|pair| pair[0] == pair[1]).count(), 0);
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
