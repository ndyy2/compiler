# RustPawn Compiler (rpc)

Pawn → AMX compiler written from scratch in Rust. Targets: SA-MP and open.mp.
No `pawncc` dependency at compile time.

Status: **Phase 0/1 scaffold** — workspace + AMX instruction table + header codec.

## Workspace

| Crate | Role |
| ----- | ---- |
| `rpc-cli` | `rpc` binary (clap) |
| `rpc-core` | compiler API: session, options, pipeline |
| `rpc-source` | files, spans, source manager |
| `rpc-diagnostics` | diagnostics engine + renderer |
| `rpc-preprocessor` | macros, includes, conditionals |
| `rpc-lexer` / `rpc-parser` / `rpc-ast` | frontend |
| `rpc-hir` / `rpc-semantic` | symbol resolution, Pawn types/tags |
| `rpc-mir` | CFG, basic blocks, SSA-like values |
| `rpc-optimizer` | pass manager, O0–O3/Oz |
| `rpc-amx` | opcodes, encoding, header, tables, writer |
| `rpc-backend` | MIR → AMX lowering, stack layout, emit |
| `rpc-target` | `samp-legacy`, `openmp-legacy` profiles |
| `rpc-debug` | debug info, source mapping |
| `rpc-driver` | wires frontend → backend |
| `rpc-test` | fixtures, differential testing |

## Spec references (behavioral, not copied code)

- `openmultiplayer/compiler` `source/compiler/sc6.c` — opcode table + emit rules (see `../novalife-src`).
- `compuphase/pawn` `amx/amx.h` — AMX header layout, file versions 0–11.
- [Pawn language docs](https://www.compuphase.com/pawn/pawn.htm).

## Build

```sh
cargo build
cargo test -p rpc-amx
```

## License

Apache-2.0. Independent implementation; spec references documented per file.
