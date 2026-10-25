//! TRIANGLES

use super::matrix_ops::matrix_mul_vec3;
use super::vec3::{vector3_cross, vector3_dot};
use crate::matrix::Matrixf;
use crate::vector::Vector3f;

/// `CheckLineTriangleIntersection` (Moller-Trumbore on a segment). Returns
/// the parametric t in [0, 1] of the hit.
pub fn check_line_triangle_intersection(start: Vector3f, end: Vector3f, p0: Vector3f, p1: Vector3f, p2: Vector3f, skip_backfacing: bool) -> Option<f32> {
    let delta = end - start;
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;

    if skip_backfacing {
        let normal = vector3_cross(edge1, edge2);
        if vector3_dot(normal, delta) < 0.0 {
            return None;
        }
    }

    let d_cross_e = vector3_cross(delta, edge2);
    let e_dot_n = vector3_dot(edge1, d_cross_e);
    if e_dot_n.abs() < 0.00001 {
        return None; // Line is parallel
    }
    let f = 1.0 / e_dot_n;
    let tri_to_start = start - p0;

    let u = f * vector3_dot(tri_to_start, d_cross_e);
    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = vector3_cross(tri_to_start, edge1);
    let v = f * vector3_dot(delta, q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * vector3_dot(edge2, q);
    if t < 0.0 || t > 1.0 {
        return None;
    }
    Some(t)
}

/// Result of [`check_line_tri_mesh_intersection`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriMeshHit {
    /// World-space hit position (on the original, untransformed line).
    pub pos: Vector3f,
    pub t: f32,
    /// Index into the index array of the hit triangle's first index.
    pub tri_index: i32,
}

/// `CheckLineTriMeshIntersection`: closest hit of a segment against an
/// indexed mesh. `inv_mesh_mtx` is the INVERSE of the mesh transform.
/// Vertex `i` is read at `vertices[i * stride..]`.
pub fn check_line_tri_mesh_intersection(
    start: Vector3f,
    end: Vector3f,
    inv_mesh_mtx: &Matrixf,
    indices: &[u32],
    vertices: &[f32],
    stride: usize,
    skip_backfacing: bool,
) -> Option<TriMeshHit> {
    let local_start = matrix_mul_vec3(inv_mesh_mtx, start);
    let local_end = matrix_mul_vec3(inv_mesh_mtx, end);

    let vertex = |i: u32| {
        let b = i as usize * stride;
        Vector3f::new(vertices[b], vertices[b + 1], vertices[b + 2])
    };

    let mut min_t = 99999.0f32;
    let mut tri_idx = -1i32;
    for (n, tri) in indices.chunks_exact(3).enumerate() {
        let (p0, p1, p2) = (vertex(tri[0]), vertex(tri[1]), vertex(tri[2]));
        if let Some(t) = check_line_triangle_intersection(local_start, local_end, p0, p1, p2, skip_backfacing) {
            if t < min_t {
                min_t = t;
                tri_idx = (n * 3) as i32;
            }
        }
    }

    if min_t < 0.0 || min_t > 1.0 {
        return None;
    }
    Some(TriMeshHit { pos: start + (end - start) * min_t, t: min_t, tri_index: tri_idx })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_hits_triangle() {
        let t = check_line_triangle_intersection(
            Vector3f::new(0.2, 0.2, -1.0),
            Vector3f::new(0.2, 0.2, 1.0),
            Vector3f::new(0.0, 0.0, 0.0),
            Vector3f::new(1.0, 0.0, 0.0),
            Vector3f::new(0.0, 1.0, 0.0),
            true,
        );
        assert_eq!(t, Some(0.5));
    }

    #[test]
    fn mesh_reports_closest() {
        #[rustfmt::skip]
        let verts = [
            0.0, 0.0, 0.0,  1.0, 0.0, 0.0,  0.0, 1.0, 0.0,
            0.0, 0.0, 0.5,  1.0, 0.0, 0.5,  0.0, 1.0, 0.5,
        ];
        let idx = [3, 4, 5, 0, 1, 2];
        let hit = check_line_tri_mesh_intersection(
            Vector3f::new(0.2, 0.2, -1.0),
            Vector3f::new(0.2, 0.2, 1.0),
            &Matrixf::IDENTITY,
            &idx,
            &verts,
            3,
            true,
        )
        .unwrap();
        assert_eq!(hit.tri_index, 3);
        assert_eq!(hit.t, 0.5);
    }
}
