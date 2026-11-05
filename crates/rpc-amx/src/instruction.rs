//! AMX instruction set — source of truth.
//!
//! Reference (behavioral, not copied code):
//! - `openmultiplayer/compiler` `source/compiler/sc6.c` (`opcodelist`, emit
//!   functions `parm0..parm5`, `do_call`, `do_jump`).
//! - `compuphase/pawn` `amx/amx.h` (file version semantics, debug opcodes).
//!
//! Raw values are the exact byte encoded in the code segment.

/// Static metadata for one opcode, indexed by raw opcode value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpInfo {
    pub name: &'static str,
    /// Number of cell operands after the opcode byte.
    pub operands: u8,
    /// Lowest AMX file version that contains this opcode (0 = original).
    pub min_version: u8,
    /// True for debug-interface opcodes (not part of normal code emission).
    pub debug: bool,
}

/// One entry per raw opcode value, 0..=157.
///
/// `None` = reserved/invalid (raw 0 is the "invalid instruction" node).
pub const OPCODES: &[Option<OpInfo>] = &[
    Some(OpInfo { name: "invalid", operands: 0, min_version: 0, debug: true }),
    Some(OpInfo { name: "load.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "load.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "load.s.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "load.s.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "lref.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "lref.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "lref.s.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "lref.s.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "load.i", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "lodb.i", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "const.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "const.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "addr.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "addr.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "stor.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "stor.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "stor.s.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "stor.s.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "sref.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "sref.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "sref.s.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "sref.s.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "stor.i", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "strb.i", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "lidx", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "lidx.b", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "idxaddr", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "idxaddr.b", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "align.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "align.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "lctrl", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "sctrl", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "move.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "move.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "xchg", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "push.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "push.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "push.r", operands: 1, min_version: 0, debug: false }), // obsolete, never generated
    Some(OpInfo { name: "push.c", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "push", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "push.s", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "pop.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "pop.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "stack", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "heap", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "proc", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "ret", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "retn", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "call", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "call.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "jump", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jrel", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jzer", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jnz", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jeq", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jneq", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jless", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jleq", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jgrtr", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jgeq", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jsless", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jsleq", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jsgrtr", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "jsgeq", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "shl", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "shr", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sshr", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "shl.c.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "shl.c.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "shr.c.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "shr.c.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "smul", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sdiv", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sdiv.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "umul", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "udiv", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "udiv.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "add", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sub", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sub.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "and", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "or", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "xor", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "not", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "neg", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "invert", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "add.c", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "smul.c", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "zero.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "zero.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "zero", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "zero.s", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "sign.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sign.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "eq", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "neq", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "less", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "leq", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "grtr", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "geq", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sless", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sleq", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sgrtr", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sgeq", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "eq.c.pri", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "eq.c.alt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "inc.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "inc.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "inc", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "inc.s", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "inc.i", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "dec.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "dec.alt", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "dec", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "dec.s", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "dec.i", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "movs", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "cmps", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "fill", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "halt", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "bounds", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "sysreq.pri", operands: 0, min_version: 0, debug: false }),
    Some(OpInfo { name: "sysreq.c", operands: 1, min_version: 0, debug: false }),
    Some(OpInfo { name: "file", operands: 0, min_version: 0, debug: true }),
    Some(OpInfo { name: "line", operands: 2, min_version: 0, debug: true }),
    Some(OpInfo { name: "symbol", operands: 3, min_version: 0, debug: true }),
    Some(OpInfo { name: "srange", operands: 2, min_version: 1, debug: true }),
    Some(OpInfo { name: "jump.pri", operands: 0, min_version: 1, debug: false }),
    Some(OpInfo { name: "switch", operands: 1, min_version: 1, debug: false }),
    Some(OpInfo { name: "casetbl", operands: 2, min_version: 1, debug: false }),
    Some(OpInfo { name: "swap.pri", operands: 0, min_version: 4, debug: false }),
    Some(OpInfo { name: "swap.alt", operands: 0, min_version: 4, debug: false }),
    Some(OpInfo { name: "push.adr", operands: 1, min_version: 4, debug: false }),
    Some(OpInfo { name: "nop", operands: 0, min_version: 6, debug: false }),
    Some(OpInfo { name: "sysreq.n", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "symtag", operands: 1, min_version: 7, debug: true }),
    Some(OpInfo { name: "break", operands: 0, min_version: 8, debug: false }),
    Some(OpInfo { name: "push2.c", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "push2", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "push2.s", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "push2.adr", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "push3.c", operands: 3, min_version: 9, debug: false }),
    Some(OpInfo { name: "push3", operands: 3, min_version: 9, debug: false }),
    Some(OpInfo { name: "push3.s", operands: 3, min_version: 9, debug: false }),
    Some(OpInfo { name: "push3.adr", operands: 3, min_version: 9, debug: false }),
    Some(OpInfo { name: "push4.c", operands: 4, min_version: 9, debug: false }),
    Some(OpInfo { name: "push4", operands: 4, min_version: 9, debug: false }),
    Some(OpInfo { name: "push4.s", operands: 4, min_version: 9, debug: false }),
    Some(OpInfo { name: "push4.adr", operands: 4, min_version: 9, debug: false }),
    Some(OpInfo { name: "push5.c", operands: 5, min_version: 9, debug: false }),
    Some(OpInfo { name: "push5", operands: 5, min_version: 9, debug: false }),
    Some(OpInfo { name: "push5.s", operands: 5, min_version: 9, debug: false }),
    Some(OpInfo { name: "push5.adr", operands: 5, min_version: 9, debug: false }),
    Some(OpInfo { name: "load.both", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "load.s.both", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "const", operands: 2, min_version: 9, debug: false }),
    Some(OpInfo { name: "const.s", operands: 2, min_version: 9, debug: false }),
];

/// Look up opcode metadata by raw value. `None` for values above 157.
pub fn opinfo(raw: u8) -> Option<OpInfo> {
    OPCODES.get(raw as usize).and_then(|o| *o)
}

/// Highest file version this opcode table describes (per amx.h CUR_FILE_VERSION).
pub const MAX_FILE_VERSION: u8 = 11;

/// Size of a symbol-table definition record (`AMX_FUNCSTUBNT`: address +
/// nameofs), 32-bit cells.
pub const FUNCSTUB_SIZE: u16 = 8;

/// AMX magic numbers by cell size (amx.h).
pub const AMX_MAGIC_16: u16 = 0xf1e2;
pub const AMX_MAGIC_32: u16 = 0xf1e0;
pub const AMX_MAGIC_64: u16 = 0xf1e1;

/// Header flag bits — 3.10.x `amx.h` values (NOTE: bit positions differ from
/// the newer compuphase main branch; this is the legacy target's layout).
pub mod flag {
    pub const DEBUG: i16 = 0x02; // symbolic info available
    pub const COMPACT: i16 = 0x04; // compact (7-bit) cell encoding
    pub const SLEEP: i16 = 0x08; // script uses the sleep instruction
    pub const NOCHECKS: i16 = 0x10; // no bounds checks; no BREAK opcodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_full_opcode_space() {
        assert_eq!(OPCODES.len(), 158);
        // every raw value 1..=157 must resolve to a named opcode
        for raw in 1..=157u16 {
            let info = opinfo(raw as u8).expect("opcode slot missing");
            assert!(!info.name.is_empty());
        }
        assert!(opinfo(158).is_none());
    }

    #[test]
    fn spot_check_names_raw_and_operands() {
        assert_eq!(opinfo(78).unwrap().name, "add");
        assert_eq!(opinfo(78).unwrap().operands, 0);
        assert_eq!(opinfo(120).unwrap().name, "halt");
        assert_eq!(opinfo(49).unwrap().name, "call");
        assert_eq!(opinfo(49).unwrap().operands, 1);
        assert_eq!(opinfo(157).unwrap().name, "const.s");
        assert_eq!(opinfo(157).unwrap().operands, 2);
        assert_eq!(opinfo(157).unwrap().min_version, 9);
        assert_eq!(opinfo(134).unwrap().name, "nop");
        assert_eq!(opinfo(134).unwrap().min_version, 6);
    }

    #[test]
    fn version_gates() {
        assert_eq!(opinfo(128).unwrap().min_version, 1); // jump.pri
        assert_eq!(opinfo(129).unwrap().min_version, 1); // switch
        assert_eq!(opinfo(130).unwrap().min_version, 1); // casetbl
        assert_eq!(opinfo(133).unwrap().min_version, 4); // push.adr
        assert_eq!(opinfo(135).unwrap().min_version, 9); // sysreq.n
        assert_eq!(opinfo(137).unwrap().min_version, 8); // break
    }

    #[test]
    fn debug_opcodes_flagged() {
        for raw in [124, 125, 126, 127, 136] {
            assert!(opinfo(raw as u8).unwrap().debug, "raw {raw} should be debug");
        }
        assert!(!opinfo(78).unwrap().debug);
    }
}
