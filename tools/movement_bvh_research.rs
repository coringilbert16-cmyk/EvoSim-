// Standalone, dependency-free Rust benchmark for the movement BVH investigation.
// Input: the movement-spatial-audit.log produced by PR #368.
// This is research-only; it does not participate in simulation decisions.

use std::env;
use std::fs;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    id: usize,
}
impl Bounds {
    fn overlaps(self, other: Self) -> bool {
        self.min_x <= other.max_x + 1.0e-9
            && other.min_x <= self.max_x + 1.0e-9
            && self.min_y <= other.max_y + 1.0e-9
            && other.min_y <= self.max_y + 1.0e-9
    }
    fn union(a: Self, b: Self) -> Self {
        Self {
            min_x: a.min_x.min(b.min_x),
            max_x: a.max_x.max(b.max_x),
            min_y: a.min_y.min(b.min_y),
            max_y: a.max_y.max(b.max_y),
            id: 0,
        }
    }
}
#[derive(Clone, Copy)]
struct Event { min_x: f64, max_x: f64, id: usize, target: bool }

#[derive(Default)]
struct PassBounds {
    targets: Vec<(u64, Bounds)>,
    candidates: Vec<(u64, Bounds)>,
}
fn parse_snapshots(text: &str) -> Vec<(Vec<Bounds>, Vec<Bounds>)> {
    use std::collections::BTreeMap;
    let mut by_pass: BTreeMap<usize, PassBounds> = BTreeMap::new();
    for line in text.lines().filter(|line| line.contains("MOVEMENT_SPATIAL_BOUNDS ")) {
        let field = |key: &str| line.split_once(key).expect("bound field").1.split_whitespace().next().expect("bound value");
        let pass: usize = field("pass=").parse().expect("pass number");
        let identity: u64 = field("identity=").parse().expect("material identity");
        let bounds = Bounds {
            id: 0,
            min_x: field("min_x=").parse().expect("min_x"),
            max_x: field("max_x=").parse().expect("max_x"),
            min_y: field("min_y=").parse().expect("min_y"),
            max_y: field("max_y=").parse().expect("max_y"),
        };
        let entry = by_pass.entry(pass).or_default();
        if line.contains("set=target ") {
            entry.targets.push((identity, bounds));
        } else if line.contains("set=candidate ") {
            entry.candidates.push((identity, bounds));
        }
    }
    let mut snapshots = Vec::new();
    for (pass, mut sets) in by_pass {
        sets.targets.sort_by_key(|(identity, _)| *identity);
        sets.candidates.sort_by_key(|(identity, _)| *identity);
        let target_ids: Vec<_> = sets.targets.iter().map(|(id, _)| *id).collect();
        let candidate_ids: Vec<_> = sets.candidates.iter().map(|(id, _)| *id).collect();
        assert_eq!(target_ids, candidate_ids, "target/candidate identity mismatch at pass {pass}");
        let targets = sets.targets.into_iter().enumerate().map(|(id, (_, mut b))| {
            b.id = id; b
        }).collect();
        let candidates = sets.candidates.into_iter().enumerate().map(|(id, (_, mut b))| {
            b.id = id; b
        }).collect();
        snapshots.push((targets, candidates));
    }
    snapshots
}

#[derive(Clone, Copy)]
struct Node { bounds: Bounds, left: Option<usize>, right: Option<usize>, lo: usize, hi: usize }
struct Bvh { nodes: Vec<Node>, items: Vec<Bounds>, leaf_size: usize, leaf_for_id: Vec<usize>, item_pos: Vec<usize> }
impl Bvh {
    fn new(items: Vec<Bounds>, leaf_size: usize) -> Self {
        let max_id = items.iter().map(|b| b.id).max().unwrap_or(0);
        let mut tree = Self {
            nodes: Vec::with_capacity(items.len() * 2),
            leaf_for_id: vec![usize::MAX; max_id + 1],
            item_pos: vec![usize::MAX; max_id + 1],
            items, leaf_size,
        };
        tree.build(0, tree.items.len());
        // BVH construction partitions the item array; rebuild the identity map
        // afterward so future refits update each object's actual leaf slot.
        for (i, b) in tree.items.iter().enumerate() { tree.item_pos[b.id] = i; }
        tree
    }
    fn range_bounds(&self, lo: usize, hi: usize) -> Bounds {
        let mut b = self.items[lo];
        for item in &self.items[lo + 1..hi] { b = Bounds::union(b, *item); }
        b
    }
    fn build(&mut self, lo: usize, hi: usize) -> usize {
        let index = self.nodes.len();
        let bounds = self.range_bounds(lo, hi);
        self.nodes.push(Node { bounds, left: None, right: None, lo, hi });
        if hi - lo > self.leaf_size {
            let split_x = (bounds.max_x - bounds.min_x) >= (bounds.max_y - bounds.min_y);
            let mid = (lo + hi) / 2;
            self.items[lo..hi].select_nth_unstable_by(mid - lo, |a, b| {
                let ca = if split_x { a.min_x + a.max_x } else { a.min_y + a.max_y };
                let cb = if split_x { b.min_x + b.max_x } else { b.min_y + b.max_y };
                ca.total_cmp(&cb).then_with(|| a.id.cmp(&b.id))
            });
            let left = self.build(lo, mid);
            let right = self.build(mid, hi);
            self.nodes[index].left = Some(left);
            self.nodes[index].right = Some(right);
        } else {
            for item in &self.items[lo..hi] { self.leaf_for_id[item.id] = index; }
        }
        index
    }
    fn refit_node(&mut self, node: usize, dirty: &[bool]) -> Bounds {
        let n = self.nodes[node];
        let updated = if let (Some(l), Some(r)) = (n.left, n.right) {
            Bounds::union(self.refit_node(l, dirty), self.refit_node(r, dirty))
        } else if dirty[node] {
            self.range_bounds(n.lo, n.hi)
        } else { n.bounds };
        self.nodes[node].bounds = updated;
        updated
    }
    fn refit(&mut self, updates: &[Bounds]) {
        let mut dirty = vec![false; self.nodes.len()];
        for b in updates {
            let pos = self.item_pos[b.id];
            if pos == usize::MAX { continue; }
            let old = self.items[pos];
            if old.min_x != b.min_x || old.max_x != b.max_x || old.min_y != b.min_y || old.max_y != b.max_y {
                self.items[pos] = *b;
                dirty[self.leaf_for_id[b.id]] = true;
            }
        }
        self.refit_node(0, &dirty);
    }
    fn query_node(&self, node: usize, q: Bounds, out: &mut Vec<usize>, visits: &mut u64) {
        *visits += 1;
        let n = self.nodes[node];
        if !n.bounds.overlaps(q) { return; }
        if let (Some(l), Some(r)) = (n.left, n.right) {
            self.query_node(l, q, out, visits);
            self.query_node(r, q, out, visits);
        } else {
            for b in &self.items[n.lo..n.hi] { if b.overlaps(q) { out.push(b.id); } }
        }
    }
}

fn sweep_cross(q: &[Bounds], t: &[Bounds]) -> (Vec<(usize, usize)>, u64) {
    let mut events = Vec::with_capacity(q.len() + t.len());
    events.extend(q.iter().map(|b| Event { min_x: b.min_x, max_x: b.max_x, id: b.id, target: false }));
    events.extend(t.iter().map(|b| Event { min_x: b.min_x, max_x: b.max_x, id: b.id, target: true }));
    events.sort_by(|a,b| a.min_x.total_cmp(&b.min_x).then_with(|| b.target.cmp(&a.target)).then_with(|| a.id.cmp(&b.id)));
    let mut active: Vec<Event> = Vec::new();
    let mut pairs = Vec::new();
    let mut comparisons = 0;
    for e in events {
        active.retain(|x| x.max_x + 1.0e-9 >= e.min_x);
        for x in &active {
            if x.target == e.target || x.id == e.id { continue; }
            comparisons += 1;
            let (qb, tb) = if e.target { (&q[x.id], &t[e.id]) } else { (&q[e.id], &t[x.id]) };
            if qb.overlaps(*tb) { pairs.push((qb.id, tb.id)); }
        }
        active.push(e);
    }
    pairs.sort_unstable();
    (pairs, comparisons)
}
fn sweep_self(v: &[Bounds]) -> (Vec<(usize, usize)>, u64) {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a,&b| v[a].min_x.total_cmp(&v[b].min_x).then_with(|| a.cmp(&b)));
    let mut active: Vec<usize> = Vec::new();
    let mut pairs = Vec::new();
    let mut comparisons = 0;
    for i in order {
        active.retain(|&j| v[j].max_x + 1.0e-9 >= v[i].min_x);
        for &j in &active {
            comparisons += 1;
            if v[i].overlaps(v[j]) { pairs.push((v[i].id.min(v[j].id), v[i].id.max(v[j].id))); }
        }
        active.push(i);
    }
    pairs.sort_unstable();
    (pairs, comparisons)
}
fn oracle(q: &[Bounds], t: &[Bounds]) -> (Vec<(usize,usize)>, Vec<(usize,usize)>) {
    let mut cross = Vec::new();
    for a in q { for b in t { if a.id != b.id && a.overlaps(*b) { cross.push((a.id,b.id)); } } }
    let mut self_pairs = Vec::new();
    for i in 0..q.len() { for j in i+1..q.len() { if q[i].overlaps(q[j]) {
        self_pairs.push((q[i].id.min(q[j].id), q[i].id.max(q[j].id)));
    }}}
    cross.sort_unstable();
    self_pairs.sort_unstable();
    (cross, self_pairs)
}
fn main() {
    let path = env::args().nth(1).expect("usage: movement_bvh_research <movement-spatial-audit.log>");
    let text = fs::read_to_string(path).expect("read audit log");
    let snapshots = parse_snapshots(&text);
    println!("captured_consecutive_passes={} first_pass_targets={} first_pass_candidates={}",
        snapshots.len(), snapshots.first().map_or(0, |s| s.0.len()),
        snapshots.first().map_or(0, |s| s.1.len()));
    assert!(snapshots.len() >= 2, "need at least two consecutive captured passes");
    let updates = snapshots.len() - 1;
    for leaf in [8usize, 16, 32, 64] {
        let build_started = Instant::now();
        let mut target_tree = Bvh::new(snapshots[0].0.clone(), leaf);
        let mut candidate_tree = Bvh::new(snapshots[0].1.clone(), leaf);
        let build_ms = build_started.elapsed().as_secs_f64() * 1000.0;
        let mut target_refit = 0.0;
        let mut candidate_refit = 0.0;
        let mut bvh_query = 0.0;
        let mut sweep_query = 0.0;
        let mut visits_total = 0u64;
        let mut comparisons_total = 0u64;
        let mut changed_targets_total = 0usize;
        let mut changed_candidates_total = 0usize;
        let mut exact = true;
        let mut previous_targets = snapshots[0].0.clone();
        let mut previous_candidates = snapshots[0].1.clone();
        for (next_targets, next_candidates) in snapshots.iter().skip(1) {
            changed_targets_total += next_targets.iter().zip(&previous_targets)
                .filter(|(a,b)| a.min_x != b.min_x || a.max_x != b.max_x || a.min_y != b.min_y || a.max_y != b.max_y)
                .count();
            changed_candidates_total += next_candidates.iter().zip(&previous_candidates)
                .filter(|(a,b)| a.min_x != b.min_x || a.max_x != b.max_x || a.min_y != b.min_y || a.max_y != b.max_y)
                .count();
            let t0 = Instant::now();
            target_tree.refit(next_targets);
            target_refit += t0.elapsed().as_secs_f64() * 1000.0;
            let t0 = Instant::now();
            candidate_tree.refit(next_candidates);
            candidate_refit += t0.elapsed().as_secs_f64() * 1000.0;

            let mut bvh_cross = Vec::new();
            let mut bvh_self = Vec::new();
            let mut hits = Vec::new();
            let mut visits = 0;
            let t0 = Instant::now();
            for _ in 0..10 {
                bvh_cross.clear();
                bvh_self.clear();
                for q in next_candidates {
                    hits.clear();
                    target_tree.query_node(0, *q, &mut hits, &mut visits);
                    for &id in &hits {
                        if id != q.id { bvh_cross.push((q.id, id)); }
                    }
                    hits.clear();
                    candidate_tree.query_node(0, *q, &mut hits, &mut visits);
                    for &id in &hits {
                        if id > q.id { bvh_self.push((q.id, id)); }
                    }
                }
                bvh_cross.sort_unstable();
                bvh_self.sort_unstable();
            }
            bvh_query += t0.elapsed().as_secs_f64() * 100.0;

            let t0 = Instant::now();
            let mut sweep_x = Vec::new();
            let mut sweep_m = Vec::new();
            let mut comparisons = 0;
            for _ in 0..10 {
                let (x, cx) = sweep_cross(next_candidates, next_targets);
                let (m, cm) = sweep_self(next_candidates);
                sweep_x = x;
                sweep_m = m;
                comparisons = cx + cm;
            }
            sweep_query += t0.elapsed().as_secs_f64() * 100.0;
            let (oracle_x, oracle_m) = oracle(next_candidates, next_targets);
            exact &= bvh_cross == oracle_x
                && bvh_self == oracle_m
                && sweep_x == oracle_x
                && sweep_m == oracle_m;
            visits_total += visits / 10;
            comparisons_total += comparisons;
            previous_targets.clone_from(next_targets);
            previous_candidates.clone_from(next_candidates);
        }
        println!("leaf={} actual_refits={} initial_build_ms={:.5} target_refit_avg_ms={:.5} candidate_refit_avg_ms={:.5} bvh_query_avg_ms={:.5} sweep_query_avg_ms={:.5} changed_target_bounds_avg={:.1} changed_candidate_bounds_avg={:.1} bvh_node_visits_avg={} sweep_comparisons_avg={} exact_all_passes={}",
            leaf, updates, build_ms, target_refit / updates as f64,
            candidate_refit / updates as f64, bvh_query / updates as f64,
            sweep_query / updates as f64, changed_targets_total as f64 / updates as f64,
            changed_candidates_total as f64 / updates as f64,
            visits_total / updates as u64, comparisons_total / updates as u64, exact);
        assert!(exact, "pair mismatch for leaf size {leaf}");
    }
}
