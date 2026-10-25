//! `cColor` (`graphics/Color.h`). It lives here because the math module's
//! colour conversions use it; the graphics crate will re-export it.

use std::ops::{Add, Div, Mul, Sub};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    /// `cColor()`: opaque black.
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

impl Color {
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// `cColor(r, g, b)`: alpha 1.
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// `cColor(afVal)`: grey, alpha 1.
    #[inline]
    pub const fn grey(v: f32) -> Self {
        Self { r: v, g: v, b: v, a: 1.0 }
    }

    /// `cColor(afVal, afA)`
    #[inline]
    pub const fn grey_alpha(v: f32, a: f32) -> Self {
        Self { r: v, g: v, b: v, a }
    }

    /// `FromVec`
    #[inline]
    pub fn from_slice(v: &[f32]) -> Self {
        Self { r: v[0], g: v[1], b: v[2], a: v[3] }
    }
}

impl Mul<f32> for Color {
    type Output = Self;
    #[inline]
    fn mul(self, v: f32) -> Self {
        Self::new(self.r * v, self.g * v, self.b * v, self.a * v)
    }
}

impl Div<f32> for Color {
    type Output = Self;
    #[inline]
    fn div(self, v: f32) -> Self {
        Self::new(self.r / v, self.g / v, self.b / v, self.a / v)
    }
}

impl Add for Color {
    type Output = Self;
    #[inline]
    fn add(self, o: Self) -> Self {
        Self::new(self.r + o.r, self.g + o.g, self.b + o.b, self.a + o.a)
    }
}

impl Sub for Color {
    type Output = Self;
    #[inline]
    fn sub(self, o: Self) -> Self {
        Self::new(self.r - o.r, self.g - o.g, self.b - o.b, self.a - o.a)
    }
}

impl Mul for Color {
    type Output = Self;
    #[inline]
    fn mul(self, o: Self) -> Self {
        Self::new(self.r * o.r, self.g * o.g, self.b * o.b, self.a * o.a)
    }
}

impl Div for Color {
    type Output = Self;
    #[inline]
    fn div(self, o: Self) -> Self {
        Self::new(self.r / o.r, self.g / o.g, self.b / o.b, self.a / o.a)
    }
}
