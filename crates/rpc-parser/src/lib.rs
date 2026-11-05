//! Pawn recursive-descent parser (MVP subset: integer arithmetic, control flow, calls).
//!
//! Grammar (loose):
//! ```text
//! program   := (global | function)*
//! global    := "new" IDENT ("=" expr)? ";"
//! function  := ("public"|"stock")? IDENT "(" [IDENT ("," IDENT)*] ")" block
//! stmt      := decl | assign | expr ";" | return | if | while | for | block
//! ```

use rpc_ast::{BinOp, Expr, Function, Global, Program, Stmt, UnOp};
use rpc_lexer::{lex, LexError, Token, TokenKind};

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub line: u32,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error (line {}): {}", self.line, self.msg)
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self {
            msg: e.msg,
            line: e.line,
        }
    }
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

fn same_kind(a: &TokenKind, b: &TokenKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }
    fn peek_kind(&self) -> &TokenKind {
        &self.toks[self.pos].kind
    }
    fn at(&self, k: &TokenKind) -> bool {
        same_kind(self.peek_kind(), k)
    }
    fn next(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn err(&self, msg: &str) -> ParseError {
        ParseError {
            msg: msg.into(),
            line: self.peek().line,
        }
    }
    fn expect(&mut self, k: &TokenKind, what: &str) -> Result<(), ParseError> {
        if self.at(k) {
            self.next();
            Ok(())
        } else {
            Err(self.err(&format!("expected {what}, found {:?}", self.peek().kind)))
        }
    }
    fn eat(&mut self, k: &TokenKind) -> bool {
        if self.at(k) {
            self.next();
            true
        } else {
            false
        }
    }
    fn ident(&mut self) -> Result<String, ParseError> {
        match self.next().kind {
            TokenKind::Ident(s) => Ok(s),
            other => Err(self.err(&format!("expected identifier, found {other:?}"))),
        }
    }

    // ---- expressions (precedence climbing) ----
    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_prec(1)
    }

    fn bin_prec(k: &TokenKind) -> Option<(BinOp, u8)> {
        match k {
            // C-like precedence, higher = tighter
            TokenKind::Eq => Some((BinOp::Eq, 6)),
            TokenKind::Ne => Some((BinOp::Ne, 6)),
            TokenKind::Lt => Some((BinOp::Lt, 6)),
            TokenKind::Le => Some((BinOp::Le, 6)),
            TokenKind::Gt => Some((BinOp::Gt, 6)),
            TokenKind::Ge => Some((BinOp::Ge, 6)),
            TokenKind::Shl => Some((BinOp::Shl, 8)),
            TokenKind::Shr => Some((BinOp::Shr, 8)),
            TokenKind::Plus => Some((BinOp::Add, 9)),
            TokenKind::Minus => Some((BinOp::Sub, 9)),
            TokenKind::Star => Some((BinOp::Mul, 10)),
            TokenKind::Slash => Some((BinOp::Div, 10)),
            TokenKind::Percent => Some((BinOp::Mod, 10)),
            TokenKind::BitAnd => Some((BinOp::BitAnd, 5)),
            TokenKind::BitXor => Some((BinOp::BitXor, 4)),
            TokenKind::BitOr => Some((BinOp::BitOr, 3)),
            _ => None,
        }
    }

    fn parse_prec(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        loop {
            match Self::bin_prec(self.peek_kind()) {
                Some((op, prec)) if prec >= min_prec => {
                    self.next();
                    let rhs = self.parse_prec(prec + 1)?;
                    lhs = Expr::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.at(&TokenKind::Minus) {
            self.next();
            let e = self.unary()?;
            return Ok(Expr::Unary { op: UnOp::Neg, e: Box::new(e) });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind().clone() {
            TokenKind::Int(v) => {
                self.next();
                Ok(Expr::Int(v))
            }
            TokenKind::Ident(name) => {
                self.next();
                if self.at(&TokenKind::LParen) {
                    let args = self.arg_list()?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            TokenKind::LParen => {
                self.next();
                let e = self.expr()?;
                self.expect(&TokenKind::RParen, ")")?;
                Ok(e)
            }
            other => Err(self.err(&format!("unexpected {other:?} in expression"))),
        }
    }

    fn arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.expect(&TokenKind::LParen, "(")?;
        let mut args = Vec::new();
        if self.at(&TokenKind::RParen) {
            self.next();
            return Ok(args);
        }
        loop {
            args.push(self.expr()?);
            if self.at(&TokenKind::RParen) {
                self.next();
                return Ok(args);
            }
            self.expect(&TokenKind::Comma, ",")?;
        }
    }

    /// Expression, or `ident = expr` (used for for-loop steps).
    fn assign_or_expr(&mut self) -> Result<Expr, ParseError> {
        if let TokenKind::Ident(name) = self.peek_kind().clone() {
            if self.peek_ahead_is_assign() {
                self.next();
                self.next(); // "="
                let rhs = self.expr()?;
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(rhs),
                });
            }
        }
        self.expr()
    }

    fn peek_ahead_is_assign(&self) -> bool {
        let mut i = self.pos;
        // skip the ident (1 token)
        i += 1;
        matches!(self.toks.get(i).map(|t| &t.kind), Some(TokenKind::Assign))
    }

    // ---- statements ----
    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(&TokenKind::LBrace, "{")?;
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            stmts.push(self.stmt()?);
        }
        self.expect(&TokenKind::RBrace, "}")?;
        Ok(stmts)
    }

    fn stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_kind().clone() {
            TokenKind::KwNew => {
                self.next();
                let name = self.ident()?;
                let init = if self.eat(&TokenKind::Assign) {
                    Some(self.expr()?)
                } else {
                    None
                };
                self.eat(&TokenKind::Semicolon);
                Ok(Stmt::Decl { name, init })
            }
            TokenKind::KwReturn => {
                self.next();
                let e = if self.at(&TokenKind::Semicolon) {
                    None
                } else {
                    Some(self.expr()?)
                };
                self.eat(&TokenKind::Semicolon);
                Ok(Stmt::Return(e))
            }
            TokenKind::KwIf => {
                self.next();
                let cond = self.paren_expr()?;
                let then = Box::new(self.stmt()?);
                let else_ = if self.eat(&TokenKind::KwElse) {
                    Some(Box::new(self.stmt()?))
                } else {
                    None
                };
                Ok(Stmt::If { cond, then, else_ })
            }
            TokenKind::KwWhile => {
                self.next();
                let cond = self.paren_expr()?;
                let body = Box::new(self.stmt()?);
                Ok(Stmt::While { cond, body })
            }
            TokenKind::KwFor => {
                self.next();
                self.expect(&TokenKind::LParen, "(")?;
                let init = Box::new(self.stmt()?);
                self.eat(&TokenKind::Semicolon);
                let cond = self.expr()?;
                self.eat(&TokenKind::Semicolon);
                let step = self.assign_or_expr()?;
                self.expect(&TokenKind::RParen, ")")?;
                let body = Box::new(self.stmt()?);
                Ok(Stmt::For { init, cond, step, body })
            }
            TokenKind::LBrace => Ok(Stmt::Block(self.block()?)),
            TokenKind::Ident(_) => {
                let name = self.ident()?;
                if self.eat(&TokenKind::Assign) {
                    let value = self.expr()?;
                    self.eat(&TokenKind::Semicolon);
                    Ok(Stmt::Assign { name, value })
                } else {
                    // expression statement (usually a call)
                    let full = if self.at(&TokenKind::LParen) {
                        let args = self.arg_list()?;
                        Expr::Call { name, args }
                    } else {
                        Expr::Ident(name)
                    };
                    self.eat(&TokenKind::Semicolon);
                    Ok(Stmt::Expr(full))
                }
            }
            other => Err(self.err(&format!("unexpected {other:?} in statement"))),
        }
    }

    fn paren_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(&TokenKind::LParen, "(")?;
        let e = self.expr()?;
        self.expect(&TokenKind::RParen, ")")?;
        Ok(e)
    }

    fn function(&mut self) -> Result<Function, ParseError> {
        let public = self.eat(&TokenKind::KwPublic);
        let _stock = self.eat(&TokenKind::KwStock);
        let name = self.ident()?;
        self.expect(&TokenKind::LParen, "(")?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                params.push(self.ident()?);
                if self.at(&TokenKind::RParen) {
                    self.next();
                    break;
                }
                self.expect(&TokenKind::Comma, ",")?;
            }
        } else {
            self.next(); // empty param list
        }
        let body = self.block()?;
        Ok(Function { name, public, params, body })
    }

    /// `native name[(']' ...)] ;` — MVP records the name only (arg types
    /// are not modeled in this subset).
    fn native_decl(&mut self) -> Result<String, ParseError> {
        self.next(); // "native"
        let name = self.ident()?;
        if self.at(&TokenKind::LParen) {
            self.next();
            let mut depth = 1;
            while depth > 0 {
                match self.peek_kind().clone() {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth -= 1,
                    TokenKind::Eof => return Err(self.err("unterminated native declaration")),
                    _ => {}
                }
                self.next();
            }
        }
        self.eat(&TokenKind::Semicolon);
        Ok(name)
    }

    fn global(&mut self) -> Result<Global, ParseError> {
        self.eat(&TokenKind::KwNew);
        let name = self.ident()?;
        let init = if self.eat(&TokenKind::Assign) {
            Some(self.expr()?)
        } else {
            None
        };
        self.eat(&TokenKind::Semicolon);
        Ok(Global { name, init })
    }
}

pub fn parse(src: &str) -> Result<Program, ParseError> {
    let toks = lex(src)?;
    let mut p = Parser { toks, pos: 0 };
    let mut prog = Program::default();
    while !p.at(&TokenKind::Eof) {
        if p.at(&TokenKind::KwNative) {
            let name = p.native_decl()?;
            prog.natives.push(name);
        } else if p.at(&TokenKind::KwNew) {
            // "new" at top level: global if followed by IDENT then (; | =);
            // function bodies never start with a top-level "new" identifier.
            prog.globals.push(p.global()?);
        } else if p.at(&TokenKind::KwPublic) || p.at(&TokenKind::KwStock) {
            prog.functions.push(p.function()?);
        } else if matches!(p.peek_kind(), TokenKind::Ident(_)) {
            prog.functions.push(p.function()?);
        } else {
            return Err(p.err("expected global or function definition"));
        }
    }
    Ok(prog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_program() {
        let src = r#"
            new g = 5
            public main()
            {
                new x = 1 + 2 * 3
                x = x - g
                return x
            }
        "#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.globals.len(), 1);
        assert_eq!(prog.globals[0].name, "g");
        assert_eq!(prog.functions.len(), 1);
        assert_eq!(prog.functions[0].name, "main");
        assert!(prog.functions[0].public);
    }

    #[test]
    fn precedence() {
        let prog = parse("public main() { return 1 + 2 * 3 }").unwrap();
        match &prog.functions[0].body[0] {
            Stmt::Return(Some(Expr::Binary { op, .. })) => {
                assert_eq!(*op, BinOp::Add);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_if_while_for() {
        let src = r#"
            public main()
            {
                new i = 0
                while (i < 3) { i = i + 1 }
                if (i == 3) { i = i * 2 }
                for (new j = 0; j < 2; j = j + 1) { i = i + 1 }
                return i
            }
        "#;
        let prog = parse(src).unwrap();
        assert_eq!(prog.functions[0].body.len(), 5);
    }
}
