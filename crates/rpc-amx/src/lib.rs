//! AMX binary format: instruction definitions, encoding, header, tables, writer.
//!
//! Reference (behavioral, not copied code): openmultiplayer/compiler sc6.c,
//! compuphase/pawn amx.h.

pub mod assembler;
pub mod header;
pub mod instruction;
pub mod writer;
