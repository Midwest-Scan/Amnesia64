//! PLANES

use super::matrix_ops::{matrix_inverse, matrix_mul_vec3};
use super::vec3::{vector3_cross, vector3_dot, vector3_normalize};
use super::{LineHit, aabb_corners};
use crate::matrix::Matrixf;
use crate::types::{Collision, Planef};
use crate::vector::Vector3f;

/// `PlaneDot`: 4D dot product of two planes.
#[inline]
pub fn plane_dot(a: &Planef, b: &Planef) -> f32 {
    a.a * b.a + a.b * b.b + a.c * b.c + a.d * b.d
}

/// `PlaneParallel`. Faithful: only same-facing planes count as parallel.
#[inline]
pub fn plane_parallel(a: &Planef, b: &Planef) -> bool {
    let dot = a.a * b.a + a.b * b.b + a.c * b.c;
    dot > 0.99999
}

/// `TransformPlane`: transform a normalised plane by a matrix.
pub fn transform_plane(transform: &Matrixf, plane: &Planef) -> Planef {
    let normal = plane.normal();
    let point = normal * -plane.d;
    let normal_trans = matrix_inverse(&transform.rotation()).transpose();
    Planef::from_normal_point(vector3_normalize(matrix_mul_vec3(&normal_trans, normal)), matrix_mul_vec3(transform, point))
}

/// `PlaneToPointDist`: signed distance (>0 in front). Plane must be normalised.
#[inline]
pub fn plane_to_point_dist(plane: &Planef, v: Vector3f) -> f32 {
    (plane.a * v.x) + (plane.b * v.y) + (plane.c * v.z) + plane.d
}

/// `PlaneIntersectionLine`: returns (direction, point on line).
///
/// Faithful: like the C++ (see its TODO) this only works when no plane
/// coefficient is zero.
pub fn plane_intersection_line(pa: &Planef, pb: &Planef) -> (Vector3f, Vector3f) {
    let dir = vector3_cross(Vector3f::new(pa.a, pa.b, pa.c), Vector3f::new(pb.a, pb.b, pb.c));
    let val = (-pb.b) / pa.b;
    let z = ((pa.d * val) + pb.d) / ((pa.c * val) + pb.c);
    let y = (pa.d - (pa.c * z)) / pa.b;
    (dir, Vector3f::new(0.0, y, z))
}

/// `PlaneIntersectionPoint`: the point where three planes meet, or `None` if
/// any two are parallel.
pub fn plane_intersection_point(p1: &Planef, p2: &Planef, p3: &Planef) -> Option<Vector3f> {
    if plane_parallel(p1, p2) || plane_parallel(p1, p3) || plane_parallel(p2, p3) {
        return None;
    }
    let n1 = p1.normal();
    let n2 = p2.normal();
    let n3 = p3.normal();
    let cross12 = vector3_cross(n1, n2);
    let cross31 = vector3_cross(n3, n1);
    let cross23 = vector3_cross(n2, n3);
    Some((cross23 * -p1.d + cross31 * -p2.d + cross12 * -p3.d) / vector3_dot(n1, cross23))
}

/// `CheckPlaneSphereCollision`
#[inline]
pub fn check_plane_sphere_collision(plane: &Planef, center: Vector3f, radius: f32) -> Collision {
    let dist = plane_to_point_dist(plane, center);
    if dist.abs() <= radius {
        Collision::Intersect
    } else if dist > radius {
        Collision::Inside
    } else {
        Collision::Outside
    }
}

/// `CheckPlaneAABBCollision(plane, min, max)`
pub fn check_plane_aabb_collision(plane: &Planef, min: Vector3f, max: Vector3f) -> Collision {
    let mut in_count = 8;
    for corner in aabb_corners(min, max) {
        if plane_to_point_dist(plane, corner) < 0.0 {
            in_count -= 1;
        }
    }
    match in_count {
        0 => Collision::Outside,
        8 => Collision::Inside,
        _ => Collision::Intersect,
    }
}

/// `CheckPlaneAABBCollision(plane, min, max, sphereCenter, sphereRadius)`:
/// sphere early-out, then the box test.
pub fn check_plane_aabb_sphere_collision(plane: &Planef, min: Vector3f, max: Vector3f, sphere_center: Vector3f, sphere_radius: f32) -> Collision {
    let dist = plane_to_point_dist(plane, sphere_center);
    if dist < -sphere_radius {
        return Collision::Outside;
    }
    if dist > sphere_radius {
        return Collision::Inside;
    }
    check_plane_aabb_collision(plane, min, max)
}

/// The raw line/plane intersection behind `CheckPlaneLineIntersection`:
/// `None` if the line is parallel to the plane, otherwise the hit on the
/// infinite line (t may be outside [0, 1]).
#[inline]
pub fn plane_line_intersection(plane: &Planef, start: Vector3f, end: Vector3f) -> Option<LineHit> {
    let delta = end - start;
    let neg_start_to_plane_dist = -plane_to_point_dist(plane, start);
    let delta_plane_normal_mul = plane.a * delta.x + plane.b * delta.y + plane.c * delta.z;
    if delta_plane_normal_mul == 0.0 {
        return None;
    }
    let t = neg_start_to_plane_dist / delta_plane_normal_mul;
    Some(LineHit { pos: start + delta * t, t })
}

/// `CheckPlaneLineIntersection`: the segment hit, if within [0, 1].
#[inline]
pub fn check_plane_line_intersection(plane: &Planef, start: Vector3f, end: Vector3f) -> Option<LineHit> {
    plane_line_intersection(plane, start, end).filter(|h| segment_t_in_range(h.t))
}

/// `fT < 0 || fT > 1` rejected. Faithful: NaN is accepted.
#[inline]
pub(crate) fn segment_t_in_range(t: f32) -> bool {
    !(t < 0.0 || t > 1.0)
}

/// `CheckPointsPlanesCollision`: all points inside every plane (Inside),
/// all outside one plane (Outside), otherwise Intersect.
pub fn check_points_planes_collision(vertices: &[Vector3f], planes: &[Planef]) -> Collision {
    let mut total_in = 0;
    for plane in planes {
        let mut in_count = vertices.len();
        for v in vertices {
            if plane_to_point_dist(plane, *v) < 0.0 {
                in_count -= 1;
            }
        }
        if in_count == 0 {
            return Collision::Outside;
        }
        if in_count == vertices.len() {
            total_in += 1;
        }
    }
    if total_in == planes.len() { Collision::Inside } else { Collision::Intersect }
}

/// `CheckPointsAABBPlanesCollision`
pub fn check_points_aabb_planes_collision(vertices: &[Vector3f], min: Vector3f, max: Vector3f) -> Collision {
    let planes = [
        Planef::new(-1.0, 0.0, 0.0, max.x),
        Planef::new(1.0, 0.0, 0.0, -min.x),
        Planef::new(0.0, -1.0, 0.0, max.y),
        Planef::new(0.0, 1.0, 0.0, -min.y),
        Planef::new(0.0, 0.0, -1.0, max.z),
        Planef::new(0.0, 0.0, 1.0, -min.z),
    ];
    check_points_planes_collision(vertices, &planes)
}

/// `CheckSeparatingAxisIntersection`: SAT test using the plane normals of A
/// and B as axes. Empty slices are skipped, as NULL/0 are in C++.
pub fn check_separating_axis_intersection(vertices_a: &[Vector3f], planes_a: &[Planef], vertices_b: &[Vector3f], planes_b: &[Planef]) -> bool {
    let vertex_sets = [vertices_a, vertices_b];
    let mut min = [0.0f32; 2];
    let mut max = [0.0f32; 2];

    for planes in [planes_a, planes_b] {
        for plane in planes {
            let normal = plane.normal();
            for (set, vertices) in vertex_sets.iter().enumerate() {
                if vertices.is_empty() {
                    continue;
                }
                let first = vector3_dot(vertices[0], normal);
                min[set] = first;
                max[set] = first;
                for v in &vertices[1..] {
                    let dot = vector3_dot(*v, normal);
                    if min[set] > dot {
                        min[set] = dot;
                    } else if max[set] < dot {
                        max[set] = dot;
                    }
                }
            }
            if min[0] > max[1] || min[1] > max[0] {
                return false;
            }
        }
    }
    true
}

/// `CheckSphereInPlanes`: false if the sphere is fully behind any plane.
pub fn check_sphere_in_planes(center: Vector3f, radius: f32, planes: &[Planef]) -> bool {
    planes.iter().all(|p| !(plane_to_point_dist(p, center) < -radius))
}
