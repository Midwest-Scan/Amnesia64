//! QUATERNIONS

use crate::consts::EPSILON;
use crate::quaternion::Quaternion;

/// `QuaternionDot`
#[inline]
pub fn quaternion_dot(a: &Quaternion, b: &Quaternion) -> f32 {
    a.w * b.w + a.v.x * b.v.x + a.v.y * b.v.y + a.v.z * b.v.z
}

/// `QuaternionMul`: A * B, B is applied before A.
#[inline]
pub fn quaternion_mul(a: &Quaternion, b: &Quaternion) -> Quaternion {
    *a * *b
}

/// `QuaternionSlerp`: spherical interpolation from `a` (t = 0) to `b` (t = 1).
pub fn quaternion_slerp(t: f32, a: Quaternion, b: Quaternion, shortest_path: bool) -> Quaternion {
    let cos = quaternion_dot(&a, &b);

    // Nearly identical rotations: normalised linear interpolation.
    if (cos - 1.0).abs() <= EPSILON {
        return (a * (1.0 - t) + b * t).normalized();
    }

    let angle = cos.acos();
    let sin = angle.sin();
    let inv_sin = 1.0 / sin;
    let mut coeff0 = ((1.0 - t) * angle).sin() * inv_sin;
    let coeff1 = (t * angle).sin() * inv_sin;

    if cos < 0.0 && shortest_path {
        coeff0 = -coeff0;
        // taking the complement requires renormalisation
        (a * coeff0 + b * coeff1).normalized()
    } else {
        a * coeff0 + b * coeff1
    }
}
