//! AStar GetPath setup/output; native references, containers and physics stay C++.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_ai::astar_path::{self, Endpoint, PathHost};
use hpl_math::Vector3f;
use std::ffi::c_void;

/// All callbacks are required and must not throw across Rust. Endpoint selectors
/// are int32_t (0=start, 1=goal). Null handles mean exhaustion/absence. Visibility
/// separates its result from callback success; false success saves an exception
/// for C++ to rethrow after Rust returns. Native endpoint references remain live.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAStarPathHost {
    pub ctx: *mut c_void,
    pub max_height: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub endpoint: unsafe extern "C" fn(ctx: *mut c_void, endpoint: i32) -> HplVector3f,
    pub direct_visible: unsafe extern "C" fn(ctx: *mut c_void, visible: *mut bool) -> bool,
    pub clear_goal: unsafe extern "C" fn(ctx: *mut c_void),
    pub reset_search: unsafe extern "C" fn(ctx: *mut c_void) -> bool,
    pub set_goal_position: unsafe extern "C" fn(ctx: *mut c_void, goal: HplVector3f),
    pub max_distance: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub begin_candidates: unsafe extern "C" fn(ctx: *mut c_void, endpoint: i32, radius: f32) -> bool,
    pub next_candidate: unsafe extern "C" fn(ctx: *mut c_void, endpoint: i32) -> *mut c_void,
    pub position: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> HplVector3f,
    pub visible: unsafe extern "C" fn(ctx: *mut c_void, endpoint: i32, node: *mut c_void, visible: *mut bool) -> bool,
    pub add_start: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void, distance: f32) -> bool,
    pub add_goal: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> bool,
    pub iterate: unsafe extern "C" fn(ctx: *mut c_void) -> bool,
    pub goal_node: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    pub has_output: unsafe extern "C" fn(ctx: *mut c_void) -> bool,
    pub append_path: unsafe extern "C" fn(ctx: *mut c_void, search: *mut c_void) -> bool,
    pub parent: unsafe extern "C" fn(ctx: *mut c_void, search: *mut c_void) -> *mut c_void,
}

fn handle(pointer: *mut c_void) -> Option<*mut c_void> { if pointer.is_null() { None } else { Some(pointer) } }

struct Host(HplAStarPathHost);

impl PathHost for Host {
    type Node = *mut c_void;
    type Search = *mut c_void;
    fn max_height(&self) -> f32 { unsafe { (self.0.max_height)(self.0.ctx) } }
    fn endpoint(&self, endpoint: Endpoint) -> Vector3f { unsafe { (self.0.endpoint)(self.0.ctx, endpoint as i32).to_rs() } }
    fn direct_visible(&mut self) -> Option<bool> {
        let mut visible = false;
        if unsafe { (self.0.direct_visible)(self.0.ctx, &mut visible) } { Some(visible) } else { None }
    }
    fn clear_goal(&mut self) { unsafe { (self.0.clear_goal)(self.0.ctx) } }
    fn reset_search(&mut self) -> bool { unsafe { (self.0.reset_search)(self.0.ctx) } }
    fn set_goal_position(&mut self, goal: Vector3f) { unsafe { (self.0.set_goal_position)(self.0.ctx, HplVector3f::from_rs(goal)) } }
    fn max_distance(&self) -> f32 { unsafe { (self.0.max_distance)(self.0.ctx) } }
    fn begin_candidates(&mut self, endpoint: Endpoint, radius: f32) -> bool {
        unsafe { (self.0.begin_candidates)(self.0.ctx, endpoint as i32, radius) }
    }
    fn next_candidate(&mut self, endpoint: Endpoint) -> Option<Self::Node> {
        handle(unsafe { (self.0.next_candidate)(self.0.ctx, endpoint as i32) })
    }
    fn position(&self, node: Self::Node) -> Vector3f { unsafe { (self.0.position)(self.0.ctx, node).to_rs() } }
    fn visible(&mut self, endpoint: Endpoint, node: Self::Node) -> Option<bool> {
        let mut visible = false;
        if unsafe { (self.0.visible)(self.0.ctx, endpoint as i32, node, &mut visible) } { Some(visible) } else { None }
    }
    fn add_start(&mut self, node: Self::Node, distance: f32) -> bool { unsafe { (self.0.add_start)(self.0.ctx, node, distance) } }
    fn add_goal(&mut self, node: Self::Node) -> bool { unsafe { (self.0.add_goal)(self.0.ctx, node) } }
    fn iterate(&mut self) -> bool { unsafe { (self.0.iterate)(self.0.ctx) } }
    fn goal_node(&self) -> Option<Self::Search> { handle(unsafe { (self.0.goal_node)(self.0.ctx) }) }
    fn has_output(&self) -> bool { unsafe { (self.0.has_output)(self.0.ctx) } }
    fn append_path(&mut self, search: Self::Search) -> bool { unsafe { (self.0.append_path)(self.0.ctx, search) } }
    fn parent(&self, search: Self::Search) -> Option<Self::Search> { handle(unsafe { (self.0.parent)(self.0.ctx, search) }) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_astar_get_path(host: *const HplAStarPathHost) -> bool {
    let host = unsafe { host.read() };
    astar_path::get_path(&mut Host(host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum Event { Direct, Clear, Reset, Save([u32; 3]), Begin(i32, u32), Visible(i32, usize),
        Start(usize, u32), Goal(usize), Iterate, Append(usize) }
    struct Native {
        nodes: [u8; 1],
        searches: [u8; 2],
        endpoints: [HplVector3f; 2],
        distance: f32,
        cursors: [usize; 2],
        empty: bool,
        direct: Option<bool>,
        visibility: Option<bool>,
        found: Option<usize>,
        iteration_result: Option<usize>,
        output_enabled: bool,
        fail_append: Option<usize>,
        output: Vec<usize>,
        position_reads: usize,
        parent_reads: usize,
        selectors: Vec<i32>,
        events: Vec<Event>,
    }
    impl Native {
        fn new() -> Self {
            Self { nodes: [0], searches: [0; 2], endpoints: [HplVector3f { x: 0.0, y: 0.0, z: 0.0 }; 2],
                distance: 2.0, cursors: [0; 2], empty: false, direct: Some(false), visibility: Some(true),
                found: Some(1), iteration_result: Some(1), output_enabled: true, fail_append: None,
                output: vec![99], position_reads: 0, parent_reads: 0, selectors: Vec::new(), events: Vec::new() }
        }
        fn node_index(&self, node: *mut c_void) -> usize { unsafe { node.cast::<u8>().offset_from(self.nodes.as_ptr()) as usize } }
        fn search_index(&self, search: *mut c_void) -> usize { unsafe { search.cast::<u8>().offset_from(self.searches.as_ptr()) as usize } }
        fn search(&mut self, index: usize) -> *mut c_void { unsafe { self.searches.as_mut_ptr().add(index).cast() } }
    }
    fn host(native: &mut Native) -> HplAStarPathHost {
        unsafe extern "C" fn max_height(_: *mut c_void) -> f32 { 1.0 }
        unsafe extern "C" fn endpoint(ctx: *mut c_void, side: i32) -> HplVector3f {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.selectors.push(side);
            native.endpoints[side as usize]
        }
        unsafe extern "C" fn direct_visible(ctx: *mut c_void, out: *mut bool) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.events.push(Event::Direct);
            // Failure deliberately leaves a true output; success status must win.
            unsafe { out.write(native.direct.unwrap_or(true)); }
            native.direct.is_some()
        }
        unsafe extern "C" fn clear_goal(ctx: *mut c_void) {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.events.push(Event::Clear); native.found = None;
        }
        unsafe extern "C" fn reset_search(ctx: *mut c_void) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.events.push(Event::Reset); native.found = None; true
        }
        unsafe extern "C" fn set_goal_position(ctx: *mut c_void, goal: HplVector3f) {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.events.push(Event::Save([goal.x.to_bits(), goal.y.to_bits(), goal.z.to_bits()]));
        }
        unsafe extern "C" fn max_distance(ctx: *mut c_void) -> f32 { unsafe { (*ctx.cast::<Native>()).distance } }
        unsafe extern "C" fn begin_candidates(ctx: *mut c_void, side: i32, radius: f32) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.selectors.push(side); native.events.push(Event::Begin(side, radius.to_bits())); true
        }
        unsafe extern "C" fn next_candidate(ctx: *mut c_void, side: i32) -> *mut c_void {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.selectors.push(side);
            if native.empty || native.cursors[side as usize] != 0 { return std::ptr::null_mut(); }
            native.cursors[side as usize] += 1;
            native.nodes.as_mut_ptr().cast()
        }
        unsafe extern "C" fn position(ctx: *mut c_void, node: *mut c_void) -> HplVector3f {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.position_reads += 1;
            HplVector3f { x: native.node_index(node) as f32 + 1.0, y: 0.0, z: 0.0 }
        }
        unsafe extern "C" fn visible(ctx: *mut c_void, side: i32, node: *mut c_void, out: *mut bool) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.selectors.push(side); native.events.push(Event::Visible(side, native.node_index(node)));
            unsafe { out.write(native.visibility.unwrap_or(true)); }
            native.visibility.is_some()
        }
        unsafe extern "C" fn add_start(ctx: *mut c_void, node: *mut c_void, distance: f32) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.events.push(Event::Start(native.node_index(node), distance.to_bits())); true
        }
        unsafe extern "C" fn add_goal(ctx: *mut c_void, node: *mut c_void) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.events.push(Event::Goal(native.node_index(node))); true
        }
        unsafe extern "C" fn iterate(ctx: *mut c_void) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.events.push(Event::Iterate); native.found = native.iteration_result; true
        }
        unsafe extern "C" fn goal_node(ctx: *mut c_void) -> *mut c_void {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.found.map_or(std::ptr::null_mut(), |index| native.search(index))
        }
        unsafe extern "C" fn has_output(ctx: *mut c_void) -> bool { unsafe { (*ctx.cast::<Native>()).output_enabled } }
        unsafe extern "C" fn append_path(ctx: *mut c_void, search: *mut c_void) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            let index = native.search_index(search);
            native.events.push(Event::Append(index));
            if native.fail_append == Some(index) { return false; }
            native.output.push(index); true
        }
        unsafe extern "C" fn parent(ctx: *mut c_void, search: *mut c_void) -> *mut c_void {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.parent_reads += 1;
            if native.search_index(search) == 1 { native.search(0) } else { std::ptr::null_mut() }
        }
        HplAStarPathHost { ctx: (native as *mut Native).cast(), max_height, endpoint, direct_visible, clear_goal, reset_search,
            set_goal_position, max_distance, begin_candidates, next_candidate, position, visible, add_start, add_goal,
            iterate, goal_node, has_output, append_path, parent }
    }

    #[test]
    fn fallback_ffi_uses_start_zero_goal_one_and_preserves_opaque_handle_identity() {
        let mut native = Native::new();
        let host = host(&mut native);
        assert!(unsafe { hpl_astar_get_path(&host) });
        assert!(native.selectors.iter().all(|&side| side == 0 || side == 1));
        let begins: Vec<_> = native.events.iter().filter_map(|event| if let Event::Begin(side, radius) = event {
            Some((*side, *radius))
        } else { None }).collect();
        assert_eq!(begins, [(0, 4.0_f32.to_bits()), (1, 4.0_f32.to_bits())]);
        assert!(native.events.contains(&Event::Start(0, 1.0_f32.to_bits())));
        assert!(native.events.contains(&Event::Goal(0)));
        assert_eq!(native.output, [99, 1, 0]);
        assert_eq!(native.parent_reads, 2);
    }

    #[test]
    fn direct_visibility_failure_ignores_true_output_while_success_skips_setup() {
        for direct in [None, Some(true)] {
            let mut native = Native::new();
            native.direct = direct;
            let host = host(&mut native);
            assert_eq!(unsafe { hpl_astar_get_path(&host) }, direct.is_some());
            if direct.is_some() { assert_eq!(native.events, [Event::Direct, Event::Clear]); }
            else { assert_eq!(native.events, [Event::Direct]); }
            assert_eq!(native.output, [99]);
            assert_eq!(native.position_reads, 0);
        }
    }

    #[test]
    fn blocked_candidate_visibility_continues_but_failure_stops_before_goal_phase() {
        for visibility in [Some(false), None] {
            let mut native = Native::new();
            native.visibility = visibility;
            native.iteration_result = None;
            let host = host(&mut native);
            assert!(!unsafe { hpl_astar_get_path(&host) });
            assert!(!native.events.iter().any(|event| matches!(event, Event::Start(_, _) | Event::Goal(_))));
            if visibility.is_some() { assert_eq!(native.events.last(), Some(&Event::Iterate)); }
            else { assert_eq!(native.events.last(), Some(&Event::Visible(0, 0))); }
            assert_eq!(native.output, [99]);
        }
    }

    #[test]
    fn null_candidate_and_goal_handles_end_normally_and_null_output_skips_parent_reads() {
        let mut native = Native::new();
        native.empty = true; native.iteration_result = None;
        let host = host(&mut native);
        assert!(!unsafe { hpl_astar_get_path(&host) });
        assert_eq!((native.position_reads, native.parent_reads), (0, 0));
        native.iteration_result = Some(1); native.output_enabled = false;
        assert!(unsafe { hpl_astar_get_path(&host) });
        assert_eq!((native.position_reads, native.parent_reads), (0, 0));
        assert_eq!(native.output, [99]);
    }

    #[test]
    fn endpoint_and_radius_bits_survive_the_ffi_boundary() {
        let mut native = Native::new();
        native.endpoints[1] = HplVector3f { x: f32::from_bits(0x7fc12345), y: -0.0, z: f32::from_bits(1) };
        native.distance = f32::from_bits(0x3f000001);
        let host = host(&mut native);
        assert!(unsafe { hpl_astar_get_path(&host) });
        assert!(native.events.contains(&Event::Save([0x7fc12345, 0x80000000, 1])));
        assert!(native.events.contains(&Event::Begin(0, 0x3f800001)));
        assert!(native.events.contains(&Event::Begin(1, 0x3f800001)));
        assert!(native.events.contains(&Event::Start(0, 0x3f800000)));
    }

    #[test]
    fn append_failure_returns_without_reading_the_failed_nodes_parent_or_erasing_prefix() {
        let mut native = Native::new();
        native.fail_append = Some(0);
        let host = host(&mut native);
        assert!(!unsafe { hpl_astar_get_path(&host) });
        assert_eq!(native.output, [99, 1]);
        assert_eq!(native.parent_reads, 1);
        assert_eq!(native.events.last(), Some(&Event::Append(0)));
        assert_eq!(native.found, Some(1));
    }
}
