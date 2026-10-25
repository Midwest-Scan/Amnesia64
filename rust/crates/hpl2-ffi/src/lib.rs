//! `hpl2_rs`: the static library linked into HPL2.
//!
//! Every Rust module of the engine is exported to C++ from this one crate:
//! a C++ program can only link one Rust static library (each would carry its
//! own copy of `std`). New modules add a file here and depend on their
//! `hpl-*` crate.
//!
//! The C header `include/hpl2_rs.h` is generated from these declarations by
//! `build.rs` (cbindgen) on every build.

pub mod bridge;
pub mod geometry;
pub mod math;
pub mod misc;
pub mod types;

#[cfg(test)]
mod tests {
    use crate::types::*;
    use std::mem::offset_of;

    #[test]
    fn field_offsets_match_hpl_math() {
        assert_eq!(offset_of!(HplQuaternion, w), offset_of!(hpl_math::Quaternion, w));
        assert_eq!(offset_of!(HplSpheref, center), offset_of!(hpl_math::Spheref, center));
        assert_eq!(offset_of!(HplTriangleData, facing_light), offset_of!(hpl_math::TriangleData, facing_light));
        assert_eq!(offset_of!(HplTriEdge, invert_tri2), offset_of!(hpl_math::TriEdge, invert_tri2));
        assert_eq!(offset_of!(HplTriEdge, tri2), offset_of!(hpl_math::TriEdge, tri2));
        assert_eq!(offset_of!(HplBvExtents, radius), offset_of!(hpl_math::bounding_volume::Extents, radius));
        assert_eq!(offset_of!(HplBvExtents, pivot), offset_of!(hpl_math::bounding_volume::Extents, pivot));
    }

    #[test]
    fn matrix_roundtrip_through_ffi() {
        let a = HplMatrixf::from_rs(hpl_math::math::matrix_rotate_y(0.3));
        let b = HplMatrixf::from_rs(hpl_math::math::matrix_translate(hpl_math::Vector3f::new(1.0, 2.0, 3.0)));
        let c = unsafe { crate::math::hpl_math_matrix_mul(&a, &b) };
        assert_eq!(c.to_rs(), hpl_math::math::matrix_mul(&a.to_rs(), &b.to_rs()));
    }
}
