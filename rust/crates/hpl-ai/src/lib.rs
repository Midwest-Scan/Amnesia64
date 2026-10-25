//! HPL2 AI. `nav` owns the navigation graph and pathfinding state (phase 2);
//! the other modules are the algorithms it runs, and the state-machine and
//! node-generator logic driven by C++ (phase 1).

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::manual_range_contains)]

pub mod astar;
pub mod astar_path;
pub mod nav;
pub mod node_container;
pub mod node_compile;
pub mod node_generator;
pub mod state_machine;
