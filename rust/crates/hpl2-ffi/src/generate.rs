//! `cVoxelMap`: C++ owns storage and graphics; Rust supplies the algorithms.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_generate::voxel_map::{self, RenderHost};
use hpl_math::{Vector3f, Vector3l};
use std::ffi::c_void;

/// Reentrant access to a live C++ voxel map. All callbacks are required.
/// No buffer borrow is retained across draw_sphere, which may resize the map.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplVoxelRenderHost {
    pub ctx: *mut c_void,
    pub size: unsafe extern "C" fn(ctx: *mut c_void) -> HplVector3l,
    pub byte: unsafe extern "C" fn(ctx: *mut c_void, index: usize) -> u8,
    pub position: unsafe extern "C" fn(ctx: *mut c_void) -> HplVector3f,
    pub voxel_size: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    /// C++ reads the current referenced color immediately before DrawSphere.
    /// False requests an immediate return so C++ can rethrow a saved exception.
    pub draw_sphere: unsafe extern "C" fn(ctx: *mut c_void, center: HplVector3f, radius: f32) -> bool,
}

struct Host(HplVoxelRenderHost);

impl RenderHost for Host {
    fn size(&self) -> Vector3l { unsafe { (self.0.size)(self.0.ctx).to_rs() } }
    fn byte(&self, index: usize) -> u8 { unsafe { (self.0.byte)(self.0.ctx, index) } }
    fn position(&self) -> Vector3f { unsafe { (self.0.position)(self.0.ctx).to_rs() } }
    fn voxel_size(&self) -> f32 { unsafe { (self.0.voxel_size)(self.0.ctx) } }
    fn draw_sphere(&mut self, center: Vector3f, radius: f32) -> bool {
        unsafe { (self.0.draw_sphere)(self.0.ctx, HplVector3f::from_rs(center), radius) }
    }
}

/// `SetSize` allocation arithmetic. C++ retains delete/new[] and zeroing so
/// allocation exceptions cannot unwind through a Rust callback frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_voxel_map_set_size(size: HplVector3l) -> usize {
    voxel_map::allocation_size(size.to_rs())
}

/// `SetVoxel`: `data` must cover the current allocation and be initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_voxel_map_set_voxel(data: *mut u8, size: HplVector3l, pos: HplVector3l, value: u8) {
    let size = size.to_rs();
    let len = voxel_map::allocation_size(size);
    let data = if len == 0 { &mut [] } else { unsafe { std::slice::from_raw_parts_mut(data, len) } };
    voxel_map::set_voxel(data, size, pos.to_rs(), value);
}

/// `DebugRender`: callbacks access live state; they must not throw exceptions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_voxel_map_debug_render(host: *const HplVoxelRenderHost) {
    let host = unsafe { host.read() };
    voxel_map::debug_render(&mut Host(host));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_and_byte_alias_through_ffi() {
        let size = HplVector3l { x: 3, y: 2, z: 1 };
        let mut data = [0u8; 6];
        unsafe {
            assert_eq!(hpl_voxel_map_set_size(size), data.len());
            hpl_voxel_map_set_voxel(data.as_mut_ptr(), size, HplVector3l { x: -1, y: 1, z: 0 }, 255);
        }
        assert_eq!(data, [0, 0, 255, 0, 0, 0]);
    }
}
