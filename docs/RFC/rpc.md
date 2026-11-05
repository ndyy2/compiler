# RPC — RustPawn Compiler

Pawn → AMX compiler in Rust, from scratch (no `pawncc` code dependency).
Targets: SA-MP (`samp-legacy`) and open.mp (`openmp-legacy`), 32-bit cells,
uncompressed encoding.

## Scope

- CLI: `rpc <file.p> [-o <out.amx>]`
- Pipeline: lexer → preprocessor → parser → AST → semantic → MIR → optimizer → AMX
- Differential testing against `pawncc 3.10.8` / `3.10.11` via `pawnruns`
- Compatibility before optimization

## Workspace (18 crates)

Active (MVP pipeline):

```
rpc-lexer → rpc-parser → rpc-ast → rpc-backend → rpc-amx → rpc-driver → rpc-cli
                                                              ↑
                                                      rpc-test (differential harness)
```

Stubs (planned): `rpc-core`, `rpc-source`, `rpc-diagnostics`,
`rpc-preprocessor`, `rpc-hir`, `rpc-semantic`, `rpc-mir`, `rpc-optimizer`,
`rpc-target`, `rpc-debug`.

## AMX format (legacy profile, verified)

- file_version 8 / amx_version 8, magic `0xf1e0` (32-bit cell), `defsize 8`
- **56-byte header, 17 fields, NO `overlays`** (post-file_version-9 field absent)
- uncompressed: every field — opcode *and* operand — is a 4-byte LE cell
- section order: `header(56) | publics | natives | libraries | pubvars |
  tags | nametable | code | data`; `cod` 4-aligned
- `size` = `hea` (uncompressed); `stp = hea + 16384`
- publics `address` and call/jump operands are **code-relative**; `nameofs`
  file-absolute
- code starts `halt 0` (8 bytes); `cip = publics[0].address` (`main` first)
- plain globals are NOT in the pubvars table; global initializers are
  constant-folded (floored div/mod)

## Calling convention (verified against `pawncc -O0 -d0 -C-` disasm)

- `call` pushes return address; `proc` pushes old `frm`, `frm = stk`
- frame layout (cell offsets from `frm`):
  - `+0` old frm, `+4` return address, `+8` arg count (4·n)
  - `+12+4i` param i (source order)
  - `-4(i+1)` local i (declaration order; `for`-init vars included — all
    locals allocated in the prologue)
- binary op: `lhs; push.pri; rhs; pop.alt` → `pri=rhs, alt=lhs`
  - `add/smul/and/or/xor/eq/neq` direct
  - `sub` = `sub.alt`; `div` = `sdiv.alt`; `mod` = `sdiv.alt; move.pri`
    (quotient → pri, remainder → alt; floored division, matching remainder)
  - `shl/sshr` and ordered comparisons (`sless/sleq/sgrtr/sgeq`) read
    pri⋅alt → `xchg` first
- call site (user function): args **reverse** source order, each
  `<expr>; push.pri`; `push.c 4n`; `call <off>`. Callee `retn` pops
  args+count. Result in pri.
- native call: per arg (source order, then reverse-pushed like user calls):
  `<expr>; heap 4; stor.i; move.pri; push.pri` (value → heap cell, cell
  **address** pushed); `push.c 4n; sysreq.c <native_id>; stack 4(n+1);
  heap -4n`. Natives receive `params[i]` = address; value natives deref.
  Native id = index in natives table (host resolves by name).
- epilogue inlined at each `return` + fallthrough: `stack 4·nloc` (only if
  nloc > 0) + `retn`; bare/fallthrough return = `zero.pri` first.
- locals must be declared at the top of a compound block (Pawn 3.10 error
  003) — MVP: function-level `new` only.

## Raw opcodes used (verified against runtime dispatch table, `amx.c`)

`load.pri` 1 · `load.s.pri` 3 · `const.pri` 11 · `const.alt` 12 ·
`stor.pri` 15 · `stor.s.pri` 17 · `stor.i` 23 · `move.pri` 33 · `xchg` 35 ·
`push.pri` 36 · `push.c` 39 · `pop.alt` 43 · `stack` 44 · `heap` 45 ·
`proc` 46 · `retn` 48 · `call` 49 · `jump` 51 · `jzer` 53 · `shl` 65 ·
`sshr` 67 · `smul` 72 · `sdiv.alt` 74 · `add` 78 · `sub.alt` 80 · `and` 81 ·
`or` 82 · `xor` 83 · `not` 84 · `neg` 85 · `zero.pri` 89 · `eq` 95 · `neq`
96 · `sless` 101 · `sleq` 102 · `sgrtr` 103 · `sgeq` 104 · `sysreq.c` 123 ·
`halt` 120

## Current status (MVP)

Done, differential-verified vs `pawncc 3.10.11` + `pawnruns`:

- integer arithmetic: `+ - * / %` (floored), bitwise `& | ^`, shifts `<< >>`
- comparisons `== != < <= > >=` (signed), `!` unary, `-` negation
- control flow: `if/else`, `while`, `for`
- calls: user functions (forward refs via labels), native trampoline
- globals (const-folded init), locals, multi-arg calls
- `.amx` writer: 56B header, tables, nametable, code, data

Fixture returns match reference exactly: `d`=42, `cf`=114, `cc`=7, `p1`=10,
`cp`=-4, `pp`=321410, `fl`=30, `nn`=0, `pv`=3.

## Roadmap

1. Preprocessor: `#define`/`#if`/`#include`, macros
2. Tags (`int`/`float`/`string` typed where it matters), `stock`
3. Float arithmetic (FPU opcodes, file_version 9+ layout)
4. Strings, arrays, `switch`
5. Semantic pass (type checking, forward declarations) via `rpc-semantic`
6. Optimizer (`-O1..-O3`) in `rpc-optimizer`, kept off by default for
   compatibility runs
7. Targets: `samp-legacy` vs `openmp-legacy` codegen flags in `rpc-target`
8. Diagnostics with source spans (`rpc-diagnostics`)
