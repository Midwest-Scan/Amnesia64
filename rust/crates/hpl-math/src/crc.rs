//! `cCRCTable` / `cCRC` (`math/CRC.h`): table-driven, MSB-first CRC-32 with
//! a configurable polynomial and zero initial register.

pub type CrcKey = u32;

/// `cCRCTable`
#[derive(Clone)]
pub struct CrcTable {
    key: CrcKey,
    table: [CrcKey; 256],
}

impl CrcTable {
    pub fn new(key: CrcKey) -> Self {
        let mut t = Self { key, table: [0; 256] };
        Self::fill(&mut t.table, key);
        t
    }

    /// Fills `table` for polynomial `key` (`cCRCTable::Init`).
    pub fn fill(table: &mut [CrcKey; 256], key: CrcKey) {
        for (i, entry) in table.iter_mut().enumerate() {
            let mut reg = (i as CrcKey) << 24;
            for _ in 0..8 {
                let top_bit = (reg & 0x8000_0000) != 0;
                reg <<= 1;
                if top_bit {
                    reg ^= key;
                }
            }
            *entry = reg;
        }
    }

    pub fn key(&self) -> CrcKey {
        self.key
    }

    pub fn entries(&self) -> &[CrcKey; 256] {
        &self.table
    }
}

/// `cCRC::PutByte` on a raw register.
#[inline]
pub fn put_byte(table: &[CrcKey; 256], register: CrcKey, byte: u32) -> CrcKey {
    let top = (register >> 24) ^ byte;
    // Faithful: C++ indexes with an unmasked `unsigned`; callers only pass bytes.
    (register << 8) ^ table[top as usize]
}

/// `cCRC::PutData` on a raw register.
#[inline]
pub fn put_data(table: &[CrcKey; 256], mut register: CrcKey, data: &[u8]) -> CrcKey {
    for &b in data {
        register = put_byte(table, register, b as u32);
    }
    register
}

/// `cCRC`
#[derive(Clone)]
pub struct Crc {
    table: CrcTable,
    register: CrcKey,
}

impl Crc {
    pub fn new(key: CrcKey) -> Self {
        Self { table: CrcTable::new(key), register: 0 }
    }

    pub fn put_byte(&mut self, byte: u8) {
        self.register = put_byte(&self.table.table, self.register, byte as u32);
    }

    pub fn put_data(&mut self, data: &[u8]) {
        self.register = put_data(&self.table.table, self.register, data);
    }

    /// `Done`: returns the CRC and resets the register.
    pub fn done(&mut self) -> CrcKey {
        std::mem::take(&mut self.register)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_mpeg2_without_init() {
        // Polynomial 0x04C11DB7, init 0, no reflection, no xor-out.
        let mut crc = Crc::new(0x04C1_1DB7);
        crc.put_data(b"123456789");
        assert_eq!(crc.done(), 0x89A1_897F);
        assert_eq!(crc.done(), 0);
    }
}
