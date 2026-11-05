//! AMX assembler: instruction emission with label/branch resolution.
//!
//! Uncompressed (default) encoding: every field — opcode *and* each operand —
//! is a full 32-bit little-endian cell (verified byte-for-byte against
//! `pawncc 3.10.11 -C-`). Branch/call targets are absolute byte offsets from
//! the start of the code segment (the runtime does `cip = code + operand`).

use crate::instruction::opinfo;

/// A single emitted instruction before label resolution.
struct RawInstr {
    op: u8,
    operands: Vec<Operand>,
}

#[derive(Clone, Copy)]
enum Operand {
    Imm(i32),
    /// index into the label slot table; resolved to a code offset at finish
    Label(usize),
}

/// A label: bound to an instruction index once `label()` is called.
struct LabelSlot {
    name: String,
    /// instruction index the label points at (set by `label`)
    instr: Option<usize>,
}

/// Builder that emits code cells and resolves branch labels at `finish()`.
pub struct CodeBuilder {
    raw: Vec<RawInstr>,
    labels: Vec<LabelSlot>,
    /// code offset (bytes) at which each raw instruction starts
    instr_offset: Vec<usize>,
}

impl Default for CodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeBuilder {
    pub fn new() -> Self {
        Self {
            raw: Vec::new(),
            labels: Vec::new(),
            instr_offset: Vec::new(),
        }
    }

    /// Size in bytes of one instruction with `n` operands (uncompressed).
    fn instr_size(nops: usize) -> usize {
        (1 + nops) * 4
    }

    fn label_slot(&mut self, name: &str) -> usize {
        if let Some(i) = self.labels.iter().position(|s| s.name == name) {
            i
        } else {
            self.labels.push(LabelSlot {
                name: name.to_string(),
                instr: None,
            });
            self.labels.len() - 1
        }
    }

    /// Emit an instruction. `op` must be a valid raw opcode; operand count is
    /// cross-checked against the opcode table.
    pub fn emit(&mut self, op: u8, operands: &[i32]) -> &mut Self {
        let info = opinfo(op).expect("unknown opcode");
        assert_eq!(
            operands.len() as u8,
            info.operands,
            "opcode {:?} expects {} operand(s), got {}",
            info.name,
            info.operands,
            operands.len()
        );
        self.raw.push(RawInstr {
            op,
            operands: operands.iter().map(|&v| Operand::Imm(v)).collect(),
        });
        self
    }

    /// Emit an instruction whose single operand is the code offset of `label`.
    pub fn emit_label(&mut self, op: u8, label: &str) -> &mut Self {
        let info = opinfo(op).expect("unknown opcode");
        assert_eq!(info.operands, 1, "opcode {:?} takes 1 operand", info.name);
        let li = self.label_slot(label);
        self.raw.push(RawInstr {
            op,
            operands: vec![Operand::Label(li)],
        });
        self
    }

    /// Bind a label to the *next* instruction to be emitted.
    pub fn label(&mut self, name: &str) -> &mut Self {
        let li = self.label_slot(name);
        self.labels[li].instr = Some(self.raw.len());
        self
    }

    /// Number of emitted instructions (pre-resolution).
    pub fn instr_count(&self) -> usize {
        self.raw.len()
    }

    /// Byte offset (in the final code segment) of the next instruction to
    /// be emitted.
    pub fn code_offset(&self) -> usize {
        self.raw.iter().map(|r| (1 + r.operands.len()) * 4).sum()
    }

    /// Lay out offsets, resolve labels, and emit the final code segment bytes.
    pub fn finish(mut self) -> Vec<u8> {
        let mut off: usize = 0;
        for r in &self.raw {
            self.instr_offset.push(off);
            off += Self::instr_size(r.operands.len());
        }

        let label_val: Vec<i32> = self
            .labels
            .iter()
            .map(|s| {
                let i = s
                    .instr
                    .unwrap_or_else(|| panic!("label {:?} never bound", s.name));
                self.instr_offset[i] as i32
            })
            .collect();

        let mut out: Vec<u8> = Vec::with_capacity(off);
        for r in &self.raw {
            out.extend_from_slice(&(r.op as u32).to_le_bytes());
            for op in &r.operands {
                let v = match op {
                    Operand::Imm(v) => *v,
                    Operand::Label(i) => label_val[*i],
                };
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADD: u8 = 78;
    const CALL: u8 = 49;
    const CONST_PRI: u8 = 11;
    const JNZ: u8 = 54;
    const RET: u8 = 47;
    const HALT: u8 = 120;

    #[test]
    fn emits_plain_cells() {
        let mut b = CodeBuilder::new();
        b.emit(HALT, &[0]); // halt 0
        let code = b.finish();
        assert_eq!(code, vec![120, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn forward_label() {
        let mut b = CodeBuilder::new();
        b.emit(CONST_PRI, &[5]); // off 0
        b.emit_label(JNZ, "end"); // off 8, operand -> "end"
        b.label("end"); // binds to next instruction
        b.emit(CONST_PRI, &[1]); // off 16
        b.emit(RET, &[]); // off 24
        let code = b.finish();
        // instr sizes: 8 + 8 + 8 + 4 = 28
        // jnz: op 8..12, operand 12..16
        let end_off: u32 = 16; // code offset of the instruction after label("end")
        assert_eq!(
            code[12..16],
            end_off.to_le_bytes(),
            "jnz operand must be the code offset of 'end'"
        );
        assert_eq!(code.len(), 28);
    }

    #[test]
    #[should_panic(expected = "expects 0 operand")]
    fn operand_count_checked() {
        let mut b = CodeBuilder::new();
        b.emit(ADD, &[1]); // wrong: add takes 0
        let _ = b.finish();
    }
}
