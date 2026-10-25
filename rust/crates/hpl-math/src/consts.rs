//! Constants from `math/MathTypes.h`.
//!
//! These are the engine's own literals (not `std::f32::consts`) so values are
//! identical to the C++ `#define`s.

#![allow(clippy::approx_constant, clippy::excessive_precision)]

/// `kPif`
pub const PI: f32 = 3.141592654;
/// `kPi2f` (pi / 2)
pub const PI_2: f32 = 1.570796327;
/// `kPi4f` (pi / 4)
pub const PI_4: f32 = 0.7853981634;
/// `k2Pif` (2 pi)
pub const TWO_PI: f32 = 6.283185307;
/// `kEpsilonf`
pub const EPSILON: f32 = 0.0001;
/// `kSqrt2f`
pub const SQRT_2: f32 = 1.414213562;
