//! Character units: `u8` for `tString`, [`WChar`] for `tWString`.

use std::fmt::Debug;

/// The platform's `wchar_t`: 16-bit (UTF-16 code unit) on Windows, 32-bit
/// on other platforms (where it is a signed `int`; the bit pattern is kept).
#[cfg(windows)]
pub type WChar = u16;
#[cfg(not(windows))]
pub type WChar = u32;

/// A string code unit (`char` or `wchar_t`).
pub trait Unit: Copy + Eq + Ord + Default + Debug + Send + Sync + 'static {
    /// Size of the unit in bits (8, 16 or 32).
    const BITS: u32;

    /// Widen a byte (`(wchar_t)(unsigned char)c`).
    fn from_u8(b: u8) -> Self;

    /// The unit's value, zero-extended (`(Uint32)c`).
    fn to_u32(self) -> u32;

    /// C cast from an integer, truncating (`(wchar_t)x`).
    fn from_u32_trunc(v: u32) -> Self;

    /// `c == 'x'` for an ASCII literal.
    #[inline]
    fn is(self, ascii: u8) -> bool {
        self == Self::from_u8(ascii)
    }
}

impl Unit for u8 {
    const BITS: u32 = 8;
    #[inline]
    fn from_u8(b: u8) -> Self {
        b
    }
    #[inline]
    fn to_u32(self) -> u32 {
        self as u32
    }
    #[inline]
    fn from_u32_trunc(v: u32) -> Self {
        v as u8
    }
}

impl Unit for u16 {
    const BITS: u32 = 16;
    #[inline]
    fn from_u8(b: u8) -> Self {
        b as u16
    }
    #[inline]
    fn to_u32(self) -> u32 {
        self as u32
    }
    #[inline]
    fn from_u32_trunc(v: u32) -> Self {
        v as u16
    }
}

impl Unit for u32 {
    const BITS: u32 = 32;
    #[inline]
    fn from_u8(b: u8) -> Self {
        b as u32
    }
    #[inline]
    fn to_u32(self) -> u32 {
        self
    }
    #[inline]
    fn from_u32_trunc(v: u32) -> Self {
        v
    }
}

/// An ASCII string literal as units of `C`.
#[inline]
pub fn lit<C: Unit>(s: &str) -> Vec<C> {
    s.bytes().map(C::from_u8).collect()
}

/// Narrow text to units of `C` (each byte widened).
#[inline]
pub fn widen<C: Unit>(s: &[u8]) -> Vec<C> {
    s.iter().map(|&b| C::from_u8(b)).collect()
}
