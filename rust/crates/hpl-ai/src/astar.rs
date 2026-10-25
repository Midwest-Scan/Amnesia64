//! `cAStarHandler` search core (`ai/AStar.cpp`).
//!
//! Audit of the original before porting:
//! - Already-open nodes are never relaxed, and closed nodes never reopened.
//! - Best-node ties follow the C++ set's graph-pointer order; a first NaN
//!   cost cannot be displaced, while later NaNs are ignored.
//! - Edge count is captured before callbacks, but edge contents, parent
//!   distance, the callback pointer and iteration limit remain live.
//! - GetPath's direct-path fast return preserves old sets and the caller's
//!   list; successful graph paths append goal-to-start. Setup/output decisions
//!   now live in astar_path; allocation and set ownership remain in C++.
//! - Empty GetBestNode, invalidated edge/parent pointers, reading a newly
//!   allocated node's uninitialized costs, and signed iteration overflow
//!   are undefined in C++. The first and last abort at the bridge/increment;
//!   callers must still keep graph storage valid, just as in C++.

use hpl_math::Vector3f;

/// Select from the host's existing pointer order, without sorting or copying
/// its containers. Empty input is represented explicitly for Rust callers.
pub fn best_node<N>(nodes: impl IntoIterator<Item = (N, f32)>) -> Option<N> {
    let mut nodes = nodes.into_iter();
    let (mut best, mut best_cost) = nodes.next()?;
    for (node, node_cost) in nodes {
        // Faithful: strict > retains the first tie, and preserves NaN rules.
        if best_cost > node_cost {
            best = node;
            best_cost = node_cost;
        }
    }
    Some(best)
}

/// `Cost`: the height multiplier applies to the entire traveled distance.
pub fn cost(distance: f32, node_y: f32, parent_y: Option<f32>) -> f32 {
    if let Some(parent_y) = parent_y {
        // Faithful: MSVC's fabs(float), addition and multiply are all f32.
        let height = 1.0 + (node_y - parent_y).abs();
        // Faithful: MSVC selects the height NaN when both operands are NaN.
        // Rust debug/release may otherwise preserve different payloads.
        if height.is_nan() { quiet_nan(height) } else { distance * height }
    } else {
        distance
    }
}

pub fn heuristic(start: Vector3f, goal: Vector3f) -> f32 {
    let dx = goal.x - start.x;
    let dy = goal.y - start.y;
    let dz = goal.z - start.z;
    let xx = dx * dx;
    let yy = dy * dy;
    // Faithful: native MSVC Vector3Dist chooses a NaN from y, then x, then z.
    // The finite sum still evaluates (xx + yy) + zz. Keep this observed
    // register/NaN rule local; ordinary Rust debug/release disagree here.
    let xy = if yy.is_nan() { quiet_nan(yy) } else { add_squared(xx, yy) };
    add_squared(xy, dz * dz).sqrt()
}

fn quiet_nan(value: f32) -> f32 {
    f32::from_bits(value.to_bits() | 0x00400000)
}

fn add_squared(left: f32, right: f32) -> f32 {
    if left.is_nan() { quiet_nan(left) } else if right.is_nan() { quiet_nan(right) } else { left + right }
}

/// Live access to the existing graph and search state. Handles are copied,
/// never borrowed across a callback. The host owns all allocations and sets.
pub trait SearchHost {
    type SearchNode: Copy;
    type Node: Copy;
    type Edge: Copy;

    fn open_empty(&self) -> bool;
    fn max_iterations(&self) -> i32;
    /// None requests immediate return (e.g. a saved C++ allocation exception).
    fn best_node(&mut self) -> Option<Self::SearchNode>;
    fn graph_node(&self, search: Self::SearchNode) -> Self::Node;
    fn is_goal(&self, node: Self::Node) -> bool;
    fn set_goal(&mut self, search: Self::SearchNode);
    fn edge_count(&self, node: Self::Node) -> i32;
    fn edge(&self, node: Self::Node, index: i32) -> Self::Edge;
    fn edge_node(&self, edge: Self::Edge) -> Self::Node;
    /// None stops immediately; Some(false) merely rejects this edge.
    fn can_add(&mut self, parent: Self::Node, child: Self::Node) -> Option<bool>;
    fn edge_distance(&self, edge: Self::Edge) -> f32;
    fn search_distance(&self, search: Self::SearchNode) -> f32;
    /// False stops immediately so the caller can rethrow a saved exception.
    /// Faithful: the native insertion does not improve already-open nodes.
    fn add_open(&mut self, node: Self::Node, parent: Self::SearchNode, distance: f32) -> bool;
}

/// `IterateAlgorithm`. Only the loop counter and current handles are local;
/// a callback may change the remaining search state, or start another search.
pub fn iterate<H: SearchHost>(host: &mut H) {
    let mut iteration = 0i32;
    while !host.open_empty() {
        let limit = host.max_iterations();
        if !(limit < 0 || iteration < limit) { break; }
        let Some(search) = host.best_node() else { return; };
        let node = host.graph_node(search);
        if host.is_goal(node) {
            host.set_goal(search);
            break;
        }

        // Faithful: only this count is captured before CanAddNode. The edge
        // pointer remains the same; callbacks may edit its fields in place.
        let count = host.edge_count(node);
        for index in 0..count {
            let edge = host.edge(node, index);
            let child = host.edge_node(edge);
            let Some(allowed) = host.can_add(node, child) else { return; };
            if allowed {
                let distance = host.search_distance(search) + host.edge_distance(edge);
                let child = host.edge_node(edge);
                if !host.add_open(child, search, distance) { return; }
            }
        }
        // Deviation: original signed overflow is UB; fail through the engine
        // panic hook instead of wrapping or behaving differently by profile.
        iteration = iteration.checked_add(1).expect("AStar iteration count overflow");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_keeps_pointer_order_ties() {
        assert_eq!(best_node([(9, 4.0), (2, 1.0), (7, 1.0)]), Some(2));
        assert_eq!(best_node([(1, -0.0), (2, 0.0)]), Some(1));
        assert_eq!(best_node::<usize>([]), None);
    }

    #[test]
    fn best_preserves_nan_comparisons() {
        assert_eq!(best_node([(1, f32::NAN), (2, f32::NEG_INFINITY)]), Some(1));
        assert_eq!(best_node([(1, 2.0), (2, f32::NAN), (3, 1.0)]), Some(3));
        assert_eq!(best_node([(1, f32::INFINITY), (2, f32::INFINITY)]), Some(1));
    }

    #[test]
    fn root_cost_is_unchanged_even_for_nan_or_negative_zero() {
        for bits in [0x80000000, 0xff800000, 0x7fc01234, 0x7f801234, 0xffc05678] {
            assert_eq!(cost(f32::from_bits(bits), f32::NAN, None).to_bits(), bits);
        }
    }

    #[test]
    fn height_multiplies_entire_distance_in_both_directions() {
        assert_eq!(cost(7.0, 6.0, Some(2.0)), 35.0);
        assert_eq!(cost(7.0, 2.0, Some(6.0)), 35.0);
        assert_eq!(cost(-7.0, 2.0, Some(6.0)), -35.0);
        assert_eq!(cost(-0.0, 2.0, Some(2.0)).to_bits(), 0x80000000);
    }

    #[test]
    fn height_addition_rounds_before_multiplication() {
        // 1 + 2^-24 rounds to 1 in f32; retaining a double intermediate
        // would round the product up for this traveled distance.
        let d = f32::from_bits(0x3f800001);
        assert_eq!(cost(d, f32::from_bits(0x33800000), Some(0.0)).to_bits(), d.to_bits());
        assert!(cost(0.0, f32::INFINITY, Some(0.0)).is_nan());
    }

    #[test]
    fn heuristic_is_euclidean_in_three_dimensions() {
        assert_eq!(heuristic(Vector3f::new(1.0, 2.0, 3.0), Vector3f::new(3.0, 5.0, 9.0)), 7.0);
        assert_eq!(heuristic(Vector3f::splat(0.0), Vector3f::splat(0.0)).to_bits(), 0);
        assert!(heuristic(Vector3f::splat(0.0), Vector3f::new(f32::MAX, 0.0, 0.0)).is_infinite());
    }

    #[test]
    fn height_nan_payload_wins_over_distance_nan() {
        for distance in [0x7fc12345, 0xffc54321, 0x7f812345, 0xff854321] {
            for parent in [0x7fc12345, 0xffc54321, 0x7f812345, 0xff854321] {
                assert_eq!(cost(f32::from_bits(distance), 0.0, Some(f32::from_bits(parent))).to_bits(),
                           (parent & 0x7fffffff) | 0x00400000);
            }
        }
    }

    #[test]
    fn heuristic_keeps_native_y_then_x_then_z_nan_priority() {
        let a = f32::from_bits(0xff854321);
        let b = f32::from_bits(0x7fc12345);
        let goal = Vector3f::splat(0.0);
        assert_eq!(heuristic(Vector3f::new(a, b, b), goal).to_bits(), 0x7fc12345);
        assert_eq!(heuristic(Vector3f::new(a, 0.0, b), goal).to_bits(), 0xffc54321);
        assert_eq!(heuristic(Vector3f::new(0.0, a, b), goal).to_bits(), 0xffc54321);
        assert_eq!(heuristic(Vector3f::new(0.0, 0.0, a), goal).to_bits(), 0xffc54321);
        assert_eq!(heuristic(Vector3f::new(b, a, a), goal).to_bits(), 0xffc54321);
    }

    #[derive(Default)]
    struct Host {
        open: Vec<usize>,
        edges: Vec<(usize, f32)>,
        limit: i32,
        goal: Option<usize>,
        found: Option<usize>,
        distance: f32,
        visits: Vec<usize>,
        adds: Vec<(usize, usize, f32)>,
        calls: usize,
        mode: u8,
    }

    impl SearchHost for Host {
        type SearchNode = usize;
        type Node = usize;
        type Edge = usize;
        fn open_empty(&self) -> bool { self.open.is_empty() }
        fn max_iterations(&self) -> i32 { self.limit }
        fn best_node(&mut self) -> Option<usize> {
            if self.mode == 6 { return None; }
            let node = self.open.remove(0);
            self.visits.push(node);
            Some(node)
        }
        fn graph_node(&self, search: usize) -> usize { search }
        fn is_goal(&self, node: usize) -> bool { self.goal == Some(node) }
        fn set_goal(&mut self, search: usize) { self.found = Some(search); }
        fn edge_count(&self, _: usize) -> i32 { self.edges.len() as i32 }
        fn edge(&self, _: usize, index: i32) -> usize { index as usize }
        fn edge_node(&self, edge: usize) -> usize { self.edges[edge].0 }
        fn can_add(&mut self, _: usize, _: usize) -> Option<bool> {
            self.calls += 1;
            match self.mode {
                1 => return Some(false),
                2 => return None,
                3 => { self.edges[0] = (77, 3.5); self.distance = 8.0; }
                4 => self.limit = 1,
                5 if self.calls == 1 => self.edges.push((99, 10.0)),
                _ => (),
            }
            Some(true)
        }
        fn edge_distance(&self, edge: usize) -> f32 { self.edges[edge].1 }
        fn search_distance(&self, _: usize) -> f32 { self.distance }
        fn add_open(&mut self, node: usize, parent: usize, distance: f32) -> bool {
            self.adds.push((node, parent, distance));
            self.mode != 7
        }
    }

    fn host() -> Host {
        Host { open: vec![1, 2], edges: vec![(3, 2.0), (4, 4.0)], limit: -1, distance: 5.0, ..Host::default() }
    }

    #[test]
    fn iteration_limits_and_empty_search() {
        for (limit, visits) in [(0, 0), (1, 1), (2, 2), (-1, 2), (i32::MIN, 2)] {
            let mut h = host();
            h.limit = limit;
            iterate(&mut h);
            assert_eq!(h.visits.len(), visits);
        }
        iterate(&mut Host::default());
    }

    #[test]
    fn goal_is_checked_before_expanding_edges() {
        let mut h = host();
        h.goal = Some(1);
        iterate(&mut h);
        assert_eq!(h.found, Some(1));
        assert_eq!(h.open, [2]);
        assert_eq!(h.calls, 0);
    }

    #[test]
    fn accumulated_distance_and_edge_order() {
        let mut h = host();
        h.limit = 1;
        iterate(&mut h);
        assert_eq!(h.adds, [(3, 1, 7.0), (4, 1, 9.0)]);
    }

    #[test]
    fn rejected_edges_continue_but_saved_exceptions_stop() {
        let mut h = host();
        h.mode = 1;
        iterate(&mut h);
        assert_eq!(h.calls, 4);
        assert!(h.adds.is_empty());
        let mut h = host();
        h.mode = 2;
        iterate(&mut h);
        assert_eq!(h.calls, 1);
        assert_eq!(h.open, [2]);
        assert!(h.adds.is_empty());
    }

    #[test]
    fn live_edge_and_parent_distance_after_callback() {
        let mut h = host();
        h.limit = 1;
        h.mode = 3;
        iterate(&mut h);
        assert_eq!(h.adds, [(77, 1, 11.5), (4, 1, 12.0)]);
    }

    #[test]
    fn callback_changes_next_iteration_limit() {
        let mut h = host();
        h.mode = 4;
        iterate(&mut h);
        assert_eq!(h.visits, [1]);
        assert_eq!(h.calls, 2);
    }

    #[test]
    fn edge_count_is_captured_per_expansion() {
        let mut h = host();
        h.mode = 5;
        iterate(&mut h);
        assert_eq!(h.calls, 5); // two old edges on node 1; three on node 2
        assert_eq!(h.adds.last(), Some(&(99, 2, 15.0)));
    }

    #[test]
    fn allocation_failure_stops_without_later_host_calls() {
        let mut h = host();
        h.mode = 6;
        iterate(&mut h);
        assert_eq!(h.open, [1, 2]);
        assert_eq!(h.calls, 0);
        let mut h = host();
        h.mode = 7;
        iterate(&mut h);
        assert_eq!(h.calls, 1);
        assert_eq!(h.adds, [(3, 1, 7.0)]);
    }
}
