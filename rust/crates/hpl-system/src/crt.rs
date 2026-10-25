//! The parts of the C runtime the engine's string code depends on, with the
//! behaviour of the MSVC runtime (UCRT) in the default "C" locale. Where the
//! GNU C library differs (overflow and NaN spelling), the other platforms
//! follow glibc so the differential test against the original C++ can run
//! on Linux.

/// `isspace` in the "C" locale.
#[inline]
pub fn is_space(c: u32) -> bool {
    matches!(c, 0x20 | 0x09..=0x0d)
}

/// `atoi(s)`. Parsing stops at the first non-digit (or NUL).
///
/// Overflow follows the platform's C library: the UCRT saturates to
/// `INT_MIN`/`INT_MAX`; glibc computes `(int)strtol(s)`, i.e. saturates to
/// a 64-bit `long` and truncates.
pub fn atoi(s: &[u8]) -> i32 {
    if cfg!(windows) { atoi_ucrt(s) } else { atoi_glibc(s) }
}

/// Sign and digits of an `atoi` argument as an exact (possibly huge) value,
/// capped at 2^64 so it cannot overflow.
fn atoi_value(s: &[u8]) -> i128 {
    let mut i = 0;
    while i < s.len() && is_space(s[i] as u32) {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let mut value: i128 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        value = (value * 10 + (s[i] - b'0') as i128).min(1 << 64);
        i += 1;
    }
    if negative { -value } else { value }
}

/// `atoi` of the Windows UCRT: saturates to the `int` range.
pub fn atoi_ucrt(s: &[u8]) -> i32 {
    atoi_value(s).clamp(i32::MIN as i128, i32::MAX as i128) as i32
}

/// `atoi` of glibc: `(int)strtol(s)` with a 64-bit `long`.
pub fn atoi_glibc(s: &[u8]) -> i32 {
    atoi_value(s).clamp(i64::MIN as i128, i64::MAX as i128) as i64 as i32
}

/// `atof(s)`: `strtod` without an end pointer. Returns 0 when nothing parses.
pub fn atof(s: &[u8]) -> f64 {
    strtod(s).0
}

/// `strtod(s, &end)`: returns the value and the number of bytes consumed
/// (0 if no conversion was performed).
pub fn strtod(s: &[u8]) -> (f64, usize) {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut i = 0;
    while i < s.len() && is_space(s[i] as u32) {
        i += 1;
    }
    let mut negative = false;
    if at(i) == b'+' || at(i) == b'-' {
        negative = at(i) == b'-';
        i += 1;
    }
    let sign = |v: f64| if negative { -v } else { v };

    // inf / infinity
    if starts_with_ci(&s[i.min(s.len())..], b"inf") {
        let len = if starts_with_ci(&s[i..], b"infinity") { 8 } else { 3 };
        return (sign(f64::INFINITY), i + len);
    }
    // nan / nan(n-char-sequence)
    if starts_with_ci(&s[i.min(s.len())..], b"nan") {
        let mut end = i + 3;
        if at(end) == b'(' {
            let mut j = end + 1;
            while at(j).is_ascii_alphanumeric() || at(j) == b'_' {
                j += 1;
            }
            if at(j) == b')' {
                end = j + 1;
            }
        }
        return (sign(f64::NAN), end);
    }
    // hexadecimal
    if at(i) == b'0' && (at(i + 1) == b'x' || at(i + 1) == b'X') {
        let digits_start = i + 2;
        let has_digit = at(digits_start).is_ascii_hexdigit() || (at(digits_start) == b'.' && at(digits_start + 1).is_ascii_hexdigit());
        if has_digit {
            let (v, end) = parse_hex(s, digits_start);
            return (sign(v), end);
        }
        // "0x" without digits: only the "0" is converted.
        return (sign(0.0), i + 1);
    }
    // decimal
    let start = i;
    let mut digits = 0;
    while at(i).is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if at(i) == b'.' {
        i += 1;
        while at(i).is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return (0.0, 0);
    }
    if at(i) == b'e' || at(i) == b'E' {
        let mut j = i + 1;
        if at(j) == b'+' || at(j) == b'-' {
            j += 1;
        }
        if at(j).is_ascii_digit() {
            while at(j).is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    let text = std::str::from_utf8(&s[start..i]).unwrap_or("0");
    let v: f64 = text.parse().unwrap_or(0.0);
    (sign(v), i)
}

fn starts_with_ci(s: &[u8], pat: &[u8]) -> bool {
    s.len() >= pat.len() && s[..pat.len()].eq_ignore_ascii_case(pat)
}

/// Hexadecimal significand and optional binary exponent, correctly rounded.
fn parse_hex(s: &[u8], mut i: usize) -> (f64, usize) {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut mantissa: u64 = 0; // top 60 bits of significant digits
    let mut sticky = false; // any non-zero digit dropped
    let mut exp: i64 = 0; // binary exponent adjustment
    let mut seen_point = false;
    loop {
        let c = at(i);
        if c == b'.' && !seen_point {
            seen_point = true;
            i += 1;
            continue;
        }
        let Some(d) = (c as char).to_digit(16) else { break };
        if mantissa >> 56 == 0 {
            mantissa = (mantissa << 4) | d as u64;
            if seen_point {
                exp -= 4;
            }
        } else {
            sticky |= d != 0;
            if !seen_point {
                exp += 4;
            }
        }
        i += 1;
    }
    if at(i) == b'p' || at(i) == b'P' {
        let mut j = i + 1;
        let mut neg = false;
        if at(j) == b'+' || at(j) == b'-' {
            neg = at(j) == b'-';
            j += 1;
        }
        if at(j).is_ascii_digit() {
            let mut e: i64 = 0;
            while at(j).is_ascii_digit() {
                e = (e * 10 + (at(j) - b'0') as i64).min(1 << 40);
                j += 1;
            }
            exp += if neg { -e } else { e };
            i = j;
        }
    }
    (compose_binary(mantissa, sticky, exp), i)
}

/// `mantissa * 2^exp` (plus a sticky bit below the mantissa) rounded to the
/// nearest double, ties to even.
fn compose_binary(mantissa: u64, sticky: bool, exp: i64) -> f64 {
    if mantissa == 0 {
        return 0.0;
    }
    let lz = mantissa.leading_zeros() as i64;
    let m = mantissa << lz; // top bit at 63
    let e = exp - lz + 63; // value = 1.xxx * 2^e
    if e > 1023 {
        return f64::INFINITY;
    }
    // Bits of precision available: 53 for normals, fewer for subnormals.
    let keep: i64 = if e >= -1022 { 53 } else { 53 - (-1022 - e) };
    if keep <= 0 {
        if keep == 0 {
            // Value in [2^-1075, 2^-1074): rounds to min subnormal if above half.
            let above_half = (m << 1) != 0 || sticky;
            return if above_half { f64::from_bits(1) } else { 0.0 };
        }
        return 0.0;
    }
    let drop = 64 - keep;
    let mut q = m >> drop;
    let rem = m & ((1u64 << drop) - 1);
    let half = 1u64 << (drop - 1);
    let round_up = rem > half || (rem == half && (sticky || q & 1 == 1));
    if round_up {
        q += 1;
    }
    // Assemble: q has `keep` (or keep+1 after carry) significant bits.
    let value = (q as f64) * pow2(e - (keep - 1));
    if value.is_infinite() { f64::INFINITY } else { value }
}

fn pow2(e: i64) -> f64 {
    // Exact powers of two over the whole double range, including subnormals.
    if e >= -1022 {
        if e > 1023 { f64::INFINITY } else { f64::from_bits(((e + 1023) as u64) << 52) }
    } else if e >= -1074 {
        f64::from_bits(1u64 << (e + 1074))
    } else {
        0.0
    }
}

/// `printf("%0*d", width, value)` and friends: integer text with optional
/// zero padding; a negative width left-justifies with spaces.
pub fn format_int(value: i64, width: i32) -> String {
    let digits = value.unsigned_abs().to_string();
    let sign = if value < 0 { "-" } else { "" };
    pad(sign, &digits, width)
}

/// `printf("%0*u", width, value)`.
pub fn format_uint(value: u64, width: i32) -> String {
    pad("", &value.to_string(), width)
}

fn pad(sign: &str, digits: &str, width: i32) -> String {
    let len = sign.len() + digits.len();
    let w = (width as i64).unsigned_abs() as usize;
    if width < 0 {
        // '-' flag overrides '0': pad with spaces on the right.
        let mut s = format!("{sign}{digits}");
        while s.len() < w {
            s.push(' ');
        }
        s
    } else if len < w {
        format!("{sign}{}{digits}", "0".repeat(w - len))
    } else {
        format!("{sign}{digits}")
    }
}

/// `printf("%.*f", precision, value)`. A negative precision means the
/// default (6), as in C.
pub fn format_fixed(value: f64, precision: i32) -> String {
    let precision = if precision < 0 { 6 } else { precision as usize };
    if value.is_nan() {
        return nan_text(value);
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf".into() } else { "inf".into() };
    }
    format!("{value:.precision$}")
}

/// How the C runtime prints a NaN with `%f`.
fn nan_text(value: f64) -> String {
    if cfg!(windows) { nan_text_ucrt(value) } else { nan_text_glibc(value) }
}

/// UCRT: "-nan(ind)" for the x86 default NaN, "nan(snan)" for a signalling
/// NaN, "nan" / "-nan" otherwise.
fn nan_text_ucrt(value: f64) -> String {
    let bits = value.to_bits();
    let negative = bits >> 63 == 1;
    let quiet = bits & (1u64 << 51) != 0;
    let payload = bits & ((1u64 << 51) - 1);
    let sign = if negative { "-" } else { "" };
    if !quiet {
        format!("{sign}nan(snan)")
    } else if negative && payload == 0 {
        "-nan(ind)".into()
    } else {
        format!("{sign}nan")
    }
}

/// glibc: "nan" or "-nan".
fn nan_text_glibc(value: f64) -> String {
    if value.is_sign_negative() { "-nan".into() } else { "nan".into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atoi_cases() {
        assert_eq!(atoi(b"  -42abc"), -42);
        assert_eq!(atoi(b"+7"), 7);
        assert_eq!(atoi(b"x1"), 0);
        assert_eq!(atoi(b""), 0);
        assert_eq!(atoi(b"2147483647"), i32::MAX);
        // Overflow
        assert_eq!(atoi_ucrt(b"2147483648"), i32::MAX);
        assert_eq!(atoi_ucrt(b"-2147483649"), i32::MIN);
        assert_eq!(atoi_ucrt(b"99999999999999999999999"), i32::MAX);
        assert_eq!(atoi_glibc(b"2147483648"), i32::MIN); // (int)2147483648L
        assert_eq!(atoi_glibc(b"4294967297"), 1);
        assert_eq!(atoi_glibc(b"99999999999999999999999"), -1); // (int)LONG_MAX
        assert_eq!(atoi_glibc(b"-99999999999999999999999"), 0); // (int)LONG_MIN
    }

    #[test]
    fn nan_spelling() {
        let default_nan = f64::from_bits(0xFFF8_0000_0000_0000); // x86 "indefinite"
        assert_eq!(nan_text_ucrt(default_nan), "-nan(ind)");
        assert_eq!(nan_text_ucrt(f64::from_bits(0x7FF8_0000_0000_0000)), "nan");
        assert_eq!(nan_text_ucrt(f64::from_bits(0x7FF0_0000_0000_0001)), "nan(snan)");
        assert_eq!(nan_text_ucrt(f64::from_bits(0xFFF8_0000_0000_0001)), "-nan");
        assert_eq!(nan_text_glibc(default_nan), "-nan");
    }

    #[test]
    fn atof_cases() {
        assert_eq!(atof(b" 1.5f"), 1.5);
        assert_eq!(atof(b"-.25e1x"), -2.5);
        assert_eq!(atof(b"1e"), 1.0);
        assert_eq!(atof(b"1.e2"), 100.0);
        assert_eq!(atof(b"."), 0.0);
        assert_eq!(atof(b"0x1p4"), 16.0);
        assert_eq!(atof(b"0x.8"), 0.5);
        assert_eq!(atof(b"0x"), 0.0);
        assert_eq!(atof(b"0x1.fffffffffffff8p0"), 2.0); // ties to even
        assert_eq!(atof(b"0x1p-1074"), f64::from_bits(1));
        assert_eq!(atof(b"InFinity"), f64::INFINITY);
        assert!(atof(b"nan(123)").is_nan());
        assert_eq!(strtod(b"0x").1, 1);
    }

    #[test]
    fn format_cases() {
        assert_eq!(format_int(-5, 4), "-005");
        assert_eq!(format_int(5, -3), "5  ");
        assert_eq!(format_uint(7, 0), "7");
        assert_eq!(format_fixed(0.125, 2), "0.12");
        assert_eq!(format_fixed(-0.0, 1), "-0.0");
        assert_eq!(format_fixed(1.5, -1), "1.500000");
    }
}
