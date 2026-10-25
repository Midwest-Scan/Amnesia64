//! Voxel storage arithmetic and debug sphere traversal.

use hpl_math::{Vector3f, Vector3l};

/// Byte count for `SetSize`. C++ still performs allocation and zeroing.
pub fn allocation_size(size: Vector3l) -> usize {
    // Faithful: signed, left-to-right products precede conversion to size_t.
    // Negative dimensions can give a positive count. A negative total must
    // still reach C++ new[] as a huge request (and throw there, not through Rust).
    // Deviation: signed intermediate overflow is C++ UB; abort with the Rust
    // panic hook rather than allocating an incorrectly sized buffer.
    let count = size.x.checked_mul(size.y).and_then(|xy| xy.checked_mul(size.z))
        .expect("voxel allocation size overflows a C++ int");
    count as usize
}

/// Original flat index, with defined failure for overflow/out-of-buffer access.
pub fn linear_index(size: Vector3l, pos: Vector3l) -> usize {
    // Faithful: no per-axis bounds check! (-1, 1, 0), for example, can alias
    // the last byte of the preceding row. Preserve the exact signed expression.
    // Deviation: overflowing intermediates or an index outside the allocation
    // are C++ UB. Panic instead of corrupting or reading unrelated memory.
    let z_term = pos.z.checked_mul(size.x).and_then(|zx| zx.checked_mul(size.y));
    let y_term = pos.y.checked_mul(size.x);
    let index = z_term.zip(y_term).and_then(|(z, y)| z.checked_add(y)).and_then(|zy| zy.checked_add(pos.x))
        .expect("voxel index overflows a C++ int");
    let index = usize::try_from(index).expect("negative voxel index");
    assert!(index < allocation_size(size), "voxel index outside allocation");
    index
}

/// `SetVoxel`: store the byte pattern (C++ `char` converted to unsigned char).
pub fn set_voxel(data: &mut [u8], size: Vector3l, pos: Vector3l, value: u8) {
    data[linear_index(size, pos)] = value;
}

/// Access to the current C++ map and renderer, without retaining its buffer.
pub trait RenderHost {
    fn size(&self) -> Vector3l;
    fn byte(&self, index: usize) -> u8;
    fn position(&self) -> Vector3f;
    fn voxel_size(&self) -> f32;
    /// The host reads the current referenced color and calls DrawSphere.
    /// False stops traversal immediately (the bridge then rethrows a captured
    /// C++ exception, after the Rust frames have returned).
    fn draw_sphere(&mut self, center: Vector3f, radius: f32) -> bool;
}

fn add_position(position: f32, offset: f32) -> f32 {
    // Faithful: MSVC's original vector expression keeps the offset's NaN
    // when both operands are NaN (including the negative NaN from 0*infinity).
    // Rust debug/release may otherwise choose different operands. Preserve
    // that payload/sign explicitly and quiet a signaling NaN as SSE addition does.
    if offset.is_nan() { f32::from_bits(offset.to_bits() | 0x0040_0000) } else { position + offset }
}

/// `DebugRender`: visit z/y/x and draw only bytes equal to one.
pub fn debug_render(host: &mut impl RenderHost) {
    // Faithful: DrawSphere is virtual and may resize the map, change later
    // voxels, position, voxel size or the referenced color. Recheck loop bounds
    // and read current data after every callback; do not use captured ranges.
    let mut z = 0;
    while z < host.size().z {
        let mut y = 0;
        while y < host.size().y {
            let mut x = 0;
            while x < host.size().x {
                let index = linear_index(host.size(), Vector3l::new(x, y, z));
                if host.byte(index) == 1 {
                    let position = host.position();
                    let voxel_size = host.voxel_size();
                    let half = voxel_size / 2.0_f32;
                    // Faithful: each operation is f32, with the two additions
                    // kept in their original order; no (coords + 0.5) rewrite.
                    let offset = Vector3f::new(x as f32, y as f32, z as f32) * voxel_size;
                    let center = Vector3f::new(add_position(position.x, offset.x), add_position(position.y, offset.y),
                        add_position(position.z, offset.z)) + Vector3f::splat(half);
                    if !host.draw_sphere(center, half) { return; }
                }
                x += 1;
            }
            y += 1;
        }
        z += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Host {
        size: Vector3l,
        data: Vec<u8>,
        position: Vector3f,
        voxel_size: f32,
        spheres: Vec<(Vector3f, f32)>,
        change: u8,
    }

    impl Host {
        fn new(size: Vector3l) -> Self {
            Self { size, data: vec![0; allocation_size(size)], position: Vector3f::splat(0.0),
                voxel_size: 0.1, spheres: Vec::new(), change: 0 }
        }
    }

    impl RenderHost for Host {
        fn size(&self) -> Vector3l { self.size }
        fn byte(&self, index: usize) -> u8 { self.data[index] }
        fn position(&self) -> Vector3f { self.position }
        fn voxel_size(&self) -> f32 { self.voxel_size }
        fn draw_sphere(&mut self, center: Vector3f, radius: f32) -> bool {
            self.spheres.push((center, radius));
            if self.spheres.len() != 1 { return true; }
            match self.change {
                1 => { self.data[1] = 1; self.position = Vector3f::splat(10.0); self.voxel_size = 2.0; }
                2 => { self.size = Vector3l::new(3, 2, 2); self.data = vec![1; 12]; }
                3 => { self.size = Vector3l::new(0, 0, 0); self.data.clear(); }
                4 => return false,
                _ => {}
            }
            true
        }
    }

    #[test]
    fn count_preserves_signed_products_and_size_t_conversion() {
        assert_eq!(allocation_size(Vector3l::new(3, 4, 5)), 60);
        assert_eq!(allocation_size(Vector3l::new(-3, -4, 5)), 60);
        assert_eq!(allocation_size(Vector3l::new(0, i32::MIN, i32::MIN)), 0);
        assert_eq!(allocation_size(Vector3l::new(-1, 1, 1)), usize::MAX);
    }

    #[test]
    #[should_panic(expected = "allocation size overflows")]
    fn intermediate_count_overflow_is_not_hidden_by_zero_depth() {
        allocation_size(Vector3l::new(i32::MAX, 2, 0));
    }

    #[test]
    fn flat_aliases_and_all_byte_patterns_are_preserved() {
        let size = Vector3l::new(3, 2, 2);
        assert_eq!(linear_index(size, Vector3l::new(2, 1, 1)), 11);
        assert_eq!(linear_index(size, Vector3l::new(-1, 1, 0)), 2);
        assert_eq!(linear_index(size, Vector3l::new(3, 0, 0)), 3);
        assert_eq!(linear_index(size, Vector3l::new(0, -1, 1)), 3);
        let mut data = [0; 12];
        for byte in 0..=255 {
            set_voxel(&mut data, size, Vector3l::new(-1, 1, 0), byte);
            assert_eq!(data[2], byte);
        }
    }

    #[test]
    #[should_panic(expected = "voxel index overflows")]
    fn index_intermediate_overflow_is_rejected() {
        linear_index(Vector3l::new(2, 1, 1), Vector3l::new(0, 0, i32::MAX));
    }

    #[test]
    #[should_panic(expected = "outside allocation")]
    fn one_past_buffer_is_rejected() {
        set_voxel(&mut [0; 12], Vector3l::new(3, 2, 2), Vector3l::new(12, 0, 0), 1);
    }

    #[test]
    #[should_panic(expected = "negative voxel index")]
    fn negative_flat_index_is_rejected() {
        linear_index(Vector3l::new(2, 1, 1), Vector3l::new(-1, 0, 0));
    }

    #[test]
    fn spheres_use_x_fastest_order_and_exactly_one() {
        let mut host = Host::new(Vector3l::new(2, 2, 2));
        host.voxel_size = 2.0;
        host.data = vec![1, 2, 1, 255, 0, 1, 1, 1];
        debug_render(&mut host);
        assert_eq!(host.spheres, vec![
            (Vector3f::new(1.0, 1.0, 1.0), 1.0), (Vector3f::new(1.0, 3.0, 1.0), 1.0),
            (Vector3f::new(3.0, 1.0, 3.0), 1.0), (Vector3f::new(1.0, 3.0, 3.0), 1.0),
            (Vector3f::new(3.0, 3.0, 3.0), 1.0),
        ]);
    }

    #[test]
    fn callbacks_can_change_later_bytes_and_render_properties() {
        let mut host = Host::new(Vector3l::new(2, 1, 1));
        host.data[0] = 1;
        host.change = 1;
        debug_render(&mut host);
        assert_eq!(host.spheres.len(), 2);
        assert_eq!(host.spheres[1], (Vector3f::new(13.0, 11.0, 11.0), 1.0));
    }

    #[test]
    fn callbacks_can_grow_and_shrink_loop_bounds_and_replace_buffer() {
        let mut grow = Host::new(Vector3l::new(1, 1, 1));
        grow.data[0] = 1;
        grow.change = 2;
        debug_render(&mut grow);
        assert_eq!(grow.spheres.len(), 12);
        let mut shrink = Host::new(Vector3l::new(3, 2, 2));
        shrink.data.fill(1);
        shrink.change = 3;
        debug_render(&mut shrink);
        assert_eq!(shrink.spheres.len(), 1);
    }

    #[test]
    fn negative_dimensions_and_zero_volume_draw_nothing() {
        for size in [Vector3l::new(-2, -3, 4), Vector3l::new(0, 2, 3), Vector3l::new(2, 0, 3)] {
            let mut host = Host::new(size);
            host.data.fill(1);
            debug_render(&mut host);
            assert!(host.spheres.is_empty());
        }
    }

    #[test]
    fn failed_draw_stops_traversal_immediately() {
        let mut host = Host::new(Vector3l::new(2, 2, 2));
        host.data.fill(1);
        host.change = 4;
        debug_render(&mut host);
        assert_eq!(host.spheres.len(), 1);
    }

    #[test]
    fn special_voxel_sizes_are_not_clamped() {
        for voxel_size in [-2.0, 0.0, -0.0, f32::from_bits(1), f32::INFINITY, f32::NAN] {
            let mut host = Host::new(Vector3l::new(1, 1, 1));
            host.data[0] = 1;
            host.voxel_size = voxel_size;
            debug_render(&mut host);
            let radius = host.spheres[0].1;
            if voxel_size.is_nan() { assert!(radius.is_nan()); }
            else { assert_eq!(radius.to_bits(), (voxel_size / 2.0).to_bits()); }
        }
    }

    #[test]
    fn offset_nan_wins_over_position_nan_in_both_build_profiles() {
        assert_eq!(add_position(f32::from_bits(0x7fc0_1234), f32::from_bits(0xffc0_5678)).to_bits(), 0xffc0_5678);
        assert_eq!(add_position(f32::from_bits(0x7fc0_1234), f32::from_bits(0xff80_5678)).to_bits(), 0xffc0_5678);
        let mut host = Host::new(Vector3l::new(1, 1, 1));
        host.data[0] = 1;
        host.position = Vector3f::splat(f32::from_bits(0x7fc0_1234));
        host.voxel_size = f32::INFINITY;
        debug_render(&mut host);
        for component in host.spheres[0].0.to_array() { assert_eq!(component.to_bits(), 0xffc0_0000); }
    }
}
