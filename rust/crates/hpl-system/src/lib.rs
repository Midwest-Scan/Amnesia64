//! Rust port of parts of the HPL2 `system` module
//! (`HPL2/core/{include,sources}/system`).
//!
//! | C++                    | Rust             |
//! |------------------------|------------------|
//! | `cString`              | [`string`]       |
//! | `SHA1`                 | [`sha1`]         |
//! | `cPreprocessParser`    | [`preprocess`]   |
//! | `cLogicTimer`          | [`logic_timer`]  |
//! | `VideoComp`            | [`platform`]     |
//!
//! The C++ engine uses byte strings (`tString`, any 8-bit encoding) and
//! `wchar_t` strings (`tWString`, UTF-16 code units on Windows, 32-bit on
//! Linux). This port keeps those semantics: narrow strings are `[u8]`, wide
//! strings are `[WChar]`, and functions that behave the same for both are
//! generic over [`chars::Unit`]. C runtime behaviour the engine relies on
//! (`atoi`, `atof`, `printf("%f")`, the "C" locale) is reproduced in [`crt`].
//!
//! As in `hpl-math`, operation order and quirks of the original are kept so
//! results match the C++ exactly; deliberate quirks are marked `Faithful:`.

// Comparisons are written exactly as in C++ (see hpl-math).
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::manual_range_contains)]

pub mod chars;
pub mod crt;
pub mod logic_timer;
pub mod platform;
pub mod preprocess;
pub mod sha1;
pub mod string;

pub use chars::{Unit, WChar};
