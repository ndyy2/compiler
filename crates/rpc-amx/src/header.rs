//! AMX file header — explicit little-endian serialization.
//!
//! Layout matches `tagAMX_HEADER` in `compuphase/pawn amx/amx.h` (60 bytes,
//! packed, little-endian on x86/ARM targets). We never transmute or copy the
//! native struct; every field is written field-by-field so endianness,
//! alignment and sizes stay under our control.

use crate::instruction::{DEF_SIZE_32, AMX_MAGIC_32};

/// AMX file header (memory format == file format).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AmxHeader {
    pub size: i32,      // size of the "file"
    pub magic: u16,     // signature (0xf1e0 for 32-bit cells)
    pub file_version: u8,
    pub amx_version: u8,
    pub flags: i16,
    pub defsize: i16,
    pub cod: i32,
    pub dat: i32,
    pub hea: i32,
    pub stp: i32,
    pub cip: i32,
    pub publics: i32,
    pub natives: i32,
    pub libraries: i32,
    pub pubvars: i32,
    pub tags: i32,
    pub nametable: i32,
    pub overlays: i32,
}

/// Size of the on-disk header in bytes.
pub const HEADER_SIZE: usize = 60;

impl AmxHeader {
    /// Serialize all 18 fields, little-endian, in declaration order.
    pub fn encode_le(&self, out: &mut Vec<u8>) {
        let before = out.len();
        out.extend_from_slice(&self.size.to_le_bytes());
        out.extend_from_slice(&self.magic.to_le_bytes());
        out.push(self.file_version);
        out.push(self.amx_version);
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&self.defsize.to_le_bytes());
        for f in [self.cod, self.dat, self.hea, self.stp, self.cip, self.publics,
            self.natives, self.libraries, self.pubvars, self.tags, self.nametable,
            self.overlays] {
            out.extend_from_slice(&f.to_le_bytes());
        }
        assert_eq!(out.len() - before, HEADER_SIZE);
    }

    /// Parse from a byte slice; returns `None` if fewer than 60 bytes.
    pub fn decode_le(bytes: &[u8]) -> Option<AmxHeader> {
        fn read_i32(b: &[u8]) -> Option<i32> {
            Some(i32::from_le_bytes(b.try_into().ok()?))
        }
        if bytes.len() < HEADER_SIZE {
            return None;
        }
        Some(AmxHeader {
            size: read_i32(&bytes[0..4])?,
            magic: u16::from_le_bytes(bytes[4..6].try_into().ok()?),
            file_version: bytes[6],
            amx_version: bytes[7],
            flags: i16::from_le_bytes(bytes[8..10].try_into().ok()?),
            defsize: i16::from_le_bytes(bytes[10..12].try_into().ok()?),
            cod: read_i32(&bytes[12..16])?,
            dat: read_i32(&bytes[16..20])?,
            hea: read_i32(&bytes[20..24])?,
            stp: read_i32(&bytes[24..28])?,
            cip: read_i32(&bytes[28..32])?,
            publics: read_i32(&bytes[32..36])?,
            natives: read_i32(&bytes[36..40])?,
            libraries: read_i32(&bytes[40..44])?,
            pubvars: read_i32(&bytes[44..48])?,
            tags: read_i32(&bytes[48..52])?,
            nametable: read_i32(&bytes[52..56])?,
            overlays: read_i32(&bytes[56..60])?,
        })
    }
}

/// Header for a fresh 32-bit (SA-MP / open.mp compatible) AMX image with no
/// optional tables. `cod`/`dat`/`cip` expect a 60-byte header followed by the
/// code segment.
pub fn legacy32_header(code_bytes: usize, data_bytes: usize) -> AmxHeader {
    let cod = HEADER_SIZE as i32;
    let dat = HEADER_SIZE as i32 + code_bytes as i32;
    AmxHeader {
        size: (HEADER_SIZE + code_bytes + data_bytes) as i32,
        magic: AMX_MAGIC_32,
        file_version: 0, // legacy profile; bump per target profile
        amx_version: 0,
        flags: 0,
        defsize: DEF_SIZE_32,
        cod,
        dat,
        hea: dat,
        stp: dat + data_bytes as i32,
        cip: cod,
        // 0 = absent tables
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let h = legacy32_header(100, 16);
        let mut buf = Vec::new();
        h.encode_le(&mut buf);
        assert_eq!(buf.len(), HEADER_SIZE);
        let back = AmxHeader::decode_le(&buf).unwrap();
        assert_eq!(h, back);
    }

    #[test]
    fn legacy_layout_offsets() {
        let h = legacy32_header(100, 16);
        assert_eq!(h.magic, 0xf1e0);
        assert_eq!(h.cod, 60);
        assert_eq!(h.dat, 160);
        assert_eq!(h.hea, 160);
        assert_eq!(h.stp, 176);
        assert_eq!(h.cip, 60);
        assert_eq!(h.size, 60 + 100 + 16);
        assert_eq!(h.defsize, 8);
    }

    #[test]
    fn decode_short_input() {
        assert!(AmxHeader::decode_le(&[0u8; 59]).is_none());
    }
}
