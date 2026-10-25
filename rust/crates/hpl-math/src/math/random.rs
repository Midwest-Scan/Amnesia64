//! RANDOM GENERATION
//!
//! Migration note: these deliberately use the C runtime's `rand`/`srand` so
//! the Rust and remaining C++ code share one generator and the game keeps the
//! exact sequence and distribution (on MSVC, `RAND_MAX` is 32767 and the state
//! is per thread). Swap for a Rust RNG once no C++ caller is left.

use crate::color::Color;
use crate::vector::{Vector2f, Vector3f};
use std::ffi::{c_int, c_uint};

unsafe extern "C" {
    safe fn rand() -> c_int;
    safe fn srand(seed: c_uint);
}

#[cfg(target_env = "msvc")]
const RAND_MAX: c_int = 0x7fff;
#[cfg(not(target_env = "msvc"))]
const RAND_MAX: c_int = 0x7fff_ffff;

/// `RandRectl`: integer in `[min, max]`.
#[inline]
pub fn rand_rect_i(min: i32, max: i32) -> i32 {
    // Faithful: wrapping arithmetic like the C++ (a zero range still divides by zero).
    rand().wrapping_rem(max.wrapping_sub(min).wrapping_add(1)).wrapping_add(min)
}

/// `RandRectf`: float in `[min, max]`.
#[inline]
pub fn rand_rect_f(min: f32, max: f32) -> f32 {
    let r = rand() as f32 / RAND_MAX as f32;
    min + r * (max - min)
}

// Faithful: MSVC and GCC on x64 evaluate constructor arguments right to
// left, so in `cVector3f(RandRectf(..x), RandRectf(..y), RandRectf(..z))` the
// z component draws first. The draws below keep that order so a seeded
// sequence produces the same vectors as the C++ engine.

/// `RandRectVector2f`
#[inline]
pub fn rand_rect_vector2f(min: Vector2f, max: Vector2f) -> Vector2f {
    let y = rand_rect_f(min.y, max.y);
    let x = rand_rect_f(min.x, max.x);
    Vector2f::new(x, y)
}

/// `RandRectVector3f`
#[inline]
pub fn rand_rect_vector3f(min: Vector3f, max: Vector3f) -> Vector3f {
    let z = rand_rect_f(min.z, max.z);
    let y = rand_rect_f(min.y, max.y);
    let x = rand_rect_f(min.x, max.x);
    Vector3f::new(x, y, z)
}

/// `RandRectColor`
#[inline]
pub fn rand_rect_color(min: Color, max: Color) -> Color {
    let a = rand_rect_f(min.a, max.a);
    let b = rand_rect_f(min.b, max.b);
    let g = rand_rect_f(min.g, max.g);
    let r = rand_rect_f(min.r, max.r);
    Color::new(r, g, b, a)
}

/// `RandomSphereSurfacePoint`
#[inline]
pub fn random_sphere_surface_point(radius: f32) -> Vector3f {
    let y = rand_rect_f(0.0, 1.0);
    let x = rand_rect_f(0.0, 1.0);
    super::vector3_sphere_surface_point(Vector2f::new(x, y), radius)
}

/// `RandomSphereInsidePoint` (uniform in volume).
#[inline]
pub fn random_sphere_inside_point(radius: f32) -> Vector3f {
    let r = rand_rect_f(0.0, 1.0).powf(1.0 / 3.0);
    let y = rand_rect_f(0.0, 1.0);
    let x = rand_rect_f(0.0, 1.0);
    super::vector3_sphere_surface_point(Vector2f::new(x, y), radius * r)
}

/// `Randomize`: seed the generator; `-1` seeds from the clock.
pub fn randomize(seed: i32) {
    if seed == -1 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        srand(now as c_uint);
    } else {
        srand(seed as c_uint);
    }
}
