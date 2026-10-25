//! AI node/grid helpers. Native containers, graph nodes and allocation stay C++.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_ai::node_container::{self, GridDimensions, GridHost};
use hpl_math::{Vector2f, Vector2l, Vector3f};
use std::ffi::c_void;

/// Live grid construction callbacks. Every callback is required and must not
/// throw. Resize/push report false to stop, then C++ rethrows its saved exception.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAINodeGridHost {
    pub ctx: *mut c_void,
    pub node_count: unsafe extern "C" fn(ctx: *mut c_void) -> usize,
    pub node: unsafe extern "C" fn(ctx: *mut c_void, index: usize) -> *mut c_void,
    pub position: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> HplVector3f,
    pub nodes_per_grid: unsafe extern "C" fn(ctx: *mut c_void) -> i32,
    pub set_bounds: unsafe extern "C" fn(ctx: *mut c_void, minimum: HplVector2f, maximum: HplVector2f),
    pub set_map_size: unsafe extern "C" fn(ctx: *mut c_void, size: HplVector2l),
    pub resize_grids: unsafe extern "C" fn(ctx: *mut c_void, count: usize) -> bool,
    /// Called only after bounds/map-size assignment; does not read cell size.
    pub dimensions: unsafe extern "C" fn(ctx: *mut c_void) -> HplAINodeGridDimensions,
    pub set_cell_size: unsafe extern "C" fn(ctx: *mut c_void, size: HplVector2f),
    pub cell_size: unsafe extern "C" fn(ctx: *mut c_void) -> HplVector2f,
    pub grid_count: unsafe extern "C" fn(ctx: *mut c_void) -> usize,
    pub push_node: unsafe extern "C" fn(ctx: *mut c_void, index: usize, node: *mut c_void) -> bool,
}

struct Host(HplAINodeGridHost);

impl GridHost for Host {
    type Node = *mut c_void;
    fn node_count(&self) -> usize { unsafe { (self.0.node_count)(self.0.ctx) } }
    fn node(&self, index: usize) -> Self::Node { unsafe { (self.0.node)(self.0.ctx, index) } }
    fn position(&self, node: Self::Node) -> Vector3f { unsafe { (self.0.position)(self.0.ctx, node).to_rs() } }
    fn nodes_per_grid(&self) -> i32 { unsafe { (self.0.nodes_per_grid)(self.0.ctx) } }
    fn set_bounds(&mut self, minimum: Vector2f, maximum: Vector2f) {
        unsafe { (self.0.set_bounds)(self.0.ctx, HplVector2f::from_rs(minimum), HplVector2f::from_rs(maximum)) }
    }
    fn set_map_size(&mut self, size: Vector2l) { unsafe { (self.0.set_map_size)(self.0.ctx, HplVector2l::from_rs(size)) } }
    fn resize_grids(&mut self, count: usize) -> bool { unsafe { (self.0.resize_grids)(self.0.ctx, count) } }
    fn dimensions(&self) -> GridDimensions { unsafe { (self.0.dimensions)(self.0.ctx).to_rs() } }
    fn set_cell_size(&mut self, size: Vector2f) { unsafe { (self.0.set_cell_size)(self.0.ctx, HplVector2f::from_rs(size)) } }
    fn cell_size(&self) -> Vector2f { unsafe { (self.0.cell_size)(self.0.ctx).to_rs() } }
    fn grid_count(&self) -> usize { unsafe { (self.0.grid_count)(self.0.ctx) } }
    fn push_node(&mut self, index: usize, node: Self::Node) -> bool { unsafe { (self.0.push_node)(self.0.ctx, index, node) } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_add_edge(start: HplVector3f, end: HplVector3f, distance: *mut f32, squared: *mut f32) {
    let (distance_value, squared_value) = node_container::edge_distances(start.to_rs(), end.to_rs());
    unsafe {
        write_plain(distance, distance_value);
        write_plain(squared, squared_value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_build_grid(host: *const HplAINodeGridHost) {
    let host = unsafe { host.read() };
    node_container::build_grid(&mut Host(host));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_grid_position(local: HplVector2f, cell: HplVector2f, map_size: HplVector2l) -> HplVector2l {
    HplVector2l::from_rs(node_container::grid_position(local.to_rs(), cell.to_rs(), map_size.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_grid_index(position: HplVector2l, map_size: HplVector2l, count: usize) -> usize {
    node_container::grid_index(position.to_rs(), map_size.to_rs(), count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_iterator_increment(position: HplVector2l, start: HplVector2l, end: HplVector2l,
                                                     out: *mut HplVector2l) -> bool {
    let mut position = position.to_rs();
    let result = node_container::increment_grid(&mut position, start.to_rs(), end.to_rs());
    unsafe { write_opt(out, position); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_outputs_allow_null_and_alias_after_reading_inputs() {
        let start = HplVector3f { x: 0.0, y: 0.0, z: 0.0 };
        let end = HplVector3f { x: 1.0, y: 1.0, z: 0.0 };
        let mut value: f32 = 99.0;
        let out = &mut value as *mut f32;
        unsafe {
            hpl_ai_node_add_edge(start, end, out, out);
            assert_eq!(value, 2.0);
            hpl_ai_node_add_edge(start, end, out, std::ptr::null_mut());
            assert_eq!(value.to_bits(), 0x3fb504f3);
        }
    }

    #[test]
    fn iterator_writes_exhausted_state_and_accepts_null_output() {
        let position = HplVector2l { x: 3, y: 4 };
        let start = HplVector2l { x: 2, y: 3 };
        let mut out = position;
        unsafe {
            assert!(!hpl_ai_node_iterator_increment(position, start, position, &mut out));
            assert_eq!(out.to_rs(), Vector2l::new(2, 5));
            assert!(!hpl_ai_node_iterator_increment(position, start, position, std::ptr::null_mut()));
        }
    }
}
