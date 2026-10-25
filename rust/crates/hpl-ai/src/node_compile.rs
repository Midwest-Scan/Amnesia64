//! Connection filtering/trimming from `cAINodeContainer::Compile`.
//!
//! Original audit: existing/duplicate edges survive, native unstable sorting
//! precedes both trims, and the maximum count wins over the minimum. The grid
//! iterator radius is captured by C++ once per source; all later limits and
//! positions are live. NaN distance passes rejection, NaN height fails acceptance.
//! Native sorting requires a strict weak order (mixed NaN/finite distances may
//! violate it); invalidating the native outer node iterator is also original UB.
//! No new deviations are introduced here. Grid UB handling is in node_container.

use hpl_math::{Vector3f, math};

/// Non-owning access to one source's native candidate iterator and edge vector.
/// Mutators return false on a saved native exception; visibility uses None for
/// that failure and Some(false) for an ordinary blocked path. Stop immediately
/// so C++ can rethrow only after Rust returns. Every callback is required.
pub trait CompileHost {
    type Node: Copy + Eq;
    fn next_candidate(&mut self) -> Option<Self::Node>;
    fn position(&self, node: Self::Node) -> Vector3f;
    fn max_distance(&self) -> f32;
    fn max_height(&self) -> f32;
    fn visible(&mut self, candidate: Self::Node) -> Option<bool>;
    fn add_edge(&mut self, candidate: Self::Node) -> bool;
    fn sort_edges(&mut self) -> bool;
    fn max_edges(&self) -> i32;
    fn edge_count(&self) -> usize;
    fn resize_edges(&mut self, count: usize) -> bool;
    fn edge_distance(&self, index: usize) -> f32;
    fn min_edges(&self) -> i32;
}

/// Compile one source; C++ retains the outer iteration and constructs its grid
/// iterator with the original `max_distance * 1.5f` radius before entering Rust.
pub fn compile_node<H: CompileHost>(host: &mut H, source: H::Node) {
    while let Some(candidate) = host.next_candidate() {
        if candidate == source { continue; }
        let distance = math::vector3_dist(host.position(source), host.position(candidate));
        if distance > host.max_distance() * 2.0 { continue; }
        let height = (host.position(source).y - host.position(candidate).y).abs();
        if height <= host.max_height() {
            let Some(visible) = host.visible(candidate) else { return; };
            // Faithful: native AddEdge reads positions again after visibility;
            // its callbacks may have changed positions, limits or existing edges.
            if visible && !host.add_edge(candidate) { return; }
        }
    }

    // Faithful: keep the platform's unstable std::sort, including tie ordering.
    if !host.sort_edges() { return; }
    let maximum = host.max_edges();
    // Faithful: MSVC's (int)size_t narrowing keeps the signed low 32 bits.
    if maximum > 0 && (host.edge_count() as i32) > maximum && !host.resize_edges(maximum as usize) { return; }

    let mut index = 0;
    while index < host.edge_count() {
        if host.edge_distance(index) > host.max_distance() && (index as i32) >= host.min_edges() {
            // Faithful: truncate the entire suffix at the first violation.
            host.resize_edges(index);
            break;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_container;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Edge { node: usize, distance: f32, squared: f32 }

    #[derive(Debug, PartialEq)]
    enum Event { Next(Option<usize>), Visible(usize), Add(usize), Sort, Resize(usize) }

    struct Graph {
        positions: Vec<Vector3f>,
        candidates: Vec<usize>,
        cursor: usize,
        edges: Vec<Edge>,
        max_distance: f32,
        max_height: f32,
        maximum: i32,
        minimum: i32,
        blocked: Vec<usize>,
        fail_visible: Option<usize>,
        fail_add: Option<usize>,
        fail_sort: bool,
        fail_resize: Option<usize>,
        on_visible: Option<fn(&mut Graph, usize)>,
        synthetic_count: Option<usize>,
        events: Vec<Event>,
    }

    impl Graph {
        fn new(positions: Vec<Vector3f>, candidates: Vec<usize>) -> Self {
            Self { positions, candidates, cursor: 0, edges: Vec::new(), max_distance: 100.0, max_height: 100.0,
                   maximum: 0, minimum: 0, blocked: Vec::new(), fail_visible: None, fail_add: None, fail_sort: false,
                   fail_resize: None, on_visible: None, synthetic_count: None, events: Vec::new() }
        }
        fn with_edges(distances: &[f32]) -> Self {
            let mut graph = Self::new(vec![Vector3f::splat(0.0)], Vec::new());
            graph.edges = distances.iter().enumerate().map(|(node, &distance)| Edge {
                node, distance, squared: distance * distance,
            }).collect();
            graph
        }
        fn visible_nodes(&self) -> Vec<usize> {
            self.events.iter().filter_map(|event| if let Event::Visible(node) = event { Some(*node) } else { None }).collect()
        }
        fn distances(&self) -> Vec<f32> { self.edges.iter().map(|edge| edge.distance).collect() }
    }

    impl CompileHost for Graph {
        type Node = usize;
        fn next_candidate(&mut self) -> Option<usize> {
            let next = self.candidates.get(self.cursor).copied();
            self.cursor += usize::from(next.is_some());
            self.events.push(Event::Next(next));
            next
        }
        fn position(&self, node: usize) -> Vector3f { self.positions[node] }
        fn max_distance(&self) -> f32 { self.max_distance }
        fn max_height(&self) -> f32 { self.max_height }
        fn visible(&mut self, candidate: usize) -> Option<bool> {
            self.events.push(Event::Visible(candidate));
            if let Some(change) = self.on_visible { change(self, candidate); }
            if self.fail_visible == Some(candidate) { None } else { Some(!self.blocked.contains(&candidate)) }
        }
        fn add_edge(&mut self, candidate: usize) -> bool {
            self.events.push(Event::Add(candidate));
            if self.fail_add == Some(candidate) { return false; }
            let (distance, squared) = node_container::edge_distances(self.positions[0], self.positions[candidate]);
            self.edges.push(Edge { node: candidate, distance, squared });
            true
        }
        fn sort_edges(&mut self) -> bool {
            self.events.push(Event::Sort);
            if self.fail_sort { return false; }
            // This mock tests delegation and phase ordering. Native tie ordering
            // belongs to the C++ differential test; no NaN edges enter this sort.
            self.edges.sort_unstable_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
            true
        }
        fn max_edges(&self) -> i32 { self.maximum }
        fn edge_count(&self) -> usize { self.synthetic_count.unwrap_or(self.edges.len()) }
        fn resize_edges(&mut self, count: usize) -> bool {
            self.events.push(Event::Resize(count));
            if self.fail_resize == Some(count) { return false; }
            self.synthetic_count = None;
            self.edges.truncate(count);
            true
        }
        fn edge_distance(&self, index: usize) -> f32 { self.edges[index].distance }
        fn min_edges(&self) -> i32 { self.minimum }
    }

    #[test]
    fn distance_rejection_is_strict_and_height_acceptance_is_inclusive() {
        let mut graph = Graph::new(vec![Vector3f::splat(0.0), Vector3f::new(f32::from_bits(4.0_f32.to_bits() - 1), 0.0, 0.0),
            Vector3f::new(4.0, 0.0, 0.0), Vector3f::new(f32::from_bits(4.0_f32.to_bits() + 1), 0.0, 0.0)], vec![1, 2, 3]);
        graph.max_distance = 2.0;
        graph.minimum = 9;
        compile_node(&mut graph, 0);
        assert_eq!(graph.visible_nodes(), [1, 2]);
        assert_eq!(graph.edges.len(), 2);

        let mut graph = Graph::new(vec![Vector3f::splat(0.0), Vector3f::new(0.0, f32::from_bits(1.0_f32.to_bits() - 1), 0.0),
            Vector3f::new(0.0, -1.0, 0.0), Vector3f::new(0.0, f32::from_bits(1.0_f32.to_bits() + 1), 0.0)], vec![1, 2, 3]);
        graph.max_height = 1.0;
        compile_node(&mut graph, 0);
        assert_eq!(graph.visible_nodes(), [1, 2]);
    }

    #[test]
    fn nan_distance_passes_rejection_but_nan_height_fails_acceptance() {
        let mut graph = Graph::new(vec![Vector3f::splat(0.0), Vector3f::new(f32::NAN, 0.0, 0.0),
            Vector3f::new(0.0, f32::NAN, 0.0)], vec![1, 2]);
        graph.blocked.push(1); // Avoid invoking native-style sorting on NaN edges.
        compile_node(&mut graph, 0);
        assert_eq!(graph.visible_nodes(), [1]);
        assert!(graph.edges.is_empty());

        let mut graph = Graph::new(vec![Vector3f::splat(0.0), Vector3f::new(1.0, 0.0, 0.0)], vec![1]);
        graph.max_height = f32::NAN;
        compile_node(&mut graph, 0);
        assert!(graph.visible_nodes().is_empty());
    }

    #[test]
    fn final_distance_trim_retains_equality_and_removes_the_entire_suffix() {
        let above = f32::from_bits(2.0_f32.to_bits() + 1);
        let mut graph = Graph::with_edges(&[3.0, above, 2.0, 1.0]);
        graph.max_distance = 2.0;
        compile_node(&mut graph, 0);
        assert_eq!(graph.distances(), [1.0, 2.0]);
        assert_eq!(graph.events, [Event::Next(None), Event::Sort, Event::Resize(2)]);
    }

    #[test]
    fn positive_maximum_wins_over_minimum_and_nonpositive_caps_are_unlimited() {
        let mut graph = Graph::with_edges(&[4.0, 1.0, 3.0, 2.0]);
        graph.max_distance = 1.5;
        graph.maximum = 2;
        graph.minimum = 4;
        compile_node(&mut graph, 0);
        assert_eq!(graph.distances(), [1.0, 2.0]);
        assert_eq!(graph.events, [Event::Next(None), Event::Sort, Event::Resize(2)]);
        for maximum in [0, -1, i32::MIN] {
            let mut graph = Graph::with_edges(&[4.0, 1.0, 3.0, 2.0]);
            graph.maximum = maximum;
            compile_node(&mut graph, 0);
            assert_eq!(graph.distances(), [1.0, 2.0, 3.0, 4.0]);
        }
    }

    #[test]
    fn minimum_keeps_a_prefix_and_negative_minimum_allows_removing_everything() {
        for (minimum, expected) in [(2, vec![2.0, 3.0]), (0, vec![]), (-2, vec![]), (i32::MIN, vec![])] {
            let mut graph = Graph::with_edges(&[4.0, 2.0, 3.0]);
            graph.max_distance = 1.0;
            graph.minimum = minimum;
            compile_node(&mut graph, 0);
            assert_eq!(graph.distances(), expected);
        }
    }

    #[test]
    fn self_candidates_are_skipped_but_existing_self_edges_and_duplicates_survive() {
        let mut graph = Graph::new(vec![Vector3f::splat(0.0), Vector3f::new(1.0, 0.0, 0.0),
            Vector3f::new(2.0, 0.0, 0.0)], vec![0, 2, 1, 2]);
        graph.edges = vec![Edge { node: 0, distance: 0.0, squared: 0.0 }, Edge { node: 1, distance: 0.25, squared: 0.0625 }];
        compile_node(&mut graph, 0);
        assert_eq!(graph.visible_nodes(), [2, 1, 2]);
        assert_eq!(graph.edges.iter().map(|edge| edge.node).collect::<Vec<_>>(), [0, 1, 1, 2, 2]);
        assert_eq!(graph.distances(), [0.0, 0.25, 1.0, 2.0, 2.0]);
    }

    #[test]
    fn visibility_changes_later_filters_trims_and_the_metrics_of_the_current_edge() {
        let mut graph = Graph::new(vec![Vector3f::splat(0.0), Vector3f::new(1.0, 0.0, 0.0),
            Vector3f::new(16.0, 0.0, 0.0), Vector3f::new(6.0, 2.0, 0.0), Vector3f::new(9.0, 0.0, 0.0)], vec![1, 2, 3, 4]);
        graph.on_visible = Some(|graph, candidate| {
            if candidate != 1 { return; }
            graph.positions[0].x = 4.0;
            graph.positions[1].x = 8.0;
            graph.max_distance = 5.0;
            graph.max_height = 1.0;
            graph.maximum = 2;
            graph.minimum = 2;
            graph.edges.push(Edge { node: 0, distance: 0.5, squared: 0.25 });
        });
        compile_node(&mut graph, 0);
        assert_eq!(graph.visible_nodes(), [1, 4]);
        assert_eq!(graph.distances(), [0.5, 4.0]);
        assert_eq!(graph.edges[1], Edge { node: 1, distance: 4.0, squared: 16.0 });
    }

    #[test]
    fn blocked_visibility_continues_but_saved_visibility_failure_keeps_unsorted_partial_edges() {
        let positions = vec![Vector3f::splat(0.0), Vector3f::new(1.0, 0.0, 0.0), Vector3f::new(2.0, 0.0, 0.0),
            Vector3f::new(3.0, 0.0, 0.0)];
        let mut graph = Graph::new(positions.clone(), vec![1, 2, 3]);
        graph.blocked.push(2);
        compile_node(&mut graph, 0);
        assert_eq!(graph.visible_nodes(), [1, 2, 3]);
        assert_eq!(graph.distances(), [1.0, 3.0]);

        let mut graph = Graph::new(positions, vec![1, 2, 3]);
        graph.edges.push(Edge { node: 3, distance: 9.0, squared: 81.0 });
        graph.fail_visible = Some(2);
        compile_node(&mut graph, 0);
        assert_eq!(graph.distances(), [9.0, 1.0]);
        assert_eq!(graph.events, [Event::Next(Some(1)), Event::Visible(1), Event::Add(1), Event::Next(Some(2)), Event::Visible(2)]);
    }

    #[test]
    fn failed_append_stops_before_later_candidates_or_sorting() {
        let mut graph = Graph::new(vec![Vector3f::splat(0.0), Vector3f::new(1.0, 0.0, 0.0),
            Vector3f::new(2.0, 0.0, 0.0), Vector3f::new(3.0, 0.0, 0.0)], vec![1, 2, 3]);
        graph.fail_add = Some(2);
        compile_node(&mut graph, 0);
        assert_eq!(graph.distances(), [1.0]);
        assert_eq!(graph.events, [Event::Next(Some(1)), Event::Visible(1), Event::Add(1), Event::Next(Some(2)),
                                 Event::Visible(2), Event::Add(2)]);
    }

    #[test]
    fn failed_sort_and_resize_stop_without_running_later_phases() {
        let mut graph = Graph::with_edges(&[4.0, 1.0, 3.0, 2.0]);
        graph.fail_sort = true;
        graph.maximum = 2;
        graph.max_distance = 0.0;
        compile_node(&mut graph, 0);
        assert_eq!(graph.distances(), [4.0, 1.0, 3.0, 2.0]);
        assert_eq!(graph.events, [Event::Next(None), Event::Sort]);

        let mut graph = Graph::with_edges(&[4.0, 1.0, 3.0, 2.0]);
        graph.maximum = 2;
        graph.max_distance = 0.0;
        graph.fail_resize = Some(2);
        compile_node(&mut graph, 0);
        assert_eq!(graph.distances(), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(graph.events, [Event::Next(None), Event::Sort, Event::Resize(2)]);

        let mut graph = Graph::with_edges(&[2.0, 1.0]);
        graph.max_distance = 0.0;
        graph.fail_resize = Some(0);
        compile_node(&mut graph, 0);
        assert_eq!(graph.distances(), [1.0, 2.0]);
        assert_eq!(graph.events, [Event::Next(None), Event::Sort, Event::Resize(0)]);
    }

    #[test]
    fn huge_native_counts_use_signed_low_32_bits_before_the_maximum_cap() {
        // A synthetic host exposes huge native sizes without huge allocations.
        // The first distance violation always terminates traversal at index 0.
        let mut graph = Graph::with_edges(&[9.0, 8.0, 7.0]);
        graph.synthetic_count = Some(i32::MAX as usize + 1);
        graph.maximum = 2;
        graph.max_distance = 0.0;
        compile_node(&mut graph, 0);
        assert_eq!(graph.events, [Event::Next(None), Event::Sort, Event::Resize(0)]);
        if usize::BITS > 32 {
            let mut graph = Graph::with_edges(&[9.0, 8.0, 7.0]);
            graph.synthetic_count = Some((u64::from(u32::MAX) + 4) as usize);
            graph.maximum = 2;
            graph.max_distance = 0.0;
            compile_node(&mut graph, 0);
            assert_eq!(graph.events, [Event::Next(None), Event::Sort, Event::Resize(2), Event::Resize(0)]);
        }
    }
}
