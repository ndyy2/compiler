//! Differential test helpers: compile with `rpc` and with the reference
//! `pawncc`, run both with `pawnruns`, compare the returned value.
//!
//! Reference toolchain location: env `RPC_REF_DIR`
//! (default `/home/andy/lab/novalife-src/source/compiler/build`).
//! Tests are skipped when the toolchain is absent.

use std::path::Path;
use std::process::Command;

/// Default reference build directory.
pub const DEFAULT_REF_DIR: &str = "/home/andy/lab/novalife-src/source/compiler/build";

pub fn ref_dir() -> String {
    std::env::var("RPC_REF_DIR").unwrap_or_else(|_| DEFAULT_REF_DIR.to_string())
}

pub fn have_reference() -> bool {
    let d = ref_dir();
    Path::new(&d).join("pawncc").exists() && Path::new(&d).join("pawnruns").exists()
}

/// Run one `.amx` with the reference runtime; returns the printed return
/// value, or `None` if the run failed.
pub fn run_amx(amx: &str) -> Option<i32> {
    let out = Command::new(Path::new(&ref_dir()).join("pawnruns"))
        .arg(amx)
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    // format: "<file>.amx returns <n>"
    s.lines()
        .find(|l| l.contains("returns"))
        .and_then(|l| l.rsplit("returns").next()?.trim().parse().ok())
}

/// Write `src` to `dir/<stem>.p` and compile with reference `pawncc`.
/// Options: `-O0 -d0 -C-` (unoptimized, no checks, uncompressed).
/// `pawncc` writes output to its CWD, so the `-o` path must be absolute.
pub fn ref_compile(dir: &str, stem: &str, src: &str) -> bool {
    let src_path = Path::new(dir).join(format!("{stem}.p"));
    if std::fs::write(&src_path, src).is_err() {
        return false;
    }
    let out_path = format!("{dir}/{stem}");
    match Command::new(Path::new(&ref_dir()).join("pawncc"))
        .arg(&src_path)
        .arg(format!("-o{out_path}"))
        .arg("-O0")
        .arg("-d0")
        .arg("-C-")
        .output()
    {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// Differential check: rpc output and reference output must return the same
/// value. Returns `Err(message)` on mismatch, `Ok(())` when equal (and
/// `Ok(())` when no reference toolchain is available — skip).
pub fn differential(src: &str) -> Result<(), String> {
    if !have_reference() {
        return Ok(());
    }
    let stem = {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let i = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("difftest{}_{}", std::process::id(), i)
    };
    let tmp = std::env::temp_dir().join("rpc-diff");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let dir = tmp.to_string_lossy().into_owned();

    // reference compile + run
    if !ref_compile(&dir, &stem, src) {
        return Err("reference pawncc failed to compile fixture".into());
    }
    let ref_ret = run_amx(&Path::new(&dir).join(format!("{stem}.amx")).to_string_lossy())
        .ok_or_else(|| "reference runtime failed".to_string())?;

    // rpc compile + run
    let bytes = rpc_driver::compile_source(src)
        .map_err(|e| format!("rpc failed to compile: {e}"))?;
    let rpc_path = Path::new(&dir).join(format!("{stem}_rpc.amx"));
    std::fs::write(&rpc_path, &bytes).map_err(|e| e.to_string())?;
    let rpc_ret = run_amx(&rpc_path.to_string_lossy())
        .ok_or_else(|| "rpc runtime failed".to_string())?;

    if rpc_ret != ref_ret {
        return Err(format!("return mismatch: rpc={rpc_ret} ref={ref_ret}"));
    }
    Ok(())
}
