//! OS-independent algorithms from `system/Platform.cpp`.

use hpl_math::Vector2l;

/// `VideoComp::operator()`: compare display, width, then height.
pub fn video_mode_less(display_a: i32, size_a: Vector2l, display_b: i32, size_b: Vector2l) -> bool {
    // Faithful: bits-per-pixel, refresh rate and the custom flag do not
    // participate in this ordering, even when all three keys are equal.
    if display_a != display_b { return display_a < display_b; }
    if size_a.x != size_b.x { return size_a.x < size_b.x; }
    if size_a.y != size_b.y { return size_a.y < size_b.y; }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_have_display_width_height_priority() {
        assert!(video_mode_less(-1, Vector2l::new(999, 999), 0, Vector2l::new(1, 1)));
        assert!(video_mode_less(0, Vector2l::new(1, 999), 0, Vector2l::new(2, 1)));
        assert!(video_mode_less(0, Vector2l::new(1, 1), 0, Vector2l::new(1, 2)));
        assert!(!video_mode_less(0, Vector2l::new(1, 2), 0, Vector2l::new(1, 1)));
    }

    #[test]
    fn equal_and_extreme_keys_do_not_subtract_or_overflow() {
        let low = Vector2l::new(i32::MIN, i32::MAX);
        let high = Vector2l::new(i32::MAX, i32::MIN);
        assert!(!video_mode_less(0, low, 0, low));
        assert!(video_mode_less(0, low, 0, high));
        assert!(video_mode_less(i32::MIN, high, i32::MAX, low));
    }
}
