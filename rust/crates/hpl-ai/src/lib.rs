//! HPL2 AI algorithms. C++ retains graph ownership and native callbacks.

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::manual_range_contains)]

pub mod astar;
pub mod astar_path;
pub mod node_container;
pub mod node_compile;
pub mod node_generator;
pub mod state_machine;
