//! Screen-space clip rectangles (`GetNormalizedClipRectFromBV`,
//! `GetClipRectFromBV`, `GetClipRectFromSphere`, ...).

use super::matrix_ops::matrix_mul_vec3;
use super::scalar::{max_i, min_i};
use super::aabb_corners;
use crate::frustum::{FrustumProjection, ProjectionType};
use crate::types::Rect2l;
use crate::util::f32_to_i32;
use crate::vector::{Vector2f, Vector2l, Vector3f};

/// For each AABB corner (in [`aabb_corners`] order), the three corners it
/// shares an edge with.
const BV_VTX_CONNECTIONS: [[usize; 3]; 8] = [
    [1, 2, 4],
    [0, 3, 5],
    [0, 3, 6],
    [1, 2, 7],
    [0, 5, 6],
    [1, 4, 7],
    [2, 4, 7],
    [3, 5, 6],
];

/// Near-plane half extents used by the clip-rect functions.
fn near_extents(projection: &FrustumProjection, tan_half_fov: f32) -> (f32, f32, f32) {
    let tan = if tan_half_fov > 0.0 { tan_half_fov } else { (projection.fov * 0.5).tan() };
    let near_plane = projection.near_plane;
    let near_top = tan * near_plane;
    let near_right = projection.aspect * near_top;
    (near_plane, near_top, near_right)
}

/// `GetNormalizedClipRectFromBV`: screen bounds (-1..1, may exceed) of the
/// world AABB `min`/`max`. `None` if every corner is behind the camera.
/// The returned z components are the C++ initial values (+/-10000).
pub fn get_normalized_clip_rect_from_bv(min: Vector3f, max: Vector3f, projection: &FrustumProjection, tan_half_fov: f32) -> Option<(Vector3f, Vector3f)> {
    let (near_plane, near_top, near_right) = near_extents(projection, tan_half_fov);

    // Transform to camera space
    let corners = aabb_corners(min, max).map(|c| matrix_mul_vec3(&projection.view, c));

    let mut screen_positions: Vec<Vector3f> = Vec::with_capacity(24);
    let mut visible = false;
    for (i, &pos) in corners.iter().enumerate() {
        if pos.z < 0.0 {
            visible = true;
        }
        if projection.projection_type == ProjectionType::Perspective {
            let mut z = pos.z;
            if z > 0.0 {
                // Behind the camera: clip each edge to a connected corner in
                // front of the near plane.
                for &conn in &BV_VTX_CONNECTIONS[i] {
                    let connect = corners[conn];
                    if connect.z >= 0.0 {
                        continue;
                    }
                    let delta = pos - connect;
                    let connect_to_near = (-near_plane) - connect.z;
                    let t = connect_to_near / delta.z;
                    let mut screen = connect + delta * t;
                    screen.x /= near_right;
                    screen.y /= near_top;
                    screen_positions.push(screen);
                }
            } else {
                if z == 0.0 {
                    z = 0.0001;
                }
                let mut p = pos;
                p.x = ((p.x / z) * -near_plane) / near_right;
                p.y = ((p.y / z) * -near_plane) / near_top;
                screen_positions.push(p);
            }
        } else {
            screen_positions.push(matrix_mul_vec3(&projection.proj, pos));
        }
    }

    if !visible {
        return None;
    }

    let mut dest_max = Vector3f::splat(-10000.0);
    let mut dest_min = Vector3f::splat(10000.0);
    for p in &screen_positions {
        if p.x < dest_min.x {
            dest_min.x = p.x;
        }
        if p.x > dest_max.x {
            dest_max.x = p.x;
        }
        if p.y < dest_min.y {
            dest_min.y = p.y;
        }
        if p.y > dest_max.y {
            dest_max.y = p.y;
        }
    }
    Some((dest_min, dest_max))
}

/// `GetClipRectFromNormalizedMinMax`: normalised bounds to a pixel rect.
pub fn get_clip_rect_from_normalized_min_max(min: Vector3f, max: Vector3f, screen_size: Vector2l) -> Rect2l {
    let mut min = min;
    let mut max = max;
    if min.x < -1.0 {
        min.x = -1.0;
    }
    if min.y < -1.0 {
        min.y = -1.0;
    }
    if max.x > 1.0 {
        max.x = 1.0;
    }
    if max.y > 1.0 {
        max.y = 1.0;
    }

    let half = Vector2f::new(screen_size.x as f32, screen_size.y as f32) * 0.5;
    let top_left = Vector2l::new(f32_to_i32(half.x + min.x * half.x), f32_to_i32(half.y + max.y * -half.y));
    let bottom_right = Vector2l::new(f32_to_i32(half.x + max.x * half.x), f32_to_i32(half.y + min.y * -half.y));

    Rect2l::new(top_left.x, top_left.y, bottom_right.x.wrapping_sub(top_left.x), bottom_right.y.wrapping_sub(top_left.y))
}

/// `GetClipRectFromBV`: pixel rect of a world AABB; the full screen if it
/// is entirely behind the camera.
pub fn get_clip_rect_from_bv(min: Vector3f, max: Vector3f, projection: &FrustumProjection, screen_size: Vector2l, tan_half_fov: f32) -> Rect2l {
    match get_normalized_clip_rect_from_bv(min, max, projection, tan_half_fov) {
        Some((n_min, n_max)) => get_clip_rect_from_normalized_min_max(n_min, n_max, screen_size),
        None => Rect2l::new(0, 0, screen_size.x, screen_size.y),
    }
}

/// `GetClipRectFromSphere`: pixel rect of a sphere. Handles spheres that
/// cross the near plane.
pub fn get_clip_rect_from_sphere(
    position: Vector3f,
    radius: f32,
    projection: &FrustumProjection,
    screen_size: Vector2l,
    position_is_screen_space: bool,
    tan_half_fov: f32,
) -> Rect2l {
    let light_pos = if position_is_screen_space { position } else { matrix_mul_vec3(&projection.view, position) };
    let (near_plane, near_top, near_right) = near_extents(projection, tan_half_fov);
    let half = Vector2f::new(screen_size.x as f32, screen_size.y as f32) * 0.5;

    let mut right = screen_size.x.wrapping_sub(1);
    let mut top = 0;
    let mut left = 0;
    let mut bottom = screen_size.y.wrapping_sub(1);

    let sqr_radius = radius * radius;
    let sqr_pos = light_pos * light_pos;

    // XZ planes
    let mut d = sqr_radius * sqr_pos.x - (sqr_pos.x + sqr_pos.z) * (sqr_radius - sqr_pos.z);
    if d >= 0.0 {
        d = d.sqrt();
        let nx1 = (radius * light_pos.x + d) / (sqr_pos.x + sqr_pos.z);
        let nx2 = (radius * light_pos.x - d) / (sqr_pos.x + sqr_pos.z);
        let nz1 = (radius - nx1 * light_pos.x) / light_pos.z;
        let nz2 = (radius - nx2 * light_pos.x) / light_pos.z;
        let pz1 = (sqr_pos.x + sqr_pos.z - sqr_radius) / (light_pos.z - (nz1 / nx1) * light_pos.x);
        let pz2 = (sqr_pos.x + sqr_pos.z - sqr_radius) / (light_pos.z - (nz2 / nx2) * light_pos.x);

        for (nx, nz, pz) in [(nx1, nz1, pz1), (nx2, nz2, pz2)] {
            if pz < 0.0 {
                let mut near_x = nz * near_plane / nx;
                near_x /= near_right;
                let screen_x = f32_to_i32(half.x + half.x * near_x);
                let px = -pz * nz / nx;
                if px < light_pos.x {
                    left = max_i(left, screen_x);
                } else {
                    right = min_i(right, screen_x);
                }
            }
        }
    }

    // YZ planes
    let mut d = sqr_radius * sqr_pos.y - (sqr_pos.y + sqr_pos.z) * (sqr_radius - sqr_pos.z);
    if d >= 0.0 {
        d = d.sqrt();
        let ny1 = (radius * light_pos.y + d) / (sqr_pos.y + sqr_pos.z);
        let ny2 = (radius * light_pos.y - d) / (sqr_pos.y + sqr_pos.z);
        let nz1 = (radius - ny1 * light_pos.y) / light_pos.z;
        let nz2 = (radius - ny2 * light_pos.y) / light_pos.z;
        let pz1 = (sqr_pos.y + sqr_pos.z - sqr_radius) / (light_pos.z - (nz1 / ny1) * light_pos.y);
        let pz2 = (sqr_pos.y + sqr_pos.z - sqr_radius) / (light_pos.z - (nz2 / ny2) * light_pos.y);

        for (ny, nz, pz) in [(ny1, nz1, pz1), (ny2, nz2, pz2)] {
            if pz < 0.0 {
                let mut near_y = nz * near_plane / ny;
                near_y /= near_top;
                let screen_y = f32_to_i32(half.y - half.y * near_y);
                let py = -pz * nz / ny;
                if py > light_pos.y {
                    top = max_i(top, screen_y);
                } else {
                    bottom = min_i(bottom, screen_y);
                }
            }
        }
    }

    Rect2l::new(left, top, right.wrapping_sub(left).wrapping_add(1), bottom.wrapping_sub(top).wrapping_add(1))
}
