//! AMX image writer: assembles header + tables + name table + code + data.
//!
//! Reproduces the on-disk layout of `pawncc 3.10.x` (uncompressed,
//! `defsize=8` name-table format), verified against `pawndisasm` and executed
//! by the reference `pawnruns` runtime. Section order (from the reference
//! writer in `sc6.c`):
//!
//! ```text
//! header(56) | publics | natives | libraries | pubvars | tags | nametable | code | data
//! ```
//!
//! Offsets in the header are file-absolute. Public `address` and call/jump
//! operands are **code-relative** (the runtime does `cip = code + operand`).

use crate::assembler::CodeBuilder;
use crate::header::AmxHeader;
use crate::instruction::FUNCSTUB_SIZE;

/// One symbol-table row. `addr` is code-relative for publics, data-relative
/// for pubvars, and 0 for natives/libraries/tags.
#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub name: String,
    pub addr: i32,
}

/// Everything the writer needs to produce an `.amx` image.
pub struct Image {
    pub publics: Vec<Entry>,
    pub natives: Vec<Entry>,
    pub libraries: Vec<Entry>,
    pub pubvars: Vec<Entry>,
    pub tags: Vec<Entry>,
    /// Code segment bytes (uncompressed: opcode + operand cells, 4-byte LE).
    pub code: Vec<u8>,
    /// Data segment: global initial values, 4 bytes per cell.
    pub data: Vec<u8>,
    /// Stack/heap size in bytes (default 16384 = 4096 cells).
    pub stksize: usize,
    /// Extra header flag bits. Keep COMPACT (0x04) clear for plain encoding.
    pub flags: i16,
    /// File/amx version (default 8/8: JIT-compatible legacy profile).
    pub file_version: u8,
    pub amx_version: u8,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            publics: Vec::new(),
            natives: Vec::new(),
            libraries: Vec::new(),
            pubvars: Vec::new(),
            tags: Vec::new(),
            code: Vec::new(),
            data: Vec::new(),
            stksize: 16384,
            flags: 0,
            file_version: 8,
            amx_version: 8,
        }
    }
}

const HEADER_BYTES: usize = 56;
const DATA_ALIGN: usize = 4;

/// Serialize the image to bytes (the full `.amx` file).
pub fn write_image(img: &Image) -> Vec<u8> {
    let stub = FUNCSTUB_SIZE as usize;
    let npub = img.publics.len();
    let nnat = img.natives.len();
    let nlib = img.libraries.len();
    let npubv = img.pubvars.len();
    let ntag = img.tags.len();

    // ---- name table: u16 maxlen + each unique name, NUL-terminated ----
    let mut names: Vec<&str> = Vec::new();
    for t in [&img.publics, &img.natives, &img.libraries, &img.pubvars, &img.tags] {
        for e in t {
            if !names.iter().any(|n| *n == e.name) {
                names.push(&e.name);
            }
        }
    }
    let maxlen = names.iter().map(|n| n.len()).max().unwrap_or(0) as u16;
    let mut names_bytes: Vec<u8> = Vec::new();
    for n in &names {
        names_bytes.extend_from_slice(n.as_bytes());
        names_bytes.push(0);
    }
    let nametable_size = 2 + names_bytes.len();

    // ---- section offsets (file-absolute) ----
    let publics = HEADER_BYTES;
    let natives = publics + npub * stub;
    let libraries = natives + nnat * stub;
    let pubvars = libraries + nlib * stub;
    let tags = pubvars + npubv * stub;
    let nametable = tags + ntag * stub;
    let pad = (DATA_ALIGN - (HEADER_BYTES + nametable_size) % DATA_ALIGN) % DATA_ALIGN;
    let cod = nametable + nametable_size + pad;
    let dat = cod + img.code.len();
    let hea = dat + img.data.len();
    let stp = hea + img.stksize;

    // name file offsets: names live at nametable+2, in `names` order
    let mut name_off = nametable + 2;
    let mut name_file_off: Vec<usize> = Vec::with_capacity(names.len());
    for n in &names {
        name_file_off.push(name_off);
        name_off += n.len() + 1;
    }
    let off_of = |name: &str| name_file_off[names.iter().position(|n| *n == name).unwrap()];

    // ---- header ----
    let hdr = AmxHeader {
        size: hea as i32,
        magic: 0xf1e0,
        file_version: img.file_version,
        amx_version: img.amx_version,
        flags: img.flags,
        defsize: FUNCSTUB_SIZE as i16,
        cod: cod as i32,
        dat: dat as i32,
        hea: hea as i32,
        stp: stp as i32,
        // cip: code-relative offset of the entry function (publics[0]); 0 = halt
        cip: img.publics.first().map(|p| p.addr).unwrap_or(0),
        publics: publics as i32,
        natives: natives as i32,
        libraries: libraries as i32,
        pubvars: pubvars as i32,
        tags: tags as i32,
        nametable: nametable as i32,
    };
    let mut out: Vec<u8> = Vec::with_capacity(hea);
    hdr.encode_le(&mut out);

    // ---- tables + name table + code (zero the whole region, patch in place) ----
    out.resize(cod, 0);
    let write_table = |out: &mut Vec<u8>, base: usize, rows: &[Entry]| {
        for (i, e) in rows.iter().enumerate() {
            let at = base + i * stub;
            out[at..at + 4].copy_from_slice(&e.addr.to_le_bytes());
            out[at + 4..at + 8].copy_from_slice(&(off_of(&e.name) as u32).to_le_bytes());
        }
    };
    write_table(&mut out, publics, &img.publics);
    write_table(&mut out, natives, &img.natives);
    write_table(&mut out, libraries, &img.libraries);
    write_table(&mut out, pubvars, &img.pubvars);
    write_table(&mut out, tags, &img.tags);

    // ---- name table ----
    out[nametable..nametable + 2].copy_from_slice(&maxlen.to_le_bytes());
    out[nametable + 2..nametable + 2 + names_bytes.len()].copy_from_slice(&names_bytes);

    // ---- code, then data (globals) ----
    out.extend_from_slice(&img.code);
    out.extend_from_slice(&img.data);

    assert_eq!(out.len(), hea, "image size mismatch");
    out
}

/// Convenience: build the code segment through the assembler and return bytes.
pub fn build_code<F: FnOnce(&mut CodeBuilder)>(emit: F) -> Vec<u8> {
    let mut b = CodeBuilder::new();
    emit(&mut b);
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_layout_matches_reference_offsets() {
        // 1 public "main" at code offset 0, no natives, 1 global init cell.
        let img = Image {
            publics: vec![Entry {
                name: "main".to_string(),
                addr: 0,
            }],
            data: vec![1, 0, 0, 0],
            code: vec![120, 0, 0, 0, 0, 0, 0, 0], // halt 0
            ..Default::default()
        };
        let bytes = write_image(&img);
        let h = crate::header::AmxHeader::decode_le(&bytes).unwrap();
        assert_eq!(h.publics, 56);
        assert_eq!(h.natives, 64);
        assert_eq!(h.nametable, 64);
        assert_eq!(h.cod, 72);
        assert_eq!(h.dat, 80);
        assert_eq!(h.hea, 84);
        assert_eq!(h.size, 84);
        assert_eq!(bytes.len(), 84);
        // public entry: addr=0, nameofs=66
        let (addr, nameofs) = (
            i32::from_le_bytes(bytes[56..60].try_into().unwrap()),
            i32::from_le_bytes(bytes[60..64].try_into().unwrap()),
        );
        assert_eq!(addr, 0);
        assert_eq!(nameofs, 66);
        assert_eq!(&bytes[66..70], b"main");
        // data cell
        assert_eq!(&bytes[80..84], &[1, 0, 0, 0]);
    }
}
