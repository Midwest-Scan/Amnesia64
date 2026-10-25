//! `cSpring<T>` (`math/Spring.h`): damped spring force.

use std::ops::{Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring {
    /// Stiffness.
    pub k: f32,
    /// Damping.
    pub b: f32,
}

impl Spring {
    pub const fn new(k: f32, b: f32) -> Self {
        Self { k, b }
    }

    /// `Output`: `error * k - velocity * b`.
    #[inline]
    pub fn output<T>(&self, error: T, velocity: T) -> T
    where
        T: Mul<f32, Output = T> + Sub<Output = T>,
    {
        error * self.k - velocity * self.b
    }
}
