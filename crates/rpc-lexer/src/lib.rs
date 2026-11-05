//! Pawn lexer (MVP subset: integers, identifiers, operators, punctuation).
//!
//! MVP: no strings, no floats, no preprocessor (that is the next milestone —
//! see docs/RFC). Comments `//` and `/* */` are skipped.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Int(i32),
    Ident(String),
    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq, // ==
    Ne, // !=
    Lt,
    Le,
    Gt,
    Ge,
    BitAnd, // &
    BitOr, // |
    BitXor, // ^
    Shl, // <<
    Shr, // >>
    Assign, // =
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Comma,
    Dot,
    // keywords
    KwNew,
    KwPublic,
    KwStock,
    KwNative,
    KwReturn,
    KwIf,
    KwElse,
    KwWhile,
    KwFor,
    KwVoid,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug)]
pub struct LexError {
    pub msg: String,
    pub line: u32,
    pub col: u32,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "syntax error (line {}: {}): {}", self.line, self.col, self.msg)
    }
}

impl std::error::Error for LexError {}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let bytes = src.as_bytes();
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let mut line = 1u32;
    let mut col = 1u32;

    macro_rules! bump {
        () => {{
            let c = *bytes.get(i).unwrap_or(&0);
            if c == b'\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
            c
        }};
    }

    while i < bytes.len() {
        let start_line = line;
        let start_col = col;
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => {
                bump!();
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    bump!();
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                bump!();
                bump!();
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    bump!();
                }
                if i + 1 >= bytes.len() {
                    return Err(LexError {
                        msg: "unterminated comment".into(),
                        line: start_line,
                        col: start_col,
                    });
                }
                bump!();
                bump!();
            }
            b'0'..=b'9' => {
                let mut v: i32 = 0;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    v = v
                        .checked_mul(10)
                        .and_then(|v| v.checked_add((bytes[i] - b'0') as i32))
                        .ok_or_else(|| LexError {
                            msg: "integer literal overflow".into(),
                            line: start_line,
                            col: start_col,
                        })?;
                    bump!();
                }
                out.push(Token {
                    kind: TokenKind::Int(v),
                    line: start_line,
                    col: start_col,
                });
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let mut s = String::new();
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    s.push(bytes[i] as char);
                    bump!();
                }
                let kind = match s.as_str() {
                    "new" => TokenKind::KwNew,
                    "public" => TokenKind::KwPublic,
                    "stock" => TokenKind::KwStock,
                    "native" => TokenKind::KwNative,
                    "return" => TokenKind::KwReturn,
                    "if" => TokenKind::KwIf,
                    "else" => TokenKind::KwElse,
                    "while" => TokenKind::KwWhile,
                    "for" => TokenKind::KwFor,
                    "void" => TokenKind::KwVoid,
                    other => TokenKind::Ident(other.to_string()),
                };
                out.push(Token {
                    kind,
                    line: start_line,
                    col: start_col,
                });
            }
            b'+' if bytes.get(i + 1) == Some(&b'+') => {
                // MVP: no ++ (parse as two plus) — fall through to single
                bump!();
                out.push(Token {
                    kind: TokenKind::Plus,
                    line: start_line,
                    col: start_col,
                });
                // loop continues; second '+' lexed next iteration
            }
            b'+' => {
                bump!();
                out.push(Token { kind: TokenKind::Plus, line: start_line, col: start_col });
            }
            b'-' => {
                bump!();
                out.push(Token { kind: TokenKind::Minus, line: start_line, col: start_col });
            }
            b'*' => {
                bump!();
                out.push(Token { kind: TokenKind::Star, line: start_line, col: start_col });
            }
            b'/' => {
                bump!();
                out.push(Token { kind: TokenKind::Slash, line: start_line, col: start_col });
            }
            b'%' => {
                bump!();
                out.push(Token { kind: TokenKind::Percent, line: start_line, col: start_col });
            }
            b'=' if bytes.get(i + 1) == Some(&b'=') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::Eq, line: start_line, col: start_col });
            }
            b'=' => {
                bump!();
                out.push(Token { kind: TokenKind::Assign, line: start_line, col: start_col });
            }
            b'!' if bytes.get(i + 1) == Some(&b'=') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::Ne, line: start_line, col: start_col });
            }
            b'<' if bytes.get(i + 1) == Some(&b'<') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::Shl, line: start_line, col: start_col });
            }
            b'<' if bytes.get(i + 1) == Some(&b'=') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::Le, line: start_line, col: start_col });
            }
            b'<' => {
                bump!();
                out.push(Token { kind: TokenKind::Lt, line: start_line, col: start_col });
            }
            b'>' if bytes.get(i + 1) == Some(&b'=') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::Ge, line: start_line, col: start_col });
            }
            b'>' if bytes.get(i + 1) == Some(&b'>') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::Shr, line: start_line, col: start_col });
            }
            b'>' => {
                bump!();
                out.push(Token { kind: TokenKind::Gt, line: start_line, col: start_col });
            }
            b'&' if bytes.get(i + 1) == Some(&b'&') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::BitAnd, line: start_line, col: start_col });
            }
            b'&' => {
                bump!();
                out.push(Token { kind: TokenKind::BitAnd, line: start_line, col: start_col });
            }
            b'|' if bytes.get(i + 1) == Some(&b'|') => {
                bump!();
                bump!();
                out.push(Token { kind: TokenKind::BitOr, line: start_line, col: start_col });
            }
            b'|' => {
                bump!();
                out.push(Token { kind: TokenKind::BitOr, line: start_line, col: start_col });
            }
            b'^' => {
                bump!();
                out.push(Token { kind: TokenKind::BitXor, line: start_line, col: start_col });
            }
            b'.' => {
                bump!();
                out.push(Token { kind: TokenKind::Dot, line: start_line, col: start_col });
            }
            b'(' => {
                bump!();
                out.push(Token { kind: TokenKind::LParen, line: start_line, col: start_col });
            }
            b')' => {
                bump!();
                out.push(Token { kind: TokenKind::RParen, line: start_line, col: start_col });
            }
            b'{' => {
                bump!();
                out.push(Token { kind: TokenKind::LBrace, line: start_line, col: start_col });
            }
            b'}' => {
                bump!();
                out.push(Token { kind: TokenKind::RBrace, line: start_line, col: start_col });
            }
            b';' => {
                bump!();
                out.push(Token { kind: TokenKind::Semicolon, line: start_line, col: start_col });
            }
            b',' => {
                bump!();
                out.push(Token { kind: TokenKind::Comma, line: start_line, col: start_col });
            }
            other => {
                return Err(LexError {
                    msg: format!("unexpected character '{}'", other as char),
                    line: start_line,
                    col: start_col,
                });
            }
        }
    }
    out.push(Token { kind: TokenKind::Eof, line, col });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_simple() {
        // new x = 10 * 5 ; return x <eof>
        let toks = lex("new x = 10 * 5; return x").unwrap();
        assert!(matches!(toks.last().unwrap().kind, TokenKind::Eof));
        assert_eq!(toks.len(), 10);
        assert!(matches!(toks[0].kind, TokenKind::KwNew));
        assert!(matches!(toks[1].kind, TokenKind::Ident(_)));
        assert!(matches!(toks[2].kind, TokenKind::Assign));
        assert!(matches!(toks[3].kind, TokenKind::Int(10)));
        assert!(matches!(toks[5].kind, TokenKind::Int(5)));
        assert!(matches!(toks[7].kind, TokenKind::KwReturn));
    }

    #[test]
    fn skip_comments() {
        let toks = lex("a // comment\n/* block */ b").unwrap();
        let idents: Vec<_> = toks
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Ident(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(idents, vec!["a", "b"]);
    }

    #[test]
    fn line_numbers() {
        let toks = lex("x\ny").unwrap();
        assert_eq!(toks[0].line, 1);
        assert_eq!(toks[1].line, 2);
    }
}
