//! `SHA1` (`system/SHA1.h`): FIPS 180-1 message digest (Paul E. Jones'
//! implementation as integrated into HPL2).
//!
//! The engine itself never uses this class; it is ported for completeness
//! and is not forwarded from C++.

use crate::chars::Unit;
use crate::string::to_8char;

/// Incremental SHA-1 hasher with the C++ class's state rules: feeding input
/// after the result was read, or more than 2^64 - 1 bits, marks it corrupted
/// and every later result fails until [`Sha1::reset`].
#[derive(Clone, Debug)]
pub struct Sha1 {
    h: [u32; 5],
    length_low: u32,
    length_high: u32,
    block: [u8; 64],
    block_index: usize,
    computed: bool,
    corrupted: bool,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    pub fn new() -> Self {
        Self {
            h: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            length_low: 0,
            length_high: 0,
            block: [0; 64],
            block_index: 0,
            computed: false,
            corrupted: false,
        }
    }

    /// `Reset`
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// `Input(const unsigned char*, unsigned)`
    pub fn input(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        if self.computed || self.corrupted {
            self.corrupted = true;
            return;
        }
        for &b in data {
            if self.corrupted {
                break;
            }
            self.block[self.block_index] = b;
            self.block_index += 1;
            self.length_low = self.length_low.wrapping_add(8);
            if self.length_low == 0 {
                self.length_high = self.length_high.wrapping_add(1);
                if self.length_high == 0 {
                    self.corrupted = true; // message is too long
                }
            }
            if self.block_index == 64 {
                self.process_message_block();
            }
        }
    }

    /// `Input(const tWString&)`: hashes `To8Char` of the text.
    pub fn input_wide<C: Unit>(&mut self, text: &[C]) {
        self.input(&to_8char(text));
    }

    /// `Result(unsigned*)`: the five digest words, or `None` if corrupted.
    pub fn result(&mut self) -> Option<[u32; 5]> {
        if self.corrupted {
            return None;
        }
        if !self.computed {
            self.pad_message();
            self.computed = true;
        }
        Some(self.h)
    }

    /// `Result(tString&)`: 40 lowercase hex digits (`"%08x"` per word).
    pub fn result_hex(&mut self) -> Option<String> {
        self.result().map(|h| h.iter().map(|w| format!("{w:08x}")).collect())
    }

    fn process_message_block(&mut self) {
        const K: [u32; 4] = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];
        let mut w = [0u32; 80];
        for (t, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([self.block[t * 4], self.block[t * 4 + 1], self.block[t * 4 + 2], self.block[t * 4 + 3]]);
        }
        for t in 16..80 {
            w[t] = (w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = self.h;
        for (t, &wt) in w.iter().enumerate() {
            let (f, k) = match t {
                0..20 => ((b & c) | (!b & d), K[0]),
                20..40 => (b ^ c ^ d, K[1]),
                40..60 => ((b & c) | (b & d) | (c & d), K[2]),
                _ => (b ^ c ^ d, K[3]),
            };
            let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(wt).wrapping_add(k);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        for (h, v) in self.h.iter_mut().zip([a, b, c, d, e]) {
            *h = h.wrapping_add(v);
        }
        self.block_index = 0;
    }

    fn pad_message(&mut self) {
        self.block[self.block_index] = 0x80;
        self.block_index += 1;
        if self.block_index > 56 {
            self.block[self.block_index..].fill(0);
            self.block_index = 64;
            self.process_message_block();
        }
        self.block[self.block_index..56].fill(0);
        self.block[56..60].copy_from_slice(&self.length_high.to_be_bytes());
        self.block[60..64].copy_from_slice(&self.length_low.to_be_bytes());
        self.process_message_block();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(data: &[u8]) -> String {
        let mut s = Sha1::new();
        s.input(data);
        s.result_hex().unwrap()
    }

    #[test]
    fn fips_vectors() {
        assert_eq!(hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
        let million = vec![b'a'; 1_000_000];
        assert_eq!(hex(&million), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
        // Every padding boundary.
        for len in 50..70 {
            let data = vec![b'x'; len];
            let mut split = Sha1::new();
            split.input(&data[..len / 3]);
            split.input(&data[len / 3..]);
            assert_eq!(split.result_hex().unwrap(), hex(&data));
        }
    }

    #[test]
    fn state_rules() {
        let mut s = Sha1::new();
        s.input(b"abc");
        let first = s.result().unwrap();
        assert_eq!(s.result().unwrap(), first); // result is cached
        s.input(b"");
        assert!(s.result().is_some()); // empty input is ignored
        s.input(b"d");
        assert!(s.result().is_none()); // input after result corrupts
        s.reset();
        s.input_wide(&[b'a' as u16, b'b' as u16, b'c' as u16]);
        assert_eq!(s.result().unwrap(), first);
    }
}
