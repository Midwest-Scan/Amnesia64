//! Per-source AI connection compilation; native containers and physics stay C++.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_ai::node_compile::{self, CompileHost};
use hpl_math::Vector3f;
use std::ffi::c_void;

/// All callbacks are required and must not throw across Rust. The native
/// iterator returns null only at exhaustion. Mutators return false on a saved
/// exception; visible writes its result separately from its success status.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAINodeCompileHost {
    pub ctx: *mut c_void,
    pub next_candidate: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    pub position: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> HplVector3f,
    pub max_distance: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub max_height: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub visible: unsafe extern "C" fn(ctx: *mut c_void, candidate: *mut c_void, visible: *mut bool) -> bool,
    pub add_edge: unsafe extern "C" fn(ctx: *mut c_void, candidate: *mut c_void) -> bool,
    pub sort_edges: unsafe extern "C" fn(ctx: *mut c_void) -> bool,
    pub max_edges: unsafe extern "C" fn(ctx: *mut c_void) -> i32,
    pub edge_count: unsafe extern "C" fn(ctx: *mut c_void) -> usize,
    pub resize_edges: unsafe extern "C" fn(ctx: *mut c_void, count: usize) -> bool,
    pub edge_distance: unsafe extern "C" fn(ctx: *mut c_void, index: usize) -> f32,
    pub min_edges: unsafe extern "C" fn(ctx: *mut c_void) -> i32,
}

struct Host(HplAINodeCompileHost);

impl CompileHost for Host {
    type Node = *mut c_void;
    fn next_candidate(&mut self) -> Option<Self::Node> {
        let node = unsafe { (self.0.next_candidate)(self.0.ctx) };
        if node.is_null() { None } else { Some(node) }
    }
    fn position(&self, node: Self::Node) -> Vector3f { unsafe { (self.0.position)(self.0.ctx, node).to_rs() } }
    fn max_distance(&self) -> f32 { unsafe { (self.0.max_distance)(self.0.ctx) } }
    fn max_height(&self) -> f32 { unsafe { (self.0.max_height)(self.0.ctx) } }
    fn visible(&mut self, candidate: Self::Node) -> Option<bool> {
        let mut visible = false;
        if unsafe { (self.0.visible)(self.0.ctx, candidate, &mut visible) } { Some(visible) } else { None }
    }
    fn add_edge(&mut self, candidate: Self::Node) -> bool { unsafe { (self.0.add_edge)(self.0.ctx, candidate) } }
    fn sort_edges(&mut self) -> bool { unsafe { (self.0.sort_edges)(self.0.ctx) } }
    fn max_edges(&self) -> i32 { unsafe { (self.0.max_edges)(self.0.ctx) } }
    fn edge_count(&self) -> usize { unsafe { (self.0.edge_count)(self.0.ctx) } }
    fn resize_edges(&mut self, count: usize) -> bool { unsafe { (self.0.resize_edges)(self.0.ctx, count) } }
    fn edge_distance(&self, index: usize) -> f32 { unsafe { (self.0.edge_distance)(self.0.ctx, index) } }
    fn min_edges(&self) -> i32 { unsafe { (self.0.min_edges)(self.0.ctx) } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_compile(source: *mut c_void, host: *const HplAINodeCompileHost) {
    let host = unsafe { host.read() };
    node_compile::compile_node(&mut Host(host), source);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    enum Visibility { Accept, BlockFirst, FailFirst }

    struct Native {
        nodes: [u8; 3],
        candidates: Vec<usize>,
        cursor: usize,
        next_calls: usize,
        position_calls: usize,
        visible_nodes: Vec<usize>,
        added: Vec<usize>,
        sorted: usize,
        visibility: Visibility,
    }

    impl Native {
        fn new(candidates: Vec<usize>, visibility: Visibility) -> Self {
            Self { nodes: [0; 3], candidates, cursor: 0, next_calls: 0, position_calls: 0,
                   visible_nodes: Vec::new(), added: Vec::new(), sorted: 0, visibility }
        }
        fn node_index(&self, node: *mut c_void) -> usize {
            // All fixture handles point into this same live byte array.
            unsafe { node.cast::<u8>().offset_from(self.nodes.as_ptr()) as usize }
        }
    }

    fn host(native: &mut Native) -> HplAINodeCompileHost {
        unsafe extern "C" fn next_candidate(ctx: *mut c_void) -> *mut c_void {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.next_calls += 1;
            let Some(&node) = native.candidates.get(native.cursor) else { return std::ptr::null_mut(); };
            native.cursor += 1;
            unsafe { native.nodes.as_mut_ptr().add(node).cast() }
        }
        unsafe extern "C" fn position(ctx: *mut c_void, node: *mut c_void) -> HplVector3f {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.position_calls += 1;
            HplVector3f { x: native.node_index(node) as f32, y: 0.0, z: 0.0 }
        }
        unsafe extern "C" fn max_distance(_: *mut c_void) -> f32 { 10.0 }
        unsafe extern "C" fn max_height(_: *mut c_void) -> f32 { 0.0 }
        unsafe extern "C" fn visible(ctx: *mut c_void, node: *mut c_void, out: *mut bool) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            let index = native.node_index(node);
            native.visible_nodes.push(index);
            if index == 1 {
                match native.visibility {
                    Visibility::BlockFirst => { unsafe { out.write(false); } return true; }
                    // A stale or even true output must never override failure.
                    Visibility::FailFirst => { unsafe { out.write(true); } return false; }
                    Visibility::Accept => {}
                }
            }
            unsafe { out.write(true); }
            true
        }
        unsafe extern "C" fn add_edge(ctx: *mut c_void, node: *mut c_void) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.added.push(native.node_index(node));
            true
        }
        unsafe extern "C" fn sort_edges(ctx: *mut c_void) -> bool {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.sorted += 1;
            true
        }
        unsafe extern "C" fn max_edges(_: *mut c_void) -> i32 { 0 }
        unsafe extern "C" fn edge_count(ctx: *mut c_void) -> usize { unsafe { (*ctx.cast::<Native>()).added.len() } }
        unsafe extern "C" fn resize_edges(ctx: *mut c_void, count: usize) -> bool {
            unsafe { (*ctx.cast::<Native>()).added.truncate(count); }
            true
        }
        unsafe extern "C" fn edge_distance(_: *mut c_void, _: usize) -> f32 { 1.0 }
        unsafe extern "C" fn min_edges(_: *mut c_void) -> i32 { 0 }
        HplAINodeCompileHost { ctx: (native as *mut Native).cast(), next_candidate, position, max_distance, max_height,
            visible, add_edge, sort_edges, max_edges, edge_count, resize_edges, edge_distance, min_edges }
    }

    #[test]
    fn visibility_rejection_continues_to_later_candidates_through_ffi() {
        let mut native = Native::new(vec![1, 2], Visibility::BlockFirst);
        let source = native.nodes.as_mut_ptr().cast();
        let host = host(&mut native);
        unsafe { hpl_ai_node_compile(source, &host); }
        assert_eq!(native.visible_nodes, [1, 2]);
        assert_eq!(native.added, [2]);
        assert_eq!((native.sorted, native.next_calls), (1, 3));
    }

    #[test]
    fn saved_visibility_failure_ignores_output_and_stops_through_ffi() {
        let mut native = Native::new(vec![1, 2], Visibility::FailFirst);
        let source = native.nodes.as_mut_ptr().cast();
        let host = host(&mut native);
        unsafe { hpl_ai_node_compile(source, &host); }
        assert_eq!(native.visible_nodes, [1]);
        assert!(native.added.is_empty());
        assert_eq!((native.sorted, native.next_calls), (0, 1));
    }

    #[test]
    fn null_candidate_exhaustion_never_reads_a_node_and_still_sorts() {
        let mut native = Native::new(Vec::new(), Visibility::Accept);
        let source = native.nodes.as_mut_ptr().cast();
        let host = host(&mut native);
        unsafe { hpl_ai_node_compile(source, &host); }
        assert_eq!((native.position_calls, native.sorted, native.next_calls), (0, 1, 1));
        assert!(native.visible_nodes.is_empty());
    }

    #[test]
    fn opaque_source_identity_skips_self_without_losing_the_next_candidate() {
        let mut native = Native::new(vec![0, 1], Visibility::Accept);
        let source = native.nodes.as_mut_ptr().cast();
        let host = host(&mut native);
        unsafe { hpl_ai_node_compile(source, &host); }
        assert_eq!(native.visible_nodes, [1]);
        assert_eq!(native.added, [1]);
        assert_eq!((native.sorted, native.next_calls), (1, 3));
    }
}
