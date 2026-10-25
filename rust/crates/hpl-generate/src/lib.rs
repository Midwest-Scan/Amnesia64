//! Algorithms from `HPL2/core/sources/generate/VoxelMap.cpp`.
//!
//! C++ retains allocation, destruction and the `cGenerate` factory shell.
//! Rendering uses a host so graphics callbacks may resize or edit the map
//! without Rust retaining a borrowed buffer or stale loop bounds.

pub mod voxel_map;
