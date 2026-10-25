//! BV AND AABB OPERATIONS, BV AND RECT INTERSECTION

use super::matrix_ops::{matrix_mul, matrix_mul_divide_w, matrix_mul_vec3};
use super::plane::check_plane_line_intersection;
use super::vec3::{vector3_cross, vector3_dot};
use crate::consts::EPSILON;
use crate::matrix::Matrixf;
use crate::types::{Planef, Rect2f, Rect2l};
use crate::vector::{Vector2f, Vector3f};

/// `ExpandAABB`: grow `base` to contain `add`.
#[inline]
pub fn expand_aabb(base_min: &mut Vector3f, base_max: &mut Vector3f, add_min: Vector3f, add_max: Vector3f) {
    if base_min.x > add_min.x {
        base_min.x = add_min.x;
    }
    if base_min.y > add_min.y {
        base_min.y = add_min.y;
    }
    if base_min.z > add_min.z {
        base_min.z = add_min.z;
    }
    if base_max.x < add_max.x {
        base_max.x = add_max.x;
    }
    if base_max.y < add_max.y {
        base_max.y = add_max.y;
    }
    if base_max.z < add_max.z {
        base_max.z = add_max.z;
    }
}

/// `CheckRectIntersection(cRect2l, cRect2l)`. Negative sizes are flipped.
pub fn check_rect_intersection_l(mut r1: Rect2l, mut r2: Rect2l) -> bool {
    for r in [&mut r1, &mut r2] {
        if r.w < 0 {
            r.w = r.w.wrapping_neg();
            r.x = r.x.wrapping_sub(r.w);
        }
        if r.h < 0 {
            r.h = r.h.wrapping_neg();
            r.y = r.y.wrapping_sub(r.h);
        }
    }
    let end = |p: i32, s: i32| p.wrapping_add(s.wrapping_sub(1));
    !(r1.x > end(r2.x, r2.w) || r2.x > end(r1.x, r1.w) || r1.y > end(r2.y, r2.h) || r2.y > end(r1.y, r1.h))
}

/// `CheckRectIntersection(cRect2f, cRect2f)`. Negative sizes are flipped.
pub fn check_rect_intersection_f(mut r1: Rect2f, mut r2: Rect2f) -> bool {
    for r in [&mut r1, &mut r2] {
        if r.w < 0.0 {
            r.w *= -1.0;
            r.x -= r.w;
        }
        if r.h < 0.0 {
            r.h *= -1.0;
            r.y -= r.h;
        }
    }
    !(r1.x > r2.x + r2.w || r2.x > r1.x + r1.w || r1.y > r2.y + r2.h || r2.y > r1.y + r1.h)
}

/// `CheckPointInRectIntersection`
#[inline]
pub fn check_point_in_rect_intersection(p: Vector2f, r: Rect2f) -> bool {
    !(p.x < r.x || p.x > r.x + r.w || p.y < r.y || p.y > r.y + r.h)
}

/// `CheckRectFit(cRect2l, cRect2l)`: `src` fits inside `dest`.
pub fn check_rect_fit_l(src: Rect2l, dest: Rect2l) -> bool {
    let (sx2, sy2) = (src.x.wrapping_add(src.w), src.y.wrapping_add(src.h));
    let (dx2, dy2) = (dest.x.wrapping_add(dest.w), dest.y.wrapping_add(dest.h));
    if src.w > dest.w || src.h > dest.h || sx2 > dx2 || sy2 > dy2 {
        return false;
    }
    !(src.x < dest.x || src.y < dest.y || src.x > dx2 || src.y > dy2)
}

/// `CheckRectFit(cRect2f, cRect2f)`: `src` fits inside `dest`.
pub fn check_rect_fit_f(src: Rect2f, dest: Rect2f) -> bool {
    if src.w > dest.w || src.h > dest.h || src.x + src.w > dest.x + dest.w || src.y + src.h > dest.y + dest.h {
        return false;
    }
    !(src.x < dest.x || src.y < dest.y || src.x > dest.x + dest.w || src.y > dest.y + dest.h)
}

/// `GetClipRect`: `src` clipped to `dest`.
pub fn get_clip_rect(src: Rect2f, dest: Rect2f) -> Rect2f {
    let mut out = src;
    if out.x < dest.x {
        out.w -= dest.x - out.x;
        out.x = dest.x;
    }
    if out.y < dest.y {
        out.h -= dest.y - out.y;
        out.y = dest.y;
    }
    if out.x + out.w > dest.x + dest.w {
        out.w -= (out.x + out.w) - (dest.x + dest.w);
    }
    if out.y + out.h > dest.y + dest.h {
        out.h -= (out.y + out.h) - (dest.y + dest.h);
    }
    out
}

/// `CheckPointInSphereIntersection`
#[inline]
pub fn check_point_in_sphere_intersection(point: Vector3f, sphere_pos: Vector3f, sphere_radius: f32) -> bool {
    (point - sphere_pos).sqr_length() <= sphere_radius * sphere_radius
}

/// `CheckSphereIntersection`
#[inline]
pub fn check_sphere_intersection(pos_a: Vector3f, radius_a: f32, pos_b: Vector3f, radius_b: f32) -> bool {
    let radius_sum = radius_a + radius_b;
    (pos_a - pos_b).sqr_length() <= radius_sum * radius_sum
}

/// Result of [`check_sphere_line_intersection`]. A `t` of -1 means that
/// root lies beyond the segment end (as in C++).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SphereLineHits {
    pub t1: f32,
    pub pos1: Vector3f,
    pub t2: f32,
    pub pos2: Vector3f,
    pub hit1: bool,
    pub hit2: bool,
}

/// `CheckSphereLineIntersection`. Returns `None` when the infinite line
/// misses the sphere. Otherwise both roots are reported; `hitN` says whether
/// that root is at or before the segment end (the C++ returns true if either
/// is). Faithful: roots behind the start are not rejected.
pub fn check_sphere_line_intersection(sphere_pos: Vector3f, sphere_radius: f32, start: Vector3f, end: Vector3f) -> Option<SphereLineHits> {
    let mut dir = end - start;
    let segment_length = dir.length();
    dir.normalize();

    let start_to_sphere = start - sphere_pos;
    let b = 2.0 * vector3_dot(dir, start_to_sphere);
    let c = vector3_dot(start_to_sphere, start_to_sphere) - (sphere_radius * sphere_radius);
    let discriminant = b * b - 4.0 * c;
    if !(discriminant >= 0.0) {
        return None;
    }

    let root = discriminant.sqrt();
    let t1 = 0.5 * (-b + root);
    let t2 = 0.5 * (-b - root);
    let mut hits = SphereLineHits::default();
    if t1 <= segment_length {
        hits.hit1 = true;
        hits.t1 = t1;
        hits.pos1 = start + dir * t1;
    } else {
        hits.t1 = -1.0;
    }
    if t2 <= segment_length {
        hits.hit2 = true;
        hits.t2 = t2;
        hits.pos2 = start + dir * t2;
    } else {
        hits.t2 = -1.0;
    }
    Some(hits)
}

/// `CheckAABBIntersection` (also `CheckBVIntersection` on world bounds).
#[inline]
pub fn check_aabb_intersection(min1: Vector3f, max1: Vector3f, min2: Vector3f, max2: Vector3f) -> bool {
    !(max1.x < min2.x || max1.y < min2.y || max1.z < min2.z || max2.x < min1.x || max2.y < min1.y || max2.z < min1.z)
}

/// `CheckAABBInside`: `inside` is completely within `outside`.
#[inline]
pub fn check_aabb_inside(inside_min: Vector3f, inside_max: Vector3f, outside_min: Vector3f, outside_max: Vector3f) -> bool {
    inside_min.x >= outside_min.x
        && inside_min.y >= outside_min.y
        && inside_min.z >= outside_min.z
        && inside_max.x <= outside_max.x
        && inside_max.y <= outside_max.y
        && inside_max.z <= outside_max.z
}

/// `CheckPointInAABBIntersection` (also `CheckPointInBVIntersection`).
#[inline]
pub fn check_point_in_aabb_intersection(p: Vector3f, min: Vector3f, max: Vector3f) -> bool {
    !(p.x > max.x || p.y > max.y || p.z > max.z || p.x < min.x || p.y < min.y || p.z < min.z)
}

/// Result of [`check_aabb_line_intersection`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AabbLineHit {
    /// Whether the segment hits the box.
    pub hit: bool,
    /// Closest hit position, if any.
    pub pos: Option<Vector3f>,
    /// Closest t, or 1000 if nothing was hit (as in C++).
    pub t: f32,
}

/// `CheckAABBLineIntersection`
pub fn check_aabb_line_intersection(min: Vector3f, max: Vector3f, start: Vector3f, end: Vector3f) -> AabbLineHit {
    let planes = [
        Planef::from_normal_point(Vector3f::new(-1.0, 0.0, 0.0), min), // Left
        Planef::from_normal_point(Vector3f::new(1.0, 0.0, 0.0), max),  // Right
        Planef::from_normal_point(Vector3f::new(0.0, -1.0, 0.0), min), // Bottom
        Planef::from_normal_point(Vector3f::new(0.0, 1.0, 0.0), max),  // Top
        Planef::from_normal_point(Vector3f::new(0.0, 0.0, -1.0), min), // Back
        Planef::from_normal_point(Vector3f::new(0.0, 0.0, 1.0), max),  // Front
    ];
    let larger_min = min - Vector3f::splat(0.001);
    let larger_max = max + Vector3f::splat(0.001);

    let mut result = AabbLineHit { hit: false, pos: None, t: 1000.0 };
    for plane in &planes {
        if let Some(h) = check_plane_line_intersection(plane, start, end) {
            if h.t < result.t && check_point_in_aabb_intersection(h.pos, larger_min, larger_max) {
                result = AabbLineHit { hit: true, pos: Some(h.pos), t: h.t };
            }
        }
    }
    result
}

/// `CheckPointSameSideOfEdge2D`: `p1` and `p2` on the same side of edge a-b.
#[inline]
pub fn check_point_same_side_of_edge_2d(p1: Vector3f, p2: Vector3f, a: Vector3f, b: Vector3f) -> bool {
    let edge = b - a;
    let cp1 = vector3_cross(edge, p1 - a);
    let cp2 = vector3_cross(edge, p2 - a);
    vector3_dot(cp1, cp2) >= 0.0
}

/// `CheckPointInsideTriangle2D` (normalised coordinates; z is ignored).
pub fn check_point_inside_triangle_2d(point: Vector2f, triangle: &[Vector3f; 3]) -> bool {
    let p = Vector3f::new(point.x, point.y, 0.0);
    let t = triangle.map(|v| Vector3f::new(v.x, v.y, 0.0));
    check_point_same_side_of_edge_2d(p, t[0], t[1], t[2])
        && check_point_same_side_of_edge_2d(p, t[1], t[0], t[2])
        && check_point_same_side_of_edge_2d(p, t[2], t[0], t[1])
}

/// Result of [`check_point_in_flat_polygon`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlatPolygonHit {
    /// Index (into the index array) of the first index of the hit triangle.
    pub tri_index: u32,
    pub intersection_world: Vector3f,
    pub triangle_world: [Vector3f; 3],
}

/// `CheckPointInFlatPolygon`: pick the closest front-facing triangle under a
/// screen point. `vertex_pos` is indexed with `index * stride`.
#[allow(clippy::too_many_arguments)]
pub fn check_point_in_flat_polygon(
    ray_start: Vector3f,
    ray_end: Vector3f,
    point: Vector2f,
    viewport_size: Vector2f,
    indices: &[u32],
    vertex_pos: &[f32],
    stride: usize,
    obj_world: &Matrixf,
    cam_view: &Matrixf,
    cam_proj: &Matrixf,
) -> Option<FlatPolygonHit> {
    let screen_point = (point * 2.0) / viewport_size - 1.0;
    let screen_transform = matrix_mul(cam_proj, cam_view);
    let forward = Vector3f::new(0.0, 0.0, 1.0);

    let mut shortest_t = 1000.0f32;
    let mut best: Option<FlatPolygonHit> = None;

    for (i, tri) in indices.chunks_exact(3).enumerate() {
        let mut world = [Vector3f::default(); 3];
        let mut screen = [Vector3f::default(); 3];
        for j in 0..3 {
            let base = tri[j] as usize * stride;
            world[j] = matrix_mul_vec3(obj_world, Vector3f::new(vertex_pos[base], vertex_pos[base + 1], vertex_pos[base + 2]));
            screen[j] = matrix_mul_divide_w(&screen_transform, world[j]);
        }

        // Skip triangles facing away.
        let mut normal = vector3_cross(screen[1] - screen[0], screen[2] - screen[0]);
        normal.normalize();
        if vector3_dot(normal, forward) >= -EPSILON {
            continue;
        }

        if check_point_inside_triangle_2d(screen_point, &screen) {
            let plane = Planef::from_points(world[0], world[1], world[2]);
            if let Some(h) = check_plane_line_intersection(&plane, ray_start, ray_end) {
                if h.t < shortest_t {
                    shortest_t = h.t;
                    best = Some(FlatPolygonHit { tri_index: (i * 3) as u32, intersection_world: h.pos, triangle_world: world });
                }
            }
        }
    }
    best
}
