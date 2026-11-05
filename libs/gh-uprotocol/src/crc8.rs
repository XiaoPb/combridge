//! 表驱动 CRC8：poly 0x07（MSB-first），初值 0xFF，与 SDK `g_uchCrc8TabArr` 一致。

const fn make_table() -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u8;
        let mut bit = 0;
        while bit < 8 {
            c = if c & 0x80 != 0 {
                (c << 1) ^ 0x07
            } else {
                c << 1
            };
            bit += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

pub const TABLE: [u8; 256] = make_table();

#[inline]
pub fn update(crc: u8, byte: u8) -> u8 {
    TABLE[(crc ^ byte) as usize]
}

pub fn crc8(data: &[u8]) -> u8 {
    let mut c = 0xFFu8;
    for &b in data {
        c = TABLE[(c ^ b) as usize];
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_head_matches_sdk() {
        assert_eq!(&TABLE[..8], &[0x00, 0x07, 0x0E, 0x09, 0x1C, 0x1B, 0x12, 0x15]);
    }

    #[test]
    fn known_vectors() {
        assert_eq!(crc8(&[0xAA, 0x11, 0x19, 0x00]), 0x91);
        assert_eq!(crc8(&[0xAA, 0x11, 0x19, 0x01, 0x01]), 0xEC);
        assert_eq!(crc8(&[0xAA, 0x11, 0xA0, 0x00]), 0x63);
        assert_eq!(crc8(&[0xAA, 0x11, 0x17, 0x01, 0xC3]), 0x80);
        assert_eq!(crc8(&[0xAA, 0x11, 0x16, 0x01, 0x02]), 0xA2);
    }
}
