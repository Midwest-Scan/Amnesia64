//! Rust port of the HPL2 engine math module.
//!
//! Source of truth for behaviour is the original C++ in
//! `HPL2/core/include/math` and `HPL2/core/sources/math`. Every function keeps
//! the original operation order so results are bit-identical to the C++
//! engine (the differential test in `rust/tools/math-diff` enforces this).
//! Quirks of the original are preserved on purpose and marked `Faithful:`.
//!
//! | C++                         | Rust                                   |
//! |-----------------------------|----------------------------------------|
//! | `cVector2<T>`, `cVector3<T>`| [`Vector2`], [`Vector3`]               |
//! | `cMatrixf`                  | [`Matrixf`]                            |
//! | `cQuaternion`               | [`Quaternion`]                         |
//! | `cPlanef`, `cSpheref`, `cRect2<T>` | [`Planef`], [`Spheref`], [`Rect2`] |
//! | `cColor`                    | [`Color`]                              |
//! | `cMath::*`                  | [`math`] (free functions, snake_case)  |
//! | `cFrustum`                  | [`frustum`]                            |
//! | `cBoundingVolume`           | [`bounding_volume`]                    |
//! | `cCRC`                      | [`crc`]                                |
//! | `cSpring`, `cPidController` | [`Spring`], [`PidController`]          |

// Comparisons are written exactly as in C++ so NaN behaves the same:
// `!(a < b)` is not `a >= b`, and `range.contains(&t)` rejects NaN where
// `t < 0 || t > 1` does not.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::manual_range_contains)]

pub mod bounding_volume;
pub mod color;
pub mod consts;
pub mod crc;
pub mod frustum;
pub mod math;
pub mod matrix;
pub mod mesh_types;
pub mod pid;
pub mod quaternion;
pub mod spring;
pub mod types;
pub mod util;
pub mod vector;

pub use bounding_volume::BoundingVolume;
pub use color::Color;
pub use frustum::{Frustum, FrustumProjection, FrustumRef, ProjectionType};
pub use matrix::Matrixf;
pub use mesh_types::{TriEdge, TriangleData};
pub use pid::PidController;
pub use quaternion::Quaternion;
pub use spring::Spring;
pub use types::{Collision, EulerRotationOrder, FrustumPlane, LinearOscillation, Planef, Rect2, Rect2f, Rect2l, SmoothInterpolation, Spheref};
pub use vector::{Vector2, Vector2f, Vector2l, Vector3, Vector3f, Vector3l};
