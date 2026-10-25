//! `cVector2<T>` and `cVector3<T>` (`math/Vector2.h`, `math/Vector3.h`).
//!
//! Layout is `#[repr(C)]` and identical to the C++ classes, so slices of these
//! can be shared with C++ without copying.

use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Sub, SubAssign};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

pub type Vector2f = Vector2<f32>;
pub type Vector2l = Vector2<i32>;
pub type Vector3f = Vector3<f32>;
pub type Vector3l = Vector3<i32>;

impl<T> Vector2<T> {
    #[inline]
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Copy> Vector2<T> {
    /// `cVector2(T afVal)`
    #[inline]
    pub const fn splat(v: T) -> Self {
        Self { x: v, y: v }
    }

    /// `FromVec`
    #[inline]
    pub fn from_slice(v: &[T]) -> Self {
        Self { x: v[0], y: v[1] }
    }

    #[inline]
    pub fn to_array(self) -> [T; 2] {
        [self.x, self.y]
    }
}

impl<T> Vector3<T> {
    #[inline]
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T: Copy> Vector3<T> {
    /// `cVector3(T afVal)`
    #[inline]
    pub const fn splat(v: T) -> Self {
        Self { x: v, y: v, z: v }
    }

    /// `FromVec`
    #[inline]
    pub fn from_slice(v: &[T]) -> Self {
        Self { x: v[0], y: v[1], z: v[2] }
    }

    #[inline]
    pub fn to_array(self) -> [T; 3] {
        [self.x, self.y, self.z]
    }
}

impl<T: Copy + Default> From<Vector2<T>> for Vector3<T> {
    /// `cVector3(const cVector2<T>&)`: z is zero.
    #[inline]
    fn from(v: Vector2<T>) -> Self {
        Self { x: v.x, y: v.y, z: T::default() }
    }
}

impl<T: PartialOrd + Copy> Vector3<T> {
    /// C++ `operator<`: lexicographic on x, y, z using `!=` then `<`.
    #[inline]
    pub fn lex_lt(&self, o: &Self) -> bool {
        if self.x != o.x {
            return self.x < o.x;
        }
        if self.y != o.y {
            return self.y < o.y;
        }
        self.z < o.z
    }

    /// C++ `operator>`.
    #[inline]
    pub fn lex_gt(&self, o: &Self) -> bool {
        if self.x != o.x {
            return self.x > o.x;
        }
        if self.y != o.y {
            return self.y > o.y;
        }
        self.z > o.z
    }
}

impl<T> Index<usize> for Vector2<T> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        match i {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Vector2 index {i} out of range"),
        }
    }
}

impl<T> IndexMut<usize> for Vector2<T> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        match i {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Vector2 index {i} out of range"),
        }
    }
}

impl<T> Index<usize> for Vector3<T> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vector3 index {i} out of range"),
        }
    }
}

impl<T> IndexMut<usize> for Vector3<T> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        match i {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("Vector3 index {i} out of range"),
        }
    }
}

macro_rules! vec_ops {
    ($t:ty) => {
        // ---- Vector2 ----
        impl Add for Vector2<$t> { type Output = Self; #[inline] fn add(self, o: Self) -> Self { Self::new(self.x + o.x, self.y + o.y) } }
        impl Sub for Vector2<$t> { type Output = Self; #[inline] fn sub(self, o: Self) -> Self { Self::new(self.x - o.x, self.y - o.y) } }
        impl Mul for Vector2<$t> { type Output = Self; #[inline] fn mul(self, o: Self) -> Self { Self::new(self.x * o.x, self.y * o.y) } }
        impl Div for Vector2<$t> { type Output = Self; #[inline] fn div(self, o: Self) -> Self { Self::new(self.x / o.x, self.y / o.y) } }
        impl Add<$t> for Vector2<$t> { type Output = Self; #[inline] fn add(self, v: $t) -> Self { Self::new(self.x + v, self.y + v) } }
        impl Sub<$t> for Vector2<$t> { type Output = Self; #[inline] fn sub(self, v: $t) -> Self { Self::new(self.x - v, self.y - v) } }
        impl Mul<$t> for Vector2<$t> { type Output = Self; #[inline] fn mul(self, v: $t) -> Self { Self::new(self.x * v, self.y * v) } }
        impl Div<$t> for Vector2<$t> { type Output = Self; #[inline] fn div(self, v: $t) -> Self { Self::new(self.x / v, self.y / v) } }
        impl AddAssign for Vector2<$t> { #[inline] fn add_assign(&mut self, o: Self) { self.x += o.x; self.y += o.y; } }
        impl SubAssign for Vector2<$t> { #[inline] fn sub_assign(&mut self, o: Self) { self.x -= o.x; self.y -= o.y; } }
        impl MulAssign for Vector2<$t> { #[inline] fn mul_assign(&mut self, o: Self) { self.x *= o.x; self.y *= o.y; } }
        impl DivAssign for Vector2<$t> { #[inline] fn div_assign(&mut self, o: Self) { self.x /= o.x; self.y /= o.y; } }

        // ---- Vector3 ----
        impl Add for Vector3<$t> { type Output = Self; #[inline] fn add(self, o: Self) -> Self { Self::new(self.x + o.x, self.y + o.y, self.z + o.z) } }
        impl Sub for Vector3<$t> { type Output = Self; #[inline] fn sub(self, o: Self) -> Self { Self::new(self.x - o.x, self.y - o.y, self.z - o.z) } }
        impl Mul for Vector3<$t> { type Output = Self; #[inline] fn mul(self, o: Self) -> Self { Self::new(self.x * o.x, self.y * o.y, self.z * o.z) } }
        impl Div for Vector3<$t> { type Output = Self; #[inline] fn div(self, o: Self) -> Self { Self::new(self.x / o.x, self.y / o.y, self.z / o.z) } }
        impl Add<$t> for Vector3<$t> { type Output = Self; #[inline] fn add(self, v: $t) -> Self { Self::new(self.x + v, self.y + v, self.z + v) } }
        impl Sub<$t> for Vector3<$t> { type Output = Self; #[inline] fn sub(self, v: $t) -> Self { Self::new(self.x - v, self.y - v, self.z - v) } }
        impl Mul<$t> for Vector3<$t> { type Output = Self; #[inline] fn mul(self, v: $t) -> Self { Self::new(self.x * v, self.y * v, self.z * v) } }
        impl Div<$t> for Vector3<$t> { type Output = Self; #[inline] fn div(self, v: $t) -> Self { Self::new(self.x / v, self.y / v, self.z / v) } }
        impl AddAssign for Vector3<$t> { #[inline] fn add_assign(&mut self, o: Self) { self.x += o.x; self.y += o.y; self.z += o.z; } }
        impl SubAssign for Vector3<$t> { #[inline] fn sub_assign(&mut self, o: Self) { self.x -= o.x; self.y -= o.y; self.z -= o.z; } }
        impl MulAssign for Vector3<$t> { #[inline] fn mul_assign(&mut self, o: Self) { self.x *= o.x; self.y *= o.y; self.z *= o.z; } }
        impl DivAssign for Vector3<$t> { #[inline] fn div_assign(&mut self, o: Self) { self.x /= o.x; self.y /= o.y; self.z /= o.z; } }

        // ---- Vector3 (op) Vector2: z is passed through ----
        impl Add<Vector2<$t>> for Vector3<$t> { type Output = Self; #[inline] fn add(self, o: Vector2<$t>) -> Self { Self::new(self.x + o.x, self.y + o.y, self.z) } }
        impl Sub<Vector2<$t>> for Vector3<$t> { type Output = Self; #[inline] fn sub(self, o: Vector2<$t>) -> Self { Self::new(self.x - o.x, self.y - o.y, self.z) } }
        impl Mul<Vector2<$t>> for Vector3<$t> { type Output = Self; #[inline] fn mul(self, o: Vector2<$t>) -> Self { Self::new(self.x * o.x, self.y * o.y, self.z) } }
        impl Div<Vector2<$t>> for Vector3<$t> { type Output = Self; #[inline] fn div(self, o: Vector2<$t>) -> Self { Self::new(self.x / o.x, self.y / o.y, self.z) } }
        impl AddAssign<Vector2<$t>> for Vector3<$t> { #[inline] fn add_assign(&mut self, o: Vector2<$t>) { self.x += o.x; self.y += o.y; } }
        impl SubAssign<Vector2<$t>> for Vector3<$t> { #[inline] fn sub_assign(&mut self, o: Vector2<$t>) { self.x -= o.x; self.y -= o.y; } }
        impl MulAssign<Vector2<$t>> for Vector3<$t> { #[inline] fn mul_assign(&mut self, o: Vector2<$t>) { self.x *= o.x; self.y *= o.y; } }
        impl DivAssign<Vector2<$t>> for Vector3<$t> { #[inline] fn div_assign(&mut self, o: Vector2<$t>) { self.x /= o.x; self.y /= o.y; } }

        impl Vector2<$t> {
            /// `SqrLength`
            #[inline]
            pub fn sqr_length(&self) -> $t { self.x * self.x + self.y * self.y }
        }

        impl Vector3<$t> {
            /// `SqrLength`
            #[inline]
            pub fn sqr_length(&self) -> $t { self.x * self.x + self.y * self.y + self.z * self.z }
        }
    };
}

vec_ops!(f32);
vec_ops!(f64);
vec_ops!(i32);

macro_rules! vec_float {
    ($t:ty) => {
        impl Vector2<$t> {
            /// `Length`
            #[inline]
            pub fn length(&self) -> $t {
                (self.x * self.x + self.y * self.y).sqrt()
            }

            /// `Normalize`: normalises in place and returns the old length.
            /// Vectors shorter than 1e-8 are left unchanged.
            #[inline]
            pub fn normalize(&mut self) -> $t {
                let length = (self.x * self.x + self.y * self.y).sqrt();
                // Faithful: the C++ compares against the double literal 1e-08.
                if (length as f64) > 1e-08 {
                    let inv_length = 1.0 / length;
                    self.x *= inv_length;
                    self.y *= inv_length;
                }
                length
            }

            /// Copy of `self`, normalised.
            #[inline]
            pub fn normalized(mut self) -> Self {
                self.normalize();
                self
            }
        }

        impl Vector3<$t> {
            /// `Length`
            #[inline]
            pub fn length(&self) -> $t {
                (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
            }

            /// `Normalize`: normalises in place and returns the old length.
            /// Vectors shorter than 1e-8 are left unchanged.
            #[inline]
            pub fn normalize(&mut self) -> $t {
                let length = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
                if (length as f64) > 1e-08 {
                    let inv_length = 1.0 / length;
                    self.x *= inv_length;
                    self.y *= inv_length;
                    self.z *= inv_length;
                }
                length
            }

            /// Copy of `self`, normalised.
            #[inline]
            pub fn normalized(mut self) -> Self {
                self.normalize();
                self
            }
        }
    };
}

vec_float!(f32);
vec_float!(f64);

impl fmt::Display for Vector2<f32> {
    /// Same text as C++ `ToString()` (`"%f : %f"`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6} : {:.6}", self.x, self.y)
    }
}

impl fmt::Display for Vector3<f32> {
    /// Same text as C++ `ToString()` (`"%f : %f : %f"`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6} : {:.6} : {:.6}", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_cpp() {
        assert_eq!(std::mem::size_of::<Vector2f>(), 8);
        assert_eq!(std::mem::size_of::<Vector3f>(), 12);
        assert_eq!(std::mem::size_of::<Vector3l>(), 12);
        assert_eq!(std::mem::align_of::<Vector3f>(), 4);
    }

    #[test]
    fn normalize_skips_tiny_vectors() {
        let mut v = Vector3f::new(1e-9, 0.0, 0.0);
        assert_eq!(v.normalize(), 1e-9);
        assert_eq!(v, Vector3f::new(1e-9, 0.0, 0.0));
        let mut w = Vector3f::new(3.0, 0.0, 4.0);
        assert_eq!(w.normalize(), 5.0);
        assert_eq!(w, Vector3f::new(0.6, 0.0, 0.8));
    }

    #[test]
    fn lexicographic_order() {
        let a = Vector3f::new(1.0, 2.0, 3.0);
        let b = Vector3f::new(1.0, 2.0, 4.0);
        assert!(a.lex_lt(&b));
        assert!(b.lex_gt(&a));
        assert!(!a.lex_lt(&a));
    }

    #[test]
    fn mixed_vector3_vector2_ops_keep_z() {
        let v = Vector3f::new(1.0, 2.0, 3.0) + Vector2f::new(1.0, 1.0);
        assert_eq!(v, Vector3f::new(2.0, 3.0, 3.0));
    }
}
