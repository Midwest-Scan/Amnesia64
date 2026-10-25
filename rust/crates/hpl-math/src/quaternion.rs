//! `cQuaternion` (`math/Quaternion.h`, `sources/math/Quaternion.cpp`).
//! Layout: `v` (x, y, z) followed by `w`, as in C++.

use crate::matrix::Matrixf;
use crate::vector::Vector3f;
use std::ops::{Add, Div, Mul, Sub};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quaternion {
    pub v: Vector3f,
    pub w: f32,
}

impl Quaternion {
    /// `cQuaternion::Identity`
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 0.0);

    /// `cQuaternion(afW, afX, afY, afZ)`
    #[inline]
    pub const fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self { v: Vector3f::new(x, y, z), w }
    }

    /// `cQuaternion(afAngle, avAxis)`. The axis must be unit length.
    #[inline]
    pub fn from_angle_axis(angle: f32, axis: Vector3f) -> Self {
        let half_angle = 0.5 * angle;
        let sin = half_angle.sin();
        Self { w: half_angle.cos(), v: Vector3f::new(sin * axis.x, sin * axis.y, sin * axis.z) }
    }

    /// `Normalize`
    #[inline]
    pub fn normalize(&mut self) {
        let len = self.w * self.w + self.v.x * self.v.x + self.v.y * self.v.y + self.v.z * self.v.z;
        let factor = 1.0 / len.sqrt();
        self.v = self.v * factor;
        self.w *= factor;
    }

    /// Copy of `self`, normalised.
    #[inline]
    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }

    /// `ToRotationMatrix`. Faithful: only the upper 3x3 of `dest` is written.
    pub fn to_rotation_matrix(&self, dest: &mut Matrixf) {
        let v = self.v;
        let w = self.w;
        let tx = 2.0 * v.x;
        let ty = 2.0 * v.y;
        let tz = 2.0 * v.z;
        let twx = tx * w;
        let twy = ty * w;
        let twz = tz * w;
        let txx = tx * v.x;
        let txy = ty * v.x;
        let txz = tz * v.x;
        let tyy = ty * v.y;
        let tyz = tz * v.y;
        let tzz = tz * v.z;

        let m = &mut dest.m;
        m[0][0] = 1.0 - (tyy + tzz);
        m[0][1] = txy - twz;
        m[0][2] = txz + twy;
        m[1][0] = txy + twz;
        m[1][1] = 1.0 - (txx + tzz);
        m[1][2] = tyz - twx;
        m[2][0] = txz - twy;
        m[2][1] = tyz + twx;
        m[2][2] = 1.0 - (txx + tyy);
    }

    /// `FromRotationMatrix` / `cQuaternion(const cMatrixf&)`.
    pub fn from_rotation_matrix(rot: &Matrixf) -> Self {
        let m = &rot.m;
        let trace = m[0][0] + m[1][1] + m[2][2];
        let mut q = Quaternion::default();
        if trace > 0.0 {
            // |w| > 1/2, may as well choose w > 1/2
            let mut root = (trace + 1.0).sqrt(); // 2w
            q.w = 0.5 * root;
            root = 0.5 / root; // 1/(4w)
            q.v.x = (m[2][1] - m[1][2]) * root;
            q.v.y = (m[0][2] - m[2][0]) * root;
            q.v.z = (m[1][0] - m[0][1]) * root;
        } else {
            // |w| <= 1/2
            const NEXT: [usize; 3] = [1, 2, 0];
            let mut i = 0;
            if m[1][1] > m[0][0] {
                i = 1;
            }
            if m[2][2] > m[i][i] {
                i = 2;
            }
            let j = NEXT[i];
            let k = NEXT[j];

            let mut root = (m[i][i] - m[j][j] - m[k][k] + 1.0).sqrt();
            q.v[i] = 0.5 * root;
            root = 0.5 / root;
            q.w = (m[k][j] - m[j][k]) * root;
            q.v[j] = (m[j][i] + m[i][j]) * root;
            q.v[k] = (m[k][i] + m[i][k]) * root;
        }
        q
    }
}

impl Add for Quaternion {
    type Output = Self;
    #[inline]
    fn add(self, b: Self) -> Self {
        Self { v: self.v + b.v, w: self.w + b.w }
    }
}

impl Sub for Quaternion {
    type Output = Self;
    #[inline]
    fn sub(self, b: Self) -> Self {
        Self { v: self.v - b.v, w: self.w - b.w }
    }
}

impl Mul for Quaternion {
    type Output = Self;
    /// Hamilton product (`operator*`).
    #[inline]
    fn mul(self, b: Self) -> Self {
        let (a, w) = (self.v, self.w);
        Self {
            w: w * b.w - a.x * b.v.x - a.y * b.v.y - a.z * b.v.z,
            v: Vector3f::new(
                w * b.v.x + a.x * b.w + a.y * b.v.z - a.z * b.v.y,
                w * b.v.y + a.y * b.w + a.z * b.v.x - a.x * b.v.z,
                w * b.v.z + a.z * b.w + a.x * b.v.y - a.y * b.v.x,
            ),
        }
    }
}

impl Div for Quaternion {
    type Output = Self;
    /// Faithful: component-wise division, as in the C++ `operator/`.
    #[inline]
    fn div(self, b: Self) -> Self {
        Self { v: self.v / b.v, w: self.w / b.w }
    }
}

impl Mul<f32> for Quaternion {
    type Output = Self;
    #[inline]
    fn mul(self, s: f32) -> Self {
        Self { v: self.v * s, w: self.w * s }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_cpp() {
        assert_eq!(std::mem::size_of::<Quaternion>(), 16);
        assert_eq!(std::mem::offset_of!(Quaternion, w), 12);
    }

    #[test]
    fn matrix_roundtrip() {
        let q = Quaternion::from_angle_axis(0.7, Vector3f::new(0.0, 1.0, 0.0));
        let mut m = Matrixf::IDENTITY;
        q.to_rotation_matrix(&mut m);
        let q2 = Quaternion::from_rotation_matrix(&m);
        assert!((q.w - q2.w).abs() < 1e-6);
        assert!((q.v.y - q2.v.y).abs() < 1e-6);
    }
}
