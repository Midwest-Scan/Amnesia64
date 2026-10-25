//! `cMatrixf` (`math/Matrix.h`). Row-major `m[row][col]`, translation in
//! column 3, same memory layout as the C++ class.

use crate::vector::Vector3f;
use std::fmt;
use std::ops::Mul;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrixf {
    pub m: [[f32; 4]; 4],
}

impl Default for Matrixf {
    /// Identity. (The C++ default constructor leaves the matrix
    /// uninitialised; every engine call site assigns before reading.)
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrixf {
    /// `cMatrixf::Identity`
    pub const IDENTITY: Self = Self::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// `cMatrixf::Zero`
    pub const ZERO: Self = Self::from_rows([[0.0; 4]; 4]);

    #[inline]
    pub const fn from_rows(m: [[f32; 4]; 4]) -> Self {
        Self { m }
    }

    /// The 16-argument C++ constructor, in row order.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub const fn new(
        a00: f32, a01: f32, a02: f32, a03: f32,
        a10: f32, a11: f32, a12: f32, a13: f32,
        a20: f32, a21: f32, a22: f32, a23: f32,
        a30: f32, a31: f32, a32: f32, a33: f32,
    ) -> Self {
        Self { m: [[a00, a01, a02, a03], [a10, a11, a12, a13], [a20, a21, a22, a23], [a30, a31, a32, a33]] }
    }

    /// `FromVec` (row-major array of 16).
    #[inline]
    pub fn from_slice(v: &[f32]) -> Self {
        let mut m = [[0.0; 4]; 4];
        for (r, row) in m.iter_mut().enumerate() {
            row.copy_from_slice(&v[r * 4..r * 4 + 4]);
        }
        Self { m }
    }

    /// `FromTranspose` (column-major array of 16, e.g. from OpenGL).
    #[inline]
    pub fn from_transpose(v: &[f32]) -> Self {
        Self::from_slice(v).transpose()
    }

    /// The 16 floats in row-major order (`v[]` in C++).
    #[inline]
    pub fn as_array(&self) -> &[f32; 16] {
        // SAFETY: [[f32; 4]; 4] and [f32; 16] have identical layout.
        unsafe { &*(self.m.as_ptr() as *const [f32; 16]) }
    }

    #[inline]
    pub fn right(&self) -> Vector3f {
        Vector3f::new(self.m[0][0], self.m[0][1], self.m[0][2])
    }
    #[inline]
    pub fn set_right(&mut self, v: Vector3f) {
        self.m[0][0] = v.x;
        self.m[0][1] = v.y;
        self.m[0][2] = v.z;
    }
    #[inline]
    pub fn up(&self) -> Vector3f {
        Vector3f::new(self.m[1][0], self.m[1][1], self.m[1][2])
    }
    #[inline]
    pub fn set_up(&mut self, v: Vector3f) {
        self.m[1][0] = v.x;
        self.m[1][1] = v.y;
        self.m[1][2] = v.z;
    }
    #[inline]
    pub fn forward(&self) -> Vector3f {
        Vector3f::new(self.m[2][0], self.m[2][1], self.m[2][2])
    }
    #[inline]
    pub fn set_forward(&mut self, v: Vector3f) {
        self.m[2][0] = v.x;
        self.m[2][1] = v.y;
        self.m[2][2] = v.z;
    }
    #[inline]
    pub fn translation(&self) -> Vector3f {
        Vector3f::new(self.m[0][3], self.m[1][3], self.m[2][3])
    }
    #[inline]
    pub fn set_translation(&mut self, v: Vector3f) {
        self.m[0][3] = v.x;
        self.m[1][3] = v.y;
        self.m[2][3] = v.z;
    }

    /// `GetRotation`: upper 3x3, rest identity.
    #[inline]
    pub fn rotation(&self) -> Self {
        let m = &self.m;
        Self::new(
            m[0][0], m[0][1], m[0][2], 0.0,
            m[1][0], m[1][1], m[1][2], 0.0,
            m[2][0], m[2][1], m[2][2], 0.0,
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `GetTranspose`
    #[inline]
    pub fn transpose(&self) -> Self {
        let m = &self.m;
        Self::new(
            m[0][0], m[1][0], m[2][0], m[3][0],
            m[0][1], m[1][1], m[2][1], m[3][1],
            m[0][2], m[1][2], m[2][2], m[3][2],
            m[0][3], m[1][3], m[2][3], m[3][3],
        )
    }
}

impl Mul for Matrixf {
    type Output = Matrixf;
    /// `cMath::MatrixMul(A, B)`: B is applied before A.
    #[inline]
    fn mul(self, rhs: Matrixf) -> Matrixf {
        crate::math::matrix_mul(&self, &rhs)
    }
}

impl Mul<Vector3f> for Matrixf {
    type Output = Vector3f;
    /// `cMath::MatrixMul(A, v)` (point transform, w = 1).
    #[inline]
    fn mul(self, rhs: Vector3f) -> Vector3f {
        crate::math::matrix_mul_vec3(&self, rhs)
    }
}

impl fmt::Display for Matrixf {
    /// Same text as C++ `ToString()`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, r) in self.m.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "[{:.6} : {:.6} : {:.6} : {:.6}]", r[0], r[1], r[2], r[3])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_cpp() {
        assert_eq!(std::mem::size_of::<Matrixf>(), 64);
        let mut m = Matrixf::IDENTITY;
        m.set_translation(Vector3f::new(1.0, 2.0, 3.0));
        assert_eq!(m.as_array()[3], 1.0);
        assert_eq!(m.as_array()[7], 2.0);
        assert_eq!(m.as_array()[11], 3.0);
    }

    #[test]
    fn transpose_roundtrip() {
        let v: Vec<f32> = (0..16).map(|i| i as f32).collect();
        let m = Matrixf::from_slice(&v);
        assert_eq!(m.transpose().transpose(), m);
        assert_eq!(Matrixf::from_transpose(&v).m[1][0], 1.0);
    }
}
