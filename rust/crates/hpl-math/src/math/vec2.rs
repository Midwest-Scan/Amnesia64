//! VECTOR 2D

use super::scalar::{get_angle_from_points_2d, increase_to, max_f, min_f, to_deg, to_rad};
use crate::vector::{Vector2f, Vector3f};

/// `Vector2Dist`
#[inline]
pub fn vector2_dist(a: Vector2f, b: Vector2f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// `Vector2DistXY`
#[inline]
pub fn vector2_dist_xy(a: Vector3f, b: Vector3f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// `Vector2DistXZ`
#[inline]
pub fn vector2_dist_xz(a: Vector3f, b: Vector3f) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    (dx * dx + dz * dz).sqrt()
}

/// `Vector2DistSqr`
#[inline]
pub fn vector2_dist_sqr(a: Vector2f, b: Vector2f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// `Vector2DistSqrXY`
#[inline]
pub fn vector2_dist_sqr_xy(a: Vector3f, b: Vector3f) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// `Vector2DistSqrXZ`
#[inline]
pub fn vector2_dist_sqr_xz(a: Vector3f, b: Vector3f) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    dx * dx + dz * dz
}

/// `Vector2IncreaseTo`
#[inline]
pub fn vector2_increase_to(x: Vector2f, add: Vector2f, dest: Vector2f) -> Vector2f {
    Vector2f::new(increase_to(x.x, add.x, dest.x), increase_to(x.y, add.y, dest.y))
}

/// `GetVectorFromAngle2D`: angle 0 points along -y.
#[inline]
pub fn get_vector_from_angle_2d(angle: f32, length: f32) -> Vector2f {
    Vector2f::new(angle.sin() * length, -angle.cos() * length)
}

/// `GetAngleFromVector`: returns `(angle, length)`.
#[inline]
pub fn get_angle_from_vector(v: Vector2f) -> (f32, f32) {
    let length = (v.x * v.x + v.y * v.y).sqrt();
    let angle = get_angle_from_points_2d(Vector2f::splat(0.0), v);
    (angle, length)
}

/// `ProjectVector2D`: project `src` onto `dest`.
#[inline]
pub fn project_vector_2d(src: Vector2f, dest: Vector2f) -> Vector2f {
    let t = (src.x * dest.x + src.y * dest.y) / (dest.x * dest.x + dest.y * dest.y);
    dest * t
}

/// `Vector2Floor`
#[inline]
pub fn vector2_floor(v: Vector2f) -> Vector2f {
    Vector2f::new(v.x.floor(), v.y.floor())
}

/// `Vector2Ceil`
#[inline]
pub fn vector2_ceil(v: Vector2f) -> Vector2f {
    Vector2f::new(v.x.ceil(), v.y.ceil())
}

/// `Vector2Abs`
#[inline]
pub fn vector2_abs(v: Vector2f) -> Vector2f {
    Vector2f::new(v.x.abs(), v.y.abs())
}

/// `Vector2Min`
#[inline]
pub fn vector2_min(a: Vector2f, b: Vector2f) -> Vector2f {
    Vector2f::new(min_f(a.x, b.x), min_f(a.y, b.y))
}

/// `Vector2Max`
#[inline]
pub fn vector2_max(a: Vector2f, b: Vector2f) -> Vector2f {
    Vector2f::new(max_f(a.x, b.x), max_f(a.y, b.y))
}

/// `Vector2Less`: every component strictly less.
#[inline]
pub fn vector2_less(v: Vector2f, cmp: Vector2f) -> bool {
    !(v.x >= cmp.x || v.y >= cmp.y)
}

/// `Vector2Greater`
#[inline]
pub fn vector2_greater(v: Vector2f, cmp: Vector2f) -> bool {
    !(v.x <= cmp.x || v.y <= cmp.y)
}

/// `Vector2LessEqual`
#[inline]
pub fn vector2_less_equal(v: Vector2f, cmp: Vector2f) -> bool {
    !(v.x > cmp.x || v.y > cmp.y)
}

/// `Vector2GreaterEqual`
#[inline]
pub fn vector2_greater_equal(v: Vector2f, cmp: Vector2f) -> bool {
    !(v.x < cmp.x || v.y < cmp.y)
}

/// `Vector2Normalize`
#[inline]
pub fn vector2_normalize(v: Vector2f) -> Vector2f {
    v.normalized()
}

/// `Vector2ToRad`
#[inline]
pub fn vector2_to_rad(v: Vector2f) -> Vector2f {
    Vector2f::new(to_rad(v.x), to_rad(v.y))
}

/// `Vector2ToDeg`
#[inline]
pub fn vector2_to_deg(v: Vector2f) -> Vector2f {
    Vector2f::new(to_deg(v.x), to_deg(v.y))
}
