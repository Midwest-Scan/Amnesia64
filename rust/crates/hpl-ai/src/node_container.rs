//! Numeric and traversal helpers from `ai/AINodeContainer.cpp`.
//!
//! Original audit: rebuilding never clears retained cell lists; dimensions
//! include an extra border row/column; bounds retain the first NaN/signed zero;
//! insertion guards zero cell extents but queries do not. Native allocation,
//! list ownership, sorting, physics and iterator list state remain in C++.
//! Empty builds, invalid float casts, signed dimension/index/increment overflow
//! and out-of-buffer indices are undefined in C++; replacements are marked below.

use hpl_math::{Vector2f, Vector2l, Vector3f, util::f32_to_i32};

/// Initialized grid members available before cell-size calculation. This is a
/// numeric snapshot, never a memory view of the C++ container.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GridDimensions {
    pub minimum: Vector2f,
    pub maximum: Vector2f,
    pub map_size: Vector2l,
}

/// `AddEdge`: compute both metrics independently, then let C++ append the edge.
pub fn edge_distances(start: Vector3f, end: Vector3f) -> (f32, f32) {
    // Faithful: squaring the rounded distance changes the stored squared metric.
    (distance_squared(start, end).sqrt(), distance_squared(start, end))
}

fn distance_squared(start: Vector3f, end: Vector3f) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let dz = end.z - start.z;
    let xx = dx * dx;
    let yy = dy * dy;
    let zz = dz * dz;
    // Faithful: this AddEdge call site also selects squared-sum NaNs in y/x/z
    // priority on MSVC. In particular, a payload NaN can beat Inf-Inf's NaN.
    // Keep ordinary arithmetic (xx + yy) + zz and the observed rule local.
    if yy.is_nan() { f32::from_bits(yy.to_bits() | 0x00400000) }
    else if xx.is_nan() { f32::from_bits(xx.to_bits() | 0x00400000) }
    else if zz.is_nan() { f32::from_bits(zz.to_bits() | 0x00400000) }
    else { (xx + yy) + zz }
}

pub fn grid_dimension(node_count: usize, nodes_per_grid: i32) -> i32 {
    let rounded = (node_count as f32 / nodes_per_grid as f32).sqrt() + 0.5;
    // Deviation: invalid C++ casts use the engine's x86 INT_MIN convention.
    // Signed overflow aborts instead of becoming profile-dependent.
    f32_to_i32(rounded).checked_add(1).expect("AI grid dimension overflow")
}

pub fn grid_storage_count(dimension: i32) -> usize {
    let side = dimension.checked_add(1).expect("AI grid border overflow");
    let count = side.checked_mul(side).expect("AI grid allocation count overflow");
    count as usize
}

pub fn cell_size(dimensions: GridDimensions) -> Vector2f {
    let extent = dimensions.maximum - dimensions.minimum;
    Vector2f::new(extent.x / dimensions.map_size.x as f32, extent.y / dimensions.map_size.y as f32)
}

/// Build insertion is deliberately separate from the unconditional query casts.
pub fn insertion_position(position: Vector3f, minimum: Vector2f, cell: Vector2f) -> Vector2l {
    let local = Vector2f::new(position.x, position.z) - minimum;
    let mut grid = Vector2l::splat(0);
    if cell.x > 0.0 { grid.x = f32_to_i32(local.x / cell.x); }
    if cell.y > 0.0 { grid.y = f32_to_i32(local.y / cell.y); }
    grid
}

/// `GetGridPosFromLocal`: truncation then lower clamps then inclusive upper clamps.
pub fn grid_position(local: Vector2f, cell: Vector2f, map_size: Vector2l) -> Vector2l {
    // Deviation: NaN/out-of-range casts (including zero-extent division) use
    // the existing x86 INT_MIN result, rather than Rust's saturating cast.
    let mut grid = Vector2l::new(f32_to_i32(local.x / cell.x), f32_to_i32(local.y / cell.y));
    if grid.x < 0 { grid.x = 0; }
    if grid.y < 0 { grid.y = 0; }
    if grid.x > map_size.x { grid.x = map_size.x; }
    if grid.y > map_size.y { grid.y = map_size.y; }
    grid
}

/// `GetGrid`/build insertion use the original flat indexing, without axis bounds.
pub fn grid_index(position: Vector2l, map_size: Vector2l, count: usize) -> usize {
    // Deviation: signed overflow and invalid storage indices abort instead of
    // invoking C++ UB. Valid flat aliases are still accepted.
    let stride = map_size.x.checked_add(1).expect("AI grid stride overflow");
    let index = position.y.checked_mul(stride).and_then(|row| row.checked_add(position.x))
        .expect("AI grid index overflow");
    assert!(index >= 0 && (index as usize) < count, "AI grid index outside storage");
    index as usize
}

/// `IncGridPos`: even exhaustion leaves the coordinates advanced.
pub fn increment_grid(position: &mut Vector2l, start: Vector2l, end: Vector2l) -> bool {
    // Deviation: original signed increment overflow is UB; abort consistently.
    position.x = position.x.checked_add(1).expect("AI iterator x overflow");
    if position.x > end.x {
        position.x = start.x;
        position.y = position.y.checked_add(1).expect("AI iterator y overflow");
        if position.y > end.y { return false; }
    }
    true
}

/// Live, non-owning access to the native graph and grid. Mutating callbacks
/// return false only to stop and propagate a saved native exception.
pub trait GridHost {
    type Node: Copy;
    fn node_count(&self) -> usize;
    fn node(&self, index: usize) -> Self::Node;
    fn position(&self, node: Self::Node) -> Vector3f;
    fn nodes_per_grid(&self) -> i32;
    fn set_bounds(&mut self, minimum: Vector2f, maximum: Vector2f);
    fn set_map_size(&mut self, size: Vector2l);
    fn resize_grids(&mut self, count: usize) -> bool;
    fn dimensions(&self) -> GridDimensions;
    fn set_cell_size(&mut self, size: Vector2f);
    fn cell_size(&self) -> Vector2f;
    fn grid_count(&self) -> usize;
    /// Append, never clear: retained old lists are an observable original quirk.
    fn push_node(&mut self, index: usize, node: Self::Node) -> bool;
}

pub fn build_grid<H: GridHost>(host: &mut H) {
    // Deviation: the original reads mvNodes[0] for an empty container (UB).
    assert!(host.node_count() != 0, "AI grid build requires at least one node");
    let first = host.position(host.node(0));
    let mut minimum = Vector2f::new(first.x, first.z);
    let mut maximum = minimum;
    let mut i = 1;
    while i < host.node_count() {
        let position = host.position(host.node(i));
        // Faithful: min/max helpers change first-NaN and signed-zero behavior.
        if minimum.x > position.x { minimum.x = position.x; }
        if minimum.y > position.z { minimum.y = position.z; }
        if maximum.x < position.x { maximum.x = position.x; }
        if maximum.y < position.z { maximum.y = position.z; }
        i += 1;
    }
    host.set_bounds(minimum, maximum);
    let dimension = grid_dimension(host.node_count(), host.nodes_per_grid());
    host.set_map_size(Vector2l::splat(dimension));
    // Faithful: write bounds/dimensions before resize, cell size only after
    // success. Resizing native grids retains old lower-index lists.
    if !host.resize_grids(grid_storage_count(dimension)) { return; }
    host.set_cell_size(cell_size(host.dimensions()));

    let mut i = 0;
    while i < host.node_count() {
        let node = host.node(i);
        let dimensions = host.dimensions();
        let position = insertion_position(host.position(node), dimensions.minimum, host.cell_size());
        let index = grid_index(position, dimensions.map_size, host.grid_count());
        if !host.push_node(index, node) { return; }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_metrics_do_not_square_the_rounded_root() {
        let (distance, squared) = edge_distances(Vector3f::splat(0.0), Vector3f::new(1.0, 1.0, 0.0));
        assert_eq!(distance.to_bits(), 0x3fb504f3);
        assert_eq!(squared, 2.0);
        assert_ne!(squared.to_bits(), (distance * distance).to_bits());
        assert_eq!(edge_distances(Vector3f::splat(-0.0), Vector3f::splat(0.0)), (0.0, 0.0));
    }

    #[test]
    fn edge_nan_priority_matches_native_including_inf_self_edges() {
        let nan = f32::from_bits(0x7f812345);
        let first = Vector3f::new(f32::INFINITY, nan, f32::INFINITY);
        let (distance, squared) = edge_distances(first, first);
        assert_eq!((distance.to_bits(), squared.to_bits()), (0x7fc12345, 0x7fc12345));
        let second = Vector3f::new(f32::INFINITY, 0.0, nan);
        let (distance, squared) = edge_distances(second, second);
        assert_eq!((distance.to_bits(), squared.to_bits()), (0xffc00000, 0xffc00000));
        let (distance, squared) = edge_distances(Vector3f::new(nan, f32::from_bits(0xffc54321), nan), Vector3f::splat(0.0));
        assert_eq!((distance.to_bits(), squared.to_bits()), (0xffc54321, 0xffc54321));
    }

    #[test]
    fn grid_dimensions_round_then_add_a_border() {
        for (nodes, dimension) in [(1, 1), (2, 2), (13, 2), (14, 3), (37, 3), (38, 4)] {
            assert_eq!(grid_dimension(nodes, 6), dimension);
            assert_eq!(grid_storage_count(dimension), ((dimension + 1) * (dimension + 1)) as usize);
        }
    }

    #[test]
    fn cell_extent_divides_by_dimension_not_storage_side() {
        let dimensions = GridDimensions { minimum: Vector2f::new(-2.0, 3.0), maximum: Vector2f::new(10.0, 7.0),
                                          map_size: Vector2l::new(3, 2) };
        assert_eq!(cell_size(dimensions), Vector2f::new(4.0, 2.0));
    }

    #[test]
    fn build_uses_xz_and_guards_nonpositive_or_nan_extents() {
        let point = Vector3f::new(8.0, 999.0, 9.0);
        assert_eq!(insertion_position(point, Vector2f::new(2.0, 3.0), Vector2f::new(2.0, 3.0)), Vector2l::new(3, 2));
        for cell in [0.0, -0.0, -1.0, f32::NAN] {
            assert_eq!(insertion_position(point, Vector2f::splat(0.0), Vector2f::splat(cell)), Vector2l::splat(0));
        }
    }

    #[test]
    fn query_truncates_and_clamps_inclusive_upper_border() {
        let size = Vector2l::new(2, 3);
        assert_eq!(grid_position(Vector2f::new(-0.75, 8.9), Vector2f::splat(2.0), size), Vector2l::new(0, 3));
        assert_eq!(grid_position(Vector2f::new(4.0, 6.0), Vector2f::splat(2.0), size), size);
        assert_eq!(grid_position(Vector2f::splat(1.0), Vector2f::splat(1.0), Vector2l::new(-2, -3)), Vector2l::new(-2, -3));
    }

    #[test]
    fn undefined_query_casts_follow_x86_then_clamps() {
        for local in [0.0, 1.0, f32::NAN, f32::INFINITY, f32::MAX] {
            assert_eq!(grid_position(Vector2f::splat(local), Vector2f::splat(0.0), Vector2l::splat(3)), Vector2l::splat(0));
        }
    }

    #[test]
    fn flat_index_retains_cross_axis_aliases() {
        assert_eq!(grid_index(Vector2l::new(-1, 1), Vector2l::splat(2), 9), 2);
        assert_eq!(grid_index(Vector2l::new(3, 0), Vector2l::splat(2), 9), 3);
        assert_eq!(grid_index(Vector2l::new(2, 2), Vector2l::splat(2), 9), 8);
    }

    #[test]
    fn increment_is_x_first_and_exhaustion_keeps_the_advanced_position() {
        let start = Vector2l::new(2, 3);
        let end = Vector2l::new(3, 4);
        let mut pos = start;
        for expected in [Vector2l::new(3, 3), Vector2l::new(2, 4), Vector2l::new(3, 4)] {
            assert!(increment_grid(&mut pos, start, end));
            assert_eq!(pos, expected);
        }
        assert!(!increment_grid(&mut pos, start, end));
        assert_eq!(pos, Vector2l::new(2, 5));
    }

    #[test]
    fn inverted_iterator_bounds_are_not_normalized() {
        let mut pos = Vector2l::new(4, 7);
        assert!(!increment_grid(&mut pos, Vector2l::new(4, 7), Vector2l::new(2, 5)));
        assert_eq!(pos, Vector2l::new(4, 8));
        let mut pos = Vector2l::new(0, 7);
        assert!(increment_grid(&mut pos, Vector2l::new(0, 7), Vector2l::new(2, 5)));
        assert_eq!(pos, Vector2l::new(1, 7));
    }

    #[derive(Default)]
    struct Host {
        positions: Vec<Vector3f>,
        dimensions: GridDimensions,
        cell: Vector2f,
        grids: Vec<Vec<usize>>,
        events: Vec<&'static str>,
        fail_resize: bool,
        fail_push: Option<usize>,
        pushes: usize,
    }

    impl GridHost for Host {
        type Node = usize;
        fn node_count(&self) -> usize { self.positions.len() }
        fn node(&self, index: usize) -> usize { index }
        fn position(&self, node: usize) -> Vector3f { self.positions[node] }
        fn nodes_per_grid(&self) -> i32 { 6 }
        fn set_bounds(&mut self, minimum: Vector2f, maximum: Vector2f) {
            self.events.push("bounds"); self.dimensions.minimum = minimum; self.dimensions.maximum = maximum;
        }
        fn set_map_size(&mut self, size: Vector2l) { self.events.push("map size"); self.dimensions.map_size = size; }
        fn resize_grids(&mut self, count: usize) -> bool {
            self.events.push("resize");
            if self.fail_resize { return false; }
            self.grids.resize_with(count, Vec::new);
            true
        }
        fn dimensions(&self) -> GridDimensions { self.dimensions }
        fn set_cell_size(&mut self, size: Vector2f) { self.events.push("cell size"); self.cell = size; }
        fn cell_size(&self) -> Vector2f { self.cell }
        fn grid_count(&self) -> usize { self.grids.len() }
        fn push_node(&mut self, index: usize, node: usize) -> bool {
            self.events.push("push");
            if self.fail_push == Some(self.pushes) { return false; }
            self.pushes += 1;
            self.grids[index].push(node);
            true
        }
    }

    fn host() -> Host {
        Host { positions: vec![Vector3f::new(0.0, 99.0, 0.0), Vector3f::new(4.0, -99.0, 6.0)], ..Host::default() }
    }

    #[test]
    fn build_retains_lists_and_appends_duplicate_nodes() {
        let mut h = host();
        build_grid(&mut h);
        assert_eq!(h.dimensions.map_size, Vector2l::splat(2));
        assert_eq!(h.cell, Vector2f::new(2.0, 3.0));
        assert_eq!(h.grids[0], [0]);
        assert_eq!(h.grids[8], [1]);
        build_grid(&mut h);
        assert_eq!(h.grids[0], [0, 0]);
        assert_eq!(h.grids[8], [1, 1]);
        assert_eq!(&h.events[..4], ["bounds", "map size", "resize", "cell size"]);
    }

    #[test]
    fn resize_failure_keeps_new_bounds_and_dimensions_but_old_cells() {
        let mut h = host();
        h.cell = Vector2f::new(91.0, 92.0);
        h.grids = vec![vec![123]];
        h.fail_resize = true;
        build_grid(&mut h);
        assert_eq!(h.dimensions.maximum, Vector2f::new(4.0, 6.0));
        assert_eq!(h.dimensions.map_size, Vector2l::splat(2));
        assert_eq!(h.cell, Vector2f::new(91.0, 92.0));
        assert_eq!(h.grids, [vec![123]]);
        assert_eq!(h.events, ["bounds", "map size", "resize"]);
    }

    #[test]
    fn append_failure_stops_with_completed_prefix() {
        let mut h = host();
        h.fail_push = Some(1);
        build_grid(&mut h);
        assert_eq!(h.grids[0], [0]);
        assert!(h.grids[8].is_empty());
        assert_eq!(h.events, ["bounds", "map size", "resize", "cell size", "push", "push"]);
    }

    #[test]
    fn strict_bounds_keep_first_nan_and_signed_zero() {
        let nan = f32::from_bits(0xffc12345);
        let mut h = Host { positions: vec![Vector3f::new(nan, 0.0, -0.0), Vector3f::new(8.0, 0.0, 0.0)], ..Host::default() };
        build_grid(&mut h);
        assert_eq!(h.dimensions.minimum.x.to_bits(), nan.to_bits());
        assert_eq!(h.dimensions.maximum.x.to_bits(), nan.to_bits());
        assert_eq!(h.dimensions.minimum.y.to_bits(), 0x80000000);
        assert_eq!(h.dimensions.maximum.y.to_bits(), 0x80000000);
        assert_eq!(h.grids[0], [0, 1]);
    }

    #[test]
    #[should_panic(expected = "requires at least one node")]
    fn empty_grid_build_is_rejected() { build_grid(&mut Host::default()); }

    #[test]
    #[should_panic(expected = "allocation count overflow")]
    fn overflowing_storage_count_is_rejected() { grid_storage_count(46340); }

    #[test]
    #[should_panic(expected = "outside storage")]
    fn negative_flat_index_is_rejected() { grid_index(Vector2l::new(-1, 0), Vector2l::splat(2), 9); }

    #[test]
    #[should_panic(expected = "index overflow")]
    fn overflowing_index_is_rejected() { grid_index(Vector2l::new(0, i32::MAX), Vector2l::splat(2), usize::MAX); }

    #[test]
    #[should_panic(expected = "iterator x overflow")]
    fn overflowing_iterator_is_rejected() {
        increment_grid(&mut Vector2l::new(i32::MAX, 0), Vector2l::splat(0), Vector2l::splat(i32::MAX));
    }
}
