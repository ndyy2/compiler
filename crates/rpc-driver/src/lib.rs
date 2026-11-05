//! Compilation driver: wires frontend through backend to `.amx` bytes.

use rpc_amx::writer::{write_image, Entry, Image};
use rpc_backend::Output;

/// Read a Pawn source file, compile it, and return the `.amx` image bytes.
pub fn compile_file(path: &str) -> Result<Vec<u8>, String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {path}: {e}"))?;
    compile_source(&src)
}

/// Compile a Pawn source string to `.amx` image bytes.
pub fn compile_source(src: &str) -> Result<Vec<u8>, String> {
    let prog = rpc_parser::parse(src).map_err(|e| e.to_string())?;
    let out: Output = rpc_backend::compile(&prog).map_err(|e| e.to_string())?;

    // Publics with `main` first (matches the reference: cip = publics[0]).
    let mut publics: Vec<Entry> = out
        .publics
        .into_iter()
        .map(|(name, addr)| Entry { name, addr })
        .collect();
    if let Some(i) = publics.iter().position(|e| e.name == "main") {
        if i != 0 {
            let e = publics.remove(i);
            publics.insert(0, e);
        }
    }

    // Natives: addr 0, id = row index (the runtime/host looks up by name).
    let natives: Vec<Entry> = prog
        .natives
        .iter()
        .map(|name| Entry { name: name.clone(), addr: 0 })
        .collect();

    let data = out.data.iter().flat_map(|v| v.to_le_bytes()).collect();

    let img = Image {
        publics,
        natives,
        data,
        code: out.code,
        ..Default::default()
    };
    Ok(write_image(&img))
}
