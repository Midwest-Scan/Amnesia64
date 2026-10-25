//! `cFrustum` (`math/Frustum.h`, `sources/math/Frustum.cpp`).
//!
//! The collision logic lives on [`FrustumRef`], a borrowed view of the
//! frustum's planes, corners and bounds, so the same code serves the owning
//! Rust [`Frustum`] and the C++ `cFrustum` (which passes its own arrays).

use crate::bounding_volume::BoundingVolume;
use crate::math::{
    aabb_corners, check_aabb_intersection, check_plane_aabb_collision, check_plane_line_intersection, check_plane_sphere_collision,
    check_point_in_aabb_intersection, check_point_in_sphere_intersection, check_points_aabb_planes_collision, check_points_planes_collision,
    matrix_mul, max_i, plane_intersection_point, plane_to_point_dist, vector3_cross, vector3_normalize,
};
use crate::matrix::Matrixf;
use crate::types::{Collision, FrustumPlane, Planef, Spheref};
use crate::vector::{Vector2f, Vector3f};

/// `eProjectionType`
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectionType {
    #[default]
    Perspective = 0,
    Orthographic = 1,
}

/// The camera parameters `cMath`'s clip-rect functions read from a frustum.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrustumProjection {
    pub proj: Matrixf,
    pub view: Matrixf,
    pub near_plane: f32,
    pub far_plane: f32,
    pub fov: f32,
    pub aspect: f32,
    pub projection_type: ProjectionType,
}

/// Borrowed view of a frustum's derived geometry.
#[derive(Clone, Copy, Debug)]
pub struct FrustumRef<'a> {
    pub planes: &'a [Planef; 6],
    /// Near-plane corners 0..4 then far-plane corners 4..8.
    pub vertices: &'a [Vector3f; 8],
    pub sphere: Spheref,
    /// World AABB of `vertices`.
    pub bv_min: Vector3f,
    pub bv_max: Vector3f,
    pub inf_far_plane: bool,
}

impl<'a> FrustumRef<'a> {
    #[inline]
    fn plane_count(&self) -> usize {
        if self.inf_far_plane { 5 } else { 6 }
    }

    #[inline]
    fn active_planes(&self) -> &'a [Planef] {
        &self.planes[..self.plane_count()]
    }

    #[inline]
    fn near_plane(&self) -> &'a Planef {
        &self.planes[FrustumPlane::Near as usize]
    }

    /// `CollidePoint`
    pub fn collide_point(&self, point: Vector3f) -> bool {
        if !check_point_in_aabb_intersection(point, self.bv_min, self.bv_max) {
            return false;
        }
        self.active_planes().iter().all(|p| !(plane_to_point_dist(p, point) < 0.0))
    }

    /// `CollideFustrumSphere`: test against the frustum's bounding sphere.
    pub fn collide_frustum_sphere(&self, center: Vector3f, radius: f32) -> Collision {
        let radius_sum = self.sphere.r + radius;
        let sep_axis = self.sphere.center - center;
        if sep_axis.sqr_length() > radius_sum * radius_sum { Collision::Outside } else { Collision::Intersect }
    }

    /// `CollideSphere`. Faithful: the plane count is `Max(5 or 6, max_planes)`,
    /// so the default of 6 also tests the far plane of an infinite frustum.
    pub fn collide_sphere(&self, center: Vector3f, radius: f32, max_planes: i32) -> Collision {
        let count = max_i(self.plane_count() as i32, max_planes).clamp(0, 6) as usize;
        for plane in &self.planes[..count] {
            let dist = plane_to_point_dist(plane, center);
            if dist < -radius {
                return Collision::Outside;
            }
            if dist.abs() < radius {
                return Collision::Intersect;
            }
        }
        Collision::Inside
    }

    /// `CollideAABB`
    pub fn collide_aabb(&self, min: Vector3f, max: Vector3f) -> Collision {
        let corners = aabb_corners(min, max);
        match check_points_planes_collision(&corners, self.active_planes()) {
            Collision::Outside => return Collision::Outside,
            Collision::Inside => return Collision::Inside,
            Collision::Intersect => {}
        }
        if check_points_aabb_planes_collision(self.vertices, min, max) == Collision::Outside {
            return Collision::Outside;
        }
        Collision::Intersect
    }

    /// `CollideBoundingVolume` / `CollideNode`, given the volume's world
    /// centre, radius and AABB.
    pub fn collide_bounding_volume(&self, center: Vector3f, radius: f32, min: Vector3f, max: Vector3f) -> Collision {
        if self.collide_frustum_sphere(center, radius) == Collision::Outside {
            return Collision::Outside;
        }
        let ret = self.collide_sphere(center, radius, 6);
        if ret == Collision::Intersect {
            return self.collide_aabb(min, max);
        }
        ret
    }

    /// `CollideFrustum`
    pub fn collide_frustum(&self, other: &FrustumRef<'_>) -> Collision {
        if !check_aabb_intersection(self.bv_min, self.bv_max, other.bv_min, other.bv_max) {
            return Collision::Outside;
        }
        match check_points_planes_collision(other.vertices, self.active_planes()) {
            Collision::Outside => return Collision::Outside,
            Collision::Inside => return Collision::Inside,
            Collision::Intersect => {}
        }
        if check_points_planes_collision(self.vertices, other.active_planes()) == Collision::Outside {
            return Collision::Outside;
        }
        Collision::Intersect
    }

    /// `CheckSphereNearPlaneIntersection`
    pub fn check_sphere_near_plane_intersection(&self, center: Vector3f, radius: f32) -> bool {
        if check_plane_sphere_collision(self.near_plane(), center, radius) != Collision::Intersect {
            return false;
        }
        let near_corners = &self.vertices[..4];
        if near_corners.iter().any(|v| check_point_in_sphere_intersection(*v, center, radius)) {
            return true;
        }
        // For each near corner, a plane tangent to the sphere facing the
        // corner: if all corners are outside it, there is no intersection.
        for v in near_corners {
            let to_vtx = vector3_normalize(*v - center);
            let plane_point = center + to_vtx * radius;
            let plane_normal = to_vtx * -1.0;
            let plane = Planef::from_normal_point(plane_normal, plane_point);
            if check_points_planes_collision(near_corners, std::slice::from_ref(&plane)) == Collision::Outside {
                return false;
            }
        }
        true
    }

    /// `CheckAABBNearPlaneIntersection`
    pub fn check_aabb_near_plane_intersection(&self, min: Vector3f, max: Vector3f) -> bool {
        if check_plane_aabb_collision(self.near_plane(), min, max) != Collision::Intersect {
            return false;
        }
        let near_corners = &self.vertices[..4];
        if near_corners.iter().any(|v| check_point_in_aabb_intersection(*v, min, max)) {
            return true;
        }
        check_points_aabb_planes_collision(near_corners, min, max) != Collision::Outside
    }

    /// `CheckBVNearPlaneIntersection`, given the volume's world centre,
    /// radius and AABB.
    pub fn check_bv_near_plane_intersection(&self, center: Vector3f, radius: f32, min: Vector3f, max: Vector3f) -> bool {
        if check_plane_aabb_collision(self.near_plane(), min, max) != Collision::Intersect {
            return false;
        }
        let near_corners = &self.vertices[..4];
        if near_corners.iter().any(|v| check_point_in_aabb_intersection(*v, min, max)) {
            return true;
        }
        if self.collide_sphere(center, radius, 4) == Collision::Inside {
            return true;
        }
        check_points_aabb_planes_collision(near_corners, min, max) != Collision::Outside
    }

    /// `CheckFrustumNearPlaneIntersection`
    pub fn check_frustum_near_plane_intersection(&self, other: &FrustumRef<'_>) -> bool {
        let near = self.near_plane();
        if check_plane_sphere_collision(near, other.sphere.center, other.sphere.r) != Collision::Intersect {
            return false;
        }
        // Entirely behind (or on) the near plane?
        if other.vertices.iter().all(|v| !(plane_to_point_dist(near, *v) > 0.0)) {
            return false;
        }
        check_points_planes_collision(&self.vertices[..4], other.active_planes()) != Collision::Outside
    }

    /// `CheckLineIntersection`: closest point where the segment enters the frustum.
    ///
    /// Deviation: with NaN input the C++ can write the out-parameter while
    /// returning false; this returns `None`.
    pub fn check_line_intersection(&self, start: Vector3f, end: Vector3f) -> Option<Vector3f> {
        let mut closest_t = -1.0f32;
        let mut result = None;
        for plane in self.active_planes() {
            if let Some(hit) = check_plane_line_intersection(plane, start, end) {
                if (hit.t < closest_t || closest_t < 0.0) && self.collide_point(hit.pos + plane.normal() * 0.001) {
                    result = Some(hit.pos);
                    closest_t = hit.t;
                }
            }
        }
        if closest_t >= 0.0 { result } else { None }
    }
}

/// `UpdatePlanes`: the six planes from the view-projection matrix
/// (`custom_far_proj` replaces the projection used for the far plane).
pub fn compute_planes(view_proj: &Matrixf, view: &Matrixf, custom_far_proj: Option<&Matrixf>) -> [Planef; 6] {
    let m = &view_proj.m;
    let add = |row: usize| Planef::new(m[3][0] + m[row][0], m[3][1] + m[row][1], m[3][2] + m[row][2], m[3][3] + m[row][3]);
    let sub = |m: &[[f32; 4]; 4], row: usize| Planef::new(m[3][0] - m[row][0], m[3][1] - m[row][1], m[3][2] - m[row][2], m[3][3] - m[row][3]);

    let far = match custom_far_proj {
        Some(p) => sub(&matrix_mul(p, view).m, 2),
        None => sub(m, 2),
    };
    let mut planes = [add(0), sub(m, 0), add(1), sub(m, 1), add(2), far];
    planes.iter_mut().for_each(Planef::normalize);
    planes
}

/// `UpdateSphere`: the frustum's bounding sphere.
pub fn compute_bounding_sphere(projection: &FrustumProjection, origin: Vector3f, ortho_view_size: Vector2f) -> Spheref {
    let (near, far) = (projection.near_plane, projection.far_plane);
    let view_len = far - near;
    let radius = match projection.projection_type {
        ProjectionType::Perspective => {
            let top = far * (projection.fov * 0.5).tan();
            let right = top * projection.aspect;
            // Halfway point between near/far along z, to a far corner.
            let p = Vector3f::new(0.0, 0.0, near + view_len * 0.5);
            let q = Vector3f::new(right, top, view_len);
            (p - q).length()
        }
        ProjectionType::Orthographic => {
            Vector3f::new(ortho_view_size.x * 0.5, ortho_view_size.y * 0.5, view_len * 0.5).length()
        }
    };
    let look = projection.view.forward() * -1.0;
    let center = origin + (look * (view_len * 0.5 + near));
    Spheref::new(center, radius)
}

/// Which side planes meet the near/far plane at each corner.
const PLANE_TYPE_ORDER: [[FrustumPlane; 2]; 4] = [
    [FrustumPlane::Right, FrustumPlane::Top],
    [FrustumPlane::Right, FrustumPlane::Bottom],
    [FrustumPlane::Left, FrustumPlane::Bottom],
    [FrustumPlane::Left, FrustumPlane::Top],
];

/// `UpdateVertices`: corners from plane intersections, updated in place.
///
/// Faithful: a corner whose planes are parallel keeps its previous value.
/// Does not work with an infinite far plane (as in C++).
pub fn update_vertices(planes: &[Planef; 6], oblique_near_plane: bool, origin: Vector3f, vertices: &mut [Vector3f; 8]) {
    let p = |t: FrustumPlane| &planes[t as usize];
    let set = |v: &mut Vector3f, a: &Planef, b: &Planef, c: &Planef| {
        if let Some(point) = plane_intersection_point(a, b, c) {
            *v = point;
        }
    };
    for (i, cap) in [FrustumPlane::Near, FrustumPlane::Far].into_iter().enumerate() {
        for (j, [side0, side1]) in PLANE_TYPE_ORDER.iter().enumerate() {
            set(&mut vertices[i * 4 + j], p(cap), p(*side0), p(*side1));
        }
    }

    // An oblique near plane can put near corners behind the camera or the
    // far plane; pull them back onto the near/far intersection.
    if oblique_near_plane {
        let near = p(FrustumPlane::Near);
        let far = p(FrustumPlane::Far);
        let near_cross_far = vector3_cross(far.normal(), near.normal());
        let fake_near = Planef::from_normal_point(far.normal() * -1.0, origin);

        for (i, [side0, side1]) in PLANE_TYPE_ORDER.iter().enumerate() {
            if plane_to_point_dist(&fake_near, vertices[i]) < 0.0 || plane_to_point_dist(far, vertices[i]) < 0.0 {
                set(&mut vertices[i], far, p(*side0), p(*side1));
                let split = Planef::from_normal_point(near_cross_far, vertices[i]);
                set(&mut vertices[i], far, near, &split);
            }
        }
        for v in vertices[4..].iter_mut() {
            if plane_to_point_dist(near, *v) < 0.0 {
                let split = Planef::from_normal_point(near_cross_far, *v);
                set(v, far, near, &split);
            }
        }
    }
}

/// `UpdateBV`: AABB of the corners. Faithful: uses the C++ `else if`, so a
/// corner can only extend one side per axis.
pub fn compute_bounds(vertices: &[Vector3f; 8]) -> (Vector3f, Vector3f) {
    let mut min = vertices[0];
    let mut max = vertices[0];
    for v in &vertices[1..] {
        for axis in 0..3 {
            if max[axis] < v[axis] {
                max[axis] = v[axis];
            } else if min[axis] > v[axis] {
                min[axis] = v[axis];
            }
        }
    }
    (min, max)
}

/// Owning Rust equivalent of `cFrustum`.
#[derive(Clone, Debug, Default)]
pub struct Frustum {
    pub projection: FrustumProjection,
    view_size: Vector2f,
    inf_far_plane: bool,
    inverts_cull_mode: bool,
    oblique_near_plane: bool,
    origin: Vector3f,
    origin_bv: BoundingVolume,
    view_proj: Matrixf,
    planes: [Planef; 6],
    sphere: Spheref,
    bounding_volume: BoundingVolume,
    vertices: [Vector3f; 8],
}

impl Frustum {
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetupPerspectiveProj`
    #[allow(clippy::too_many_arguments)]
    pub fn setup_perspective_proj(
        &mut self,
        proj: &Matrixf,
        view: &Matrixf,
        far_plane: f32,
        near_plane: f32,
        fov: f32,
        aspect: f32,
        origin: Vector3f,
        inf_far_plane: bool,
        custom_far_proj: Option<&Matrixf>,
        oblique_near_plane: bool,
    ) {
        self.projection.projection_type = ProjectionType::Perspective;
        self.projection.fov = fov;
        self.projection.aspect = aspect;
        self.oblique_near_plane = oblique_near_plane;
        self.setup(proj, view, far_plane, near_plane, origin, inf_far_plane, custom_far_proj);
    }

    /// `SetupOrthoProj`
    #[allow(clippy::too_many_arguments)]
    pub fn setup_ortho_proj(&mut self, proj: &Matrixf, view: &Matrixf, far_plane: f32, near_plane: f32, view_size: Vector2f, origin: Vector3f, inf_far_plane: bool) {
        self.projection.projection_type = ProjectionType::Orthographic;
        self.view_size = view_size;
        self.setup(proj, view, far_plane, near_plane, origin, inf_far_plane, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn setup(&mut self, proj: &Matrixf, view: &Matrixf, far_plane: f32, near_plane: f32, origin: Vector3f, inf_far_plane: bool, custom_far_proj: Option<&Matrixf>) {
        self.view_proj = matrix_mul(proj, view);
        self.projection.view = *view;
        self.projection.proj = *proj;
        self.projection.far_plane = far_plane;
        self.projection.near_plane = near_plane;
        self.origin = origin;
        self.inf_far_plane = inf_far_plane;

        self.origin_bv.set_size(Vector3f::splat(near_plane * 2.0));
        self.origin_bv.set_position(origin);

        self.planes = compute_planes(&self.view_proj, view, custom_far_proj);
        self.sphere = compute_bounding_sphere(&self.projection, self.origin, self.view_size);
        update_vertices(&self.planes, self.oblique_near_plane, self.origin, &mut self.vertices);
        let (min, max) = compute_bounds(&self.vertices);
        self.bounding_volume.set_local_min_max(min, max);
        self.bounding_volume.update_size();
    }

    /// Borrowed view for the collision tests.
    pub fn as_ref(&self) -> FrustumRef<'_> {
        FrustumRef {
            planes: &self.planes,
            vertices: &self.vertices,
            sphere: self.sphere,
            bv_min: self.bounding_volume.world_min,
            bv_max: self.bounding_volume.world_max,
            inf_far_plane: self.inf_far_plane,
        }
    }

    pub fn plane(&self, plane: FrustumPlane) -> &Planef {
        &self.planes[plane as usize]
    }
    pub fn vertex(&self, idx: usize) -> Vector3f {
        self.vertices[idx]
    }
    pub fn projection_matrix(&self) -> &Matrixf {
        &self.projection.proj
    }
    pub fn view_matrix(&self) -> &Matrixf {
        &self.projection.view
    }
    pub fn far_plane(&self) -> f32 {
        self.projection.far_plane
    }
    pub fn near_plane(&self) -> f32 {
        self.projection.near_plane
    }
    pub fn aspect(&self) -> f32 {
        self.projection.aspect
    }
    pub fn fov(&self) -> f32 {
        self.projection.fov
    }
    pub fn inf_far_plane(&self) -> bool {
        self.inf_far_plane
    }
    pub fn set_inverts_cull_mode(&mut self, x: bool) {
        self.inverts_cull_mode = x;
    }
    pub fn inverts_cull_mode(&self) -> bool {
        self.inverts_cull_mode
    }
    pub fn projection_type(&self) -> ProjectionType {
        self.projection.projection_type
    }
    pub fn ortho_view_size(&self) -> Vector2f {
        self.view_size
    }
    pub fn bounding_sphere(&self) -> Spheref {
        self.sphere
    }
    pub fn bounding_volume(&self) -> &BoundingVolume {
        &self.bounding_volume
    }
    pub fn origin(&self) -> Vector3f {
        self.origin
    }
    pub fn origin_bv(&mut self) -> &mut BoundingVolume {
        &mut self.origin_bv
    }
    /// `GetForward`: the view matrix's forward row.
    pub fn forward(&self) -> Vector3f {
        self.projection.view.forward()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::matrix_perspective_projection;

    fn camera() -> Frustum {
        let proj = matrix_perspective_projection(0.1, 100.0, 1.0, 1.5, false);
        let mut f = Frustum::new();
        f.setup_perspective_proj(&proj, &Matrixf::IDENTITY, 100.0, 0.1, 1.0, 1.5, Vector3f::splat(0.0), false, None, false);
        f
    }

    #[test]
    fn point_in_front_is_inside() {
        let f = camera();
        assert!(f.as_ref().collide_point(Vector3f::new(0.0, 0.0, -10.0)));
        assert!(!f.as_ref().collide_point(Vector3f::new(0.0, 0.0, 10.0)));
    }

    #[test]
    fn box_tests() {
        let f = camera();
        let r = f.as_ref();
        let c = Vector3f::new(0.0, 0.0, -10.0);
        assert_eq!(r.collide_bounding_volume(c, 0.87, c - 0.5, c + 0.5), Collision::Inside);
        let behind = Vector3f::new(0.0, 0.0, 10.0);
        assert_eq!(r.collide_bounding_volume(behind, 0.87, behind - 0.5, behind + 0.5), Collision::Outside);
    }
}
