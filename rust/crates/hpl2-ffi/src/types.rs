//! C-visible mirrors of the engine's value types.
//!
//! Each `Hpl*` struct has exactly the memory layout of the C++ class it is
//! named after (checked by `static_assert`s in `RustBridge.h` on the C++
//! side, and by the `mirror!` assertions here against `hpl-math`), so C++
//! can pass its own objects by pointer without copying.

use hpl_math::bounding_volume::Extents;
use hpl_math::frustum::{FrustumProjection, FrustumRef, ProjectionType};
use hpl_math::{Color, Matrixf, Planef, Quaternion, Rect2f, Rect2l, Spheref, TriEdge, TriangleData, Vector2f, Vector2l, Vector3f, Vector3l};

/// `cVector2f`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplVector2f {
    pub x: f32,
    pub y: f32,
}

/// `cVector2l`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplVector2l {
    pub x: i32,
    pub y: i32,
}

/// `cVector3f`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplVector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `cVector3l`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplVector3l {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// `cMatrixf`, row-major `m[row][col]`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplMatrixf {
    pub m: [[f32; 4]; 4],
}

/// `cQuaternion`: vector part then `w`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplQuaternion {
    pub v: HplVector3f,
    pub w: f32,
}

/// `cPlanef`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplPlanef {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
}

/// `cSpheref`: radius first, then centre.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplSpheref {
    pub r: f32,
    pub center: HplVector3f,
}

/// `cRect2f`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplRect2f {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// `cRect2l`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplRect2l {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// `cColor`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// `cTriangleData`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplTriangleData {
    pub normal: HplVector3f,
    pub facing_light: bool,
}

/// `cTriEdge`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplTriEdge {
    pub point1: i32,
    pub point2: i32,
    pub tri1: i32,
    pub tri2: i32,
    pub invert_tri2: bool,
}

/// Derived size data of a `cBoundingVolume` (see `UpdateSize`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplBvExtents {
    pub max: HplVector3f,
    pub min: HplVector3f,
    pub size: HplVector3f,
    pub pivot: HplVector3f,
    pub radius: f32,
}

/// The camera parameters of a `cFrustum` used by the clip-rect functions.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HplFrustumProjection {
    pub proj: HplMatrixf,
    pub view: HplMatrixf,
    pub near_plane: f32,
    pub far_plane: f32,
    pub fov: f32,
    pub aspect: f32,
    /// `eProjectionType`: 0 perspective, 1 orthographic.
    pub projection_type: i32,
}

/// Borrowed view of a `cFrustum`'s derived geometry. The pointers must stay
/// valid for the duration of the call.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HplFrustumView {
    /// 6 planes (`mPlane`).
    pub planes: *const HplPlanef,
    /// 8 corners (`mvVertices`).
    pub vertices: *const HplVector3f,
    pub sphere: HplSpheref,
    /// World AABB of the corners (`mBoundingVolume`).
    pub bv_min: HplVector3f,
    pub bv_max: HplVector3f,
    pub inf_far_plane: bool,
}

/// `eCollision` values returned by collision functions.
pub const HPL_COLLISION_INSIDE: i32 = 0;
pub const HPL_COLLISION_OUTSIDE: i32 = 1;
pub const HPL_COLLISION_INTERSECT: i32 = 2;

/// Plain-old-data mirror of a `hpl-math` type with identical layout.
///
/// # Safety
/// Implementors must have exactly the size, alignment and field layout of
/// `Rust`, and every bit pattern valid for one must be valid for the other.
pub unsafe trait Mirror: Copy {
    type Rust: Copy;

    #[inline]
    fn to_rs(self) -> Self::Rust {
        // SAFETY: guaranteed by the trait contract.
        unsafe { std::mem::transmute_copy(&self) }
    }

    #[inline]
    fn from_rs(v: Self::Rust) -> Self {
        // SAFETY: guaranteed by the trait contract.
        unsafe { std::mem::transmute_copy(&v) }
    }
}

macro_rules! mirror {
    ($($ffi:ty => $rust:ty),* $(,)?) => {$(
        const _: () = assert!(
            std::mem::size_of::<$ffi>() == std::mem::size_of::<$rust>()
                && std::mem::align_of::<$ffi>() == std::mem::align_of::<$rust>()
        );
        // SAFETY: both are #[repr(C)] with the same fields in the same order.
        unsafe impl Mirror for $ffi {
            type Rust = $rust;
        }
    )*};
}

mirror! {
    HplVector2f => Vector2f,
    HplVector2l => Vector2l,
    HplVector3f => Vector3f,
    HplVector3l => Vector3l,
    HplMatrixf => Matrixf,
    HplQuaternion => Quaternion,
    HplPlanef => Planef,
    HplSpheref => Spheref,
    HplRect2f => Rect2f,
    HplRect2l => Rect2l,
    HplColor => Color,
    HplTriangleData => TriangleData,
    HplTriEdge => TriEdge,
    HplBvExtents => Extents,
}

/// Read a mirrored value through a pointer.
///
/// # Safety
/// `p` must be valid for reads.
#[inline]
pub unsafe fn read<T: Mirror>(p: *const T) -> T::Rust {
    unsafe { p.read() }.to_rs()
}

/// Write a mirrored value through a pointer if it is non-null.
///
/// # Safety
/// `p` must be null or valid for writes.
#[inline]
pub unsafe fn write_opt<T: Mirror>(p: *mut T, v: T::Rust) {
    if !p.is_null() {
        unsafe { p.write(T::from_rs(v)) };
    }
}

/// Write a plain value through a pointer if it is non-null.
///
/// # Safety
/// `p` must be null or valid for writes.
#[inline]
pub unsafe fn write_plain<T>(p: *mut T, v: T) {
    if !p.is_null() {
        unsafe { p.write(v) };
    }
}

/// Borrow a C array of mirrored values as a slice of the Rust type.
/// A null pointer or zero length gives an empty slice.
///
/// # Safety
/// `p` must point to `len` initialised values that outlive `'a`.
#[inline]
pub unsafe fn slice<'a, T: Mirror>(p: *const T, len: usize) -> &'a [T::Rust] {
    if p.is_null() || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(p as *const T::Rust, len) }
    }
}

/// Mutable variant of [`slice`].
///
/// # Safety
/// `p` must point to `len` values, valid for writes and not aliased.
#[inline]
pub unsafe fn slice_mut<'a, T: Mirror>(p: *mut T, len: usize) -> &'a mut [T::Rust] {
    if p.is_null() || len == 0 {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(p as *mut T::Rust, len) }
    }
}

/// Borrow a C array of plain values. Null or zero length gives an empty slice.
///
/// # Safety
/// `p` must point to `len` initialised values that outlive `'a`.
#[inline]
pub unsafe fn raw_slice<'a, T>(p: *const T, len: usize) -> &'a [T] {
    if p.is_null() || len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(p, len) } }
}

/// Number of floats a vertex array must hold for the given indices.
#[inline]
pub fn vertex_floats_for(indices: &[u32], stride: usize) -> usize {
    indices.iter().max().map_or(0, |&m| m as usize * stride + 3)
}

impl HplFrustumProjection {
    pub fn to_rs(&self) -> FrustumProjection {
        FrustumProjection {
            proj: self.proj.to_rs(),
            view: self.view.to_rs(),
            near_plane: self.near_plane,
            far_plane: self.far_plane,
            fov: self.fov,
            aspect: self.aspect,
            projection_type: if self.projection_type == 1 { ProjectionType::Orthographic } else { ProjectionType::Perspective },
        }
    }
}

impl HplFrustumView {
    /// # Safety
    /// `planes` must point to 6 planes and `vertices` to 8 vectors.
    pub unsafe fn as_ref<'a>(&self) -> FrustumRef<'a> {
        unsafe {
            FrustumRef {
                planes: &*(self.planes as *const [Planef; 6]),
                vertices: &*(self.vertices as *const [Vector3f; 8]),
                sphere: self.sphere.to_rs(),
                bv_min: self.bv_min.to_rs(),
                bv_max: self.bv_max.to_rs(),
                inf_far_plane: self.inf_far_plane,
            }
        }
    }
}
