//! `cMath` (`math/Math.h`, `sources/math/Math.cpp`) as free functions.
//!
//! Names are the C++ names in snake_case (`cMath::MatrixMul` ->
//! [`matrix_mul`]). Where C++ returns results through optional out-pointers,
//! the Rust version returns an `Option` or a small struct instead.

mod clip;
mod color_conv;
mod intersect;
mod matrix_ops;
mod plane;
mod polygon;
mod quat;
mod random;
mod scalar;
mod spherical;
mod triangle;
mod vec2;
mod vec3;

pub use clip::*;
pub use color_conv::*;
pub use intersect::*;
pub use matrix_ops::*;
pub use plane::*;
pub use polygon::*;
pub use quat::*;
pub use random::*;
pub use scalar::*;
pub use spherical::*;
pub use triangle::*;
pub use vec2::*;
pub use vec3::*;

use crate::vector::Vector3f;

/// A line/segment hit: position and parametric distance along the segment.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineHit {
    pub pos: Vector3f,
    pub t: f32,
}

/// The 8 corners of an AABB in the engine's fixed order
/// (max/max/max, max/max/min, max/min/max, ... min/min/min).
#[inline]
pub fn aabb_corners(min: Vector3f, max: Vector3f) -> [Vector3f; 8] {
    [
        Vector3f::new(max.x, max.y, max.z),
        Vector3f::new(max.x, max.y, min.z),
        Vector3f::new(max.x, min.y, max.z),
        Vector3f::new(max.x, min.y, min.z),
        Vector3f::new(min.x, max.y, max.z),
        Vector3f::new(min.x, max.y, min.z),
        Vector3f::new(min.x, min.y, max.z),
        Vector3f::new(min.x, min.y, min.z),
    ]
}
