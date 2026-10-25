//! `cAStarHandler::GetPath` setup and result traversal.
//!
//! Original audit: direct success clears only the goal pointer, preserving old
//! search sets, saved goal position and the caller's list. Fallback deletes
//! closed before open, then clears goal candidates. Height is captured at entry;
//! distance is captured after reset and live goal assignment. Endpoint references
//! remain live, but each native iterator captures its origin at construction.
//! Start distances survive visibility callbacks unchanged. Output appends the
//! captured goal-to-start chain, reading parent links after each native append.
//! Allocations, set ordering, iterators and physics remain native. Invalidated
//! pointers/iterators, uninitialized search fields and cyclic parent chains are
//! original undefined/nonterminating paths; this slice adds no deviations.

use hpl_math::{Vector3f, math};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint { Start = 0, Goal = 1 }

/// Live non-owning access to native GetPath state. False from a mutating callback
/// requests immediate return to rethrow its saved C++ exception. Visibility uses
/// None for failure, distinct from a blocked path. Every callback is required.
pub trait PathHost {
    type Node: Copy;
    type Search: Copy;
    fn max_height(&self) -> f32;
    fn endpoint(&self, endpoint: Endpoint) -> Vector3f;
    fn direct_visible(&mut self) -> Option<bool>;
    fn clear_goal(&mut self);
    fn reset_search(&mut self) -> bool;
    fn set_goal_position(&mut self, goal: Vector3f);
    fn max_distance(&self) -> f32;
    fn begin_candidates(&mut self, endpoint: Endpoint, radius: f32) -> bool;
    fn next_candidate(&mut self, endpoint: Endpoint) -> Option<Self::Node>;
    fn position(&self, node: Self::Node) -> Vector3f;
    fn visible(&mut self, endpoint: Endpoint, node: Self::Node) -> Option<bool>;
    fn add_start(&mut self, node: Self::Node, distance: f32) -> bool;
    fn add_goal(&mut self, node: Self::Node) -> bool;
    fn iterate(&mut self) -> bool;
    fn goal_node(&self) -> Option<Self::Search>;
    fn has_output(&self) -> bool;
    /// Append the native search node's graph field by reference; allocation may
    /// invoke callbacks. No Rust snapshot of that field crosses the append.
    fn append_path(&mut self, search: Self::Search) -> bool;
    fn parent(&self, search: Self::Search) -> Option<Self::Search>;
}

pub fn get_path<H: PathHost>(host: &mut H) -> bool {
    let max_height = host.max_height() * 1.5;
    let height = (host.endpoint(Endpoint::Start).y - host.endpoint(Endpoint::Goal).y).abs();
    if height <= max_height {
        let Some(visible) = host.direct_visible() else { return false; };
        if visible {
            host.clear_goal();
            return true;
        }
    }

    if !host.reset_search() { return false; }
    host.set_goal_position(host.endpoint(Endpoint::Goal));
    let max_distance = host.max_distance() * 2.0;

    for endpoint in [Endpoint::Start, Endpoint::Goal] {
        // Faithful: goal iterator is constructed only after all start callbacks;
        // its origin is live but the radius and height limit remain captured.
        if !host.begin_candidates(endpoint, max_distance) { return false; }
        while let Some(node) = host.next_candidate(endpoint) {
            let height = (host.endpoint(endpoint).y - host.position(node).y).abs();
            let distance = math::vector3_dist(host.endpoint(endpoint), host.position(node));
            if distance < max_distance && height <= max_height {
                let Some(visible) = host.visible(endpoint, node) else { return false; };
                if visible {
                    // Faithful: unlike Compile/AddEdge, start insertion keeps
                    // the distance computed before the visibility callback.
                    let inserted = match endpoint {
                        Endpoint::Start => host.add_start(node, distance),
                        Endpoint::Goal => host.add_goal(node),
                    };
                    if !inserted { return false; }
                }
            }
        }
    }

    if !host.iterate() { return false; }
    let Some(mut search) = host.goal_node() else { return false; };
    if host.has_output() {
        loop {
            if !host.append_path(search) { return false; }
            // Faithful: append may edit the chain; read the next link afterward.
            let Some(parent) = host.parent(search) else { break; };
            search = parent;
        }
    }
    // Faithful: a nested direct search during append may clear the live goal;
    // the captured successful result still returns true.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Event {
        Direct, ClearGoal, Reset, Save([u32; 3]), Begin(Endpoint, [u32; 3], u32),
        Visible(Endpoint, usize), Start(usize, u32), Goal(usize), Iterate, Append(usize),
    }
    #[derive(Clone, Copy, PartialEq)]
    enum Failure { None, Reset, Begin(Endpoint), Visible(Endpoint, usize), Start(usize), Goal(usize), Iterate, Append(usize) }
    fn bits(v: Vector3f) -> [u32; 3] { [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()] }

    struct Fixture {
        endpoints: [Vector3f; 2],
        positions: Vec<Vector3f>,
        candidates: [Vec<usize>; 2],
        cursor: [usize; 2],
        height: f32,
        distance: f32,
        direct: Option<bool>,
        blocked: Vec<(Endpoint, usize)>,
        failure: Failure,
        open: Vec<(usize, f32)>,
        goals: Vec<usize>,
        saved_goal: Vector3f,
        goal: Option<usize>,
        iteration_goal: Option<usize>,
        parents: Vec<Option<usize>>,
        output: Vec<usize>,
        output_enabled: bool,
        goal_reads: Cell<usize>,
        parent_reads: Cell<usize>,
        on_direct: Option<fn(&mut Fixture)>,
        on_reset: Option<fn(&mut Fixture)>,
        on_visible: Option<fn(&mut Fixture, Endpoint, usize)>,
        on_append: Option<fn(&mut Fixture, usize)>,
        events: Vec<Event>,
    }
    impl Fixture {
        fn new() -> Self {
            Self { endpoints: [Vector3f::splat(0.0); 2], positions: vec![Vector3f::new(1.0, 0.0, 0.0)],
                candidates: [Vec::new(), Vec::new()], cursor: [0; 2], height: 1.0, distance: 2.0, direct: Some(false),
                blocked: Vec::new(), failure: Failure::None, open: vec![(9, 9.0)], goals: vec![8],
                saved_goal: Vector3f::splat(99.0), goal: Some(2), iteration_goal: None,
                parents: vec![None, Some(0), Some(1)], output: vec![99], output_enabled: true,
                goal_reads: Cell::new(0), parent_reads: Cell::new(0), on_direct: None, on_reset: None,
                on_visible: None, on_append: None, events: Vec::new() }
        }
        fn one_candidate() -> Self {
            let mut fixture = Self::new();
            fixture.candidates = [vec![0], vec![0]];
            fixture.iteration_goal = Some(2);
            fixture
        }
    }
    impl PathHost for Fixture {
        type Node = usize;
        type Search = usize;
        fn max_height(&self) -> f32 { self.height }
        fn endpoint(&self, endpoint: Endpoint) -> Vector3f { self.endpoints[endpoint as usize] }
        fn direct_visible(&mut self) -> Option<bool> {
            self.events.push(Event::Direct);
            if let Some(action) = self.on_direct { action(self); }
            self.direct
        }
        fn clear_goal(&mut self) { self.events.push(Event::ClearGoal); self.goal = None; }
        fn reset_search(&mut self) -> bool {
            self.events.push(Event::Reset);
            if let Some(action) = self.on_reset { action(self); }
            if self.failure == Failure::Reset { return false; }
            self.open.clear(); self.goals.clear(); self.goal = None;
            true
        }
        fn set_goal_position(&mut self, goal: Vector3f) { self.events.push(Event::Save(bits(goal))); self.saved_goal = goal; }
        fn max_distance(&self) -> f32 { self.distance }
        fn begin_candidates(&mut self, endpoint: Endpoint, radius: f32) -> bool {
            self.events.push(Event::Begin(endpoint, bits(self.endpoint(endpoint)), radius.to_bits()));
            self.failure != Failure::Begin(endpoint)
        }
        fn next_candidate(&mut self, endpoint: Endpoint) -> Option<usize> {
            let side = endpoint as usize;
            let node = self.candidates[side].get(self.cursor[side]).copied();
            self.cursor[side] += usize::from(node.is_some());
            node
        }
        fn position(&self, node: usize) -> Vector3f { self.positions[node] }
        fn visible(&mut self, endpoint: Endpoint, node: usize) -> Option<bool> {
            self.events.push(Event::Visible(endpoint, node));
            if let Some(action) = self.on_visible { action(self, endpoint, node); }
            if self.failure == Failure::Visible(endpoint, node) { None } else { Some(!self.blocked.contains(&(endpoint, node))) }
        }
        fn add_start(&mut self, node: usize, distance: f32) -> bool {
            self.events.push(Event::Start(node, distance.to_bits()));
            if self.failure == Failure::Start(node) { return false; }
            self.open.push((node, distance)); true
        }
        fn add_goal(&mut self, node: usize) -> bool {
            self.events.push(Event::Goal(node));
            if self.failure == Failure::Goal(node) { return false; }
            self.goals.push(node); true
        }
        fn iterate(&mut self) -> bool {
            self.events.push(Event::Iterate);
            if self.failure == Failure::Iterate { return false; }
            self.goal = self.iteration_goal; true
        }
        fn goal_node(&self) -> Option<usize> { self.goal_reads.set(self.goal_reads.get() + 1); self.goal }
        fn has_output(&self) -> bool { self.output_enabled }
        fn append_path(&mut self, search: usize) -> bool {
            self.events.push(Event::Append(search));
            if let Some(action) = self.on_append { action(self, search); }
            if self.failure == Failure::Append(search) { return false; }
            self.output.push(search); true
        }
        fn parent(&self, search: usize) -> Option<usize> { self.parent_reads.set(self.parent_reads.get() + 1); self.parents[search] }
    }

    #[test]
    fn direct_success_only_clears_goal_and_retains_old_search_and_output() {
        let mut fixture = Fixture::one_candidate();
        fixture.direct = Some(true);
        assert!(get_path(&mut fixture));
        assert_eq!(fixture.events, [Event::Direct, Event::ClearGoal]);
        assert_eq!(fixture.open, [(9, 9.0)]);
        assert_eq!(fixture.goals, [8]);
        assert_eq!(fixture.saved_goal, Vector3f::splat(99.0));
        assert_eq!(fixture.output, [99]);
        assert_eq!(fixture.goal, None);
        assert_eq!(fixture.goal_reads.get(), 0);
    }

    #[test]
    fn direct_height_accepts_equality_but_rejects_above_and_nan_without_visibility() {
        for (height, visible) in [(1.5, true), (f32::from_bits(1.5_f32.to_bits() + 1), false), (f32::NAN, false)] {
            let mut fixture = Fixture::new();
            fixture.endpoints[1].y = height;
            fixture.direct = Some(true);
            assert_eq!(get_path(&mut fixture), visible);
            assert_eq!(fixture.events.contains(&Event::Direct), visible);
        }
    }

    #[test]
    fn height_is_frozen_at_entry_but_distance_and_saved_goal_follow_reset() {
        let mut fixture = Fixture::new();
        fixture.candidates[0] = vec![0, 1];
        fixture.positions = vec![Vector3f::new(1.0, 1.5, 0.0), Vector3f::new(2.0, 1.6, 0.0)];
        fixture.on_direct = Some(|fixture| { fixture.height = 100.0; fixture.distance = 10.0; fixture.endpoints[1].x = 8.0; });
        fixture.on_reset = Some(|fixture| { fixture.distance = 3.0; fixture.endpoints[1].x = 9.0; });
        assert!(!get_path(&mut fixture));
        assert_eq!(fixture.saved_goal, Vector3f::new(9.0, 0.0, 0.0));
        assert_eq!(fixture.open.iter().map(|&(node, _)| node).collect::<Vec<_>>(), [0]);
        let begins: Vec<_> = fixture.events.iter().filter_map(|event| if let Event::Begin(side, _, radius) = event {
            Some((*side, *radius))
        } else { None }).collect();
        assert_eq!(begins, [(Endpoint::Start, 6.0_f32.to_bits()), (Endpoint::Goal, 6.0_f32.to_bits())]);
    }

    #[test]
    fn callbacks_change_live_origins_and_later_filters_but_not_radius_or_inserted_distance() {
        let mut fixture = Fixture::new();
        fixture.positions = vec![Vector3f::new(1.0, 0.0, 0.0), Vector3f::new(12.0, 0.0, 0.0)];
        fixture.candidates = [vec![0, 1], vec![1]];
        fixture.on_visible = Some(|fixture, endpoint, node| {
            if endpoint == Endpoint::Start && node == 0 {
                fixture.positions[0].x = 100.0;
                fixture.endpoints[0].x = 10.0;
                fixture.endpoints[1].x = 11.0;
                fixture.distance = 0.0;
            }
        });
        assert!(!get_path(&mut fixture));
        assert_eq!(fixture.open, [(0, 1.0), (1, 2.0)]);
        assert_eq!(fixture.goals, [1]);
        assert_eq!(fixture.saved_goal, Vector3f::splat(0.0));
        assert!(fixture.events.contains(&Event::Begin(Endpoint::Goal, bits(Vector3f::new(11.0, 0.0, 0.0)), 4.0_f32.to_bits())));
        let last_start = fixture.events.iter().position(|event| *event == Event::Start(1, 2.0_f32.to_bits())).unwrap();
        let goal_begin = fixture.events.iter().position(|event| matches!(event, Event::Begin(Endpoint::Goal, _, _))).unwrap();
        assert!(last_start < goal_begin);
    }

    #[test]
    fn candidates_use_strict_distance_and_inclusive_height_boundaries() {
        let mut fixture = Fixture::new();
        fixture.positions = vec![Vector3f::new(4.0, 0.0, 0.0), Vector3f::new(f32::from_bits(4.0_f32.to_bits() - 1), 0.0, 0.0),
            Vector3f::new(f32::from_bits(4.0_f32.to_bits() + 1), 0.0, 0.0), Vector3f::new(0.0, 1.5, 0.0),
            Vector3f::new(0.0, f32::from_bits(1.5_f32.to_bits() + 1), 0.0)];
        fixture.candidates[0] = vec![0, 1, 2, 3, 4];
        assert!(!get_path(&mut fixture));
        assert_eq!(fixture.open.iter().map(|&(node, _)| node).collect::<Vec<_>>(), [1, 3]);
    }

    #[test]
    fn nan_candidate_metrics_reject_before_visibility_and_nonpositive_distances_add_nothing() {
        let mut fixture = Fixture::new();
        fixture.positions = vec![Vector3f::new(f32::NAN, 0.0, 0.0), Vector3f::new(0.0, f32::NAN, 0.0)];
        fixture.candidates = [vec![0, 1], vec![0, 1]];
        assert!(!get_path(&mut fixture));
        assert!(!fixture.events.iter().any(|event| matches!(event, Event::Visible(_, _))));
        for distance in [0.0, -0.0, -1.0, f32::NAN] {
            let mut fixture = Fixture::one_candidate();
            fixture.distance = distance;
            fixture.iteration_goal = None;
            assert!(!get_path(&mut fixture));
            assert!(fixture.open.is_empty() && fixture.goals.is_empty());
        }
    }

    #[test]
    fn candidate_distance_rounds_f32_sums_before_the_root_and_strict_comparison() {
        let mut fixture = Fixture::new();
        fixture.positions[0] = Vector3f::new(f32::from_bits(0x3f7ffff1), f32::from_bits(0x3a800000), f32::from_bits(0x3a800000));
        fixture.distance = f32::from_bits(0x3f000001);
        fixture.candidates[0] = vec![0];
        assert!(!get_path(&mut fixture));
        assert_eq!(fixture.open, [(0, 1.0)]);
        // A double squared sum rounds the root up to the cutoff, rejecting it.
        let p = fixture.positions[0];
        let double = ((p.x as f64).powi(2) + (p.y as f64).powi(2) + (p.z as f64).powi(2)).sqrt() as f32;
        assert_eq!(double.to_bits(), 0x3f800001);
    }

    #[test]
    fn blocked_candidates_continue_and_empty_sets_still_run_search() {
        let mut fixture = Fixture::one_candidate();
        fixture.blocked = vec![(Endpoint::Start, 0), (Endpoint::Goal, 0)];
        fixture.iteration_goal = None;
        assert!(!get_path(&mut fixture));
        assert!(fixture.open.is_empty() && fixture.goals.is_empty());
        assert_eq!(fixture.events.last(), Some(&Event::Iterate));
        let mut fixture = Fixture::new();
        assert!(!get_path(&mut fixture));
        assert_eq!(fixture.events.last(), Some(&Event::Iterate));
        assert_eq!(fixture.output, [99]);
    }

    #[test]
    fn graph_success_appends_reverse_parent_chain_and_null_output_skips_traversal() {
        let mut fixture = Fixture::one_candidate();
        assert!(get_path(&mut fixture));
        assert_eq!(fixture.output, [99, 2, 1, 0]);
        assert_eq!(fixture.parent_reads.get(), 3);
        let mut fixture = Fixture::new();
        fixture.output_enabled = false;
        fixture.iteration_goal = Some(999); // An opaque success handle is not dereferenced without output.
        assert!(get_path(&mut fixture));
        assert_eq!(fixture.parent_reads.get(), 0);
        assert_eq!(fixture.output, [99]);
    }

    #[test]
    fn output_rereads_parent_after_append_and_keeps_captured_success_when_goal_is_cleared() {
        let mut fixture = Fixture::one_candidate();
        fixture.on_append = Some(|fixture, search| {
            if search == 2 { fixture.parents[2] = Some(0); fixture.goal = None; }
        });
        assert!(get_path(&mut fixture));
        assert_eq!(fixture.output, [99, 2, 0]);
        assert_eq!(fixture.goal, None);
        assert_eq!(fixture.goal_reads.get(), 1);
    }

    #[test]
    fn direct_visibility_failure_stops_before_reset_without_clearing_prior_state() {
        let mut fixture = Fixture::one_candidate();
        fixture.direct = None;
        assert!(!get_path(&mut fixture));
        assert_eq!(fixture.events, [Event::Direct]);
        assert_eq!(fixture.open, [(9, 9.0)]);
        assert_eq!(fixture.goal, Some(2));
        assert_eq!(fixture.output, [99]);
    }

    #[test]
    fn reset_iterator_visibility_and_iteration_failures_stop_at_the_failed_operation() {
        for (failure, last) in [(Failure::Reset, Event::Reset),
            (Failure::Begin(Endpoint::Start), Event::Begin(Endpoint::Start, [0; 3], 4.0_f32.to_bits())),
            (Failure::Begin(Endpoint::Goal), Event::Begin(Endpoint::Goal, [0; 3], 4.0_f32.to_bits())),
            (Failure::Visible(Endpoint::Start, 0), Event::Visible(Endpoint::Start, 0)),
            (Failure::Visible(Endpoint::Goal, 0), Event::Visible(Endpoint::Goal, 0)),
            (Failure::Iterate, Event::Iterate)] {
            let mut fixture = Fixture::one_candidate();
            fixture.failure = failure;
            assert!(!get_path(&mut fixture));
            assert_eq!(fixture.events.last(), Some(&last));
            assert_eq!(fixture.output, [99]);
            assert_eq!(fixture.goal_reads.get(), 0);
        }
    }

    #[test]
    fn failed_insertions_and_output_keep_the_completed_prefix_without_rollback() {
        for failure in [Failure::Start(1), Failure::Goal(1)] {
            let mut fixture = Fixture::new();
            fixture.positions.push(Vector3f::new(2.0, 0.0, 0.0));
            fixture.candidates = [vec![0, 1], vec![0, 1]];
            fixture.failure = failure;
            assert!(!get_path(&mut fixture));
            if failure == Failure::Start(1) { assert_eq!(fixture.open, [(0, 1.0)]); }
            else { assert_eq!(fixture.goals, [0]); }
            assert!(!fixture.events.contains(&Event::Iterate));
        }
        let mut fixture = Fixture::one_candidate();
        fixture.failure = Failure::Append(1);
        assert!(!get_path(&mut fixture));
        assert_eq!(fixture.output, [99, 2]);
        assert_eq!(fixture.goal, Some(2));
        assert_eq!(fixture.parent_reads.get(), 1);
    }
}
