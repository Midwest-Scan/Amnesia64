//! VECTOR 3D

use super::matrix_ops::{matrix_inverse, matrix_mul_divide_w, matrix_mul_vec3, matrix_rotate_y};
use super::scalar::{get_angle_distance, get_angle_from_points_2d, increase_to, max_f, min_f, to_deg, to_rad, wrap};
use crate::consts::{EPSILON, TWO_PI};
use crate::matrix::Matrixf;
use crate::types::Rect2f;
use crate::vector::{Vector2f, Vector3f};

/// `Vector3MaxLength`
#[inline]
pub fn vector3_max_length(v: Vector3f, max_length: f32) -> Vector3f {
    let length = v.length();
    if length > max_length { (v / length) * max_length } else { v }
}

/// `Vector3MinLength`
#[inline]
pub fn vector3_min_length(v: Vector3f, min_length: f32) -> Vector3f {
    let length = v.length();
    if length < min_length { (v / length) * min_length } else { v }
}

/// `Vector3ClampLength`
#[inline]
pub fn vector3_clamp_length(v: Vector3f, min_length: f32, max_length: f32) -> Vector3f {
    let length = v.length();
    if length < min_length {
        (v / length) * min_length
    } else if length > max_length {
        (v / length) * max_length
    } else {
        v
    }
}

/// `Vector3ToRad`
#[inline]
pub fn vector3_to_rad(v: Vector3f) -> Vector3f {
    Vector3f::new(to_rad(v.x), to_rad(v.y), to_rad(v.z))
}

/// `Vector3ToDeg`
#[inline]
pub fn vector3_to_deg(v: Vector3f) -> Vector3f {
    Vector3f::new(to_deg(v.x), to_deg(v.y), to_deg(v.z))
}

/// `Vector3AngleDistance`
#[inline]
pub fn vector3_angle_distance(a: Vector3f, b: Vector3f, max_angle: f32) -> Vector3f {
    Vector3f::new(
        get_angle_distance(a.x, b.x, max_angle),
        get_angle_distance(a.y, b.y, max_angle),
        get_angle_distance(a.z, b.z, max_angle),
    )
}

/// `Vector3AngleDistanceRad`
#[inline]
pub fn vector3_angle_distance_rad(a: Vector3f, b: Vector3f) -> Vector3f {
    vector3_angle_distance(a, b, TWO_PI)
}

/// `Vector3AngleDistanceDeg`
#[inline]
pub fn vector3_angle_distance_deg(a: Vector3f, b: Vector3f) -> Vector3f {
    vector3_angle_distance(a, b, 360.0)
}

/// `Vector3Floor`
#[inline]
pub fn vector3_floor(v: Vector3f) -> Vector3f {
    Vector3f::new(v.x.floor(), v.y.floor(), v.z.floor())
}

/// `Vector3Ceil`
#[inline]
pub fn vector3_ceil(v: Vector3f) -> Vector3f {
    Vector3f::new(v.x.ceil(), v.y.ceil(), v.z.ceil())
}

/// `Vector3Abs`
#[inline]
pub fn vector3_abs(v: Vector3f) -> Vector3f {
    Vector3f::new(v.x.abs(), v.y.abs(), v.z.abs())
}

/// `Vector3Min`
#[inline]
pub fn vector3_min(a: Vector3f, b: Vector3f) -> Vector3f {
    Vector3f::new(min_f(a.x, b.x), min_f(a.y, b.y), min_f(a.z, b.z))
}

/// `Vector3Max`
#[inline]
pub fn vector3_max(a: Vector3f, b: Vector3f) -> Vector3f {
    Vector3f::new(max_f(a.x, b.x), max_f(a.y, b.y), max_f(a.z, b.z))
}

/// `Vector3Less`: every component strictly less.
#[inline]
pub fn vector3_less(v: Vector3f, cmp: Vector3f) -> bool {
    !(v.x >= cmp.x || v.y >= cmp.y || v.z >= cmp.z)
}

/// `Vector3Greater`
#[inline]
pub fn vector3_greater(v: Vector3f, cmp: Vector3f) -> bool {
    !(v.x <= cmp.x || v.y <= cmp.y || v.z <= cmp.z)
}

/// `Vector3LessEqual`
#[inline]
pub fn vector3_less_equal(v: Vector3f, cmp: Vector3f) -> bool {
    !(v.x > cmp.x || v.y > cmp.y || v.z > cmp.z)
}

/// `Vector3GreaterEqual`
#[inline]
pub fn vector3_greater_equal(v: Vector3f, cmp: Vector3f) -> bool {
    !(v.x < cmp.x || v.y < cmp.y || v.z < cmp.z)
}

/// `Vector3Normalize`
#[inline]
pub fn vector3_normalize(v: Vector3f) -> Vector3f {
    v.normalized()
}

/// `Vector3DistSqr`
#[inline]
pub fn vector3_dist_sqr(start: Vector3f, end: Vector3f) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let dz = end.z - start.z;
    dx * dx + dy * dy + dz * dz
}

/// `Vector3Dist`
#[inline]
pub fn vector3_dist(start: Vector3f, end: Vector3f) -> f32 {
    vector3_dist_sqr(start, end).sqrt()
}

/// `Vector3Cross`: A x B.
#[inline]
pub fn vector3_cross(a: Vector3f, b: Vector3f) -> Vector3f {
    Vector3f::new(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x)
}

/// `Vector3Dot`
#[inline]
pub fn vector3_dot(a: Vector3f, b: Vector3f) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// `Vector3Project`: project `src` onto `dest`.
#[inline]
pub fn vector3_project(src: Vector3f, dest: Vector3f) -> Vector3f {
    let t = (src.x * dest.x + src.y * dest.y + src.z * dest.z) / (dest.x * dest.x + dest.y * dest.y + dest.z * dest.z);
    dest * t
}

/// `Vector3Reflect`: reflect `v` about `normal`.
#[inline]
pub fn vector3_reflect(v: Vector3f, normal: Vector3f) -> Vector3f {
    v - normal * vector3_dot(v, normal) * 2.0
}

/// `Vector3Angle`: angle between two unit vectors.
#[inline]
pub fn vector3_angle(a: Vector3f, b: Vector3f) -> f32 {
    let cos = vector3_dot(a, b);
    if (cos - 1.0).abs() <= EPSILON {
        return 0.0;
    }
    cos.acos()
}

/// `Vector3SignedAngle`: angle from `a` to `b`, signed by reference normal `n`.
#[inline]
pub fn vector3_signed_angle(a: Vector3f, b: Vector3f, n: Vector3f) -> f32 {
    let cos = vector3_dot(a, b);
    if (cos - 1.0).abs() <= EPSILON {
        return 0.0;
    }
    let mut angle = cos.acos();
    let sign_value = vector3_dot(n, vector3_cross(a, b));
    if sign_value < 0.0 {
        angle *= -1.0;
    }
    angle
}

/// `Vector3UnProject`: screen position (with z in [0, 1]) to world.
pub fn vector3_unproject(v: Vector3f, screen_rect: Rect2f, view_proj: &Matrixf) -> Vector3f {
    let inv_view_proj = matrix_inverse(view_proj);
    let normalized = Vector3f::new(
        ((v.x - screen_rect.x) * 2.0 / screen_rect.w) - 1.0,
        -(((v.y - screen_rect.y) * 2.0 / screen_rect.h) - 1.0),
        2.0 * v.z - 1.0,
    );
    // Faithful: the result is negated.
    matrix_mul_divide_w(&inv_view_proj, normalized) * -1.0
}

/// `Vector3IncreaseTo`
#[inline]
pub fn vector3_increase_to(x: Vector3f, add: Vector3f, dest: Vector3f) -> Vector3f {
    Vector3f::new(increase_to(x.x, add.x, dest.x), increase_to(x.y, add.y, dest.y), increase_to(x.z, add.z, dest.z))
}

/// `Vector3ClampToLength`: shorten `v` to `max_length` if longer (no-op for
/// `max_length <= 0`).
#[inline]
pub fn vector3_clamp_to_length(v: &mut Vector3f, max_length: f32) {
    if max_length <= 0.0 {
        return;
    }
    let length = v.length();
    if length > max_length {
        *v = (*v / length) * max_length;
    }
}

/// `GetAngleFromPoints2DSimple` (file-local helper in Math.cpp).
fn get_angle_from_points_2d_simple(start: Vector3f, goal: Vector3f) -> f32 {
    let mut delta = goal - start;
    if delta.x == 0.0 {
        delta.x = -EPSILON;
    }
    if delta.y == 0.0 {
        delta.y = EPSILON;
    }

    if (delta.y >= 0.0 && delta.x <= 0.0) || (delta.y < 0.0 && delta.x <= 0.0) {
        -(delta.y / delta.x).atan()
    } else {
        log::error!("Error in GetAngle 3D code! ARGHHH run in terror");
        -1000.0
    }
}

/// `GetAngleFromPoints3D`: Euler angles (x, y) that look from `start` to `goal`.
pub fn get_angle_from_points_3d(start: Vector3f, goal: Vector3f) -> Vector3f {
    let mut angle = Vector3f::new(0.0, 0.0, 0.0);
    angle.y = -get_angle_from_points_2d(Vector2f::new(start.x, start.z), Vector2f::new(goal.x, goal.z));

    let rot = matrix_rotate_y(-angle.y);
    let delta = goal - start;
    let goal_local = matrix_mul_vec3(&rot, delta);

    angle.x = get_angle_from_points_2d_simple(Vector3f::new(0.0, 0.0, 0.0), Vector3f::new(goal_local.z, goal_local.y, 0.0));
    angle.x = wrap(angle.x, 0.0, TWO_PI);
    angle
}

/// `Vector3OrthonormalizeBasis` (Gram-Schmidt). Returns (right, up, forward).
pub fn vector3_orthonormalize_basis(v1: Vector3f, v2: Vector3f, v3: Vector3f) -> (Vector3f, Vector3f, Vector3f) {
    let u1 = v1;
    let dst1 = u1.normalized();

    let u2 = v2 - vector3_project(v2, u1);
    let dst2 = u2.normalized();

    let u3 = v3 - vector3_project(v3, u1) - vector3_project(v3, u2);
    let dst3 = u3.normalized();

    (dst1, dst2, dst3)
}

/// `Vector3SphereSurfacePoint`: deterministic point on a sphere from a seed
/// in [0, 1]^2.
#[inline]
pub fn vector3_sphere_surface_point(seed: Vector2f, radius: f32) -> Vector3f {
    let x = seed.x * 2.0 - 1.0;
    let t = TWO_PI * seed.y;
    let w = (1.0 - x * x).sqrt();
    Vector3f::new(x, w * t.sin(), w * t.cos()) * radius
}
