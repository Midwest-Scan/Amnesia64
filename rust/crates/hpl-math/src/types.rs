//! Small types from `math/MathTypes.h`: planes, spheres, rects, enums and
//! the oscillation/interpolation helpers.

use crate::vector::{Vector2, Vector3f};
use std::collections::VecDeque;
use std::ops::{Add, Div, Mul};

/// `eEulerRotationOrder`
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EulerRotationOrder {
    Xyz = 0,
    Xzy = 1,
    Yxz = 2,
    Yzx = 3,
    Zxy = 4,
    Zyx = 5,
}

impl EulerRotationOrder {
    pub fn from_i32(v: i32) -> Option<Self> {
        Some(match v {
            0 => Self::Xyz,
            1 => Self::Xzy,
            2 => Self::Yxz,
            3 => Self::Yzx,
            4 => Self::Zxy,
            5 => Self::Zyx,
            _ => return None,
        })
    }
}

/// `eFrustumPlane` (index into a frustum's plane array).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrustumPlane {
    Left = 0,
    Right = 1,
    Bottom = 2,
    Top = 3,
    Near = 4,
    Far = 5,
}

/// `eCollision`
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Collision {
    Inside = 0,
    Outside = 1,
    Intersect = 2,
}

/// `cPlanef`: `a*x + b*y + c*z + d = 0`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Planef {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
}

impl Planef {
    #[inline]
    pub const fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        Self { a, b, c, d }
    }

    /// `cPlane(normal, point)` / `FromNormalPoint`. The normal is not normalised.
    #[inline]
    pub fn from_normal_point(normal: Vector3f, point: Vector3f) -> Self {
        let (a, b, c) = (normal.x, normal.y, normal.z);
        Self { a, b, c, d: -(a * point.x + b * point.y + c * point.z) }
    }

    /// `cPlane(p0, p1, p2)` / `FromPoints`. Result is normalised.
    #[inline]
    pub fn from_points(p0: Vector3f, p1: Vector3f, p2: Vector3f) -> Self {
        let e1 = p1 - p0;
        let e2 = p2 - p0;
        let a = e1.y * e2.z - e1.z * e2.y;
        let b = e1.z * e2.x - e1.x * e2.z;
        let c = e1.x * e2.y - e1.y * e2.x;
        let d = -(a * p0.x + b * p0.y + c * p0.z);
        let mut plane = Self { a, b, c, d };
        plane.normalize();
        plane
    }

    /// `Normalize`
    #[inline]
    pub fn normalize(&mut self) {
        let mag = (self.a * self.a + self.b * self.b + self.c * self.c).sqrt();
        self.a /= mag;
        self.b /= mag;
        self.c /= mag;
        self.d /= mag;
    }

    /// `GetNormal`
    #[inline]
    pub fn normal(&self) -> Vector3f {
        Vector3f::new(self.a, self.b, self.c)
    }

    /// `FromVec`
    #[inline]
    pub fn from_slice(v: &[f32]) -> Self {
        Self { a: v[0], b: v[1], c: v[2], d: v[3] }
    }
}

impl Mul<f32> for Planef {
    type Output = Self;
    #[inline]
    fn mul(self, v: f32) -> Self {
        Self { a: self.a * v, b: self.b * v, c: self.c * v, d: self.d * v }
    }
}

/// `cSpheref`. Faithful field order: radius first, then centre.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spheref {
    pub r: f32,
    pub center: Vector3f,
}

impl Spheref {
    #[inline]
    pub const fn new(center: Vector3f, r: f32) -> Self {
        Self { r, center }
    }
}

/// `cRect2<T>`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect2<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

pub type Rect2f = Rect2<f32>;
pub type Rect2l = Rect2<i32>;

impl<T: Copy> Rect2<T> {
    #[inline]
    pub const fn new(x: T, y: T, w: T, h: T) -> Self {
        Self { x, y, w, h }
    }

    #[inline]
    pub fn from_pos_size(pos: Vector2<T>, size: Vector2<T>) -> Self {
        Self { x: pos.x, y: pos.y, w: size.x, h: size.y }
    }
}

/// `cLinearOscillation`: a value ping-ponging between `min` and `max`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LinearOscillation {
    pub max: f32,
    pub min: f32,
    pub val: f32,
    pub up_speed: f32,
    pub down_speed: f32,
    add: f32,
}

impl LinearOscillation {
    pub fn new(min: f32, max: f32, val: f32, up_speed: f32, down_speed: f32) -> Self {
        let mut o = Self::default();
        o.set_up(min, max, val, up_speed, down_speed);
        o
    }

    /// `SetUp`
    pub fn set_up(&mut self, min: f32, max: f32, val: f32, up_speed: f32, down_speed: f32) {
        self.min = min;
        self.max = max;
        self.val = val;
        self.up_speed = up_speed.abs();
        self.down_speed = -down_speed.abs();
        self.add = self.up_speed;
    }

    /// `Update`
    pub fn update(&mut self, time_step: f32) {
        self.val += self.add * time_step;
        if self.add > 0.0 {
            if self.val >= self.max {
                self.val = self.max;
                self.add = self.down_speed;
            }
        } else if self.val <= self.min {
            self.val = self.min;
            self.add = self.up_speed;
        }
    }
}

/// `cSmoothInterpolation<T>`: moving average over the last N values.
#[derive(Clone, Debug)]
pub struct SmoothInterpolation<T> {
    max_values: usize,
    values: VecDeque<T>,
}

impl<T> Default for SmoothInterpolation<T> {
    fn default() -> Self {
        Self { max_values: 1, values: VecDeque::new() }
    }
}

impl<T> SmoothInterpolation<T>
where
    T: Copy + Default + Add<Output = T> + Div<f32, Output = T>,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetMaxValues`
    pub fn set_max_values(&mut self, max: usize) {
        self.max_values = max;
    }

    /// `Add`
    pub fn add(&mut self, value: T) {
        self.values.push_back(value);
        if self.values.len() > self.max_values {
            self.values.pop_front();
        }
    }

    /// `Clear`
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// `GetOutput`. Faithful: an empty history divides by zero.
    pub fn output(&self) -> T {
        let mut total = T::default();
        for v in &self.values {
            total = total + *v;
        }
        total / self.values.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_cpp() {
        assert_eq!(std::mem::size_of::<Planef>(), 16);
        assert_eq!(std::mem::size_of::<Spheref>(), 16);
        assert_eq!(std::mem::offset_of!(Spheref, center), 4);
        assert_eq!(std::mem::size_of::<Rect2l>(), 16);
    }

    #[test]
    fn plane_from_points_is_normalised() {
        let p = Planef::from_points(
            Vector3f::new(0.0, 0.0, 0.0),
            Vector3f::new(1.0, 0.0, 0.0),
            Vector3f::new(0.0, 0.0, -1.0),
        );
        assert_eq!(p.normal(), Vector3f::new(0.0, 1.0, 0.0));
        assert_eq!(p.d, 0.0);
    }

    #[test]
    fn oscillation_bounces() {
        let mut o = LinearOscillation::new(0.0, 1.0, 0.0, 1.0, 2.0);
        o.update(1.5);
        assert_eq!(o.val, 1.0);
        o.update(0.25);
        assert_eq!(o.val, 0.5);
    }

    #[test]
    fn smooth_interpolation_averages() {
        let mut s = SmoothInterpolation::<f32>::new();
        s.set_max_values(2);
        s.add(1.0);
        s.add(2.0);
        s.add(4.0);
        assert_eq!(s.output(), 3.0);
    }
}
