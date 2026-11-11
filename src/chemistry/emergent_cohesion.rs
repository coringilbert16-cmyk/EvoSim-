#![expect(
    dead_code,
    reason = "Staged API retained for compound-stability integration"
)]
//! Compound-level cohesion derived from the topology of its realized bond graph.
//!
//! Local bond integrity is the weight of each edge. Emergent cohesion is the
//! minimum total integrity that must be removed to separate all constituents
//! into two non-empty groups. It is a graph descriptor, not a replacement for
//! evaluating a particular BREAK transaction.

use crate::structure::{PhysicalConstituentGraph, PhysicalConstituentId};
use std::collections::HashMap;

/// Return the weighted global minimum cut of a compound's physical bond graph.
///
/// The graph must have unique, nonzero constituent IDs; every bond endpoint
/// must refer to a constituent in the graph; and every edge integrity must be
/// finite and non-negative. Parallel physical bonds contribute independently.
/// A disconnected graph, or a graph with fewer than two vertices, has zero
/// emergent cohesion.
///
/// Uses the deterministic Stoer-Wagner global minimum-cut algorithm.
pub fn emergent_cohesion(graph: &PhysicalConstituentGraph) -> Option<f64> {
    let count = graph.units.len();
    if count < 2 {
        return Some(0.0);
    }

    let mut index_by_id = HashMap::<PhysicalConstituentId, usize>::with_capacity(count);
    for (index, unit) in graph.units.iter().enumerate() {
        if unit.physical_id.0 == 0 || index_by_id.insert(unit.physical_id, index).is_some() {
            return None;
        }
    }

    let mut weights = vec![vec![0.0; count]; count];
    for bond in &graph.bonds {
        if !bond.strength.is_finite() || bond.strength < 0.0 {
            return None;
        }
        let a = *index_by_id.get(&bond.endpoint_a.constituent_id)?;
        let b = *index_by_id.get(&bond.endpoint_b.constituent_id)?;
        if a == b {
            return None;
        }
        weights[a][b] += bond.strength;
        weights[b][a] += bond.strength;
        if !weights[a][b].is_finite() {
            return None;
        }
    }

    Some(stoer_wagner_min_cut(weights))
}

fn stoer_wagner_min_cut(mut weights: Vec<Vec<f64>>) -> f64 {
    let count = weights.len();
    if count < 2 {
        return 0.0;
    }

    let mut active: Vec<usize> = (0..count).collect();
    let mut minimum_cut = f64::INFINITY;

    while active.len() > 1 {
        let mut added = vec![false; count];
        let mut connection_weight = vec![0.0; count];
        let mut previous = active[0];

        for phase_index in 0..active.len() {
            let selected = active
                .iter()
                .copied()
                .filter(|vertex| !added[*vertex])
                .max_by(|left, right| {
                    connection_weight[*left]
                        .total_cmp(&connection_weight[*right])
                        .then_with(|| right.cmp(left))
                })
                .expect("each minimum-cut phase has an unselected vertex");

            if phase_index + 1 == active.len() {
                minimum_cut = minimum_cut.min(connection_weight[selected]);

                // Contract the final vertex into the penultimate vertex.
                for &vertex in &active {
                    if vertex != previous && vertex != selected {
                        weights[previous][vertex] += weights[selected][vertex];
                        weights[vertex][previous] = weights[previous][vertex];
                    }
                }
                active.retain(|vertex| *vertex != selected);
                break;
            }

            added[selected] = true;
            previous = selected;
            for &vertex in &active {
                if !added[vertex] {
                    connection_weight[vertex] += weights[selected][vertex];
                }
            }
        }
    }

    minimum_cut
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::{
        Bond, BondEndpoint, ConnectionEndpoint, PhysicalConstituentGraph, Placement, StructuralUnit,
    };

    fn graph_with_edges(
        vertex_count: usize,
        edges: &[(usize, usize, f64)],
    ) -> PhysicalConstituentGraph {
        let mut graph = PhysicalConstituentGraph::new();
        for index in 0..vertex_count {
            graph.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: index as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        for &(a, b, integrity) in edges {
            let id_a = graph.units[a].physical_id;
            let id_b = graph.units[b].physical_id;
            graph.push_bond_unchecked(Bond {
                endpoint_a: BondEndpoint::new(
                    id_a,
                    ConnectionEndpoint::Boundary { angle_radians: 0.0 },
                ),
                endpoint_b: BondEndpoint::new(
                    id_b,
                    ConnectionEndpoint::Boundary { angle_radians: 1.0 },
                ),
                strength: integrity,
                bond_energy: 0.0,
            });
        }
        graph
    }

    #[test]
    fn chain_is_limited_by_its_weakest_bridge() {
        let graph = graph_with_edges(3, &[(0, 1, 2.0), (1, 2, 3.0)]);
        assert_eq!(emergent_cohesion(&graph), Some(2.0));
    }

    #[test]
    fn ring_requires_cutting_two_edges() {
        let graph = graph_with_edges(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 0, 1.0)]);
        assert_eq!(emergent_cohesion(&graph), Some(2.0));
    }

    #[test]
    fn branch_is_limited_by_its_weak_attachment() {
        let graph = graph_with_edges(4, &[(0, 1, 1.0), (1, 2, 4.0), (1, 3, 4.0)]);
        assert_eq!(emergent_cohesion(&graph), Some(1.0));
    }

    #[test]
    fn redundant_edge_increases_emergent_cohesion_without_changing_edge_weights() {
        let chain = graph_with_edges(3, &[(0, 1, 1.0), (1, 2, 1.0)]);
        let triangle = graph_with_edges(3, &[(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]);
        assert_eq!(emergent_cohesion(&chain), Some(1.0));
        assert_eq!(emergent_cohesion(&triangle), Some(2.0));
        assert_eq!(chain.bonds[0].strength, triangle.bonds[0].strength);
    }

    #[test]
    fn disconnected_or_unbonded_constituents_do_not_add_cohesion() {
        let disconnected = graph_with_edges(3, &[(0, 1, 5.0)]);
        let unbonded = graph_with_edges(4, &[(0, 1, 5.0), (1, 2, 5.0)]);
        assert_eq!(emergent_cohesion(&disconnected), Some(0.0));
        assert_eq!(emergent_cohesion(&unbonded), Some(0.0));
    }

    #[test]
    fn parallel_bonds_contribute_independently_to_a_cut() {
        let graph = graph_with_edges(2, &[(0, 1, 0.4), (0, 1, 0.6)]);
        assert!((emergent_cohesion(&graph).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_edge_integrity_is_rejected() {
        let graph = graph_with_edges(2, &[(0, 1, f64::NAN)]);
        assert_eq!(emergent_cohesion(&graph), None);
    }

    #[test]
    fn a_single_constituent_has_no_separating_cut() {
        let graph = graph_with_edges(1, &[]);
        assert_eq!(emergent_cohesion(&graph), Some(0.0));
    }
}
