//! `cMath` exports. One `hpl_math_*` function per out-of-line `cMath`
//! function; `HPL2/core/sources/math/MathRust.cpp` forwards to these.
//!
//! Conventions: small values by value, matrices by pointer, outputs through
//! (possibly null) pointers written after all inputs are read, so C++ may
//! pass the same object as input and output.

#![allow(clippy::missing_safety_doc, clippy::too_many_arguments)]

use crate::types::*;
use hpl_math::math as m;
use hpl_math::types::EulerRotationOrder;
use hpl_math::{Matrixf, Rect2f, Vector2l};
use std::ffi::c_void;

fn euler_order(order: i32) -> EulerRotationOrder {
    EulerRotationOrder::from_i32(order).unwrap_or(EulerRotationOrder::Xyz)
}

// ---------------------------------------------------------------------------
// RANDOM GENERATION
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_rand_rect_l(min: i32, max: i32) -> i32 {
    m::rand_rect_i(min, max)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_rand_rect_f(min: f32, max: f32) -> f32 {
    m::rand_rect_f(min, max)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_rand_rect_vector2f(min: HplVector2f, max: HplVector2f) -> HplVector2f {
    HplVector2f::from_rs(m::rand_rect_vector2f(min.to_rs(), max.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_rand_rect_vector3f(min: HplVector3f, max: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::rand_rect_vector3f(min.to_rs(), max.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_rand_rect_color(min: HplColor, max: HplColor) -> HplColor {
    HplColor::from_rs(m::rand_rect_color(min.to_rs(), max.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_random_sphere_surface_point(radius: f32) -> HplVector3f {
    HplVector3f::from_rs(m::random_sphere_surface_point(radius))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_random_sphere_inside_point(radius: f32) -> HplVector3f {
    HplVector3f::from_rs(m::random_sphere_inside_point(radius))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_randomize(seed: i32) {
    m::randomize(seed)
}

// ---------------------------------------------------------------------------
// BIT OPERATIONS, COLOR CONVERSION
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_flag_bit(bit_num: i32) -> i32 {
    m::get_flag_bit(bit_num)
}

/// `RGBToHSBHelper`: writes only the fields the C++ writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_rgb_to_hsb_into(rgb: HplColor, out: *mut HplVector3f) {
    let mut v = unsafe { read(out) };
    m::rgb_to_hsb_into(rgb.to_rs(), &mut v);
    unsafe { write_opt(out, v) };
}

/// `HSBToRGBHelper`: writes only the fields the C++ writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_hsb_to_rgb_into(hsb: HplVector3f, out: *mut HplColor) {
    let mut c = unsafe { read(out) };
    m::hsb_to_rgb_into(hsb.to_rs(), &mut c);
    unsafe { write_opt(out, c) };
}

/// `HexToRGBHelper` / `HexToRGBAHelper` on `len` narrow characters.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_hex_to_color(chars: *const u8, len: usize, out: *mut HplColor, with_alpha: bool) {
    let s = unsafe { raw_slice(chars, len) };
    let mut c = unsafe { read(out) };
    m::hex_to_color_bytes(s, &mut c, with_alpha);
    unsafe { write_opt(out, c) };
}

/// `HexWToRGBHelper` / `HexWToRGBAHelper` on `len` wide characters,
/// each widened to 32 bits.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_hex_to_color_wide(chars: *const u32, len: usize, out: *mut HplColor, with_alpha: bool) {
    let s = unsafe { raw_slice(chars, len) };
    let mut c = unsafe { read(out) };
    m::hex_to_color_wide(s, &mut c, with_alpha);
    unsafe { write_opt(out, c) };
}

/// `HexStringToUChar` (whole string, `strtoul` semantics).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_hex_string_to_uchar(chars: *const u8, len: usize) -> u8 {
    m::hex_chars_to_uchar(unsafe { raw_slice(chars, len) })
}

/// `HexStringToUChar(tWString)` (whole string, `wcstoul` semantics).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_hex_string_to_uchar_wide(chars: *const u32, len: usize) -> u8 {
    m::hex_chars_to_uchar(unsafe { raw_slice(chars, len) })
}

/// `RGBToHex` / `RGBAToHex`: writes 6 (or 8 with alpha) upper-case hex
/// digits to `out` (no terminator).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_color_to_hex(color: HplColor, with_alpha: bool, out: *mut u8) {
    let c = color.to_rs();
    let s = if with_alpha { m::rgba_to_hex(c) } else { m::rgb_to_hex(c) };
    unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), out, s.len()) };
}

/// `UCharToHexString`: two upper-case hex digits into `out[0..2]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_uchar_to_hex(value: u8, out: *mut u8) {
    let s = m::uchar_to_hex_string(value);
    unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), out, 2) };
}

// ---------------------------------------------------------------------------
// BV AND RECT INTERSECTION
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_expand_aabb(base_min: *mut HplVector3f, base_max: *mut HplVector3f, add_min: HplVector3f, add_max: HplVector3f) {
    let (mut bmin, mut bmax) = unsafe { (read(base_min), read(base_max)) };
    m::expand_aabb(&mut bmin, &mut bmax, add_min.to_rs(), add_max.to_rs());
    unsafe {
        write_opt(base_min, bmin);
        write_opt(base_max, bmax);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_rect_intersection_l(r1: HplRect2l, r2: HplRect2l) -> bool {
    m::check_rect_intersection_l(r1.to_rs(), r2.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_rect_intersection_f(r1: HplRect2f, r2: HplRect2f) -> bool {
    m::check_rect_intersection_f(r1.to_rs(), r2.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_point_in_rect_intersection(p: HplVector2f, r: HplRect2f) -> bool {
    m::check_point_in_rect_intersection(p.to_rs(), r.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_rect_fit_l(src: HplRect2l, dest: HplRect2l) -> bool {
    m::check_rect_fit_l(src.to_rs(), dest.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_rect_fit_f(src: HplRect2f, dest: HplRect2f) -> bool {
    m::check_rect_fit_f(src.to_rs(), dest.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_clip_rect(src: HplRect2f, dest: HplRect2f) -> HplRect2f {
    HplRect2f::from_rs(m::get_clip_rect(src.to_rs(), dest.to_rs()))
}

/// `CheckSphereLineIntersection`. Out-pointers may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_sphere_line_intersection(
    sphere_pos: HplVector3f,
    sphere_radius: f32,
    line_start: HplVector3f,
    line_end: HplVector3f,
    out_t1: *mut f32,
    out_intersection1: *mut HplVector3f,
    out_t2: *mut f32,
    out_intersection2: *mut HplVector3f,
) -> bool {
    let Some(h) = m::check_sphere_line_intersection(sphere_pos.to_rs(), sphere_radius, line_start.to_rs(), line_end.to_rs()) else {
        return false;
    };
    unsafe {
        write_plain(out_t1, h.t1);
        if h.hit1 {
            write_opt(out_intersection1, h.pos1);
        }
        write_plain(out_t2, h.t2);
        if h.hit2 {
            write_opt(out_intersection2, h.pos2);
        }
    }
    h.hit1 || h.hit2
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_aabb_intersection(min1: HplVector3f, max1: HplVector3f, min2: HplVector3f, max2: HplVector3f) -> bool {
    m::check_aabb_intersection(min1.to_rs(), max1.to_rs(), min2.to_rs(), max2.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_aabb_inside(inside_min: HplVector3f, inside_max: HplVector3f, outside_min: HplVector3f, outside_max: HplVector3f) -> bool {
    m::check_aabb_inside(inside_min.to_rs(), inside_max.to_rs(), outside_min.to_rs(), outside_max.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_point_in_aabb_intersection(point: HplVector3f, min: HplVector3f, max: HplVector3f) -> bool {
    m::check_point_in_aabb_intersection(point.to_rs(), min.to_rs(), max.to_rs())
}

/// `GetNormalizedClipRectFromBV` for a world AABB. Outputs untouched when
/// false is returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_get_normalized_clip_rect_from_bv(
    out_min: *mut HplVector3f,
    out_max: *mut HplVector3f,
    bv_min: HplVector3f,
    bv_max: HplVector3f,
    projection: *const HplFrustumProjection,
    tan_half_fov: f32,
) -> bool {
    let projection = unsafe { (*projection).to_rs() };
    match m::get_normalized_clip_rect_from_bv(bv_min.to_rs(), bv_max.to_rs(), &projection, tan_half_fov) {
        Some((min, max)) => {
            unsafe {
                write_opt(out_min, min);
                write_opt(out_max, max);
            }
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_clip_rect_from_normalized_min_max(min: HplVector3f, max: HplVector3f, screen_size: HplVector2l) -> HplRect2l {
    HplRect2l::from_rs(m::get_clip_rect_from_normalized_min_max(min.to_rs(), max.to_rs(), screen_size.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_get_clip_rect_from_bv(
    bv_min: HplVector3f,
    bv_max: HplVector3f,
    projection: *const HplFrustumProjection,
    screen_size: HplVector2l,
    tan_half_fov: f32,
) -> HplRect2l {
    let projection = unsafe { (*projection).to_rs() };
    HplRect2l::from_rs(m::get_clip_rect_from_bv(bv_min.to_rs(), bv_max.to_rs(), &projection, screen_size.to_rs(), tan_half_fov))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_get_clip_rect_from_sphere(
    position: HplVector3f,
    radius: f32,
    projection: *const HplFrustumProjection,
    screen_size: HplVector2l,
    position_is_screen_space: bool,
    tan_half_fov: f32,
) -> HplRect2l {
    let projection = unsafe { (*projection).to_rs() };
    let size: Vector2l = screen_size.to_rs();
    HplRect2l::from_rs(m::get_clip_rect_from_sphere(position.to_rs(), radius, &projection, size, position_is_screen_space, tan_half_fov))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_sphere_in_planes(center: HplVector3f, radius: f32, planes: *const HplPlanef, plane_count: i32) -> bool {
    m::check_sphere_in_planes(center.to_rs(), radius, unsafe { slice(planes, plane_count.max(0) as usize) })
}

/// `CheckAABBLineIntersection`. `out_t` always receives the closest t
/// (1000 if nothing was hit).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_aabb_line_intersection(
    min: HplVector3f,
    max: HplVector3f,
    line_start: HplVector3f,
    line_end: HplVector3f,
    out_intersection: *mut HplVector3f,
    out_t: *mut f32,
) -> bool {
    let r = m::check_aabb_line_intersection(min.to_rs(), max.to_rs(), line_start.to_rs(), line_end.to_rs());
    unsafe {
        if let Some(p) = r.pos {
            write_opt(out_intersection, p);
        }
        write_plain(out_t, r.t);
    }
    r.hit
}

/// `CheckPointInFlatPolygon`. `out_triangle` receives 3 vectors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_point_in_flat_polygon(
    ray_start: HplVector3f,
    ray_end: HplVector3f,
    point: HplVector2f,
    viewport_size: HplVector2f,
    num_indices: i32,
    indices: *const u32,
    vertex_pos: *const f32,
    stride: i32,
    obj_world: *const HplMatrixf,
    cam_view: *const HplMatrixf,
    cam_proj: *const HplMatrixf,
    out_tri_index: *mut u32,
    out_intersection: *mut HplVector3f,
    out_triangle: *mut HplVector3f,
) -> bool {
    let stride = stride.max(0) as usize;
    let indices = unsafe { raw_slice(indices, num_indices.max(0) as usize) };
    let vertices = unsafe { raw_slice(vertex_pos, vertex_floats_for(indices, stride)) };
    let (obj_world, cam_view, cam_proj): (Matrixf, Matrixf, Matrixf) = unsafe { (read(obj_world), read(cam_view), read(cam_proj)) };
    let hit = m::check_point_in_flat_polygon(
        ray_start.to_rs(),
        ray_end.to_rs(),
        point.to_rs(),
        viewport_size.to_rs(),
        indices,
        vertices,
        stride,
        &obj_world,
        &cam_view,
        &cam_proj,
    );
    let Some(h) = hit else { return false };
    unsafe {
        write_plain(out_tri_index, h.tri_index);
        write_opt(out_intersection, h.intersection_world);
        if !out_triangle.is_null() {
            for (i, v) in h.triangle_world.iter().enumerate() {
                out_triangle.add(i).write(HplVector3f::from_rs(*v));
            }
        }
    }
    true
}

/// `CheckPointInsideTriangle2D`: `triangle` points to 3 vectors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_point_inside_triangle_2d(point: HplVector2f, triangle: *const HplVector3f) -> bool {
    let t = unsafe { slice(triangle, 3) };
    m::check_point_inside_triangle_2d(point.to_rs(), &[t[0], t[1], t[2]])
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_point_same_side_of_edge_2d(p1: HplVector3f, p2: HplVector3f, a: HplVector3f, b: HplVector3f) -> bool {
    m::check_point_same_side_of_edge_2d(p1.to_rs(), p2.to_rs(), a.to_rs(), b.to_rs())
}

// ---------------------------------------------------------------------------
// FLOAT OPERATIONS
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_round_float_to_decimals(val: f32, precision: i32) -> f32 {
    m::round_float_to_decimals(val, precision)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_fraction(val: f32) -> f32 {
    m::get_fraction(val)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_modulus(dividend: f32, divisor: f32) -> f32 {
    m::modulus(dividend, divisor)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_to_rad(angle: f32) -> f32 {
    m::to_rad(angle)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_to_deg(angle: f32) -> f32 {
    m::to_deg(angle)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_log2_to_int(x: i32) -> i32 {
    m::log2_to_int(x)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_pow2(x: i32) -> i32 {
    m::pow2(x)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_is_pow2(x: i32) -> bool {
    m::is_pow2(x)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_wrap(x: f32, min: f32, max: f32) -> f32 {
    m::wrap(x, min, max)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_clamp(x: f32, min: f32, max: f32) -> f32 {
    m::clamp(x, min, max)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_angle_distance(angle1: f32, angle2: f32, max_angle: f32) -> f32 {
    m::get_angle_distance(angle1, angle2, max_angle)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_angle_distance_rad(angle1: f32, angle2: f32) -> f32 {
    m::get_angle_distance_rad(angle1, angle2)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_angle_distance_deg(angle1: f32, angle2: f32) -> f32 {
    m::get_angle_distance_deg(angle1, angle2)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_turn_angle(angle: f32, final_angle: f32, speed: f32, max_angle: f32) -> f32 {
    m::turn_angle(angle, final_angle, speed, max_angle)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_turn_angle_rad(angle: f32, final_angle: f32, speed: f32) -> f32 {
    m::turn_angle_rad(angle, final_angle, speed)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_turn_angle_deg(angle: f32, final_angle: f32, speed: f32) -> f32 {
    m::turn_angle_deg(angle, final_angle, speed)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_interpolate(a: f32, b: f32, t: f32) -> f32 {
    m::interpolate(a, b, t)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_increase_to(x: f32, add: f32, dest: f32) -> f32 {
    m::increase_to(x, add, dest)
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_smooth_curve(x: f32) -> f32 {
    m::smooth_curve(x)
}

// ---------------------------------------------------------------------------
// VECTOR 2D
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector2_dist(a: HplVector2f, b: HplVector2f) -> f32 {
    m::vector2_dist(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector2_dist_xy(a: HplVector3f, b: HplVector3f) -> f32 {
    m::vector2_dist_xy(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector2_dist_xz(a: HplVector3f, b: HplVector3f) -> f32 {
    m::vector2_dist_xz(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector2_dist_sqr(a: HplVector2f, b: HplVector2f) -> f32 {
    m::vector2_dist_sqr(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector2_dist_sqr_xy(a: HplVector3f, b: HplVector3f) -> f32 {
    m::vector2_dist_sqr_xy(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector2_dist_sqr_xz(a: HplVector3f, b: HplVector3f) -> f32 {
    m::vector2_dist_sqr_xz(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector2_increase_to(x: HplVector2f, add: HplVector2f, dest: HplVector2f) -> HplVector2f {
    HplVector2f::from_rs(m::vector2_increase_to(x.to_rs(), add.to_rs(), dest.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_angle_from_points_2d(start: HplVector2f, goal: HplVector2f) -> f32 {
    m::get_angle_from_points_2d(start.to_rs(), goal.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_vector_from_angle_2d(angle: f32, length: f32) -> HplVector2f {
    HplVector2f::from_rs(m::get_vector_from_angle_2d(angle, length))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_get_angle_from_vector(v: HplVector2f, out_angle: *mut f32, out_length: *mut f32) {
    let (angle, length) = m::get_angle_from_vector(v.to_rs());
    unsafe {
        write_plain(out_length, length);
        write_plain(out_angle, angle);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_project_vector_2d(src: HplVector2f, dest: HplVector2f) -> HplVector2f {
    HplVector2f::from_rs(m::project_vector_2d(src.to_rs(), dest.to_rs()))
}

// ---------------------------------------------------------------------------
// VECTOR 3D
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_max_length(v: HplVector3f, max_length: f32) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_max_length(v.to_rs(), max_length))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_min_length(v: HplVector3f, min_length: f32) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_min_length(v.to_rs(), min_length))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_clamp_length(v: HplVector3f, min_length: f32, max_length: f32) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_clamp_length(v.to_rs(), min_length, max_length))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_angle_distance(a: HplVector3f, b: HplVector3f, max_angle: f32) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_angle_distance(a.to_rs(), b.to_rs(), max_angle))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_angle_distance_rad(a: HplVector3f, b: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_angle_distance_rad(a.to_rs(), b.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_angle_distance_deg(a: HplVector3f, b: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_angle_distance_deg(a.to_rs(), b.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_angle_from_points_3d(start: HplVector3f, goal: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::get_angle_from_points_3d(start.to_rs(), goal.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_cross(a: HplVector3f, b: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_cross(a.to_rs(), b.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_dot(a: HplVector3f, b: HplVector3f) -> f32 {
    m::vector3_dot(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_project(src: HplVector3f, dest: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_project(src.to_rs(), dest.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_reflect(v: HplVector3f, normal: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_reflect(v.to_rs(), normal.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_angle(a: HplVector3f, b: HplVector3f) -> f32 {
    m::vector3_angle(a.to_rs(), b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_signed_angle(a: HplVector3f, b: HplVector3f, n: HplVector3f) -> f32 {
    m::vector3_signed_angle(a.to_rs(), b.to_rs(), n.to_rs())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_vector3_unproject(v: HplVector3f, screen_rect: HplRect2f, view_proj: *const HplMatrixf) -> HplVector3f {
    let mtx: Matrixf = unsafe { read(view_proj) };
    let rect: Rect2f = screen_rect.to_rs();
    HplVector3f::from_rs(m::vector3_unproject(v.to_rs(), rect, &mtx))
}

/// `Vector3ClampToLength` (in place).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_vector3_clamp_to_length(v: *mut HplVector3f, max_length: f32) {
    let mut x = unsafe { read(v) };
    m::vector3_clamp_to_length(&mut x, max_length);
    unsafe { write_opt(v, x) };
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_increase_to(x: HplVector3f, add: HplVector3f, dest: HplVector3f) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_increase_to(x.to_rs(), add.to_rs(), dest.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_vector3_orthonormalize_basis(
    src_right: HplVector3f,
    src_up: HplVector3f,
    src_forward: HplVector3f,
    dst_right: *mut HplVector3f,
    dst_up: *mut HplVector3f,
    dst_forward: *mut HplVector3f,
) {
    let (r, u, f) = m::vector3_orthonormalize_basis(src_right.to_rs(), src_up.to_rs(), src_forward.to_rs());
    unsafe {
        write_opt(dst_right, r);
        write_opt(dst_up, u);
        write_opt(dst_forward, f);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_vector3_sphere_surface_point(seed: HplVector2f, radius: f32) -> HplVector3f {
    HplVector3f::from_rs(m::vector3_sphere_surface_point(seed.to_rs(), radius))
}

// ---------------------------------------------------------------------------
// SPHERICAL COORDS
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_spherical_coords_from_point_3d(center: HplVector3f, point: HplVector3f) -> HplVector2f {
    HplVector2f::from_rs(m::get_spherical_coords_from_point_3d(center.to_rs(), point.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_get_point_3d_from_spherical_coords(center: HplVector3f, radius: f32, coords: HplVector2f) -> HplVector3f {
    HplVector3f::from_rs(m::get_point_3d_from_spherical_coords(center.to_rs(), radius, coords.to_rs()))
}

// ---------------------------------------------------------------------------
// PLANES
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_plane_dot(a: HplPlanef, b: HplPlanef) -> f32 {
    m::plane_dot(&a.to_rs(), &b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_plane_parallel(a: HplPlanef, b: HplPlanef) -> bool {
    m::plane_parallel(&a.to_rs(), &b.to_rs())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_transform_plane(transform: *const HplMatrixf, plane: HplPlanef) -> HplPlanef {
    let mtx: Matrixf = unsafe { read(transform) };
    HplPlanef::from_rs(m::transform_plane(&mtx, &plane.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_plane_to_point_dist(plane: HplPlanef, v: HplVector3f) -> f32 {
    m::plane_to_point_dist(&plane.to_rs(), v.to_rs())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_plane_intersection_line(pa: HplPlanef, pb: HplPlanef, out_dir: *mut HplVector3f, out_point: *mut HplVector3f) {
    let (dir, point) = m::plane_intersection_line(&pa.to_rs(), &pb.to_rs());
    unsafe {
        write_opt(out_dir, dir);
        write_opt(out_point, point);
    }
}

/// `PlaneIntersectionPoint`. `out_point` is untouched when false.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_plane_intersection_point(p1: HplPlanef, p2: HplPlanef, p3: HplPlanef, out_point: *mut HplVector3f) -> bool {
    match m::plane_intersection_point(&p1.to_rs(), &p2.to_rs(), &p3.to_rs()) {
        Some(p) => {
            unsafe { write_opt(out_point, p) };
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_plane_sphere_collision(plane: HplPlanef, center: HplVector3f, radius: f32) -> i32 {
    m::check_plane_sphere_collision(&plane.to_rs(), center.to_rs(), radius) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_plane_aabb_collision(plane: HplPlanef, min: HplVector3f, max: HplVector3f) -> i32 {
    m::check_plane_aabb_collision(&plane.to_rs(), min.to_rs(), max.to_rs()) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_check_plane_aabb_sphere_collision(
    plane: HplPlanef,
    min: HplVector3f,
    max: HplVector3f,
    sphere_center: HplVector3f,
    sphere_radius: f32,
) -> i32 {
    m::check_plane_aabb_sphere_collision(&plane.to_rs(), min.to_rs(), max.to_rs(), sphere_center.to_rs(), sphere_radius) as i32
}

/// `CheckPlaneLineIntersection`. Faithful: when the line is not parallel
/// the outputs are written even if the hit is outside the segment.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_plane_line_intersection(
    plane: HplPlanef,
    line_start: HplVector3f,
    line_end: HplVector3f,
    out_intersection: *mut HplVector3f,
    out_t: *mut f32,
) -> bool {
    let Some(h) = m::plane_line_intersection(&plane.to_rs(), line_start.to_rs(), line_end.to_rs()) else {
        return false;
    };
    unsafe {
        write_plain(out_t, h.t);
        write_opt(out_intersection, h.pos);
    }
    !(h.t < 0.0 || h.t > 1.0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_points_planes_collision(vertices: *const HplVector3f, vertex_count: i32, planes: *const HplPlanef, plane_count: i32) -> i32 {
    let v = unsafe { slice(vertices, vertex_count.max(0) as usize) };
    let p = unsafe { slice(planes, plane_count.max(0) as usize) };
    m::check_points_planes_collision(v, p) as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_points_aabb_planes_collision(vertices: *const HplVector3f, vertex_count: i32, min: HplVector3f, max: HplVector3f) -> i32 {
    let v = unsafe { slice(vertices, vertex_count.max(0) as usize) };
    m::check_points_aabb_planes_collision(v, min.to_rs(), max.to_rs()) as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_separating_axis_intersection(
    vertices_a: *const HplVector3f,
    vertex_count_a: i32,
    planes_a: *const HplPlanef,
    plane_count_a: i32,
    vertices_b: *const HplVector3f,
    vertex_count_b: i32,
    planes_b: *const HplPlanef,
    plane_count_b: i32,
) -> bool {
    unsafe {
        m::check_separating_axis_intersection(
            slice(vertices_a, vertex_count_a.max(0) as usize),
            slice(planes_a, plane_count_a.max(0) as usize),
            slice(vertices_b, vertex_count_b.max(0) as usize),
            slice(planes_b, plane_count_b.max(0) as usize),
        )
    }
}

// ---------------------------------------------------------------------------
// TRIANGLES
// ---------------------------------------------------------------------------

/// `CheckLineTriangleIntersection`. `out_t` is written only on a hit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_line_triangle_intersection(
    line_start: HplVector3f,
    line_end: HplVector3f,
    p0: HplVector3f,
    p1: HplVector3f,
    p2: HplVector3f,
    out_t: *mut f32,
    skip_backfacing: bool,
) -> bool {
    match m::check_line_triangle_intersection(line_start.to_rs(), line_end.to_rs(), p0.to_rs(), p1.to_rs(), p2.to_rs(), skip_backfacing) {
        Some(t) => {
            unsafe { write_plain(out_t, t) };
            true
        }
        None => false,
    }
}

/// `CheckLineTriMeshIntersection`. `inv_mesh_mtx` is the INVERSE mesh transform.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_check_line_tri_mesh_intersection(
    line_start: HplVector3f,
    line_end: HplVector3f,
    inv_mesh_mtx: *const HplMatrixf,
    indices: *const u32,
    index_count: i32,
    vertices: *const f32,
    vtx_stride: i32,
    out_intersection: *mut HplVector3f,
    out_t: *mut f32,
    out_tri_index: *mut i32,
    skip_backfacing: bool,
) -> bool {
    let stride = vtx_stride.max(0) as usize;
    let indices = unsafe { raw_slice(indices, index_count.max(0) as usize) };
    let vertices = unsafe { raw_slice(vertices, vertex_floats_for(indices, stride)) };
    let mtx: Matrixf = unsafe { read(inv_mesh_mtx) };
    match m::check_line_tri_mesh_intersection(line_start.to_rs(), line_end.to_rs(), &mtx, indices, vertices, stride, skip_backfacing) {
        Some(h) => {
            unsafe {
                write_plain(out_t, h.t);
                write_opt(out_intersection, h.pos);
                write_plain(out_tri_index, h.tri_index);
            }
            true
        }
        None => false,
    }
}

// ---------------------------------------------------------------------------
// QUATERNIONS
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_quaternion_slerp(t: f32, a: HplQuaternion, b: HplQuaternion, shortest_path: bool) -> HplQuaternion {
    HplQuaternion::from_rs(m::quaternion_slerp(t, a.to_rs(), b.to_rs(), shortest_path))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_quaternion_dot(a: HplQuaternion, b: HplQuaternion) -> f32 {
    m::quaternion_dot(&a.to_rs(), &b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_quaternion_mul(a: HplQuaternion, b: HplQuaternion) -> HplQuaternion {
    HplQuaternion::from_rs(m::quaternion_mul(&a.to_rs(), &b.to_rs()))
}

// ---------------------------------------------------------------------------
// MATRICES
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_slerp(t: f32, a: *const HplMatrixf, b: *const HplMatrixf, shortest_path: bool) -> HplMatrixf {
    let (a, b): (Matrixf, Matrixf) = unsafe { (read(a), read(b)) };
    HplMatrixf::from_rs(m::matrix_slerp(t, &a, &b, shortest_path))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_mul(a: *const HplMatrixf, b: *const HplMatrixf) -> HplMatrixf {
    let (a, b): (Matrixf, Matrixf) = unsafe { (read(a), read(b)) };
    HplMatrixf::from_rs(m::matrix_mul(&a, &b))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_mul_vec3(a: *const HplMatrixf, v: HplVector3f) -> HplVector3f {
    let a: Matrixf = unsafe { read(a) };
    HplVector3f::from_rs(m::matrix_mul_vec3(&a, v.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_mul_3x3(a: *const HplMatrixf, v: HplVector3f) -> HplVector3f {
    let a: Matrixf = unsafe { read(a) };
    HplVector3f::from_rs(m::matrix_mul_3x3(&a, v.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_mul_divide_w(a: *const HplMatrixf, v: HplVector3f) -> HplVector3f {
    let a: Matrixf = unsafe { read(a) };
    HplVector3f::from_rs(m::matrix_mul_divide_w(&a, v.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_mul_scalar(a: *const HplMatrixf, s: f32) -> HplMatrixf {
    let a: Matrixf = unsafe { read(a) };
    HplMatrixf::from_rs(m::matrix_mul_scalar(&a, s))
}

/// `MatrixRotate`. `order` is an `eEulerRotationOrder`.
#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_rotate(rot: HplVector3f, order: i32) -> HplMatrixf {
    // Faithful: an unknown order falls through the C++ switch: identity.
    match EulerRotationOrder::from_i32(order) {
        Some(o) => HplMatrixf::from_rs(m::matrix_rotate(rot.to_rs(), o)),
        None => HplMatrixf::from_rs(Matrixf::IDENTITY),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_rotate_x(angle: f32) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_rotate_x(angle))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_rotate_y(angle: f32) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_rotate_y(angle))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_rotate_z(angle: f32) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_rotate_z(angle))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_scale(scale: HplVector3f) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_scale(scale.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_translate(trans: HplVector3f) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_translate(trans.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_quaternion(q: HplQuaternion) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_quaternion(&q.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_to_quaternion(a: *const HplMatrixf) -> HplQuaternion {
    let a: Matrixf = unsafe { read(a) };
    HplQuaternion::from_rs(m::matrix_to_quaternion(&a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_minor(a: *const HplMatrixf, r0: usize, r1: usize, r2: usize, c0: usize, c1: usize, c2: usize) -> f32 {
    let a: Matrixf = unsafe { read(a) };
    m::matrix_minor(&a, r0, r1, r2, c0, c1, c2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_adjoint(a: *const HplMatrixf) -> HplMatrixf {
    let a: Matrixf = unsafe { read(a) };
    HplMatrixf::from_rs(m::matrix_adjoint(&a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_determinant(a: *const HplMatrixf) -> f32 {
    let a: Matrixf = unsafe { read(a) };
    m::matrix_determinant(&a)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_inverse(a: *const HplMatrixf) -> HplMatrixf {
    let a: Matrixf = unsafe { read(a) };
    HplMatrixf::from_rs(m::matrix_inverse(&a))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_plane_mirror(plane: HplPlanef) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_plane_mirror(&plane.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_to_euler_angles(a: *const HplMatrixf, order: i32) -> HplVector3f {
    let a: Matrixf = unsafe { read(a) };
    // The order is ignored by the C++ too (XYZ only).
    HplVector3f::from_rs(m::matrix_to_euler_angles(&a, euler_order(order)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_matrix_euler_angle_distance(a: *const HplMatrixf, b: *const HplMatrixf) -> HplVector3f {
    let (a, b): (Matrixf, Matrixf) = unsafe { (read(a), read(b)) };
    HplVector3f::from_rs(m::matrix_euler_angle_distance(&a, &b))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_unit_vectors(right: HplVector3f, up: HplVector3f, forward: HplVector3f, translate: HplVector3f) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_unit_vectors(right.to_rs(), up.to_rs(), forward.to_rs(), translate.to_rs()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_projection_matrix_oblique_near_clip_plane(proj: *const HplMatrixf, clip_plane: HplPlanef) -> HplMatrixf {
    let p: Matrixf = unsafe { read(proj) };
    HplMatrixf::from_rs(m::projection_matrix_oblique_near_clip_plane(&p, &clip_plane.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_perspective_projection(near: f32, far: f32, fov: f32, aspect: f32, inf_far_plane: bool) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_perspective_projection(near, far, fov, aspect, inf_far_plane))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_math_matrix_orthographic_projection(near: f32, far: f32, view_size: HplVector2f) -> HplMatrixf {
    HplMatrixf::from_rs(m::matrix_orthographic_projection(near, far, view_size.to_rs()))
}

// ---------------------------------------------------------------------------
// POLYGONS
// ---------------------------------------------------------------------------

/// `CreateTriTangentVectors`. `dest` holds 4 floats per vertex; texture
/// coordinates and normals 3 floats per vertex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_create_tri_tangent_vectors(
    dest: *mut f32,
    indices: *const u32,
    index_count: i32,
    vertices: *const f32,
    vtx_stride: i32,
    tex_coords: *const f32,
    normals: *const f32,
    vertex_count: i32,
) -> bool {
    let n = vertex_count.max(0) as usize;
    let stride = vtx_stride.max(0) as usize;
    let vtx_len = if n == 0 { 0 } else { (n - 1) * stride + 3 };
    unsafe {
        let dest = if dest.is_null() || n == 0 { &mut [][..] } else { std::slice::from_raw_parts_mut(dest, n * 4) };
        m::create_tri_tangent_vectors(
            dest,
            raw_slice(indices, index_count.max(0) as usize),
            raw_slice(vertices, vtx_len),
            stride,
            raw_slice(tex_coords, n * 3),
            raw_slice(normals, n * 3),
            n,
        )
    }
}

/// `CreateTriangleData`. `triangles` must hold `index_count / 3` entries
/// (the C++ caller resizes the vector first).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_create_triangle_data(
    triangles: *mut HplTriangleData,
    indices: *const u32,
    index_count: i32,
    vertices: *const f32,
    vtx_stride: i32,
    vertex_count: i32,
) -> bool {
    let stride = vtx_stride.max(0) as usize;
    let indices = unsafe { raw_slice(indices, index_count.max(0) as usize) };
    let vtx_len = vertex_floats_for(indices, stride).max(if vertex_count > 0 { (vertex_count as usize - 1) * stride + 3 } else { 0 });
    unsafe {
        let tris = slice_mut(triangles, indices.len() / 3);
        m::create_triangle_data(tris, indices, raw_slice(vertices, vtx_len), stride)
    }
}

/// Receives the result of [`hpl_math_create_edges`] (called exactly once).
pub type HplEdgeSink = Option<unsafe extern "C" fn(context: *mut c_void, edges: *const HplTriEdge, count: usize)>;

/// `CreateEdges`: the edge list is handed to `sink` before returning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_math_create_edges(
    indices: *const u32,
    index_count: i32,
    vertices: *const f32,
    vtx_stride: i32,
    out_is_double_sided: *mut bool,
    sink_context: *mut c_void,
    sink: HplEdgeSink,
) -> bool {
    let stride = vtx_stride.max(0) as usize;
    let indices = unsafe { raw_slice(indices, index_count.max(0) as usize) };
    let vertices = unsafe { raw_slice(vertices, vertex_floats_for(indices, stride)) };
    let (edges, double_sided) = m::create_edges(indices, vertices, stride);
    unsafe {
        write_plain(out_is_double_sided, double_sided);
        if let Some(sink) = sink {
            sink(sink_context, edges.as_ptr() as *const HplTriEdge, edges.len());
        }
    }
    true
}
