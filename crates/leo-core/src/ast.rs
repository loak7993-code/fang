//! AST for Leo. Nodes carry spans; the formatter round-trips through the
//! loss-free structure.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(start: u32, end: u32, line: u32, col: u32) -> Self {
        Span { start, end, line, col }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Ptr,
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Tuple(Box<Type>, Box<Type>),
    Named(String),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "str"),
            Type::Ptr => write!(f, "ptr"),
            Type::List(t) => write!(f, "list[{}]", t),
            Type::Dict(k, v) => write!(f, "dict[{}, {}]", k, v),
            Type::Tuple(a, b) => write!(f, "({}, {})", a, b),
            Type::Named(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FStrPartAst {
    Lit(String),
    E(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nil,
    Str(String),
    FStr(Vec<FStrPartAst>),
    Ident(String),
    Unary(BinOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Compare(CmpOp, Box<Expr>, Box<Expr>),
    /// Callee expression with arguments. `f(x)`; `Point(1, 2)` until sema
    /// resolves it as a struct literal.
    Call(Box<Expr>, Vec<Expr>),
    /// recv.name(args)
    Method(Box<Expr>, String, Vec<Expr>),
    /// recv.name
    Field(Box<Expr>, String),
    /// recv[index]
    Index(Box<Expr>, Box<Expr>),
    /// recv[start:end], either side optional
    Slice(Box<Expr>, Option<Box<Expr>>, Option<Box<Expr>>),
    ListLit(Vec<Expr>),
    DictLit(Vec<(Expr, Expr)>),
    /// (a, b): exactly two values
    TupleLit(Box<Expr>, Box<Expr>),
    /// cast[T](expr)
    Cast(Type, Box<Expr>),
    /// Anonymous function value: fn(x: int) -> int: return x * 2
    FnExpr(Box<FndeclExpr>),
    /// [item for x in xs if cond]
    Comprehension {
        item: Box<Expr>,
        binders: Vec<String>,
        iter: Box<Iter>,
        cond: Option<Box<Expr>>,
    },
}

/// A function expression: parameters, optional declared return, body.
#[derive(Debug, Clone, PartialEq)]
pub struct FndeclExpr {
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Iter {
    Expr(Expr),
    Range { start: Expr, end: Expr, inclusive: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LValue {
    Ident(String),
    Index(Box<LValue>, Expr),
    Field(Box<LValue>, String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

pub type Block = Vec<Stmt>;

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let {
        binders: Vec<String>,
        ann: Option<Type>,
        value: Expr,
    },
    Assign { target: LValue, value: Expr },
    /// a, b = rhs (tuple assignment and swap)
    AssignMulti { targets: Vec<LValue>, value: Expr },
    Return(Option<Expr>),
    If {
        cond: Expr,
        then: Block,
        elifs: Vec<(Expr, Block)>,
        els: Option<Block>,
    },
    While { cond: Expr, body: Block },
    For { binders: Vec<String>, iter: Iter, body: Block },
    Try { body: Block, binder: Option<String>, handler: Block },
    Raise(Expr),
    Break,
    Continue,
    Pass,
    /// asm("inline assembly")
    Asm(String),
    Expr(Expr),
    Comment(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub is_self: bool,
    /// default value expression, if any
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    /// Struct name when this is a method declaration.
    pub receiver: Option<String>,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub body: Block,
    pub exported: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub exported: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Import { module: String, path: Option<String>, span: Span },
    Const { name: String, value: Expr, exported: bool, span: Span },
    Fn(FnDecl),
    Struct(StructDecl),
    /// extern fn sqrt(x: float) -> float from "math.h"
    ExternFn {
        name: String,
        params: Vec<Param>,
        ret: Option<Type>,
        header: String,
        span: Span,
    },
    Comment(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module name derived from the file stem.
    pub name: String,
    pub items: Vec<Item>,
}
