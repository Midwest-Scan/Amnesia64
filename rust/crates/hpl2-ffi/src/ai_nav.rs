//! Rust-owned AI navigation (phase 2): `cAINodeContainer`, `cAINode`,
//! `cAINodeIterator` and `cAStarHandler` keep their C++ API as facades over
//! `hpl_ai::nav`. See `rust/PHASE2.md`.
//!
//! Opaque objects: `HplAINavGraph` (a container's graph), `HplAINode` (one
//! node, owned by its graph) and `HplAStar` (a handler's search state). C++
//! frees what it creates with `hpl_ai_nav_new` / `hpl_ai_astar_new`; nodes
//! live as long as their graph. Node identities crossing the boundary as
//! `void*` are the `cAINode*` facades.
//!
//! Callbacks catch C++ exceptions and return false; Rust then stops at once
//! and the C++ wrapper rethrows after the call returns.

#![allow(clippy::missing_safety_doc)]

use crate::system::HplStr;
use crate::types::*;
use hpl_ai::nav::{AStar, CompileCallbacks, Edge, Handle, IterState, NavGraph, Node, PathCallbacks};
use hpl_math::Vector3f;
use std::ffi::c_void;
use std::mem::{align_of, offset_of, size_of};

/// A container's navigation graph (opaque).
pub struct HplAINavGraph {
    _private: [u8; 0],
}

/// One navigation node, owned by its graph (opaque).
pub struct HplAINode {
    _private: [u8; 0],
}

/// A `cAStarHandler`'s search state (opaque).
pub struct HplAStar {
    _private: [u8; 0],
}

/// `cAINodeEdge`: `{float mfDistance; float mfSqrDistance; cAINode *mpNode;}`.
/// `cAINode::GetEdge` returns a pointer to these in place.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HplAINodeEdge {
    pub distance: f32,
    pub sqr_distance: f32,
    pub node: *mut c_void,
}

const _: () = {
    assert!(size_of::<HplAINodeEdge>() == size_of::<Edge>() && align_of::<HplAINodeEdge>() == align_of::<Edge>());
    assert!(offset_of!(HplAINodeEdge, distance) == offset_of!(Edge, distance));
    assert!(offset_of!(HplAINodeEdge, sqr_distance) == offset_of!(Edge, sqr_distance));
    assert!(offset_of!(HplAINodeEdge, node) == offset_of!(Edge, node));
};

/// Grid cursor stored inside the `cAINodeIterator` value type.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HplAINodeIterState {
    pub start: HplVector2l,
    pub end: HplVector2l,
    pub position: HplVector2l,
    pub cell: i32,
    pub item: u32,
}

const _: () = {
    assert!(size_of::<HplAINodeIterState>() == size_of::<IterState>());
    assert!(align_of::<HplAINodeIterState>() == align_of::<IterState>());
    assert!(offset_of!(HplAINodeIterState, start) == offset_of!(IterState, start));
    assert!(offset_of!(HplAINodeIterState, end) == offset_of!(IterState, end));
    assert!(offset_of!(HplAINodeIterState, position) == offset_of!(IterState, position));
    assert!(offset_of!(HplAINodeIterState, cell) == offset_of!(IterState, cell));
    assert!(offset_of!(HplAINodeIterState, item) == offset_of!(IterState, item));
};

/// What `Compile` calls back into C++. Both callbacks are required.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAINavCompileHost {
    pub ctx: *mut c_void,
    /// `FreePath(start, end, -1, SkipDynamic | SkipVolatile)` into
    /// `*visible`; returns false if it threw.
    pub free_path: unsafe extern "C" fn(ctx: *mut c_void, start: HplVector3f, end: HplVector3f, visible: *mut bool) -> bool,
    /// `std::sort(edges, edges + count, cSortEndNodes())`; false if it threw.
    pub sort_edges: unsafe extern "C" fn(ctx: *mut c_void, edges: *mut HplAINodeEdge, count: usize) -> bool,
}

/// What `GetPath` calls back into C++. All callbacks are required.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAStarPathHost {
    pub ctx: *mut c_void,
    /// `FreePath(start, end, -1, SkipDynamic)` into `*visible`; false if it threw.
    pub free_path: unsafe extern "C" fn(ctx: *mut c_void, start: HplVector3f, end: HplVector3f, visible: *mut bool) -> bool,
    /// `mpCallback == NULL || mpCallback->CanAddNode(parent, child)` into
    /// `*allowed`; false if it threw.
    pub can_add_node: unsafe extern "C" fn(ctx: *mut c_void, parent: *mut c_void, child: *mut c_void, allowed: *mut bool) -> bool,
    /// `apNodeList->push_back(node)`; false if it threw.
    pub append: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> bool,
}

fn graph<'a>(g: *const HplAINavGraph) -> &'a NavGraph {
    // SAFETY: callers pass a pointer from hpl_ai_nav_new that is still alive.
    unsafe { &*(g as *const NavGraph) }
}

fn graph_mut<'a>(g: *mut HplAINavGraph) -> &'a mut NavGraph {
    // SAFETY: as above, and no other Rust reference to the graph is live.
    unsafe { &mut *(g as *mut NavGraph) }
}

fn node_ptr(n: *const HplAINode) -> *mut Node {
    n as *mut Node
}

fn facade(handle: Handle) -> *mut c_void {
    handle as *mut c_void
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_ai_nav_new() -> *mut HplAINavGraph {
    Box::into_raw(Box::new(NavGraph::new())) as *mut HplAINavGraph
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_free(g: *mut HplAINavGraph) {
    if !g.is_null() {
        assert!(!unsafe { NavGraph::is_active(g as *const NavGraph) }, "cAINodeContainer deleted during its own Compile or a GetPath on it");
        drop(unsafe { Box::from_raw(g as *mut NavGraph) });
    }
}

/// `ReserveSpace`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_reserve(g: *mut HplAINavGraph, count: usize) {
    graph_mut(g).reserve(count);
}

/// `AddNode`: `facade` is the new `cAINode`. Returns its Rust node, valid
/// for the lifetime of the graph.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_add_node(
    g: *mut HplAINavGraph,
    name: HplStr,
    id: i32,
    position: HplVector3f,
    user_data: *mut c_void,
    facade: *mut c_void,
) -> *mut HplAINode {
    let name = unsafe { crate::types::raw_slice(name.ptr, name.len) };
    graph_mut(g).add_node(name, id, position.to_rs(), user_data as usize, facade as Handle) as *mut HplAINode
}

/// `GetNodeNum`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_node_num(g: *const HplAINavGraph) -> i32 {
    // Faithful: (int)mvNodes.size().
    graph(g).node_count() as i32
}

/// `GetNode`: the facade, or NULL when out of range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_node(g: *const HplAINavGraph, index: i32) -> *mut c_void {
    facade(graph(g).node_handle(index))
}

/// `GetNodeFromName`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_node_from_name(g: *const HplAINavGraph, name: HplStr) -> *mut c_void {
    let name = unsafe { crate::types::raw_slice(name.ptr, name.len) };
    facade(graph(g).from_name(name))
}

/// `GetNodeFromID`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_node_from_id(g: *const HplAINavGraph, id: i32) -> *mut c_void {
    facade(graph(g).from_id(id))
}

// Properties are plain fields, written in place so a search in progress
// sees them live, as in C++.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_set_max_edges(g: *mut HplAINavGraph, count: i32) {
    unsafe { (*(g as *mut NavGraph)).max_edges = count };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_set_min_edges(g: *mut HplAINavGraph, count: i32) {
    unsafe { (*(g as *mut NavGraph)).min_edges = count };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_set_max_edge_distance(g: *mut HplAINavGraph, distance: f32) {
    unsafe { (*(g as *mut NavGraph)).max_edge_distance = distance };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_max_edge_distance(g: *const HplAINavGraph) -> f32 {
    graph(g).max_edge_distance
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_set_max_height(g: *mut HplAINavGraph, height: f32) {
    unsafe { (*(g as *mut NavGraph)).max_height = height };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_max_height(g: *const HplAINavGraph) -> f32 {
    graph(g).max_height
}

/// `BuildNodeGridMap`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_build_grid(g: *mut HplAINavGraph) {
    graph_mut(g).build_grid();
}

struct CompileHost(HplAINavCompileHost);

impl CompileCallbacks for CompileHost {
    fn free_path(&mut self, start: Vector3f, end: Vector3f) -> Option<bool> {
        let mut visible = false;
        let ok = unsafe { (self.0.free_path)(self.0.ctx, HplVector3f::from_rs(start), HplVector3f::from_rs(end), &mut visible) };
        ok.then_some(visible)
    }
    fn sort_edges(&mut self, edges: &mut [Edge]) -> bool {
        unsafe { (self.0.sort_edges)(self.0.ctx, edges.as_mut_ptr() as *mut HplAINodeEdge, edges.len()) }
    }
}

/// `Compile`. Returns early if a callback threw.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_compile(g: *mut HplAINavGraph, host: *const HplAINavCompileHost) {
    let mut host = CompileHost(unsafe { host.read() });
    unsafe { NavGraph::compile(g as *mut NavGraph, &mut host) };
}

// ---------------------------------------------------------------------------
// Iterator
// ---------------------------------------------------------------------------

fn state<'a>(s: *const HplAINodeIterState) -> &'a mut IterState {
    // SAFETY: layout checked above; C++ passes its own iterator's storage.
    unsafe { &mut *(s as *mut IterState) }
}

/// `cAINodeIterator(container, position, radius)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_iter_begin(g: *const HplAINavGraph, position: HplVector3f, radius: f32, out: *mut HplAINodeIterState) {
    *state(out) = graph(g).iter_begin(position.to_rs(), radius);
}

/// `cAINodeIterator::HasNext`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_iter_has_next(g: *const HplAINavGraph, s: *const HplAINodeIterState) -> bool {
    graph(g).iter_has_next(state(s))
}

/// `cAINodeIterator::Next`: the facade, or NULL past the end.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_nav_iter_next(g: *const HplAINavGraph, s: *mut HplAINodeIterState) -> *mut c_void {
    let graph = graph(g);
    graph.iter_next(state(s)).map_or(std::ptr::null_mut(), |i| facade(graph.node(i).handle))
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// `cAINode::GetEdgeNum`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_edge_num(n: *const HplAINode) -> i32 {
    // Faithful: (int)mvEdges.size().
    unsafe { (*node_ptr(n)).edges.len() as i32 }
}

/// `cAINode::GetEdge`: valid until an edge is added to this node.
/// Deviation: out of range (undefined in the original) returns NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_edge(n: *mut HplAINode, index: i32) -> *mut HplAINodeEdge {
    let edges = unsafe { &mut (*node_ptr(n)).edges };
    match usize::try_from(index).ok().filter(|&i| i < edges.len()) {
        Some(i) => unsafe { edges.as_mut_ptr().add(i) as *mut HplAINodeEdge },
        None => std::ptr::null_mut(),
    }
}

/// `cAINode::GetPosition`: valid for the lifetime of the graph.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_position(n: *const HplAINode) -> *const HplVector3f {
    unsafe { std::ptr::addr_of!((*node_ptr(n)).position) as *const HplVector3f }
}

/// `cAINode::GetID`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_id(n: *const HplAINode) -> i32 {
    unsafe { (*node_ptr(n)).id }
}

/// `cAINode::AddEdge`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_add_edge(n: *mut HplAINode, target: *const HplAINode) {
    assert!(!target.is_null(), "cAINode::AddEdge: NULL node");
    unsafe { NavGraph::add_edge(node_ptr(n), node_ptr(target)) };
}

/// An edge read from a `.nodes` file (`LoadFromFile`). `target` is the
/// facade, or NULL if the file names a node that does not exist.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_push_edge(n: *mut HplAINode, target: *mut c_void, distance: f32, sqr_distance: f32) {
    unsafe { NavGraph::push_edge(node_ptr(n), Edge { distance, sqr_distance, node: target as Handle }) };
}

// ---------------------------------------------------------------------------
// A*
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_ai_astar_new() -> *mut HplAStar {
    Box::into_raw(Box::new(AStar::new())) as *mut HplAStar
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_astar_free(a: *mut HplAStar) {
    if !a.is_null() {
        assert!(!unsafe { AStar::is_busy(a as *const AStar) }, "cAStarHandler deleted during its own GetPath");
        drop(unsafe { Box::from_raw(a as *mut AStar) });
    }
}

/// `SetMaxIterations`: -1 = until the open list is empty. Read live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_astar_set_max_iterations(a: *mut HplAStar, count: i32) {
    unsafe { (*(a as *mut AStar)).max_iterations = count };
}

struct PathHost(HplAStarPathHost);

impl PathCallbacks for PathHost {
    fn free_path(&mut self, start: Vector3f, end: Vector3f) -> Option<bool> {
        let mut visible = false;
        let ok = unsafe { (self.0.free_path)(self.0.ctx, HplVector3f::from_rs(start), HplVector3f::from_rs(end), &mut visible) };
        ok.then_some(visible)
    }
    fn can_add_node(&mut self, parent: Handle, child: Handle) -> Option<bool> {
        let mut allowed = false;
        let ok = unsafe { (self.0.can_add_node)(self.0.ctx, facade(parent), facade(child), &mut allowed) };
        ok.then_some(allowed)
    }
    fn append(&mut self, node: Handle) -> bool {
        unsafe { (self.0.append)(self.0.ctx, facade(node)) }
    }
}

/// `GetPath`. `has_output` is `apNodeList != NULL`. Returns false when no
/// path was found or a callback threw (the caller then rethrows).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_astar_get_path(
    a: *mut HplAStar,
    g: *mut HplAINavGraph,
    start: HplVector3f,
    goal: HplVector3f,
    has_output: bool,
    host: *const HplAStarPathHost,
) -> bool {
    let mut host = PathHost(unsafe { host.read() });
    unsafe { AStar::get_path(a as *mut AStar, g as *mut NavGraph, start.to_rs(), goal.to_rs(), has_output, &mut host) }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn free_all(_: *mut c_void, _: HplVector3f, _: HplVector3f, visible: *mut bool) -> bool {
        unsafe { *visible = true };
        true
    }

    #[test]
    fn graph_round_trip_through_the_c_abi() {
        let facades = [0u64; 3];
        let g = hpl_ai_nav_new();
        unsafe {
            let mut nodes = Vec::new();
            for (i, f) in facades.iter().enumerate() {
                let name = format!("n{i}");
                let n = hpl_ai_nav_add_node(
                    g,
                    HplStr { ptr: name.as_ptr(), len: name.len() },
                    i as i32 * 10,
                    HplVector3f { x: i as f32, y: 0.0, z: 0.0 },
                    std::ptr::null_mut(),
                    f as *const u64 as *mut c_void,
                );
                nodes.push(n);
            }
            assert_eq!(hpl_ai_nav_node_num(g), 3);
            assert_eq!(hpl_ai_nav_node(g, 1), &facades[1] as *const u64 as *mut c_void);
            assert!(hpl_ai_nav_node(g, 3).is_null());
            assert_eq!(hpl_ai_nav_node_from_id(g, 20), &facades[2] as *const u64 as *mut c_void);
            let name = b"n0";
            assert_eq!(hpl_ai_nav_node_from_name(g, HplStr { ptr: name.as_ptr(), len: 2 }), &facades[0] as *const u64 as *mut c_void);

            hpl_ai_node_add_edge(nodes[0], nodes[2]);
            assert_eq!(hpl_ai_node_edge_num(nodes[0]), 1);
            let e = &*hpl_ai_node_edge(nodes[0], 0);
            assert_eq!((e.distance, e.sqr_distance), (2.0, 4.0));
            assert_eq!(e.node, &facades[2] as *const u64 as *mut c_void);
            assert!(hpl_ai_node_edge(nodes[0], 1).is_null());
            assert_eq!((*hpl_ai_node_position(nodes[1])).x, 1.0);

            hpl_ai_nav_build_grid(g);
            let mut s = HplAINodeIterState { start: HplVector2l::default(), end: HplVector2l::default(), position: HplVector2l::default(), cell: 0, item: 0 };
            hpl_ai_nav_iter_begin(g, HplVector3f { x: 1.0, y: 0.0, z: 0.0 }, 10.0, &mut s);
            let mut seen = 0;
            while hpl_ai_nav_iter_has_next(g, &s) {
                assert!(!hpl_ai_nav_iter_next(g, &mut s).is_null());
                seen += 1;
            }
            assert_eq!(seen, 3);

            let a = hpl_ai_astar_new();
            unsafe extern "C" fn yes(_: *mut c_void, _: *mut c_void, _: *mut c_void, allowed: *mut bool) -> bool {
                unsafe { *allowed = true };
                true
            }
            unsafe extern "C" fn push(_: *mut c_void, _: *mut c_void) -> bool { true }
            let host = HplAStarPathHost { ctx: std::ptr::null_mut(), free_path: free_all, can_add_node: yes, append: push };
            let p = |x: f32| HplVector3f { x, y: 0.0, z: 0.0 };
            assert!(hpl_ai_astar_get_path(a, g, p(0.0), p(2.0), true, &host));
            hpl_ai_astar_free(a);
            hpl_ai_nav_free(g);
        }
    }
}
