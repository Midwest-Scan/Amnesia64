//! BIT OPERATIONS and COLOR CONVERSION

use crate::color::Color;
use crate::util::{f32_to_i32, float_color_to_uchar, uchar_color_to_float};
use crate::vector::Vector3f;

/// `GetFlagBit`: `1 << bit`.
#[inline]
pub fn get_flag_bit(bit_num: i32) -> i32 {
    // Faithful: shift count masked like x86 `shl` for out-of-range input.
    1i32.wrapping_shl(bit_num as u32)
}

/// `RGBToHSB`. Output is (hue in degrees, saturation, brightness);
/// hue is -1 for greys.
pub fn rgb_to_hsb(rgb: Color) -> Vector3f {
    let mut hsb = Vector3f::default();
    rgb_to_hsb_into(rgb, &mut hsb);
    hsb
}

/// `RGBToHSBHelper`. Faithful: for greys only `y`/`z` are set and `x` = -1.
pub fn rgb_to_hsb_into(rgb: Color, out: &mut Vector3f) {
    let (r, g, b) = (rgb.r, rgb.g, rgb.b);
    let min = super::min_f(r, super::min_f(g, b));
    let max = super::max_f(r, super::max_f(g, b));
    let delta = max - min;

    out.z = max;
    out.y = if max == 0.0 { 0.0 } else { delta / max };

    if out.y == 0.0 {
        out.x = -1.0;
        return;
    }

    out.x = if r == max {
        (g - b) / delta // between yellow & magenta
    } else if g == max {
        2.0 + (b - r) / delta // between cyan & yellow
    } else {
        4.0 + (r - g) / delta // between magenta & cyan
    };
    out.x *= 60.0;
    if out.x < 0.0 {
        out.x += 360.0;
    }
}

/// `HSBToRGB` (H in [0, 360], S and B in [0, 1]). Alpha is 1.
pub fn hsb_to_rgb(hsb: Vector3f) -> Color {
    let mut c = Color::default();
    hsb_to_rgb_into(hsb, &mut c);
    c
}

/// `HSBToRGBHelper`. Faithful: alpha is never written, and a hue outside
/// [0, 360] leaves r/g/b untouched.
pub fn hsb_to_rgb_into(hsb: Vector3f, out: &mut Color) {
    let (mut h, s, b) = (hsb.x, hsb.y, hsb.z);
    if s == 0.0 {
        out.r = b;
        out.g = b;
        out.b = b;
        return;
    }

    if h == 360.0 {
        h = 0.0;
    } else {
        h /= 60.0;
    }

    let sextant = f32_to_i32(h.floor());
    let fract = h - sextant as f32;

    let p = b * (1.0 - s);
    let q = b * (1.0 - s * fract);
    let t = b * (1.0 - s * (1.0 - fract));

    let (r, g, bl) = match sextant {
        0 => (b, t, p),
        1 => (q, b, p),
        2 => (p, b, t),
        3 => (p, q, b),
        4 => (t, p, b),
        5 => (b, p, q),
        _ => return,
    };
    out.r = r;
    out.g = g;
    out.b = bl;
}

/// `cString::Sub` for the fixed (start, 2) slices the hex parsers take.
fn sub<T>(s: &[T], start: usize, count: usize) -> &[T] {
    if start >= s.len() {
        return &[];
    }
    let end = (start + count).min(s.len());
    &s[start..end]
}

/// `unsigned long` is 32 bits on Windows and 64 bits on 64-bit Unix.
#[cfg(windows)]
const ULONG_MAX: u64 = u32::MAX as u64;
#[cfg(not(windows))]
const ULONG_MAX: u64 = u64::MAX;

/// `strtoul(str, NULL, 16)` / `wcstoul(str, NULL, 16)` in the "C" locale.
/// Parsing stops at the first non-digit (including NUL); overflow returns
/// `ULONG_MAX`; a leading '-' negates in unsigned arithmetic.
// On 64-bit Unix ULONG_MAX is u64::MAX, so the range check is a no-op there.
#[allow(clippy::absurd_extreme_comparisons)]
fn strtoul_hex<T: Copy + Into<u32>>(s: &[T]) -> u64 {
    let ch = |i: usize| -> Option<u32> { s.get(i).map(|&c| c.into()) };
    let hex = |c: u32| -> Option<u32> { char::from_u32(c).and_then(|c| c.to_digit(16)) };
    let is_space = |c: u32| matches!(c, 0x20 | 0x09..=0x0d);

    let mut i = 0;
    while ch(i).is_some_and(is_space) {
        i += 1;
    }
    let mut negative = false;
    if let Some(c) = ch(i) {
        if c == '+' as u32 || c == '-' as u32 {
            negative = c == '-' as u32;
            i += 1;
        }
    }
    // "0x" prefix, only if a hex digit follows it.
    if ch(i) == Some('0' as u32)
        && matches!(ch(i + 1), Some(c) if c == 'x' as u32 || c == 'X' as u32)
        && ch(i + 2).and_then(hex).is_some()
    {
        i += 2;
    }
    let mut value: u64 = 0;
    let mut overflow = false;
    while let Some(d) = ch(i).and_then(hex) {
        match value.checked_mul(16).and_then(|v| v.checked_add(d as u64)).filter(|v| *v <= ULONG_MAX) {
            Some(v) => value = v,
            None => overflow = true,
        }
        i += 1;
    }
    if overflow {
        ULONG_MAX
    } else if negative {
        value.wrapping_neg() & ULONG_MAX
    } else {
        value
    }
}

/// `HexStringToUChar` on raw characters (narrow bytes, or wide characters
/// widened to `u32`): `strtoul(s, 16)` truncated to a byte.
///
/// Deviation for wide strings: only ASCII whitespace is skipped, where
/// `wcstoul` also skips other Unicode spaces.
pub fn hex_chars_to_uchar<T: Copy + Into<u32>>(s: &[T]) -> u8 {
    strtoul_hex(s) as u8
}

/// `HexStringToUChar`: the whole string is parsed, as with `strtoul`.
pub fn hex_string_to_uchar(s: &str) -> u8 {
    hex_chars_to_uchar(s.as_bytes())
}

fn hex_to_rgb_impl<T: Copy + Into<u32>>(hex: &[T], out: &mut Color, with_alpha: bool) {
    out.r = uchar_color_to_float(hex_chars_to_uchar(sub(hex, 0, 2)));
    out.g = uchar_color_to_float(hex_chars_to_uchar(sub(hex, 2, 2)));
    out.b = uchar_color_to_float(hex_chars_to_uchar(sub(hex, 4, 2)));
    if with_alpha {
        out.a = uchar_color_to_float(hex_chars_to_uchar(sub(hex, 6, 2)));
    }
}

/// `HexToRGB` ("RRGGBB"). Alpha is 1.
pub fn hex_to_rgb(hex: &str) -> Color {
    let mut c = Color::default();
    hex_to_rgb_impl(hex.as_bytes(), &mut c, false);
    c
}

/// `HexToRGBA` ("RRGGBBAA").
pub fn hex_to_rgba(hex: &str) -> Color {
    let mut c = Color::default();
    hex_to_rgb_impl(hex.as_bytes(), &mut c, true);
    c
}

/// `HexToRGBHelper` / `HexToRGBAHelper` on raw narrow characters. Only the
/// colour channels parsed are written (alpha only if `with_alpha`).
pub fn hex_to_color_bytes(hex: &[u8], out: &mut Color, with_alpha: bool) {
    hex_to_rgb_impl(hex, out, with_alpha);
}

/// `HexWToRGBHelper` / `HexWToRGBAHelper` on wide characters (UTF-16 or
/// UTF-32 code units widened to `u32`). See [`hex_chars_to_uchar`].
pub fn hex_to_color_wide(hex: &[u32], out: &mut Color, with_alpha: bool) {
    hex_to_rgb_impl(hex, out, with_alpha);
}

/// `UCharToHexString`: two upper-case hex digits.
pub fn uchar_to_hex_string(value: u8) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut s = String::with_capacity(2);
    s.push(DIGITS[(value >> 4) as usize] as char);
    s.push(DIGITS[(value & 0xF) as usize] as char);
    s
}

/// `RGBToHex` ("RRGGBB").
pub fn rgb_to_hex(c: Color) -> String {
    let mut s = uchar_to_hex_string(float_color_to_uchar(c.r));
    s += &uchar_to_hex_string(float_color_to_uchar(c.g));
    s += &uchar_to_hex_string(float_color_to_uchar(c.b));
    s
}

/// `RGBAToHex` ("RRGGBBAA").
pub fn rgba_to_hex(c: Color) -> String {
    let mut s = rgb_to_hex(c);
    s += &uchar_to_hex_string(float_color_to_uchar(c.a));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = hex_to_rgba("FF8000C0");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 128.0 / 255.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 192.0 / 255.0);
        assert_eq!(rgba_to_hex(c), "FF8000C0");
        assert_eq!(rgb_to_hex(c), "FF8000");
    }

    #[test]
    fn strtoul_edge_cases() {
        assert_eq!(hex_string_to_uchar("0x"), 0);
        assert_eq!(hex_string_to_uchar("-1"), 0xFF);
        assert_eq!(hex_string_to_uchar(" f"), 0x0F);
        assert_eq!(hex_string_to_uchar("zz"), 0);
        assert_eq!(hex_string_to_uchar("a"), 0x0A);
        // Whole string is parsed: 0x1234 truncated to a byte.
        assert_eq!(hex_string_to_uchar("1234"), 0x34);
        // Overflow saturates to ULONG_MAX.
        assert_eq!(hex_string_to_uchar("123456789ABCDEF0123"), 0xFF);
        // Short strings: missing channels parse as 0.
        let c = hex_to_rgb("FF");
        assert_eq!((c.r, c.g, c.b, c.a), (1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn hsb_roundtrip() {
        let hsb = rgb_to_hsb(Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(hsb, Vector3f::new(0.0, 1.0, 1.0));
        assert_eq!(hsb_to_rgb(Vector3f::new(120.0, 1.0, 1.0)), Color::rgb(0.0, 1.0, 0.0));
        assert_eq!(rgb_to_hsb(Color::rgb(0.5, 0.5, 0.5)).x, -1.0);
    }
}
