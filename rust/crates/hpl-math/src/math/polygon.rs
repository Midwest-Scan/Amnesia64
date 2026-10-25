//! POLYGON MANAGEMENT: tangents, triangle normals and shadow edges.

use super::vec3::{vector3_cross, vector3_dot};
use crate::mesh_types::{TriEdge, TriangleData};
use crate::vector::Vector3f;
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[inline]
fn get_vector3(array: &[f32], idx: usize, stride: usize) -> Vector3f {
    let b = idx * stride;
    Vector3f::new(array[b], array[b + 1], array[b + 2])
}

#[inline]
fn vector3_equal(a: &[f32], idx_a: usize, b: &[f32], idx_b: usize, stride: usize) -> bool {
    a[idx_a * stride] == b[idx_b * stride] && a[idx_a * stride + 1] == b[idx_b * stride + 1] && a[idx_a * stride + 2] == b[idx_b * stride + 2]
}

/// `CreateTriTangentVectors`: per-vertex 4D tangents (xyz + handedness w)
/// written to `dest` (4 floats per vertex). Texture coordinates and normals
/// have a stride of 3; positions have `vtx_stride`.
#[allow(clippy::too_many_arguments)]
pub fn create_tri_tangent_vectors(
    dest: &mut [f32],
    indices: &[u32],
    vertices: &[f32],
    vtx_stride: usize,
    tex_coords: &[f32],
    normals: &[f32],
    vertex_num: usize,
) -> bool {
    let mut tangents1 = vec![Vector3f::splat(0.0); vertex_num];
    let mut tangents2 = vec![Vector3f::splat(0.0); vertex_num];

    for tri in indices.chunks_exact(3) {
        let (i1, i2, i3) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);

        let pos1 = get_vector3(vertices, i1, vtx_stride);
        let pos2 = get_vector3(vertices, i2, vtx_stride);
        let pos3 = get_vector3(vertices, i3, vtx_stride);
        let tex1 = get_vector3(tex_coords, i1, 3);
        let tex2 = get_vector3(tex_coords, i2, 3);
        let tex3 = get_vector3(tex_coords, i3, 3);

        let pos1_to_2 = pos2 - pos1;
        let pos1_to_3 = pos3 - pos1;
        let tex1_to_2 = tex2 - tex1;
        let tex1_to_3 = tex3 - tex1;

        let r = 1.0 / (tex1_to_2.x * tex1_to_3.y - tex1_to_2.y * tex1_to_3.x);
        let s_dir = Vector3f::new(
            (tex1_to_3.y * pos1_to_2.x - tex1_to_2.y * pos1_to_3.x) * r,
            (tex1_to_3.y * pos1_to_2.y - tex1_to_2.y * pos1_to_3.y) * r,
            (tex1_to_3.y * pos1_to_2.z - tex1_to_2.y * pos1_to_3.z) * r,
        );
        let t_dir = Vector3f::new(
            (tex1_to_2.x * pos1_to_3.x - tex1_to_3.x * pos1_to_2.x) * r,
            (tex1_to_2.x * pos1_to_3.y - tex1_to_3.x * pos1_to_2.y) * r,
            (tex1_to_2.x * pos1_to_3.z - tex1_to_3.x * pos1_to_2.z) * r,
        );

        for i in [i1, i2, i3] {
            tangents1[i] += s_dir;
        }
        for i in [i1, i2, i3] {
            tangents2[i] += t_dir;
        }
    }

    // Smooth tangents across duplicated vertices (same position and normal).
    let max_cos_angle = -1.0f32;
    for i in 0..vertex_num {
        for j in i + 1..vertex_num {
            if vector3_equal(vertices, i, vertices, j, vtx_stride) && vector3_equal(normals, i, normals, j, 3) {
                let (at1, at2) = (tangents1[i], tangents2[i]);
                let (bt1, bt2) = (tangents1[j], tangents2[j]);
                if vector3_dot(at1, bt1) >= max_cos_angle {
                    tangents1[j] += at1;
                    tangents1[i] += bt1;
                }
                if vector3_dot(at2, bt2) >= max_cos_angle {
                    tangents2[j] += at2;
                    tangents2[i] += bt2;
                }
            }
        }
    }

    for vtx in 0..vertex_num {
        let normal = get_vector3(normals, vtx, 3);
        let t1 = tangents1[vtx];
        let t2 = tangents2[vtx];

        // Gram-Schmidt orthogonalise
        let mut tan = t1 - (normal * vector3_dot(normal, t1));
        tan.normalize();

        // Handedness
        let w = if vector3_dot(vector3_cross(normal, t1), t2) < 0.0 { -1.0 } else { 1.0 };
        dest[vtx * 4..vtx * 4 + 4].copy_from_slice(&[tan.x, tan.y, tan.z, w]);
    }
    true
}

/// `CreateTriangleData`: unnormalised face normals (edge2 x edge1) into
/// `triangles[..index_count / 3]`; `triangles` must be at least that long.
pub fn create_triangle_data(triangles: &mut [TriangleData], indices: &[u32], vertices: &[f32], vtx_stride: usize) -> bool {
    for (tri, idx) in triangles.iter_mut().zip(indices.chunks_exact(3)) {
        let v0 = &vertices[idx[0] as usize * vtx_stride..];
        let v1 = &vertices[idx[1] as usize * vtx_stride..];
        let v2 = &vertices[idx[2] as usize * vtx_stride..];
        let edge1 = Vector3f::new(v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]);
        let edge2 = Vector3f::new(v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]);
        tri.normal = vector3_cross(edge2, edge1);
    }
    true
}

/// Total order wrapper reproducing a C++ strict-weak-ordering comparator.
#[derive(Clone, Copy)]
struct CppOrd<T, F: Fn(&T, &T) -> bool>(T, F);

impl<T, F: Fn(&T, &T) -> bool> PartialEq for CppOrd<T, F> {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl<T, F: Fn(&T, &T) -> bool> Eq for CppOrd<T, F> {}
impl<T, F: Fn(&T, &T) -> bool> PartialOrd for CppOrd<T, F> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl<T, F: Fn(&T, &T) -> bool> Ord for CppOrd<T, F> {
    fn cmp(&self, o: &Self) -> Ordering {
        if (self.1)(&self.0, &o.0) {
            Ordering::Less
        } else if (self.1)(&o.0, &self.0) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

/// `CreateEdges`: build the edge list of a triangle mesh for shadow volumes.
///
/// Vertices are welded by position. Each edge records the two triangles that
/// share it; an edge with only one triangle gets `tri2 = -1` and
/// `invert_tri2 = true`, which marks the mesh as double sided (the second
/// return value). Edge order and the record kept for duplicates follow the
/// C++ `std::map`/`std::set` iteration exactly.
pub fn create_edges(indices: &[u32], vertices: &[f32], vtx_stride: usize) -> (Vec<TriEdge>, bool) {
    let pos = |i: u32| get_vector3(vertices, i as usize, vtx_stride);
    let vec_less = |a: &Vector3f, b: &Vector3f| a.lex_lt(b);

    // Indices grouped by welded vertex position (std::map<cVector3f, ...>).
    let mut vtx_indices: BTreeMap<CppOrd<Vector3f, _>, Vec<usize>> = BTreeMap::new();
    for (idx, &vi) in indices.iter().enumerate() {
        vtx_indices.entry(CppOrd(pos(vi), vec_less)).or_default().push(idx);
    }

    // Edges keyed by their end-point positions, point1 > point2 (cEdgeCompare).
    let edge_greater = |e1: &(Vector3f, Vector3f), e2: &(Vector3f, Vector3f)| {
        let (a1, a2) = e1;
        let (b1, b2) = e2;
        for (x, y) in [(a1.x, b1.x), (a1.y, b1.y), (a1.z, b1.z), (a2.x, b2.x), (a2.y, b2.y), (a2.z, b2.z)] {
            if x != y {
                return x > y;
            }
        }
        false
    };
    let mut edges: BTreeMap<CppOrd<(Vector3f, Vector3f), _>, TriEdge> = BTreeMap::new();

    let mut add_edge = |mut edge: TriEdge| {
        // CheckEdgeSwitch: make point1 the "larger" position.
        if pos(edge.point1 as u32).lex_lt(&pos(edge.point2 as u32)) {
            std::mem::swap(&mut edge.point1, &mut edge.point2);
        }
        let key = CppOrd((pos(edge.point1 as u32), pos(edge.point2 as u32)), edge_greater);
        match edges.get_mut(&key) {
            None => {
                edge.tri2 = -1;
                edges.insert(key, edge);
            }
            Some(existing) => {
                if existing.tri2 != -1 || existing.tri1 == edge.tri1 {
                    return;
                }
                existing.tri2 = edge.tri1;
            }
        }
    };

    for idx_list in vtx_indices.values() {
        for &idx in idx_list {
            let tri_start = (idx / 3) * 3;
            let vtx = indices[idx] as i32;
            let tri = (tri_start / 3) as i32;
            let idx_in_tri = idx % 3;
            let next = if idx_in_tri + 1 > 2 { 0 } else { idx_in_tri + 1 };
            let prev = if idx_in_tri == 0 { 2 } else { idx_in_tri - 1 };

            let edge1 = TriEdge::new(vtx, indices[tri_start + next] as i32, tri, 0);
            let edge2 = TriEdge::new(vtx, indices[tri_start + prev] as i32, tri, 0);
            add_edge(edge1);
            add_edge(edge2);
        }
    }

    // Orient each edge so point1 -> point2 follows triangle 1's winding.
    let mut double_sided = false;
    let mut result = Vec::with_capacity(edges.len());
    for mut edge in edges.into_values() {
        let tri1 = &indices[edge.tri1 as usize * 3..edge.tri1 as usize * 3 + 3];
        if edge.tri2 == -1 {
            edge.invert_tri2 = true;
            double_sided = true;
        } else {
            edge.invert_tri2 = false;
        }

        let point1_in_tri = (0..3)
            .find(|&i| vector3_equal(vertices, tri1[i] as usize, vertices, edge.point1 as usize, vtx_stride))
            .unwrap_or(0);
        let next_in_tri = if point1_in_tri + 1 >= 3 { 0 } else { point1_in_tri + 1 };
        if vector3_equal(vertices, tri1[next_in_tri] as usize, vertices, edge.point2 as usize, vtx_stride) {
            std::mem::swap(&mut edge.point1, &mut edge.point2);
        }
        result.push(edge);
    }
    (result, double_sided)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_edges() {
        // Two triangles sharing the diagonal 0-2.
        #[rustfmt::skip]
        let verts = [0.0, 0.0, 0.0,  1.0, 0.0, 0.0,  1.0, 1.0, 0.0,  0.0, 1.0, 0.0];
        let idx = [0, 1, 2, 0, 2, 3];
        let (edges, double_sided) = create_edges(&idx, &verts, 3);
        assert_eq!(edges.len(), 5);
        assert!(double_sided); // open mesh: border edges have no second triangle
        let shared: Vec<_> = edges.iter().filter(|e| e.tri2 >= 0).collect();
        assert_eq!(shared.len(), 1);
        assert_eq!((shared[0].tri1, shared[0].tri2), (0, 1));
    }

    #[test]
    fn triangle_normals() {
        let verts = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut tris = [TriangleData::default()];
        create_triangle_data(&mut tris, &[0, 1, 2], &verts, 3);
        assert_eq!(tris[0].normal, Vector3f::new(0.0, 0.0, -1.0));
    }
}
