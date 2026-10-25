//! `cTriangleData` and `cTriEdge` (`math/MeshTypes.h`). Layout matches C++
//! so the C++ `std::vector` storage can be filled in place.

use crate::vector::Vector3f;

/// `cTriangleData`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TriangleData {
    pub normal: Vector3f,
    pub facing_light: bool,
}

impl TriangleData {
    #[inline]
    pub const fn new(normal: Vector3f) -> Self {
        Self { normal, facing_light: false }
    }
}

/// `cTriEdge`: an edge between two vertex indices shared by up to two triangles.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TriEdge {
    pub point1: i32,
    pub point2: i32,
    pub tri1: i32,
    pub tri2: i32,
    pub invert_tri2: bool,
}

impl TriEdge {
    #[inline]
    pub const fn new(point1: i32, point2: i32, tri1: i32, tri2: i32) -> Self {
        Self { point1, point2, tri1, tri2, invert_tri2: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_cpp() {
        assert_eq!(std::mem::size_of::<TriangleData>(), 16);
        assert_eq!(std::mem::offset_of!(TriangleData, facing_light), 12);
        assert_eq!(std::mem::size_of::<TriEdge>(), 20);
        assert_eq!(std::mem::offset_of!(TriEdge, invert_tri2), 16);
    }
}
