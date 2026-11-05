//! Pawn AST (MVP subset: integer arithmetic, control flow, calls).

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i32),
    Ident(String),
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnOp,
        e: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    /// `name = value` as an expression (for-loop step); value is the stored
    /// result and the expression's value is the new value.
    Assign {
        name: String,
        value: Box<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Decl {
        name: String,
        init: Option<Expr>,
    },
    Assign {
        name: String,
        value: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>),
    If {
        cond: Expr,
        then: Box<Stmt>,
        else_: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    For {
        init: Box<Stmt>,
        cond: Expr,
        step: Expr,
        body: Box<Stmt>,
    },
    Block(Vec<Stmt>),
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub public: bool,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub globals: Vec<Global>,
    pub functions: Vec<Function>,
    /// Native function names, declaration order (index = native id).
    pub natives: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub name: String,
    pub init: Option<Expr>,
}
