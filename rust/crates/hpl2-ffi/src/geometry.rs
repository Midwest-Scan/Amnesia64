//! `cFrustum`, `cBoundingVolume` and `cShadowVolumeBV` exports. The C++
//! classes keep their data members and call these for the computations.

#![allow(clippy::missing_safety_doc, clippy::too_many_arguments)]

use crate::types::*;
use hpl_math::bounding_volume::{ShadowVolume, compute_extents, compute_shadow_volume, points_min_max};
use hpl_math::frustum::{compute_bounding_sphere, compute_bounds, compute_planes, update_vertices};
use hpl_math::{Matrixf, Planef, Vector3f};

// ---------------------------------------------------------------------------
// cFrustum: setup
// ---------------------------------------------------------------------------

/// `UpdatePlanes`: writes 6 planes. `custom_far_proj` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_compute_planes(
    view_proj: *const HplMatrixf,
    view: *const HplMatrixf,
    custom_far_proj: *const HplMatrixf,
    out_planes: *mut HplPlanef,
) {
    let (vp, v): (Matrixf, Matrixf) = unsafe { (read(view_proj), read(view)) };
    let custom: Option<Matrixf> = (!custom_far_proj.is_null()).then(|| unsafe { read(custom_far_proj) });
    let planes = compute_planes(&vp, &v, custom.as_ref());
    for (i, p) in planes.into_iter().enumerate() {
        unsafe { out_planes.add(i).write(HplPlanef::from_rs(p)) };
    }
}

/// `UpdateSphere`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_compute_bounding_sphere(
    projection: *const HplFrustumProjection,
    origin: HplVector3f,
    ortho_view_size: HplVector2f,
) -> HplSpheref {
    let projection = unsafe { (*projection).to_rs() };
    HplSpheref::from_rs(compute_bounding_sphere(&projection, origin.to_rs(), ortho_view_size.to_rs()))
}

/// `UpdateVertices`: updates the 8 corners in place (a corner whose planes
/// are parallel keeps its old value).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_update_vertices(planes: *const HplPlanef, oblique_near_plane: bool, origin: HplVector3f, vertices: *mut HplVector3f) {
    let planes: [Planef; 6] = unsafe { (planes as *const [Planef; 6]).read() };
    let mut verts: [Vector3f; 8] = unsafe { (vertices as *const [Vector3f; 8]).read() };
    update_vertices(&planes, oblique_near_plane, origin.to_rs(), &mut verts);
    unsafe { (vertices as *mut [Vector3f; 8]).write(verts) };
}

/// `UpdateBV`: AABB of the 8 corners.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_compute_bounds(vertices: *const HplVector3f, out_min: *mut HplVector3f, out_max: *mut HplVector3f) {
    let verts: [Vector3f; 8] = unsafe { (vertices as *const [Vector3f; 8]).read() };
    let (min, max) = compute_bounds(&verts);
    unsafe {
        write_opt(out_min, min);
        write_opt(out_max, max);
    }
}

// ---------------------------------------------------------------------------
// cFrustum: collision
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_collide_point(frustum: *const HplFrustumView, point: HplVector3f) -> bool {
    unsafe { (*frustum).as_ref() }.collide_point(point.to_rs())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_collide_frustum_sphere(frustum: *const HplFrustumView, center: HplVector3f, radius: f32) -> i32 {
    unsafe { (*frustum).as_ref() }.collide_frustum_sphere(center.to_rs(), radius) as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_collide_sphere(frustum: *const HplFrustumView, center: HplVector3f, radius: f32, max_planes: i32) -> i32 {
    unsafe { (*frustum).as_ref() }.collide_sphere(center.to_rs(), radius, max_planes) as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_collide_aabb(frustum: *const HplFrustumView, min: HplVector3f, max: HplVector3f) -> i32 {
    unsafe { (*frustum).as_ref() }.collide_aabb(min.to_rs(), max.to_rs()) as i32
}

/// `CollideBoundingVolume` / `CollideNode`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_collide_bounding_volume(
    frustum: *const HplFrustumView,
    center: HplVector3f,
    radius: f32,
    min: HplVector3f,
    max: HplVector3f,
) -> i32 {
    unsafe { (*frustum).as_ref() }.collide_bounding_volume(center.to_rs(), radius, min.to_rs(), max.to_rs()) as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_collide_frustum(frustum: *const HplFrustumView, other: *const HplFrustumView) -> i32 {
    unsafe { (*frustum).as_ref().collide_frustum(&(*other).as_ref()) as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_check_sphere_near_plane_intersection(frustum: *const HplFrustumView, center: HplVector3f, radius: f32) -> bool {
    unsafe { (*frustum).as_ref() }.check_sphere_near_plane_intersection(center.to_rs(), radius)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_check_aabb_near_plane_intersection(frustum: *const HplFrustumView, min: HplVector3f, max: HplVector3f) -> bool {
    unsafe { (*frustum).as_ref() }.check_aabb_near_plane_intersection(min.to_rs(), max.to_rs())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_check_bv_near_plane_intersection(
    frustum: *const HplFrustumView,
    center: HplVector3f,
    radius: f32,
    min: HplVector3f,
    max: HplVector3f,
) -> bool {
    unsafe { (*frustum).as_ref() }.check_bv_near_plane_intersection(center.to_rs(), radius, min.to_rs(), max.to_rs())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_check_frustum_near_plane_intersection(frustum: *const HplFrustumView, other: *const HplFrustumView) -> bool {
    unsafe { (*frustum).as_ref().check_frustum_near_plane_intersection(&(*other).as_ref()) }
}

/// `CheckLineIntersection`. `out_intersection` is written only on a hit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_frustum_check_line_intersection(
    frustum: *const HplFrustumView,
    start: HplVector3f,
    end: HplVector3f,
    out_intersection: *mut HplVector3f,
) -> bool {
    match unsafe { (*frustum).as_ref() }.check_line_intersection(start.to_rs(), end.to_rs()) {
        Some(p) => {
            unsafe { write_opt(out_intersection, p) };
            true
        }
        None => false,
    }
}

// ---------------------------------------------------------------------------
// cBoundingVolume
// ---------------------------------------------------------------------------

/// `UpdateSize` (size part): extents of the local box under `transform`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_bv_compute_extents(transform: *const HplMatrixf, local_min: HplVector3f, local_max: HplVector3f) -> HplBvExtents {
    let t: Matrixf = unsafe { read(transform) };
    HplBvExtents::from_rs(compute_extents(&t, local_min.to_rs(), local_max.to_rs()))
}

/// `CreateFromPoints` inner loop: extend `min`/`max` (in/out) with `count`
/// points read every `stride` floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_bv_points_min_max(points: *const f32, count: i32, stride: i32, min: *mut HplVector3f, max: *mut HplVector3f) {
    let count = count.max(0) as usize;
    let stride = stride.max(0) as usize;
    let len = if count == 0 { 0 } else { (count - 1) * stride + 3 };
    let pts = unsafe { raw_slice(points, len) };
    let (mut mn, mut mx) = unsafe { (read(min), read(max)) };
    points_min_max(pts, count, stride, &mut mn, &mut mx);
    unsafe {
        write_opt(min, mn);
        write_opt(max, mx);
    }
}

/// Maximum planes / points a box shadow volume can have.
pub const HPL_SHADOW_VOLUME_MAX_PLANES: usize = 12;
pub const HPL_SHADOW_VOLUME_MAX_POINTS: usize = 48;

/// `GetShadowVolume` (computation part). `out_planes` holds
/// `HPL_SHADOW_VOLUME_MAX_PLANES`, `out_points` holds `points_capacity`
/// entries; counts are written to the `out_*_count` pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_bv_compute_shadow_volume(
    world_min: HplVector3f,
    world_max: HplVector3f,
    light_pos: HplVector3f,
    light_range: f32,
    out_planes: *mut HplPlanef,
    out_plane_count: *mut i32,
    out_cap_planes: *mut i32,
    out_points: *mut HplVector3f,
    points_capacity: usize,
    out_point_count: *mut i32,
) {
    let v: ShadowVolume = compute_shadow_volume(world_min.to_rs(), world_max.to_rs(), light_pos.to_rs(), light_range);
    let planes = &v.planes[..v.planes.len().min(HPL_SHADOW_VOLUME_MAX_PLANES)];
    let points = &v.points[..v.points.len().min(points_capacity)];
    unsafe {
        for (i, p) in planes.iter().enumerate() {
            out_planes.add(i).write(HplPlanef::from_rs(*p));
        }
        for (i, p) in points.iter().enumerate() {
            out_points.add(i).write(HplVector3f::from_rs(*p));
        }
        write_plain(out_plane_count, planes.len() as i32);
        write_plain(out_cap_planes, v.cap_planes as i32);
        write_plain(out_point_count, points.len() as i32);
    }
}

/// `cShadowVolumeBV::CollideBoundingVolume`, given the other volume's world
/// centre, radius, AABB and position.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_bv_shadow_volume_collide(
    planes: *const HplPlanef,
    plane_count: i32,
    center: HplVector3f,
    radius: f32,
    min: HplVector3f,
    max: HplVector3f,
    position: HplVector3f,
) -> bool {
    let planes = unsafe { slice(planes, plane_count.max(0) as usize) }.to_vec();
    let volume = ShadowVolume { planes, cap_planes: 0, points: Vec::new() };
    volume.collide(center.to_rs(), radius, min.to_rs(), max.to_rs(), position.to_rs())
}
