//! MATRICES

use super::quat::quaternion_slerp;
use super::scalar::{clamp, sign_f};
use super::plane::plane_dot;
use crate::matrix::Matrixf;
use crate::quaternion::Quaternion;
use crate::types::{EulerRotationOrder, Planef};
use crate::vector::{Vector2f, Vector3f};

/// `MatrixSlerp`: interpolate translation linearly and rotation spherically.
pub fn matrix_slerp(t: f32, a: &Matrixf, b: &Matrixf, shortest_path: bool) -> Matrixf {
    if t <= 0.0 {
        return *a;
    }
    if t >= 1.0 {
        return *b;
    }
    let pos = a.translation() * (1.0 - t) + b.translation() * t;
    let qa = Quaternion::from_rotation_matrix(a);
    let qb = Quaternion::from_rotation_matrix(b);
    let q = quaternion_slerp(t, qa, qb, shortest_path);
    let mut out = matrix_quaternion(&q);
    out.set_translation(pos);
    out
}

/// `MatrixMul(A, B)`: B is applied before A.
#[inline]
pub fn matrix_mul(a: &Matrixf, b: &Matrixf) -> Matrixf {
    let (a, b) = (&a.m, &b.m);
    let mut c = [[0.0f32; 4]; 4];
    for r in 0..4 {
        for col in 0..4 {
            c[r][col] = a[r][0] * b[0][col] + a[r][1] * b[1][col] + a[r][2] * b[2][col] + a[r][3] * b[3][col];
        }
    }
    Matrixf { m: c }
}

/// `MatrixMul(A, v)`: transform a point (w = 1).
#[inline]
pub fn matrix_mul_vec3(a: &Matrixf, v: Vector3f) -> Vector3f {
    let m = &a.m;
    Vector3f::new(
        m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z + m[0][3],
        m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z + m[1][3],
        m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z + m[2][3],
    )
}

/// `MatrixMul3x3`: rotate a direction (no translation).
#[inline]
pub fn matrix_mul_3x3(a: &Matrixf, v: Vector3f) -> Vector3f {
    let m = &a.m;
    Vector3f::new(
        m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
        m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
        m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
    )
}

/// `MatrixMulDivideW`: transform and perspective-divide.
#[inline]
pub fn matrix_mul_divide_w(a: &Matrixf, v: Vector3f) -> Vector3f {
    let m = &a.m;
    let inv_w = 1.0 / (m[3][0] * v.x + m[3][1] * v.y + m[3][2] * v.z + m[3][3]);
    Vector3f::new(
        (m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z + m[0][3]) * inv_w,
        (m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z + m[1][3]) * inv_w,
        (m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z + m[2][3]) * inv_w,
    )
}

/// `MatrixMulScalar`
#[inline]
pub fn matrix_mul_scalar(a: &Matrixf, s: f32) -> Matrixf {
    let mut out = *a;
    out.m.iter_mut().flatten().for_each(|v| *v *= s);
    out
}

/// `MatrixRotate`: rotation from Euler angles applied in `order`.
pub fn matrix_rotate(rot: Vector3f, order: EulerRotationOrder) -> Matrixf {
    use EulerRotationOrder::*;
    let rx = || matrix_rotate_x(rot.x);
    let ry = || matrix_rotate_y(rot.y);
    let rz = || matrix_rotate_z(rot.z);
    let steps: [&dyn Fn() -> Matrixf; 3] = match order {
        Xyz => [&rx, &ry, &rz],
        Xzy => [&rx, &rz, &ry],
        Yxz => [&ry, &rx, &rz],
        Yzx => [&ry, &rz, &rx],
        Zxy => [&rz, &rx, &ry],
        Zyx => [&rz, &ry, &rx],
    };
    let mut m = Matrixf::IDENTITY;
    for step in steps {
        m = matrix_mul(&step(), &m);
    }
    m
}

/// `MatrixRotateX` (right-handed).
#[inline]
pub fn matrix_rotate_x(angle: f32) -> Matrixf {
    let (s, c) = (angle.sin(), angle.cos());
    Matrixf::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, c, -s, 0.0,
        0.0, s, c, 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `MatrixRotateY` (right-handed).
#[inline]
pub fn matrix_rotate_y(angle: f32) -> Matrixf {
    let (s, c) = (angle.sin(), angle.cos());
    Matrixf::new(
        c, 0.0, s, 0.0,
        0.0, 1.0, 0.0, 0.0,
        -s, 0.0, c, 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `MatrixRotateZ` (right-handed).
#[inline]
pub fn matrix_rotate_z(angle: f32) -> Matrixf {
    let (s, c) = (angle.sin(), angle.cos());
    Matrixf::new(
        c, -s, 0.0, 0.0,
        s, c, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `MatrixScale`
#[inline]
pub fn matrix_scale(scale: Vector3f) -> Matrixf {
    Matrixf::new(
        scale.x, 0.0, 0.0, 0.0,
        0.0, scale.y, 0.0, 0.0,
        0.0, 0.0, scale.z, 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `MatrixTranslate`
#[inline]
pub fn matrix_translate(trans: Vector3f) -> Matrixf {
    Matrixf::new(
        1.0, 0.0, 0.0, trans.x,
        0.0, 1.0, 0.0, trans.y,
        0.0, 0.0, 1.0, trans.z,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `MatrixQuaternion`: rotation matrix from a quaternion.
#[inline]
pub fn matrix_quaternion(q: &Quaternion) -> Matrixf {
    let mut out = Matrixf::IDENTITY;
    q.to_rotation_matrix(&mut out);
    out
}

/// `MatrixToQuaternion`
#[inline]
pub fn matrix_to_quaternion(a: &Matrixf) -> Quaternion {
    Quaternion::from_rotation_matrix(a)
}

/// `MatrixMinor`: determinant of the 3x3 sub-matrix with the given rows/columns.
#[inline]
pub fn matrix_minor(a: &Matrixf, r0: usize, r1: usize, r2: usize, c0: usize, c1: usize, c2: usize) -> f32 {
    let m = &a.m;
    m[r0][c0] * (m[r1][c1] * m[r2][c2] - m[r2][c1] * m[r1][c2])
        - m[r0][c1] * (m[r1][c0] * m[r2][c2] - m[r2][c0] * m[r1][c2])
        + m[r0][c2] * (m[r1][c0] * m[r2][c1] - m[r2][c0] * m[r1][c1])
}

/// `MatrixAdjoint`
pub fn matrix_adjoint(a: &Matrixf) -> Matrixf {
    let mn = |r0, r1, r2, c0, c1, c2| matrix_minor(a, r0, r1, r2, c0, c1, c2);
    Matrixf::new(
        mn(1, 2, 3, 1, 2, 3), -mn(0, 2, 3, 1, 2, 3), mn(0, 1, 3, 1, 2, 3), -mn(0, 1, 2, 1, 2, 3),
        -mn(1, 2, 3, 0, 2, 3), mn(0, 2, 3, 0, 2, 3), -mn(0, 1, 3, 0, 2, 3), mn(0, 1, 2, 0, 2, 3),
        mn(1, 2, 3, 0, 1, 3), -mn(0, 2, 3, 0, 1, 3), mn(0, 1, 3, 0, 1, 3), -mn(0, 1, 2, 0, 1, 3),
        -mn(1, 2, 3, 0, 1, 2), mn(0, 2, 3, 0, 1, 2), -mn(0, 1, 3, 0, 1, 2), mn(0, 1, 2, 0, 1, 2),
    )
}

/// `MatrixDeterminant`
#[inline]
pub fn matrix_determinant(a: &Matrixf) -> f32 {
    let m = &a.m;
    m[0][0] * matrix_minor(a, 1, 2, 3, 1, 2, 3) - m[0][1] * matrix_minor(a, 1, 2, 3, 0, 2, 3)
        + m[0][2] * matrix_minor(a, 1, 2, 3, 0, 1, 3)
        - m[0][3] * matrix_minor(a, 1, 2, 3, 0, 1, 2)
}

/// `MatrixInverse` (adjoint / determinant; no singularity check).
#[inline]
pub fn matrix_inverse(a: &Matrixf) -> Matrixf {
    matrix_mul_scalar(&matrix_adjoint(a), 1.0 / matrix_determinant(a))
}

/// `MatrixPlaneMirror`: reflection through a normalised plane.
pub fn matrix_plane_mirror(p: &Planef) -> Matrixf {
    Matrixf::new(
        1.0 - 2.0 * p.a * p.a, -2.0 * p.b * p.a, -2.0 * p.c * p.a, -2.0 * p.a * p.d,
        -2.0 * p.a * p.b, 1.0 - 2.0 * p.b * p.b, -2.0 * p.c * p.b, -2.0 * p.b * p.d,
        -2.0 * p.a * p.c, -2.0 * p.b * p.c, 1.0 - 2.0 * p.c * p.c, -2.0 * p.c * p.d,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `MatrixToEulerAngles`. Faithful: `order` is ignored, XYZ is always used.
pub fn matrix_to_euler_angles(a: &Matrixf, _order: EulerRotationOrder) -> Vector3f {
    let m = &a.m;
    Vector3f::new(m[2][1].atan2(m[2][2]), -clamp(m[2][0], -1.0, 1.0).asin(), m[1][0].atan2(m[0][0]))
}

/// `MatrixEulerAngleDistance`: XYZ Euler angles that rotate `b` onto `a`
/// (rotation-only matrices).
pub fn matrix_euler_angle_distance(a: &Matrixf, b: &Matrixf) -> Vector3f {
    let b_trans = b.transpose();
    let local_a = matrix_mul(&b_trans, a);
    let local_a_trans = local_a.transpose();

    let mut fwd = local_a_trans.forward();
    let mut up = local_a_trans.up();
    let mut out = Vector3f::splat(0.0);

    // X rotation (align fwd vector with z-axis)
    if fwd.y.abs() < 0.0001 && fwd.z.abs() < 0.0001 {
        out.x = 0.0;
    } else {
        let yz = Vector2f::new(fwd.y, fwd.z).normalized();
        out.x = clamp(yz.y, -1.0, 1.0).acos();
        if yz.x < 0.0 {
            out.x = -out.x;
        }
    }

    // Rotate forward
    let mut rot = matrix_rotate_x(out.x);
    fwd = matrix_mul_3x3(&rot, fwd);

    // Y rotation (align fwd vector with z-axis)
    if fwd.x.abs() < 0.0001 && fwd.z.abs() < 0.0001 {
        out.y = 0.0;
    } else {
        let xz = Vector2f::new(fwd.x, fwd.z).normalized();
        out.y = clamp(xz.y, -1.0, 1.0).acos();
        if xz.x > 0.0 {
            out.y = -out.y;
        }
    }

    // Rotate up
    rot = matrix_mul(&matrix_rotate_y(out.y), &rot);
    up = matrix_mul_3x3(&rot, up);

    // Z rotation
    if up.y.abs() < 0.0001 && up.x.abs() < 0.0001 {
        out.z = 0.0;
    } else {
        let xy = Vector2f::new(up.x, up.y).normalized();
        out.z = clamp(xy.y, -1.0, 1.0).acos();
        if xy.x < 0.0 {
            out.z = -out.z;
        }
    }

    matrix_mul_3x3(b, out)
}

/// `MatrixUnitVectors`: basis vectors as columns plus translation.
#[inline]
pub fn matrix_unit_vectors(right: Vector3f, up: Vector3f, forward: Vector3f, translate: Vector3f) -> Matrixf {
    Matrixf::new(
        right.x, up.x, forward.x, translate.x,
        right.y, up.y, forward.y, translate.y,
        right.z, up.z, forward.z, translate.z,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `ProjectionMatrixObliqueNearClipPlane`: replace the near plane of a
/// perspective projection with a camera-space clip plane.
pub fn projection_matrix_oblique_near_clip_plane(proj: &Matrixf, clip_plane: &Planef) -> Matrixf {
    let mut out = *proj;
    let m = &proj.m;
    let q = Planef {
        a: (sign_f(clip_plane.a) + m[0][2]) / m[0][0],
        b: (sign_f(clip_plane.b) + m[1][2]) / m[1][1],
        c: -1.0,
        d: (1.0 + m[2][2]) / m[2][3],
    };
    let c = *clip_plane * (2.0 / plane_dot(clip_plane, &q));
    out.m[2][0] = c.a;
    out.m[2][1] = c.b;
    out.m[2][2] = c.c + 1.0;
    out.m[2][3] = c.d;
    out
}

/// `MatrixPerspectiveProjection`
pub fn matrix_perspective_projection(near: f32, far: f32, fov: f32, aspect: f32, inf_far_plane: bool) -> Matrixf {
    let top = (fov * 0.5).tan() * near;
    let bottom = -top;
    let right = aspect * top;
    let left = aspect * bottom;

    let a = (2.0 * near) / (right - left);
    let b = (2.0 * near) / (top - bottom);
    let d = -1.0;

    let (c, z) = if inf_far_plane {
        (-2.0 * near, -1.0)
    } else {
        (-(2.0 * far * near) / (far - near), -(far + near) / (far - near))
    };

    Matrixf::new(
        a, 0.0, 0.0, 0.0,
        0.0, b, 0.0, 0.0,
        0.0, 0.0, z, c,
        0.0, 0.0, d, 0.0,
    )
}

/// `MatrixOrthographicProjection`
pub fn matrix_orthographic_projection(near: f32, far: f32, view_size: Vector2f) -> Matrixf {
    let a = 2.0 / view_size.x;
    let b = 2.0 / view_size.y;
    let c = -2.0 / (far - near);
    let z = -(far + near) / (far - near);
    Matrixf::new(
        a, 0.0, 0.0, 0.0,
        0.0, b, 0.0, 0.0,
        0.0, 0.0, c, z,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// `MatrixToChar` equivalent (`"%.3f"` per element, rows in brackets).
pub fn matrix_to_string(a: &Matrixf) -> String {
    let m = &a.m;
    let row = |r: usize| format!("[{:.3}, {:.3}, {:.3}, {:.3}]", m[r][0], m[r][1], m[r][2], m[r][3]);
    format!("{} {} {} {}", row(0), row(1), row(2), row(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_of_rotation_translation() {
        let m = matrix_mul(&matrix_translate(Vector3f::new(1.0, 2.0, 3.0)), &matrix_rotate_y(0.5));
        let inv = matrix_inverse(&m);
        let id = matrix_mul(&m, &inv);
        for r in 0..4 {
            for c in 0..4 {
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!((id.m[r][c] - expected).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn rotate_order_matches_manual_product() {
        let r = Vector3f::new(0.1, 0.2, 0.3);
        let manual = matrix_mul(&matrix_rotate_z(r.z), &matrix_mul(&matrix_rotate_y(r.y), &matrix_mul(&matrix_rotate_x(r.x), &Matrixf::IDENTITY)));
        assert_eq!(matrix_rotate(r, EulerRotationOrder::Xyz), manual);
    }
}
