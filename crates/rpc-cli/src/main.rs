//! `rpc` — RustPawn compiler CLI (MVP).
//!
//! Usage: `rpc <file.p> [-o <out.amx>]`

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut input, mut output) = (None, None);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "-h" | "--help" => {
                println!("rpc — Pawn compiler (MVP integer subset)\n\nusage: rpc <file.p> [-o <out.amx>]");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("rpc: unknown option {other}");
                return ExitCode::from(2);
            }
            other => input = Some(other.to_string()),
        }
        i += 1;
    }

    let Some(input) = input else {
        eprintln!("rpc: no input file\nusage: rpc <file.p> [-o <out.amx>]");
        return ExitCode::from(2);
    };

    let out_name = match output {
        Some(o) => o,
        None => {
            let stem = Path::new(&input)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| input.clone());
            format!("{stem}.amx")
        }
    };

    match rpc_driver::compile_file(&input) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(&out_name, &bytes) {
                eprintln!("rpc: cannot write {out_name}: {e}");
                return ExitCode::FAILURE;
            }
            println!("{out_name} ({} bytes)", bytes.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rpc: {e}");
            ExitCode::from(1)
        }
    }
}
