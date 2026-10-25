//! `cString` (`system/String.h`, `sources/system/String.cpp`).
//!
//! Functions that exist in narrow and wide (`...W`) versions in C++ are one
//! generic function here, over `u8` (narrow) or [`WChar`](crate::WChar)
//! (wide). Positions and counts are `i32` like the C++ `int`s.

use crate::chars::{Unit, lit};
use crate::crt;
use hpl_math::{Color, Matrixf, Vector2f, Vector2l, Vector3f, Vector3l};

// ---------------------------------------------------------------------------
// Encodings
// ---------------------------------------------------------------------------

/// `To16Char`: local encoding to wide. In the "C" locale the MSVC runtime
/// maps each byte to the code unit of the same value. Faithful: conversion
/// stops at the first NUL, like `mbstowcs`.
///
/// Deviation (non-Windows only): glibc fails on bytes >= 0x80 in the "C"
/// locale and the C++ then throws; here those bytes are widened as well.
pub fn to_16char<C: Unit>(s: &[u8]) -> Vec<C> {
    s.iter().take_while(|&&b| b != 0).map(|&b| C::from_u8(b)).collect()
}

/// `To8Char`: wide to local encoding ([`to_8char_windows`] or
/// [`to_8char_glibc`]).
pub fn to_8char<C: Unit>(s: &[C]) -> Vec<u8> {
    if cfg!(windows) { to_8char_windows(s) } else { to_8char_glibc(s) }
}

/// `To8Char` on Windows: units above 255 become '_', the rest are
/// truncated to a byte.
pub fn to_8char_windows<C: Unit>(s: &[C]) -> Vec<u8> {
    s.iter().map(|c| if c.to_u32() > 255 { b'_' } else { c.to_u32() as u8 }).collect()
}

/// `To8Char` elsewhere (glibc `wcstombs`, "C" locale): stops at NUL, and
/// any unit above 0x7F makes the whole conversion fail (empty result).
pub fn to_8char_glibc<C: Unit>(s: &[C]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s {
        let v = c.to_u32();
        if v == 0 {
            break;
        }
        if v > 0x7F {
            return Vec::new();
        }
        out.push(v as u8);
    }
    out
}

/// `UTF8ToWChar`. With a 16-bit `wchar_t` only sequences of up to 3 bytes
/// are decoded; longer leads become U+FFFD. Malformed input is not
/// validated (continuation bytes are masked, as in C++); bytes past the end
/// read as 0, like the C++ reading the string terminator.
pub fn utf8_to_wchar<C: Unit>(s: &[u8]) -> Vec<C> {
    let at = |i: usize| s.get(i).copied().unwrap_or(0) as u32;
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let c = at(i);
        let value;
        if c & 0x80 == 0 {
            value = c;
            i += 1;
        } else if c & 0xe0 == 0xc0 {
            value = (c & 0x1f) << 6 | (at(i + 1) & 0x3f);
            i += 2;
        } else if c & 0xf0 == 0xe0 {
            value = (c & 0x0f) << 12 | (at(i + 1) & 0x3f) << 6 | (at(i + 2) & 0x3f);
            i += 3;
        } else if C::BITS == 32 && c & 0xf8 == 0xf0 {
            value = (c & 0x07) << 18 | (at(i + 1) & 0x3f) << 12 | (at(i + 2) & 0x3f) << 6 | (at(i + 3) & 0x3f);
            i += 4;
        } else if C::BITS == 32 && c & 0xfc == 0xf8 {
            value = (c & 0x03) << 24 | (at(i + 1) & 0x3f) << 18 | (at(i + 2) & 0x3f) << 12 | (at(i + 3) & 0x3f) << 6 | (at(i + 4) & 0x3f);
            i += 5;
        } else if C::BITS == 32 && c & 0xfe == 0xfc {
            value = (c & 0x01) << 30
                | (at(i + 1) & 0x3f) << 24
                | (at(i + 2) & 0x3f) << 18
                | (at(i + 3) & 0x3f) << 12
                | (at(i + 4) & 0x3f) << 6
                | (at(i + 5) & 0x3f);
            i += 6;
        } else {
            value = 0xfffd; // replacement character
            i += 1;
        }
        out.push(C::from_u32_trunc(value));
    }
    out
}

/// `S16BitToUTF8`. Faithful: the 5- and 6-byte forms (32-bit `wchar_t`
/// only) use the original's masks, including its wrong `0xFC000` mask.
pub fn s16bit_to_utf8<C: Unit>(s: &[C]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s {
        let w = c.to_u32();
        if w < 0x80 {
            out.push(w as u8);
        } else if w < 0x800 {
            out.push((((w & 0x7C0) >> 6) | 0xC0) as u8);
            out.push(((w & 0x3F) | 0x80) as u8);
        } else if w < 0x10000 {
            out.push((((w & 0xF000) >> 12) | 0xE0) as u8);
            out.push((((w & 0xFC0) >> 6) | 0x80) as u8);
            out.push(((w & 0x3F) | 0x80) as u8);
        } else if C::BITS == 32 {
            if w < 0x200000 {
                out.push((((w & 0x1C0000) >> 18) | 0xF0) as u8);
                out.push((((w & 0x3F000) >> 12) | 0x80) as u8);
                out.push((((w & 0xFC0) >> 6) | 0x80) as u8);
                out.push(((w & 0x3F) | 0x80) as u8);
            } else if w < 0x4000000 {
                out.push((((w & 0x3000000) >> 24) | 0xF8) as u8);
                out.push((((w & 0xFC000) >> 18) | 0x80) as u8);
                out.push((((w & 0x3F000) >> 12) | 0x80) as u8);
                out.push((((w & 0xFC0) >> 6) | 0x80) as u8);
                out.push(((w & 0x3F) | 0x80) as u8);
            } else if w < 0x80000000 {
                out.push((((w & 0x40000000) >> 30) | 0xFC) as u8);
                out.push((((w & 0x3F000000) >> 24) | 0x80) as u8);
                out.push((((w & 0xFC000) >> 18) | 0x80) as u8);
                out.push((((w & 0x3F000) >> 12) | 0x80) as u8);
                out.push((((w & 0xFC0) >> 6) | 0x80) as u8);
                out.push(((w & 0x3F) | 0x80) as u8);
            }
            // Faithful: values >= 0x80000000 (negative wchar_t) produce nothing.
        }
    }
    out
}

/// `Get16BitFromArray`: "65 66 67" -> "ABC" (each value cast to `wchar_t`).
pub fn get_16bit_from_array<C: Unit>(s: &[u8]) -> Vec<C> {
    get_int_vec(s, None).into_iter().map(|v| C::from_u32_trunc(v as u32)).collect()
}

/// `ParseUnicodeString`: expands the engine's `[br]` and `[uNNNN]` codes;
/// every other byte is widened.
pub fn parse_unicode_string<C: Unit>(s: &[u8]) -> Vec<C> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'[' {
            let mut command = Vec::new();
            let mut count = 1;
            while i + count < s.len() && s[i + count] != b']' && count < 16 {
                command.push(s[i + count]);
                count += 1;
            }
            let mut found = !(i + count >= s.len() || s[i + count] != b']');
            if found {
                if command == b"br" {
                    out.push(C::from_u8(b'\n'));
                } else if command.first() == Some(&b'u') {
                    let num = crt::atoi(&command[1..]);
                    out.push(C::from_u32_trunc(num as u32));
                } else {
                    found = false;
                }
            }
            if found {
                i += count;
            } else {
                out.push(C::from_u8(c));
            }
        } else {
            out.push(C::from_u8(c));
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Sub strings and searching
// ---------------------------------------------------------------------------

/// `Sub`: `count` -1 means "to the end"; out-of-range values are
/// clamped. Panics for a negative `start` (the C++ throws `out_of_range`).
pub fn sub<C: Unit>(s: &[C], start: i32, count: i32) -> Vec<C> {
    let size = s.len() as i32;
    if start >= size {
        return Vec::new();
    }
    let mut count = count;
    if start.wrapping_add(count) > size {
        count = size - start;
    }
    assert!(start >= 0, "cString::Sub: negative start {start}");
    let start = start as usize;
    if count < 0 { s[start..].to_vec() } else { s[start..start + count as usize].to_vec() }
}

/// `SubW`: like [`sub`], except that an empty string gives "" for any
/// `start` (`Sub` throws for a negative one).
pub fn sub_w<C: Unit>(s: &[C], start: i32, count: i32) -> Vec<C> {
    if s.is_empty() {
        return Vec::new();
    }
    sub(s, start, count)
}

/// `aString.substr(i, n) == pattern` at every position: first match.
/// `GetFirstStringPos`: -1 if not found. Faithful: an empty pattern matches
/// at 0 (if the string is not empty).
pub fn get_first_string_pos<C: Unit>(s: &[C], pattern: &[C]) -> i32 {
    (0..s.len()).find(|&i| substr_eq(s, i, pattern)).map_or(-1, |i| i as i32)
}

/// `GetLastStringPos`: -1 if not found. Faithful: an empty pattern matches
/// at the last position.
pub fn get_last_string_pos<C: Unit>(s: &[C], pattern: &[C]) -> i32 {
    (0..s.len()).rev().find(|&i| substr_eq(s, i, pattern)).map_or(-1, |i| i as i32)
}

fn substr_eq<C: Unit>(s: &[C], i: usize, pattern: &[C]) -> bool {
    let end = (i + pattern.len()).min(s.len());
    &s[i..end] == pattern
}

/// `pattern[0]` of a `std::string`: the terminator (0) when empty.
fn first_or_nul<C: Unit>(s: &[C]) -> C {
    s.first().copied().unwrap_or_default()
}

/// `GetLastChar`
pub fn get_last_char<C: Unit>(s: &[C]) -> Vec<C> {
    s.last().map(|c| vec![*c]).unwrap_or_default()
}

/// `CountCharsInString`: occurrences of `ch[0]`.
pub fn count_chars_in_string<C: Unit>(s: &[C], ch: &[C]) -> i32 {
    let c = first_or_nul(ch);
    s.iter().filter(|&&x| x == c).count() as i32
}

/// `GetNumericSuffix`: trailing digits and their start index (-1 if none).
///
/// Faithful: a string made only of digits has no non-digit to stop at and
/// returns ("", -1).
pub fn get_numeric_suffix<C: Unit>(s: &[C]) -> (Vec<C>, i32) {
    let size = s.len() as i32;
    let mut index = -1;
    for i in (0..s.len()).rev() {
        let c = s[i].to_u32();
        if c < b'0' as u32 || c > b'9' as u32 {
            index = i as i32 + 1;
            if index >= size {
                index = -1;
            }
            break;
        }
    }
    let suffix = if index != -1 { sub(s, index, -1) } else { Vec::new() };
    (suffix, index)
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// `GetFileName`: the part after the last '/' or '\'.
pub fn get_file_name<C: Unit>(s: &[C]) -> Vec<C> {
    let pos1 = get_last_string_pos(s, &lit("\\"));
    let pos2 = get_last_string_pos(s, &lit("/"));
    let pos = if pos1 > pos2 { pos1 } else { pos2 };
    if pos < 0 { s.to_vec() } else { s[(pos + 1) as usize..].to_vec() }
}

/// `GetFileExt`: extension without the dot, or "".
pub fn get_file_ext<C: Unit>(s: &[C]) -> Vec<C> {
    let name = get_file_name(s);
    let pos = get_last_string_pos(&name, &lit("."));
    if pos < 0 { Vec::new() } else { name[(pos + 1) as usize..].to_vec() }
}

/// `SetFileExt`: replace the extension ("" removes it; "exe" and ".exe"
/// both work). Faithful: when the file has an extension, everything from
/// the last '.' in the whole string is cut.
pub fn set_file_ext<C: Unit>(s: &[C], ext: &[C]) -> Vec<C> {
    let mut ext = ext;
    if ext.first().is_some_and(|c| c.is(b'.')) {
        ext = &ext[1..];
    }
    let mut out = s.to_vec();
    if !get_file_ext(s).is_empty() {
        out.truncate(get_last_string_pos(s, &lit(".")) as usize);
    }
    if !ext.is_empty() {
        out.push(C::from_u8(b'.'));
        out.extend_from_slice(ext);
    }
    out
}

/// `GetFilePath`: directory part including the trailing slash.
/// Faithful: a string without any '.' is returned unchanged.
pub fn get_file_path<C: Unit>(s: &[C]) -> Vec<C> {
    if get_last_string_pos(s, &lit(".")) < 0 {
        return s.to_vec();
    }
    let pos1 = get_last_string_pos(s, &lit("\\"));
    let pos2 = get_last_string_pos(s, &lit("/"));
    let pos = if pos1 > pos2 { pos1 } else { pos2 };
    if pos < 0 { Vec::new() } else { s[..(pos + 1) as usize].to_vec() }
}

/// `SetFilePath`: `path` + '/' (unless it ends in a slash) + file name.
pub fn set_file_path<C: Unit>(s: &[C], path: &[C]) -> Vec<C> {
    let last = get_last_char(path);
    let mut out = path.to_vec();
    if last != lit::<C>("/") && last != lit::<C>("\\") {
        out.push(C::from_u8(b'/'));
    }
    out.extend(get_file_name(s));
    out
}

/// `GetRelativePath`: `path` relative to `relative_to`, with '/' separators
/// and a trailing '/'. Returns `path` unchanged if nothing is shared.
pub fn get_relative_path<C: Unit>(path: &[C], relative_to: &[C]) -> Vec<C> {
    let sep: Vec<C> = lit("/\\");
    let path_folders = get_string_vec(path, Some(&sep), Separators::WithCr);
    let rel_folders = get_string_vec(relative_to, Some(&sep), Separators::WithCr);

    let count = path_folders.len().min(rel_folders.len());
    let mut difference_at = 0;
    for i in 0..count {
        if path_folders[i] != rel_folders[i] {
            difference_at = i;
            break;
        }
        difference_at = i + 1;
    }
    if difference_at == 0 {
        return path.to_vec();
    }

    let mut out = Vec::new();
    for _ in 0..rel_folders.len() - difference_at {
        out.extend(lit::<C>("../"));
    }
    for folder in &path_folders[difference_at..] {
        out.extend_from_slice(folder);
        out.push(C::from_u8(b'/'));
    }
    out
}

/// `AddSlashAtEnd`
pub fn add_slash_at_end<C: Unit>(s: &[C], slash: C) -> Vec<C> {
    let Some(&last) = s.last() else { return Vec::new() };
    let mut out = s.to_vec();
    if !(last.is(b'/') || last.is(b'\\')) {
        out.push(slash);
    }
    out
}

/// `RemoveSlashAtEnd`
pub fn remove_slash_at_end<C: Unit>(s: &[C]) -> Vec<C> {
    match s.last() {
        Some(c) if c.is(b'/') || c.is(b'\\') => s[..s.len() - 1].to_vec(),
        _ => s.to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Case and replacement
// ---------------------------------------------------------------------------

/// `ToLowerCase`: ASCII only ("C" locale `tolower`).
pub fn to_lower_case<C: Unit>(s: &[C]) -> Vec<C> {
    s.iter()
        .map(|&c| {
            let v = c.to_u32();
            if (b'A' as u32..=b'Z' as u32).contains(&v) { C::from_u32_trunc(v + 32) } else { c }
        })
        .collect()
}

/// `ToUpperCase`: ASCII only ("C" locale `toupper`).
pub fn to_upper_case<C: Unit>(s: &[C]) -> Vec<C> {
    s.iter()
        .map(|&c| {
            let v = c.to_u32();
            if (b'a' as u32..=b'z' as u32).contains(&v) { C::from_u32_trunc(v - 32) } else { c }
        })
        .collect()
}

/// `ReplaceCharTo`: replace every `old[0]` with `new[0]`, or remove them if
/// `new` is empty. Faithful: an empty `old` means the NUL character.
pub fn replace_char_to<C: Unit>(s: &[C], old: &[C], new: &[C]) -> Vec<C> {
    let old = first_or_nul(old);
    match new.first() {
        Some(&n) => s.iter().map(|&c| if c == old { n } else { c }).collect(),
        None => s.iter().copied().filter(|&c| c != old).collect(),
    }
}

/// `ReplaceStringTo`: replace every occurrence of `old` with `new`,
/// scanning left to right without overlap.
///
/// Deviation: with an empty `old` the C++ never returns (its index wraps);
/// here the input is returned unchanged.
pub fn replace_string_to(s: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    if old.is_empty() {
        return s.to_vec();
    }
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s.len() >= i + old.len() && &s[i..i + old.len()] == old {
            out.extend_from_slice(new);
            i += old.len();
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Splitting
// ---------------------------------------------------------------------------

/// Default separators when none are given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Separators {
    /// ' ', '\n', '\r', '\t', ',' (`GetStringVec`, `GetStringVecW`)
    WithCr,
    /// ' ', '\n', '\t', ',' (`GetStringWVec`)
    WithoutCr,
}

/// `GetStringVec` / `GetStringVecW` / `GetStringWVec`: split on any of the
/// separator characters, dropping empty parts.
pub fn get_string_vec<C: Unit>(s: &[C], separators: Option<&[C]>, defaults: Separators) -> Vec<Vec<C>> {
    let is_sep = |c: C| match separators {
        Some(seps) => seps.contains(&c),
        None => {
            c.is(b' ') || c.is(b'\n') || (defaults == Separators::WithCr && c.is(b'\r')) || c.is(b'\t') || c.is(b',')
        }
    };
    let mut out = Vec::new();
    let mut word = Vec::new();
    let mut started = false;
    for (i, &c) in s.iter().enumerate() {
        if is_sep(c) {
            if started {
                started = false;
                out.push(std::mem::take(&mut word));
            }
        } else {
            started = true;
            word.push(c);
            if i == s.len() - 1 {
                out.push(word.clone());
            }
        }
    }
    out
}

/// `GetIntVec`: `atoi` of each part.
pub fn get_int_vec(s: &[u8], separators: Option<&[u8]>) -> Vec<i32> {
    get_string_vec(s, separators, Separators::WithCr).iter().map(|w| crt::atoi(w)).collect()
}

/// `GetUIntVec`: `atoi` of each part, cast to unsigned.
pub fn get_uint_vec(s: &[u8], separators: Option<&[u8]>) -> Vec<u32> {
    get_int_vec(s, separators).into_iter().map(|v| v as u32).collect()
}

/// `GetFloatVec`: `(float)atof` of each part.
pub fn get_float_vec(s: &[u8], separators: Option<&[u8]>) -> Vec<f32> {
    get_string_vec(s, separators, Separators::WithCr).iter().map(|w| to_float_bytes(w)).collect()
}

// ---------------------------------------------------------------------------
// Conversion from text
// ---------------------------------------------------------------------------

fn to_float_bytes(s: &[u8]) -> f32 {
    crt::atof(s) as f32
}

/// `ToInt`: `atoi`, or `default` for a null string.
pub fn to_int(s: Option<&[u8]>, default: i32) -> i32 {
    s.map_or(default, crt::atoi)
}

/// `ToFloat`: `(float)atof`, or `default` for a null string.
pub fn to_float(s: Option<&[u8]>, default: f32) -> f32 {
    s.map_or(default, to_float_bytes)
}

/// `ToBool`: true only for "true" in any case.
pub fn to_bool(s: Option<&[u8]>, default: bool) -> bool {
    s.map_or(default, |s| to_lower_case(s) == b"true")
}

fn floats_n<const N: usize>(s: Option<&[u8]>) -> Option<[f32; N]> {
    get_float_vec(s?, None).try_into().ok()
}

fn ints_n<const N: usize>(s: Option<&[u8]>) -> Option<[i32; N]> {
    get_int_vec(s?, None).try_into().ok()
}

/// `ToColor`: exactly 4 numbers, else `default`.
pub fn to_color(s: Option<&[u8]>, default: Color) -> Color {
    floats_n::<4>(s).map_or(default, |v| Color::new(v[0], v[1], v[2], v[3]))
}

/// `ToVector2f`: exactly 2 numbers, else `default`.
pub fn to_vector2f(s: Option<&[u8]>, default: Vector2f) -> Vector2f {
    floats_n::<2>(s).map_or(default, |v| Vector2f::new(v[0], v[1]))
}

/// `ToVector3f`: exactly 3 numbers, else `default`.
pub fn to_vector3f(s: Option<&[u8]>, default: Vector3f) -> Vector3f {
    floats_n::<3>(s).map_or(default, |v| Vector3f::new(v[0], v[1], v[2]))
}

/// `ToVector2l`: exactly 2 numbers, else `default`.
pub fn to_vector2l(s: Option<&[u8]>, default: Vector2l) -> Vector2l {
    ints_n::<2>(s).map_or(default, |v| Vector2l::new(v[0], v[1]))
}

/// `ToVector3l`: exactly 3 numbers, else `default`.
pub fn to_vector3l(s: Option<&[u8]>, default: Vector3l) -> Vector3l {
    ints_n::<3>(s).map_or(default, |v| Vector3l::new(v[0], v[1], v[2]))
}

/// `ToMatrixf`: exactly 16 numbers (row-major), else `default`.
pub fn to_matrixf(s: Option<&[u8]>, default: Matrixf) -> Matrixf {
    floats_n::<16>(s).map_or(default, |v| Matrixf::from_slice(&v))
}

/// Tokens of `UIntStringToArray` / `FloatStringToArray`: space-separated,
/// stopping after `count` tokens or at the end of the string.
///
/// Deviation: the C++ keeps reading past the end of the string if there are
/// fewer than `count` numbers, and overflows its buffer for tokens longer
/// than 19 characters; here it stops at the end and accepts long tokens.
fn space_tokens(s: &[u8], count: usize) -> Vec<&[u8]> {
    let s = &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())];
    s.split(|&b| b == b' ').filter(|t| !t.is_empty()).take(count).collect()
}

/// `UIntStringToArray`: up to `count` space-separated unsigned ints.
pub fn uint_string_to_array(s: &[u8], count: usize) -> Vec<u32> {
    space_tokens(s, count).into_iter().map(|t| crt::atoi(t) as u32).collect()
}

/// `FloatStringToArray`: up to `count` space-separated floats.
pub fn float_string_to_array(s: &[u8], count: usize) -> Vec<f32> {
    space_tokens(s, count).into_iter().map(to_float_bytes).collect()
}

// ---------------------------------------------------------------------------
// Conversion to text
// ---------------------------------------------------------------------------

/// `ToString(int, padding)`: `"%0*d"`.
pub fn int_to_string(x: i32, padding_zeros: i32) -> String {
    crt::format_int(x as i64, padding_zeros)
}

/// `ToString(unsigned int, padding)`: `"%0*u"`.
pub fn uint_to_string(x: u32, padding_zeros: i32) -> String {
    crt::format_uint(x as u64, padding_zeros)
}

/// `ToString(unsigned long, padding)`: `"%0*lu"` (`unsigned long` is 32
/// bits on Windows; the caller passes the value zero-extended).
pub fn ulong_to_string(x: u64, padding_zeros: i32) -> String {
    crt::format_uint(x, padding_zeros)
}

/// `ToStringW(unsigned long, padding)`. Faithful: the C++ format is
/// `"%0*ld"`, so the value is printed as a signed `long`; the caller passes
/// `(long)x` sign-extended.
pub fn long_to_string(x: i64, padding_zeros: i32) -> String {
    crt::format_int(x, padding_zeros)
}

/// `ToString(float, decimals, removeZeros)`: `"%.*f"` (20 decimals if
/// negative), optionally without trailing zeros (and the '.', if all
/// decimals were zero).
pub fn float_to_string(x: f32, num_decimals: i32, remove_trailing_zeros: bool) -> String {
    let decimals = if num_decimals < 0 { 20 } else { num_decimals };
    let mut s = crt::format_fixed(x as f64, decimals);
    if remove_trailing_zeros && s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Hashes and data encoding
// ---------------------------------------------------------------------------

fn avalanche(mut hash: u32) -> u32 {
    hash ^= hash << 3;
    hash = hash.wrapping_add(hash >> 5);
    hash ^= hash << 4;
    hash = hash.wrapping_add(hash >> 17);
    hash ^= hash << 25;
    hash = hash.wrapping_add(hash >> 6);
    hash
}

/// `GetHash`: Paul Hsieh's SuperFastHash as compiled on Windows (16-bit
/// little-endian reads through `wchar_t`). Faithful: the tail bytes are
/// read as signed `char`.
pub fn get_hash(s: &[u8]) -> u32 {
    if s.is_empty() {
        return 0;
    }
    let read16 = |i: usize| u16::from_le_bytes([s[i], s[i + 1]]) as u32;
    let signed = |i: usize| s[i] as i8 as i32 as u32;
    let mut hash = s.len() as u32;
    let rem = s.len() & 3;
    let mut p = 0;
    for _ in 0..s.len() >> 2 {
        hash = hash.wrapping_add(read16(p));
        let temp = (read16(p + 2) << 11) ^ hash;
        hash = (hash << 16) ^ temp;
        p += 4;
        hash = hash.wrapping_add(hash >> 11);
    }
    match rem {
        3 => {
            hash = hash.wrapping_add(read16(p));
            hash ^= hash << 16;
            hash ^= signed(p + 2) << 18;
            hash = hash.wrapping_add(hash >> 11);
        }
        2 => {
            hash = hash.wrapping_add(read16(p));
            hash ^= hash << 11;
            hash = hash.wrapping_add(hash >> 17);
        }
        1 => {
            hash = hash.wrapping_add(signed(p));
            hash ^= hash << 10;
            hash = hash.wrapping_add(hash >> 1);
        }
        _ => {}
    }
    avalanche(hash)
}

/// `GetHashW`: SuperFastHash over wide characters, two per step.
pub fn get_hash_w<C: Unit>(s: &[C]) -> u32 {
    if s.is_empty() {
        return 0;
    }
    let mut hash = s.len() as u32;
    let mut p = 0;
    for _ in 0..s.len() >> 1 {
        hash = hash.wrapping_add(s[p].to_u32());
        let temp = (s[p + 1].to_u32() << 11) ^ hash;
        hash = (hash << 16) ^ temp;
        p += 2;
        hash = hash.wrapping_add(hash >> 11);
    }
    if s.len() & 1 == 1 {
        hash = hash.wrapping_add(s[p].to_u32());
        hash ^= hash << 10;
        hash = hash.wrapping_add(hash >> 1);
    }
    avalanche(hash)
}

fn encode_chunk(chunk: i32, out: &mut Vec<u8>) {
    // `char` is signed: the shifts below are on sign-extended values.
    let mut chars = [0i32; 6];
    for (dst, b) in chars.iter_mut().zip(chunk.to_le_bytes()) {
        *dst = b as i8 as i32;
    }
    // High bits of each byte go into two extra characters.
    chars[4] = (chars[2] >> 2) & 0x30;
    chars[5] = (chars[3] >> 2) & 0x30;
    chars[4] |= (chars[0] >> 4) & 0x0C;
    chars[5] |= (chars[1] >> 4) & 0x0C;
    for ch in &mut chars {
        *ch &= 63;
        if *ch != 63 {
            *ch += 64;
        }
    }
    out.extend(chars.iter().map(|&x| x as u8));
}

/// `EncodeDataToTextString`: 4 bytes -> 6 printable characters. Returns
/// `None` (output untouched) if the size is not a multiple of 4.
pub fn encode_data_to_text_string(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(data.len() / 4 * 6);
    for chunk in data.chunks_exact(4) {
        encode_chunk(i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]), &mut out);
    }
    Some(out)
}

/// `DecodeDataFromTextString`: inverse of [`encode_data_to_text_string`]
/// into `dest`. Faithful: does nothing if `dest.len()` is not a multiple of
/// 4 or the text is too short.
pub fn decode_data_from_text_string(text: &[u8], dest: &mut [u8]) {
    if dest.len() % 4 != 0 || text.len() < (dest.len() / 4) * 6 {
        return;
    }
    for (chunk, out) in text.chunks_exact(6).zip(dest.chunks_exact_mut(4)) {
        let mut front = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let back = i16::from_le_bytes([chunk[4], chunk[5]]) as i32;
        front &= 0x3F3F3F3F;
        front |= ((back << 18) | (back << 4)) & 0xC0C0C0C0u32 as i32;
        out.copy_from_slice(&front.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(get_file_name(b"dir/sub\\file.txt".as_slice()), b"file.txt");
        assert_eq!(get_file_ext(b"a.b/c".as_slice()), b"");
        assert_eq!(get_file_ext(b"a/c.tar.gz".as_slice()), b"gz");
        assert_eq!(set_file_ext(b"a/c.txt".as_slice(), b".dds"), b"a/c.dds");
        assert_eq!(set_file_ext(b"a.d/c".as_slice(), b"dds"), b"a.d/c.dds");
        assert_eq!(set_file_ext(b"c.txt".as_slice(), b""), b"c");
        assert_eq!(get_file_path(b"a/b/c.txt".as_slice()), b"a/b/");
        assert_eq!(get_file_path(b"a/b/c".as_slice()), b"a/b/c");
        assert_eq!(set_file_path(b"x/f.txt".as_slice(), b"dir"), b"dir/f.txt");
        assert_eq!(get_relative_path(b"/a/b/c/d".as_slice(), b"/a/b/x"), b"../c/d/");
        assert_eq!(remove_slash_at_end(b"a/".as_slice()), b"a");
        assert_eq!(add_slash_at_end(b"a".as_slice(), b'\\'), b"a\\");
    }

    #[test]
    fn splitting_and_numbers() {
        let v = get_string_vec(b" one, two\tthree ".as_slice(), None, Separators::WithCr);
        assert_eq!(v, vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]);
        assert_eq!(get_float_vec(b"1.5 2,3", None), vec![1.5, 2.0, 3.0]);
        assert_eq!(to_vector3f(Some(b"1 2"), Vector3f::splat(7.0)), Vector3f::splat(7.0));
        assert!(to_bool(Some(b"TrUe"), false));
        assert_eq!(float_to_string(1.5, 3, true), "1.5");
        assert_eq!(float_to_string(2.0, 3, true), "2");
        assert_eq!(int_to_string(-7, 3), "-07");
        assert_eq!(uint_string_to_array(b"1 2  3 4", 3), vec![1, 2, 3]);
    }

    #[test]
    fn unicode() {
        let s: Vec<u16> = parse_unicode_string(b"a[br]b[u233][x][u");
        let expected: Vec<u16> = "a\nb\u{e9}[x][u".encode_utf16().collect();
        assert_eq!(s, expected);
        let utf8 = "h\u{e9}\u{20ac}".as_bytes();
        let wide: Vec<u16> = utf8_to_wchar(utf8);
        assert_eq!(wide, vec![b'h' as u16, 0xe9, 0x20ac]);
        assert_eq!(s16bit_to_utf8(&wide), utf8);
        // 4-byte UTF-8 only decodes with a 32-bit wchar_t.
        let emoji = "\u{1F600}".as_bytes();
        assert_eq!(utf8_to_wchar::<u32>(emoji), vec![0x1F600]);
        assert_eq!(utf8_to_wchar::<u16>(emoji)[0], 0xfffd);
    }

    #[test]
    fn windows_wchar() {
        // 16-bit wchar_t: Latin-1 in and out, '_' for anything wider.
        let w: Vec<u16> = to_16char(b"a\xe9\xff\0b");
        assert_eq!(w, vec![0x61, 0xe9, 0xff]);
        assert_eq!(to_8char_windows(&[0x61u16, 0xe9, 0x100, 0x20ac, 0]), b"a\xe9__\0");
        assert_eq!(to_8char_glibc(&[0x61u32, 0x62, 0, 0x63]), b"ab");
        assert_eq!(to_8char_glibc(&[0x61u32, 0xe9]), b"");
        // [uNNN] is cast to a 16-bit wchar_t.
        let s: Vec<u16> = parse_unicode_string(b"[u65601][u-1]");
        assert_eq!(s, vec![0x41, 0xffff]);
        // 4-byte UTF-8 is not decoded, and every byte becomes U+FFFD or a
        // (masked) 2/3-byte sequence.
        let s: Vec<u16> = utf8_to_wchar(b"\xf0\x9f\x98\x80");
        assert_eq!(s, vec![0xfffd, 0xfffd, 0xfffd, 0xfffd]);
        assert_eq!(s16bit_to_utf8(&[0xffffu16]), b"\xef\xbf\xbf");
        assert_eq!(get_numeric_suffix(&[0x31u16, 0x100, 0x32]), (vec![0x32u16], 2));
        assert_eq!(to_lower_case(&[0xC4u16, 0x41, 0x130]), vec![0xC4u16, 0x61, 0x130]);
    }

    #[test]
    fn hash_reference_values() {
        // From a C copy of cString::GetHash with Windows' 16-bit reads. (The
        // Linux build of the original reads past the string, so the
        // differential test cannot check this function.)
        let cases: [(&[u8], u32); 13] = [
            (b"", 0x00000000),
            (b"a", 0x115ea782),
            (b"ab", 0x516b8b44),
            (b"abc", 0xd2be198a),
            (b"abcd", 0xdad8b8db),
            (b"abcde", 0x51ed072e),
            (b"\xff", 0x00000000),
            (b"\x80\x81", 0xfb1b8d10),
            (b"ab\xe9", 0xb4dfd4b5),
            (b"\xe9\xe9\xe9", 0x8c849f21),
            (b"textures/stone_wall01.dds", 0x63d10c68),
            (b"Hello, World!", 0x86956be9),
            (b"\xff\xfe\xfd\xfc\xfb\xfa\xf9", 0x7833c0f0),
        ];
        for (s, h) in cases {
            assert_eq!(get_hash(s), h, "{s:?}");
        }
    }

    #[test]
    fn data_text_roundtrip() {
        let data: Vec<u8> = (0..=255u8).collect();
        let text = encode_data_to_text_string(&data).unwrap();
        assert!(text.iter().all(|&c| (63..=127).contains(&c)));
        let mut back = vec![0u8; data.len()];
        decode_data_from_text_string(&text, &mut back);
        assert_eq!(back, data);
        assert!(encode_data_to_text_string(&[1, 2, 3]).is_none());
    }

    #[test]
    fn misc() {
        assert_eq!(get_numeric_suffix(b"door12".as_slice()), (b"12".to_vec(), 4));
        assert_eq!(get_numeric_suffix(b"123".as_slice()), (Vec::new(), -1));
        assert_eq!(replace_string_to(b"aXbXX", b"X", b"--"), b"a--b----");
        assert_eq!(replace_char_to(b"a.b.c".as_slice(), b".", b""), b"abc");
        assert_eq!(get_first_string_pos(b"abcabc".as_slice(), b"bc"), 1);
        assert_eq!(get_last_string_pos(b"abcabc".as_slice(), b"bc"), 4);
        assert_eq!(get_last_string_pos(b"abc".as_slice(), b""), 2);
        assert_eq!(sub(b"hello".as_slice(), 1, 10), b"ello");
        assert_eq!(to_lower_case(b"AbC\xc4".as_slice()), b"abc\xc4");
    }
}
