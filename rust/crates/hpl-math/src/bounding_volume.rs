//! `cBoundingVolume` and `cShadowVolumeBV` (`math/BoundingVolume.h`,
//! `sources/math/BoundingVolume.cpp`).
//!
//! The pure computations ([`compute_extents`], [`compute_shadow_volume`],
//! [`ShadowVolume::collide`], [`points_min_max`]) are what the C++ class calls
//! into. [`BoundingVolume`] is the owning Rust equivalent with the same lazy
//! update rules.

use crate::consts::SQRT_2;
use crate::math::{aabb_corners, check_point_in_aabb_intersection, matrix_mul_vec3, plane_to_point_dist, vector3_cross, vector3_dist, vector3_dot};
use crate::matrix::Matrixf;
use crate::mesh_types::TriEdge;
use crate::types::Planef;
use crate::vector::Vector3f;

/// Box edges as (corner, corner, face, face), in the engine's order.
const BV_EDGES: [TriEdge; 12] = [
    TriEdge::new(1, 0, 0, 2),
    TriEdge::new(3, 1, 0, 5),
    TriEdge::new(2, 3, 0, 3),
    TriEdge::new(0, 2, 0, 4),
    TriEdge::new(0, 4, 4, 2),
    TriEdge::new(4, 6, 4, 1),
    TriEdge::new(6, 2, 4, 3),
    TriEdge::new(4, 5, 1, 2),
    TriEdge::new(5, 7, 1, 5),
    TriEdge::new(7, 6, 1, 3),
    TriEdge::new(1, 5, 2, 5),
    TriEdge::new(3, 7, 5, 3),
];

/// Box face normals: +x, -x, +y, -y, +z, -z.
const FACE_NORMALS: [Vector3f; 6] = [
    Vector3f::new(1.0, 0.0, 0.0),
    Vector3f::new(-1.0, 0.0, 0.0),
    Vector3f::new(0.0, 1.0, 0.0),
    Vector3f::new(0.0, -1.0, 0.0),
    Vector3f::new(0.0, 0.0, 1.0),
    Vector3f::new(0.0, 0.0, -1.0),
];

/// A corner lying on each face.
const FACE_POINTS: [usize; 6] = [0, 5, 5, 6, 4, 7];

/// Derived size data of a bounding volume (the first half of `UpdateSize`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extents {
    /// Rotated AABB relative to the position.
    pub max: Vector3f,
    pub min: Vector3f,
    pub size: Vector3f,
    /// Centre offset from the position.
    pub pivot: Vector3f,
    pub radius: f32,
}

/// `UpdateSize` (size part): AABB of the local box rotated by `transform`.
pub fn compute_extents(transform: &Matrixf, local_min: Vector3f, local_max: Vector3f) -> Extents {
    let rot = transform.rotation();
    let corners = aabb_corners(local_min, local_max).map(|c| matrix_mul_vec3(&rot, c));

    let mut max = corners[0];
    let mut min = corners[0];
    for c in &corners[1..] {
        for axis in 0..3 {
            if c[axis] < min[axis] {
                min[axis] = c[axis];
            } else if c[axis] > max[axis] {
                max[axis] = c[axis];
            }
        }
    }
    let size = max - min;
    let pivot = max - (size * 0.5);
    let radius = vector3_dist(pivot, max);
    Extents { max, min, size, pivot, radius }
}

/// `CreateFromPoints` inner loop: extend `min`/`max` with `count` points read
/// every `stride` floats from `points`.
pub fn points_min_max(points: &[f32], count: usize, stride: usize, min: &mut Vector3f, max: &mut Vector3f) {
    for i in 0..count {
        let p = &points[i * stride..i * stride + 3];
        for axis in 0..3 {
            if p[axis] < min[axis] {
                min[axis] = p[axis];
            }
            if p[axis] > max[axis] {
                max[axis] = p[axis];
            }
        }
    }
}

/// `cShadowVolumeBV`: the volume shadowed by a box lit by a point light.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShadowVolume {
    /// Cap planes first, then one side plane per silhouette quad.
    pub planes: Vec<Planef>,
    pub cap_planes: usize,
    /// Silhouette quads (4 points each).
    pub points: Vec<Vector3f>,
}

impl ShadowVolume {
    /// `CollideBoundingVolume`, given the other volume's world centre,
    /// radius, AABB and position.
    pub fn collide(&self, center: Vector3f, radius: f32, min: Vector3f, max: Vector3f, position: Vector3f) -> bool {
        // Sphere test
        if self.planes.iter().any(|p| plane_to_point_dist(p, center) < -radius) {
            return false;
        }
        // AABB test, with the position added as a ninth point.
        let c = aabb_corners(min, max);
        let points = [c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], position];
        for plane in &self.planes {
            let in_count = points.iter().filter(|p| !(plane_to_point_dist(plane, **p) < 0.0)).count();
            if in_count == 0 {
                return false;
            }
        }
        true
    }
}

/// `GetShadowVolume` (computation part) for a box with world bounds
/// `min`/`max`. The caller handles the "light inside the box" early-out.
pub fn compute_shadow_volume(min: Vector3f, max: Vector3f, light_pos: Vector3f, light_range: f32) -> ShadowVolume {
    let corners = aabb_corners(min, max);
    let mut volume = ShadowVolume { planes: Vec::with_capacity(12), cap_planes: 0, points: Vec::with_capacity(24) };

    // Faces turned towards the light become cap planes.
    let mut facing = [false; 6];
    for face in 0..6 {
        let corner = corners[FACE_POINTS[face]];
        facing[face] = vector3_dot(FACE_NORMALS[face], corner - light_pos) < 0.0;
        if facing[face] {
            volume.planes.push(Planef::from_normal_point(FACE_NORMALS[face] * -1.0, corner));
        }
    }
    volume.cap_planes = volume.planes.len();

    // Extrude silhouette edges away from the light.
    let push_length = light_range * SQRT_2;
    let push = |p: Vector3f| {
        let mut dir = p - light_pos;
        dir.normalize();
        p + dir * push_length
    };
    for edge in &BV_EDGES {
        let f1 = facing[edge.tri1 as usize];
        let f2 = facing[edge.tri2 as usize];
        if f1 != f2 {
            let (a, b) = (corners[edge.point1 as usize], corners[edge.point2 as usize]);
            let (first, second) = if f1 { (a, b) } else { (b, a) };
            volume.points.extend_from_slice(&[first, second, push(second), push(first)]);
        }
    }

    // Side planes, normals pointing inwards.
    for quad in volume.points.chunks_exact(4) {
        let normal = vector3_cross(quad[1] - quad[0], quad[2] - quad[0]);
        let mut plane = Planef::from_normal_point(normal, quad[0]);
        plane.normalize();
        volume.planes.push(plane);
    }
    volume
}

/// Owning Rust equivalent of `cBoundingVolume`. Fields keep the C++ names;
/// world bounds are refreshed lazily by [`update_size`](Self::update_size).
#[derive(Clone, Debug, PartialEq)]
pub struct BoundingVolume {
    pub transform: Matrixf,
    pub local_max: Vector3f,
    pub local_min: Vector3f,
    pub max: Vector3f,
    pub min: Vector3f,
    pub pivot: Vector3f,
    pub world_max: Vector3f,
    pub world_min: Vector3f,
    /// Serialised but never updated by the engine (kept for parity).
    pub position: Vector3f,
    pub size: Vector3f,
    pub radius: f32,
    position_updated: bool,
    size_updated: bool,
    shadow_planes_need_update: bool,
    shadow_volume: ShadowVolume,
}

impl Default for BoundingVolume {
    fn default() -> Self {
        Self {
            transform: Matrixf::IDENTITY,
            local_max: Vector3f::splat(0.0),
            local_min: Vector3f::splat(0.0),
            max: Vector3f::splat(0.0),
            min: Vector3f::splat(0.0),
            pivot: Vector3f::splat(0.0),
            world_max: Vector3f::splat(0.0),
            world_min: Vector3f::splat(0.0),
            position: Vector3f::splat(0.0),
            size: Vector3f::splat(0.0),
            radius: 0.0,
            position_updated: true,
            size_updated: true,
            shadow_planes_need_update: true,
            shadow_volume: ShadowVolume::default(),
        }
    }
}

impl BoundingVolume {
    pub fn new() -> Self {
        Self::default()
    }

    /// `GetMax` (world)
    pub fn get_max(&mut self) -> Vector3f {
        self.update_size();
        self.world_max
    }

    /// `GetMin` (world)
    pub fn get_min(&mut self) -> Vector3f {
        self.update_size();
        self.world_min
    }

    pub fn local_max(&self) -> Vector3f {
        self.local_max
    }

    pub fn local_min(&self) -> Vector3f {
        self.local_min
    }

    /// `SetLocalMinMax`
    pub fn set_local_min_max(&mut self, min: Vector3f, max: Vector3f) {
        self.local_max = max;
        self.local_min = min;
        self.size_updated = true;
    }

    /// `GetLocalCenter`
    pub fn local_center(&self) -> Vector3f {
        (self.local_min + self.local_max) / 2.0
    }

    /// `GetWorldCenter`
    pub fn world_center(&mut self) -> Vector3f {
        self.update_size();
        self.transform.translation() + self.pivot
    }

    /// `SetPosition`
    pub fn set_position(&mut self, pos: Vector3f) {
        self.transform.set_translation(pos);
        self.position_updated = true;
    }

    /// `GetPosition`
    pub fn get_position(&self) -> Vector3f {
        self.transform.translation()
    }

    /// `SetTransform`
    pub fn set_transform(&mut self, transform: &Matrixf) {
        self.transform = *transform;
        self.size_updated = true;
    }

    /// `GetTransform`
    pub fn transform(&self) -> &Matrixf {
        &self.transform
    }

    /// `SetSize`: centred box of the given size.
    pub fn set_size(&mut self, size: Vector3f) {
        self.local_max = size * 0.5;
        self.local_min = size * -0.5;
        self.size_updated = true;
    }

    /// `GetSize`
    pub fn get_size(&mut self) -> Vector3f {
        self.update_size();
        self.size
    }

    /// `GetRadius`
    pub fn get_radius(&mut self) -> f32 {
        self.update_size();
        self.radius
    }

    /// `CreateFromPoints` over a list of (array, point count) pairs.
    pub fn create_from_points(&mut self, arrays: &[(&[f32], usize)], stride: usize) {
        self.local_max = Vector3f::splat(-100000.0);
        self.local_min = Vector3f::splat(100000.0);
        for (array, count) in arrays {
            points_min_max(array, *count, stride, &mut self.local_min, &mut self.local_max);
        }
        self.position_updated = true;
        self.size_updated = true;
    }

    /// `GetShadowVolume`. `None` if the light is inside the box.
    pub fn shadow_volume(&mut self, light_pos: Vector3f, light_range: f32, force_update: bool) -> Option<&ShadowVolume> {
        let (min, max) = (self.get_min(), self.get_max());
        if check_point_in_aabb_intersection(light_pos, min, max) {
            return None;
        }
        if force_update || self.shadow_planes_need_update {
            self.shadow_volume = compute_shadow_volume(min, max, light_pos, light_range);
        }
        Some(&self.shadow_volume)
    }

    /// `UpdateSize`
    pub fn update_size(&mut self) {
        if self.size_updated {
            let e = compute_extents(&self.transform, self.local_min, self.local_max);
            self.max = e.max;
            self.min = e.min;
            self.size = e.size;
            self.pivot = e.pivot;
            self.radius = e.radius;
            self.size_updated = false;
            self.position_updated = true;
        }
        if self.position_updated {
            let t = self.transform.translation();
            self.world_max = t + self.max;
            self.world_min = t + self.min;
            self.position_updated = false;
            self.shadow_planes_need_update = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::matrix_rotate_y;

    #[test]
    fn rotated_box_extents() {
        let mut bv = BoundingVolume::new();
        bv.set_size(Vector3f::new(2.0, 2.0, 2.0));
        bv.set_position(Vector3f::new(10.0, 0.0, 0.0));
        assert_eq!(bv.get_max(), Vector3f::new(11.0, 1.0, 1.0));
        assert_eq!(bv.get_radius(), 3f32.sqrt());

        let mut t = matrix_rotate_y(std::f32::consts::FRAC_PI_4);
        t.set_translation(Vector3f::new(10.0, 0.0, 0.0));
        bv.set_transform(&t);
        let half_diag = 2f32.sqrt();
        assert!((bv.get_max().x - (10.0 + half_diag)).abs() < 1e-5);
    }

    #[test]
    fn shadow_volume_shape() {
        let v = compute_shadow_volume(Vector3f::splat(-1.0), Vector3f::splat(1.0), Vector3f::new(5.0, 0.0, 0.0), 10.0);
        assert_eq!(v.cap_planes, 1);
        assert_eq!(v.points.len(), 16);
        assert_eq!(v.planes.len(), 5);
        // A box directly behind the lit box is shadowed; one beside the light is not.
        let behind = Vector3f::new(-5.0, 0.0, 0.0);
        assert!(v.collide(behind, 0.87, behind - 0.5, behind + 0.5, behind));
        let beside = Vector3f::new(5.0, 5.0, 0.0);
        assert!(!v.collide(beside, 0.87, beside - 0.5, beside + 0.5, beside));
    }
}
