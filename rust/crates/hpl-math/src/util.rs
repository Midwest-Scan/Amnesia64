//! Numeric conversions that reproduce what the C++ engine does on x86/x64.
//!
//! C++ float-to-int casts compile to `cvttss2si`/`cvttsd2si`, which return
//! `i32::MIN` for NaN and out-of-range values. Rust's `as` saturates instead,
//! so code ported from a C-style cast uses these helpers.

/// C++ `(int)afValue` for a `float`.
#[inline]
pub fn f32_to_i32(value: f32) -> i32 {
    if value >= -2_147_483_648.0 && value < 2_147_483_648.0 { value as i32 } else { i32::MIN }
}

/// C++ `(int)adValue` for a `double`.
#[inline]
pub fn f64_to_i32(value: f64) -> i32 {
    if value >= -2_147_483_648.0 && value < 2_147_483_648.0 { value as i32 } else { i32::MIN }
}

/// C++ `(unsigned char)afValue`: truncate to a 32-bit int, keep the low byte.
#[inline]
pub fn f32_to_u8(value: f32) -> u8 {
    f32_to_i32(value) as u8
}

/// `FloatColorToUChar` from `graphics/GraphicsTypes.h`.
#[inline]
pub fn float_color_to_uchar(color: f32) -> u8 {
    f32_to_u8(color * 255.0 + 0.5)
}

/// `UCharColorToFloat` from `graphics/GraphicsTypes.h`.
#[inline]
pub fn uchar_color_to_float(color: u8) -> f32 {
    color as f32 / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casts_match_x86_truncation() {
        assert_eq!(f32_to_i32(1.9), 1);
        assert_eq!(f32_to_i32(-1.9), -1);
        assert_eq!(f32_to_i32(f32::NAN), i32::MIN);
        assert_eq!(f32_to_i32(3.0e9), i32::MIN);
        assert_eq!(f32_to_i32(-3.0e9), i32::MIN);
        assert_eq!(f64_to_i32(2_147_483_647.0), i32::MAX);
        assert_eq!(f32_to_u8(383.0), 127);
        assert_eq!(float_color_to_uchar(1.0), 255);
        assert_eq!(float_color_to_uchar(0.5), 128);
    }
}
