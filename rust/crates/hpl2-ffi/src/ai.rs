//! AStar's search core. Graph/search objects and their ownership stay in C++.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_ai::astar::{self, SearchHost};
use std::ffi::c_void;

/// Traverses the native open set in its comparator order, without allocation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAStarOpenHost {
    pub ctx: *mut c_void,
    /// Return the next opaque search node and write its cost. Null ends input.
    pub next: unsafe extern "C" fn(ctx: *mut c_void, cost: *mut f32) -> *mut c_void,
}

/// Live access to C++ AStar state. Handles are opaque: Rust never dereferences
/// or borrows engine storage across a callback, including a nested GetPath.
/// Every callback is required and must return normally (never throw).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAStarSearchHost {
    pub ctx: *mut c_void,
    pub open_empty: unsafe extern "C" fn(ctx: *mut c_void) -> bool,
    pub max_iterations: unsafe extern "C" fn(ctx: *mut c_void) -> i32,
    /// Null requests return to C++ to rethrow a saved exception.
    pub best_node: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    pub graph_node: unsafe extern "C" fn(ctx: *mut c_void, search: *mut c_void) -> *mut c_void,
    pub is_goal: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> bool,
    pub set_goal: unsafe extern "C" fn(ctx: *mut c_void, search: *mut c_void),
    pub edge_count: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> i32,
    pub edge: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void, index: i32) -> *mut c_void,
    pub edge_node: unsafe extern "C" fn(ctx: *mut c_void, edge: *mut c_void) -> *mut c_void,
    /// Success is separate from permission: false stops to rethrow, while
    /// true with *allowed=false only skips this edge. Write after user code.
    pub can_add: unsafe extern "C" fn(ctx: *mut c_void, parent: *mut c_void, child: *mut c_void, allowed: *mut bool) -> bool,
    pub edge_distance: unsafe extern "C" fn(ctx: *mut c_void, edge: *mut c_void) -> f32,
    pub search_distance: unsafe extern "C" fn(ctx: *mut c_void, search: *mut c_void) -> f32,
    /// False requests immediate return to C++ to rethrow a saved exception.
    pub add_open: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void, parent: *mut c_void, distance: f32) -> bool,
}

struct Host(HplAStarSearchHost);

impl SearchHost for Host {
    type SearchNode = *mut c_void;
    type Node = *mut c_void;
    type Edge = *mut c_void;
    fn open_empty(&self) -> bool { unsafe { (self.0.open_empty)(self.0.ctx) } }
    fn max_iterations(&self) -> i32 { unsafe { (self.0.max_iterations)(self.0.ctx) } }
    fn best_node(&mut self) -> Option<Self::SearchNode> {
        let node = unsafe { (self.0.best_node)(self.0.ctx) };
        if node.is_null() { None } else { Some(node) }
    }
    fn graph_node(&self, search: Self::SearchNode) -> Self::Node { unsafe { (self.0.graph_node)(self.0.ctx, search) } }
    fn is_goal(&self, node: Self::Node) -> bool { unsafe { (self.0.is_goal)(self.0.ctx, node) } }
    fn set_goal(&mut self, search: Self::SearchNode) { unsafe { (self.0.set_goal)(self.0.ctx, search) } }
    fn edge_count(&self, node: Self::Node) -> i32 { unsafe { (self.0.edge_count)(self.0.ctx, node) } }
    fn edge(&self, node: Self::Node, index: i32) -> Self::Edge { unsafe { (self.0.edge)(self.0.ctx, node, index) } }
    fn edge_node(&self, edge: Self::Edge) -> Self::Node { unsafe { (self.0.edge_node)(self.0.ctx, edge) } }
    fn can_add(&mut self, parent: Self::Node, child: Self::Node) -> Option<bool> {
        let mut allowed = false;
        let success = unsafe { (self.0.can_add)(self.0.ctx, parent, child, &mut allowed) };
        if success { Some(allowed) } else { None }
    }
    fn edge_distance(&self, edge: Self::Edge) -> f32 { unsafe { (self.0.edge_distance)(self.0.ctx, edge) } }
    fn search_distance(&self, search: Self::SearchNode) -> f32 { unsafe { (self.0.search_distance)(self.0.ctx, search) } }
    fn add_open(&mut self, node: Self::Node, parent: Self::SearchNode, distance: f32) -> bool {
        unsafe { (self.0.add_open)(self.0.ctx, node, parent, distance) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_astar_iterate_algorithm(host: *const HplAStarSearchHost) {
    let host = unsafe { host.read() };
    astar::iterate(&mut Host(host));
}

/// Select from a nonempty native open set. The C++ wrapper then transfers the
/// result to its closed set, retaining C++ allocation/exception semantics.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_astar_get_best_node(host: *const HplAStarOpenHost) -> *mut c_void {
    let host = unsafe { host.read() };
    let nodes = std::iter::from_fn(|| {
        let mut cost = 0.0;
        let node = unsafe { (host.next)(host.ctx, &mut cost) };
        if node.is_null() { None } else { Some((node, cost)) }
    });
    // Deviation: dereferencing begin() on an empty set is C++ UB. Abort with
    // an engine diagnostic if a future caller violates this private contract.
    astar::best_node(nodes).expect("AStar GetBestNode requires a nonempty open set")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_astar_cost(distance: f32, node_y: f32, has_parent: bool, parent_y: f32) -> f32 {
    astar::cost(distance, node_y, if has_parent { Some(parent_y) } else { None })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_astar_heuristic(start: HplVector3f, goal: HplVector3f) -> f32 {
    astar::heuristic(start.to_rs(), goal.to_rs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costs_and_heuristic_through_ffi() {
        unsafe {
            assert_eq!(hpl_astar_cost(-0.0, f32::NAN, false, f32::NAN).to_bits(), 0x80000000);
            assert_eq!(hpl_astar_cost(2.0, 5.0, true, 2.0), 8.0);
            assert_eq!(hpl_astar_heuristic(HplVector3f { x: 0.0, y: 0.0, z: 0.0 },
                                         HplVector3f { x: 3.0, y: 4.0, z: 0.0 }), 5.0);
        }
    }

    #[test]
    fn native_order_selection_through_ffi() {
        struct Input { nodes: [u8; 3], costs: [f32; 3], index: usize }
        unsafe extern "C" fn next(ctx: *mut c_void, cost: *mut f32) -> *mut c_void {
            let input = unsafe { &mut *ctx.cast::<Input>() };
            if input.index == input.nodes.len() { return std::ptr::null_mut(); }
            let index = input.index;
            input.index += 1;
            unsafe { cost.write(input.costs[index]); }
            (&mut input.nodes[index] as *mut u8).cast()
        }
        let mut input = Input { nodes: [0; 3], costs: [3.0, 1.0, 1.0], index: 0 };
        let expected = (&mut input.nodes[1] as *mut u8).cast();
        let host = HplAStarOpenHost { ctx: (&mut input as *mut Input).cast(), next };
        assert_eq!(unsafe { hpl_astar_get_best_node(&host) }, expected);
        assert_eq!(input.index, 3);
    }
}
