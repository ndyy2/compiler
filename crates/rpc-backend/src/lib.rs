//! Backend: AST → AMX code segment (MVP: integer Pawn subset).
//!
//! Calling convention — verified byte-for-byte against `pawncc 3.10.11
//! -O0 -d0 -C-` disassembly (see docs/RFC/rpc.md §frame):
//!
//! - Code segment starts with `halt 0` (8 bytes); all call/jump/public
//!   `address` values are **code-relative** offsets.
//! - `call` pushes the return address; `proc` pushes `frm` and sets
//!   `frm = stk`. Frame layout (cell offsets from `frm`):
//!   `+0` old frm, `+4` return addr, `+8` arg count (4·n),
//!   `+12+4i` param i (source order), `-4(i+1)` local i (declaration order).
//! - All `new` declarations (function body, including `for`-loop inits) are
//!   allocated in the prologue: per local `stack -4` + init +
//!   `stor.s.pri -4(i+1)`.
//! - Binary ops: `<lhs>; push.pri; <rhs>; pop.alt` leaves pri=rhs, alt=lhs.
//!   `add/smul/and/or/xor` are commutative → emit directly. `sub.alt` =
//!   alt−pri. `sdiv.alt` = alt÷pri (quotient → pri, remainder → alt) with
//!   floor semantics (runtime `amx.c`). `shl/sshr` and ordered comparisons
//!   read pri⋅alt → `xchg` first.
//! - Call site: args in **reverse** source order, each `<expr>; push.pri`;
//!   then `push.c 4n` (0 for no args); `call <off>`. The callee's `retn`
//!   pops args+count — no caller cleanup. Result in pri.
//! - Epilogue (inlined at each `return` and at fallthrough): `stack 4·nloc`
//!   (only if nloc > 0) + `retn`. Fallthrough zeroes pri first.
//! - Globals: data segment, `load/stor.pri <4·index>`; initial values are
//!   constant expressions only.

use rpc_amx::assembler::CodeBuilder;
use rpc_ast::{BinOp, Expr, Function, Program, Stmt, UnOp};
use std::collections::HashMap;

#[derive(Debug)]
pub struct CompileError {
    pub msg: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for CompileError {}

/// Raw opcodes (values verified against the runtime dispatch table in
/// `source/amx/amx.c` of the reference).
mod op {
    pub const LOAD_PRI: u8 = 1;
    pub const LOAD_S_PRI: u8 = 3;
    pub const CONST_PRI: u8 = 11;
    pub const STOR_PRI: u8 = 15;
    pub const STOR_S_PRI: u8 = 17;
    pub const STOR_I: u8 = 23;
    pub const HEAP: u8 = 45;
    pub const SYSREQ_C: u8 = 123;
    pub const MOVE_PRI: u8 = 33;
    pub const XCHG: u8 = 35;
    pub const PUSH_PRI: u8 = 36;
    pub const PUSH_C: u8 = 39;
    pub const POP_ALT: u8 = 43;
    pub const STACK: u8 = 44;
    pub const PROC: u8 = 46;
    pub const RETN: u8 = 48;
    pub const CALL: u8 = 49;
    pub const JUMP: u8 = 51;
    pub const JZER: u8 = 53;
    pub const SHL: u8 = 65;
    pub const SSHR: u8 = 67;
    pub const SMUL: u8 = 72;
    pub const SDIV_ALT: u8 = 74;
    pub const ADD: u8 = 78;
    pub const SUB_ALT: u8 = 80;
    pub const AND: u8 = 81;
    pub const OR: u8 = 82;
    pub const XOR: u8 = 83;
    pub const NOT: u8 = 84;
    pub const NEG: u8 = 85;
    pub const ZERO_PRI: u8 = 89;
    pub const EQ: u8 = 95;
    pub const NEQ: u8 = 96;
    pub const SLESS: u8 = 101;
    pub const SLEQ: u8 = 102;
    pub const SGRTR: u8 = 103;
    pub const SGEQ: u8 = 104;
    pub const HALT: u8 = 120;
}

/// Everything the image writer needs from codegen.
pub struct Output {
    /// Code segment bytes (uncompressed; starts with `halt 0`).
    pub code: Vec<u8>,
    /// Public functions: (name, code-relative offset).
    pub publics: Vec<(String, i32)>,
    /// Code-relative offset of `main` (header `cip`); 0 if absent.
    pub cip: i32,
    /// Global initial values, one cell each, declaration order.
    pub data: Vec<i32>,
}

/// Compile a program to the AMX code segment + tables.
pub fn compile(prog: &Program) -> Result<Output, CompileError> {
    let mut g = Gen::new(prog)?;

    // Two passes: first record function offsets, then emit bodies. The
    // reference lays publics in source order; call targets are resolved at
    // assemble time via labels, so a single pass suffices — function offsets
    // are known before any call is emitted because we emit prologue+labels
    // ahead. (Calls only appear inside bodies, after their target's label
    // exists, so offsets can be computed with code_offset() at emit time.)
    g.builder.emit(op::HALT, &[0]); // preamble: halt 0

    for f in &prog.functions {
        g.func(f)?;
    }

    let code = g.builder.finish();
    let cip = g.func_off.get("main").copied().unwrap_or(0);
    let publics = prog
        .functions
        .iter()
        .filter(|f| f.public)
        .map(|f| (f.name.clone(), g.func_off[&f.name]))
        .collect();

    Ok(Output {
        code,
        publics,
        cip,
        data: g.globals_init.clone(),
    })
}

/// Global variable: name → data offset.
struct Gvar {
    off: i32,
}

struct Gen {
    builder: CodeBuilder,
    globals: HashMap<String, Gvar>,
    globals_init: Vec<i32>,
    func_off: HashMap<String, i32>,
    /// native name → id (index in the natives table)
    natives: HashMap<String, i32>,
}

/// Per-function environment: local name → frame offset (negative).
#[derive(Clone)]
struct Env {
    local: HashMap<String, i32>,
    nlocal: usize,
}

impl Env {
    fn resolve(&self, g: &Gen, name: &str) -> Result<(i32, bool), CompileError> {
        if let Some(off) = self.local.get(name) {
            return Ok((*off, false));
        }
        self.local0(g, name)
    }

    fn local0(&self, g: &Gen, name: &str) -> Result<(i32, bool), CompileError> {
        let v = g
            .globals
            .get(name)
            .ok_or_else(|| CompileError {
                msg: format!("undefined symbol \"{name}\""),
            })?
            .off;
        Ok((v, true))
    }
}

impl Gen {
    fn new(prog: &Program) -> Result<Self, CompileError> {
        let mut globals = HashMap::new();
        let mut globals_init = Vec::new();
        for gv in &prog.globals {
            let init = match &gv.init {
                Some(e) => const_eval(e)?,
                None => 0,
            };
            let off = (globals.len() * 4) as i32;
            globals.insert(gv.name.clone(), Gvar { off });
            globals_init.push(init);
        }
        let mut natives = HashMap::new();
        for (i, n) in prog.natives.iter().enumerate() {
            natives.insert(n.clone(), i as i32);
        }
        Ok(Self {
            builder: CodeBuilder::new(),
            globals,
            globals_init,
            func_off: HashMap::new(),
            natives,
        })
    }

    // ---- expressions (result in pri) ----
    fn expr(&mut self, e: &Expr, env: &Env) -> Result<(), CompileError> {
        match e {
            Expr::Int(v) => {
                self.builder.emit(op::CONST_PRI, &[*v]);
            }
            Expr::Ident(name) => {
                let (off, is_global) = env.resolve(&self, name)?;
                let o = if is_global { op::LOAD_PRI } else { op::LOAD_S_PRI };
                self.builder.emit(o, &[off]);
            }
            Expr::Binary { op, lhs, rhs } => {
                self.expr(lhs, env)?;
                self.builder.emit(op::PUSH_PRI, &[]);
                self.expr(rhs, env)?;
                self.builder.emit(op::POP_ALT, &[]);
                match op {
                    BinOp::Add => {
                        self.builder.emit(op::ADD, &[]);
                    }
                    BinOp::Sub => {
                        self.builder.emit(op::SUB_ALT, &[]);
                    }
                    BinOp::Mul => {
                        self.builder.emit(op::SMUL, &[]);
                    }
                    BinOp::Div => {
                        self.builder.emit(op::SDIV_ALT, &[]);
                    }
                    BinOp::Mod => {
                        self.builder.emit(op::SDIV_ALT, &[]);
                        self.builder.emit(op::MOVE_PRI, &[]);
                    }
                    BinOp::BitAnd => {
                        self.builder.emit(op::AND, &[]);
                    }
                    BinOp::BitOr => {
                        self.builder.emit(op::OR, &[]);
                    }
                    BinOp::BitXor => {
                        self.builder.emit(op::XOR, &[]);
                    }
                    BinOp::Shl => {
                        self.builder.emit(op::XCHG, &[]);
                        self.builder.emit(op::SHL, &[]);
                    }
                    BinOp::Shr => {
                        self.builder.emit(op::XCHG, &[]);
                        self.builder.emit(op::SSHR, &[]);
                    }
                    BinOp::Eq => {
                        self.builder.emit(op::EQ, &[]);
                    }
                    BinOp::Ne => {
                        self.builder.emit(op::NEQ, &[]);
                    }
                    // ordered comparisons read pri<alt → xchg first
                    BinOp::Lt => {
                        self.builder.emit(op::XCHG, &[]);
                        self.builder.emit(op::SLESS, &[]);
                    }
                    BinOp::Le => {
                        self.builder.emit(op::XCHG, &[]);
                        self.builder.emit(op::SLEQ, &[]);
                    }
                    BinOp::Gt => {
                        self.builder.emit(op::XCHG, &[]);
                        self.builder.emit(op::SGRTR, &[]);
                    }
                    BinOp::Ge => {
                        self.builder.emit(op::XCHG, &[]);
                        self.builder.emit(op::SGEQ, &[]);
                    }
                }
            }
            Expr::Unary { op, e } => {
                self.expr(e, env)?;
                let o = match op {
                    UnOp::Neg => op::NEG,
                    UnOp::Not => op::NOT,
                };
                self.builder.emit(o, &[]);
            }
            Expr::Assign { name, value } => {
                self.expr(value, env)?;
                let (off, is_global) = env.resolve(&self, name)?;
                let o = if is_global {
                    op::STOR_PRI
                } else {
                    op::STOR_S_PRI
                };
                self.builder.emit(o, &[off]);
                // pri keeps the stored value
            }
            Expr::Call { name, args } => {
                if let Some(id) = self.natives.get(name).copied() {
                    // native call: each arg value goes to a heap cell, the
                    // cell's ADDRESS is pushed (reverse source order). Native
                    // receives params[i] = address and dereferences. Verified
                    // against `pawncc` disasm of nn.p.
                    for a in args.iter().rev() {
                        self.expr(a, env)?;
                        self.builder.emit(op::HEAP, &[4]);
                        self.builder.emit(op::STOR_I, &[]);
                        self.builder.emit(op::MOVE_PRI, &[]);
                        self.builder.emit(op::PUSH_PRI, &[]);
                    }
                    self.builder.emit(op::PUSH_C, &[(4 * args.len()) as i32]);
                    self.builder.emit(op::SYSREQ_C, &[id]);
                    self.builder.emit(op::STACK, &[(4 * (args.len() + 1)) as i32]);
                    if !args.is_empty() {
                        self.builder.emit(op::HEAP, &[-4 * args.len() as i32]);
                    }
                    return Ok(());
                }
                for a in args.iter().rev() {
                    self.expr(a, env)?;
                    self.builder.emit(op::PUSH_PRI, &[]);
                }
                self.builder.emit(op::PUSH_C, &[(4 * args.len()) as i32]);
                let known = self.func_off.get(name).copied();
                if let Some(off) = known {
                    self.builder.emit(op::CALL, &[off]);
                } else {
                    // function defined later in the file: resolve at finish
                    self.builder.emit_label(op::CALL, &format!("@{name}"));
                }
            }
        }
        Ok(())
    }

    // ---- statements ----
    fn stmt(&mut self, s: &Stmt, env: &Env, fn_name: &str) -> Result<(), CompileError> {
        match s {
            Stmt::Decl { .. } => {
                // allocated in the prologue; nothing to emit here
            }
            Stmt::Assign { name, value } => {
                self.expr(
                    &Expr::Assign {
                        name: name.clone(),
                        value: Box::new(value.clone()),
                    },
                    env,
                )?;
            }
            Stmt::Expr(e) => self.expr(e, env)?,
            Stmt::Return(e) => {
                if let Some(v) = e {
                    self.expr(v, env)?;
                } else {
                    self.builder.emit(op::ZERO_PRI, &[]);
                }
                self.epilogue(env);
            }
            Stmt::If { cond, then, else_ } => {
                self.expr(cond, env)?;
                let n = self.builder.instr_count();
                let else_l = format!("_{fn_name}_e{n}");
                let end_l = format!("_{fn_name}_d{n}");
                self.builder.emit_label(op::JZER, &else_l);
                self.stmt(then, env, fn_name)?;
                if let Some(el) = else_ {
                    self.builder.emit_label(op::JUMP, &end_l);
                    self.builder.label(&else_l);
                    self.stmt(el, env, fn_name)?;
                    self.builder.label(&end_l);
                } else {
                    self.builder.label(&else_l);
                }
            }
            Stmt::While { cond, body } => {
                let n = self.builder.instr_count();
                let start = format!("_{fn_name}_w{n}");
                let end = format!("_{fn_name}_x{n}");
                self.builder.label(&start);
                self.expr(cond, env)?;
                self.builder.emit_label(op::JZER, &end);
                self.stmt(body, env, fn_name)?;
                self.builder.emit_label(op::JUMP, &start);
                self.builder.label(&end);
            }
            Stmt::For { init, cond, step, body } => {
                let n = self.builder.instr_count();
                let start = format!("_{fn_name}_f{n}");
                let end = format!("_{fn_name}_g{n}");
                // init (Decl → assign; Expr/Assign → expr)
                self.for_init(init, env)?;
                self.builder.label(&start);
                self.expr(cond, env)?;
                self.builder.emit_label(op::JZER, &end);
                self.stmt(body, env, fn_name)?;
                self.expr(step, env)?;
                self.builder.emit_label(op::JUMP, &start);
                self.builder.label(&end);
            }
            Stmt::Block(inner) => {
                for s in inner {
                    self.stmt(s, env, fn_name)?;
                }
            }
        }
        Ok(())
    }

    fn for_init(&mut self, init: &Stmt, env: &Env) -> Result<(), CompileError> {
        match init {
            Stmt::Decl { name, init } => {
                // local already allocated in the prologue
                if let Some(v) = init {
                    self.expr(&Expr::Assign {
                        name: name.clone(),
                        value: Box::new(v.clone()),
                    }, env)?;
                }
            }
            Stmt::Expr(e) => self.expr(e, env)?,
            Stmt::Assign { name, value } => self.expr(
                &Expr::Assign {
                    name: name.clone(),
                    value: Box::new(value.clone()),
                },
                env,
            )?,
            _ => {
                return Err(CompileError {
                    msg: "invalid for-loop init".into(),
                })
            }
        }
        Ok(())
    }

    fn epilogue(&mut self, env: &Env) {
        if env.nlocal > 0 {
            self.builder.emit(op::STACK, &[(4 * env.nlocal) as i32]);
        }
        self.builder.emit(op::RETN, &[]);
    }

    // ---- function emission ----
    fn func(&mut self, f: &Function) -> Result<(), CompileError> {
        let env = Env::new(f)?;

        self.builder.label(&format!("@{}", f.name));
        let off = self.builder.code_offset() as i32;
        self.func_off.insert(f.name.clone(), off);

        self.builder.emit(op::PROC, &[]);
        // prologue: locals in declaration order
        let mut i = 0usize;
        for (name, init) in declared_locals(f) {
            self.builder.emit(op::STACK, &[-4]);
            if let Some(v) = init {
                self.expr(&v, &env)?;
            } else {
                self.builder.emit(op::ZERO_PRI, &[]);
            }
            self.builder.emit(op::STOR_S_PRI, &[-4 * (i as i32 + 1)]);
            i += 1;
            let _ = name;
        }
        // body
        for s in &f.body {
            self.stmt(s, &env, &f.name)?;
        }
        // fallthrough: no explicit return on the final path
        self.builder.emit(op::ZERO_PRI, &[]);
        self.epilogue(&env);
        Ok(())
    }
}

impl Env {
    fn new(f: &Function) -> Result<Self, CompileError> {
        let mut local = HashMap::new();
        for (i, p) in f.params.iter().enumerate() {
            local.insert(p.clone(), (12 + 4 * i) as i32);
        }
        let mut nlocal = 0usize;
        for (name, _) in declared_locals(f) {
            nlocal += 1;
            if local.insert(name, -4 * nlocal as i32).is_some() {
                return Err(CompileError {
                    msg: "duplicate local variable".into(),
                });
            }
        }
        Ok(Self { local, nlocal })
    }
}

/// All local declarations in source order (function body, nested blocks and
/// for-inits included — the reference allocates all of them in the prologue).
fn declared_locals(f: &Function) -> Vec<(String, Option<Expr>)> {
    let mut out = Vec::new();
    for s in &f.body {
        walk_locals(s, &mut out);
    }
    out
}

fn walk_locals(s: &Stmt, out: &mut Vec<(String, Option<Expr>)>) {
    match s {
        Stmt::Decl { name, init } => out.push((name.clone(), init.clone())),
        Stmt::If { then, else_, .. } => {
            walk_locals(then, out);
            if let Some(e) = else_ {
                walk_locals(e, out);
            }
        }
        Stmt::While { body, .. } => {
            walk_locals(body, out);
        }
        Stmt::For { init, body, .. } => {
            walk_locals(init, out);
            walk_locals(body, out);
        }
        Stmt::Block(inner) => {
            for s in inner {
                walk_locals(s, out);
            }
        }
        _ => {}
    }
}

/// Constant-fold a global initializer (no identifiers, no calls).
fn const_eval(e: &Expr) -> Result<i32, CompileError> {
    match e {
        Expr::Int(v) => Ok(*v),
        Expr::Unary { op, e } => {
            let v = const_eval(e)?;
            Ok(match op {
                UnOp::Neg => -v,
                UnOp::Not => {
                    if v == 0 {
                        1
                    } else {
                        0
                    }
                }
            })
        }
        Expr::Binary { op, lhs, rhs } => {
            let a = const_eval(lhs)?;
            let b = const_eval(rhs)?;
            Ok(match op {
                BinOp::Add => a.wrapping_add(b),
                BinOp::Sub => a.wrapping_sub(b),
                BinOp::Mul => a.wrapping_mul(b),
                BinOp::Div => div_floor(a, b),
                BinOp::Mod => mod_floor(a, b),
                BinOp::Eq => (a == b) as i32,
                BinOp::Ne => (a != b) as i32,
                BinOp::Lt => (a < b) as i32,
                BinOp::Le => (a <= b) as i32,
                BinOp::Gt => (a > b) as i32,
                BinOp::Ge => (a >= b) as i32,
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::Shl => a << b,
                BinOp::Shr => a >> b,
            })
        }
        _ => Err(CompileError {
            msg: "global initializers must be constant expressions".into(),
        }),
    }
}

/// Pawn uses floored division + matching remainder (runtime `amx.c`).
fn div_floor(a: i32, b: i32) -> i32 {
    assert_ne!(b, 0, "division by zero in global initializer");
    let q = a / b;
    let r = a % b;
    if r != 0 && (r ^ b) < 0 {
        q - 1
    } else {
        q
    }
}

fn mod_floor(a: i32, b: i32) -> i32 {
    let q = div_floor(a, b);
    a - q * b
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpc_ast::{BinOp, Expr};

    #[test]
    fn const_eval_floor_div() {
        // -7 / 2 = -4 (floor), -7 % 2 = 1
        assert_eq!(
            const_eval(&Expr::Binary {
                op: BinOp::Div,
                lhs: Box::new(Expr::Unary {
                    op: UnOp::Neg,
                    e: Box::new(Expr::Int(7)),
                }),
                rhs: Box::new(Expr::Int(2)),
            })
            .unwrap(),
            -4
        );
        assert_eq!(div_floor(-7, 2), -4);
        assert_eq!(mod_floor(-7, 2), 1);
        assert_eq!(div_floor(7, 2), 3);
        assert_eq!(mod_floor(7, 2), 1);
    }
}
