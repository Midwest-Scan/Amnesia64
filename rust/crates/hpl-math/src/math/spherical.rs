//! SPHERICAL COORDS

use super::scalar::wrap;
use crate::consts::{EPSILON, TWO_PI};
use crate::vector::{Vector2f, Vector3f};

/// `GetSphericalCoordsFromPoint3D`: (theta, phi) of `point` around `center`.
pub fn get_spherical_coords_from_point_3d(center: Vector3f, point: Vector3f) -> Vector2f {
    let mut polar = Vector2f::splat(0.0);
    let radius_vec = point - center;
    let radius = radius_vec.length();
    let inv_radius = 1.0 / radius;

    polar.x = (radius_vec.y * inv_radius).acos();
    polar.x = wrap(polar.x, 0.0, TWO_PI);

    // Faithful: uses the absolute point, not the offset from the centre.
    let radius_in_section = Vector2f::new(point.x, point.z).length();
    if radius_in_section > EPSILON {
        polar.y = radius_vec.z.atan2(radius_vec.x);
        polar.y = wrap(polar.y, 0.0, TWO_PI);
    }
    polar
}

/// `GetPoint3DFromSphericalCoords`
pub fn get_point_3d_from_spherical_coords(center: Vector3f, radius: f32, coords: Vector2f) -> Vector3f {
    let sin_theta = coords.x.sin();
    let cos_theta = coords.x.cos();
    let sin_phi = coords.y.sin();
    let cos_phi = coords.y.cos();
    let dir = Vector3f::new(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi);
    center + dir * radius
}
