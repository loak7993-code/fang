//! Recursive-descent parser with Pratt expression parsing.

use crate::ast::*;
use crate::lexer::{FStrPart, TokKind, Token};
use crate::lexer::TokKind::*;

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (line {}, column {})", self.msg, self.line, self.col)
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub msg: String,
    pub line: u32,
    pub col: u32,
}

type PResult<T> = Result<T, ParseError>;

fn perr<T>(msg: impl Into<String>, tok: &Token) -> PResult<T> {
    Err(ParseError { msg: msg.into(), line: tok.line, col: tok.col })
}

fn tok_span(t: &Token) -> Span {
    Span::new(t.start, t.end, t.line, t.col)
}

pub struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
    /// Set when an expression just consumed an indented block (a
    /// function expression); the statement's own line already ended.
    just_finished_fnblock: bool,
}

impl<'a> Parser<'a> {
    pub fn new(toks: &'a [Token]) -> Self {
        Parser { toks, pos: 0, just_finished_fnblock: false }
    }

    pub fn parse_module(name: &str, toks: &[Token]) -> PResult<Module> {
        let mut p = Parser::new(toks);
        p.module(name)
    }

    // --- cursor ---

    fn peek(&self) -> &Token {
        // The lexer always terminates with Eof, so this is safe.
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn peek_kind(&self) -> &TokKind {
        &self.peek().kind
    }


    fn at(&self, k: &TokKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(k)
    }

    fn advance(&mut self) -> Token {
        let t = self.peek().clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, k: &TokKind) -> bool {
        if self.at(k) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, k: &TokKind, what: &str) -> PResult<Token> {
        if self.at(k) {
            Ok(self.advance())
        } else {
            perr(format!("expected {}, found {}", what, describe(self.peek_kind())), self.peek())
        }
    }

    fn expect_newline(&mut self) -> PResult<()> {
        if self.at(&Eof) {
            return Ok(());
        }
        self.expect(&Newline, "end of line").map(|_| ())
    }

    fn at_comment(&self) -> bool {
        matches!(self.peek_kind(), Comment(_))
    }

    /// End a simple statement. A trailing comment is left in the stream to
    /// become the following statement on its own line.
    fn finish_line(&mut self) -> PResult<()> {
        if self.just_finished_fnblock {
            self.just_finished_fnblock = false;
            return Ok(());
        }
        if self.at_comment() {
            return Ok(());
        }
        self.expect_newline()
    }

    /// Skip Comment tokens (used inside bracketed lists).
    fn skip_comments(&mut self) {
        while self.at(&Comment(String::new())) {
            self.advance();
        }
    }

    fn ident_tok(&mut self, what: &str) -> PResult<(String, Span)> {
        match self.peek_kind().clone() {
            TokKind::Ident(n) => {
                let t = self.advance();
                Ok((n, tok_span(&t)))
            }
            _ => perr(format!("expected {}, found {}", what, describe(self.peek_kind())), self.peek()),
        }
    }

    // --- module ---

    fn module(&mut self, name: &str) -> PResult<Module> {
        let mut items = Vec::new();
        let mut top_stmts: Vec<Stmt> = Vec::new();
        while !self.at(&Eof) {
            match self.peek_kind().clone() {
                Comment(text) => {
                    self.advance();
                    items.push(Item::Comment(text));
                    self.expect_newline()?;
                }
                Import => {
                    let t = self.advance();
                    let (module, _) = self.ident_tok("module name after 'import'")?;
                    let path = if self.eat(&From) {
                        match self.peek_kind().clone() {
                            TokKind::Str(s) => {
                                self.advance();
                                Some(s)
                            }
                            _ => {
                                return perr(
                                    "expected a path string after 'from'".to_string(),
                                    self.peek(),
                                )
                            }
                        }
                    } else {
                        None
                    };
                    items.push(Item::Import { module, path, span: tok_span(&t) });
                    self.finish_line()?;
                }
                Export => {
                    let t = self.advance();
                    match self.peek_kind() {
                        Fn => items.push(Item::Fn(self.fn_decl(true)?)),
                        Struct => items.push(Item::Struct(self.struct_decl(true)?)),
                        Let => {
                            self.advance();
                            items.push(self.const_decl(true, t)?)
                        }
                        _ => {
                            return perr(
                                "expected fn, struct, or let after 'export'".to_string(),
                                self.peek(),
                            )
                        }
                    }
                }
                Fn => items.push(Item::Fn(self.fn_decl(false)?)),
                Struct => items.push(Item::Struct(self.struct_decl(false)?)),
                Extern => items.push(self.extern_fn()?),
                Let => {
                    let t = self.advance();
                    items.push(self.const_decl(false, t)?)
                }
                _other => {
                    // Top-level statements run directly: a file without
                    // fn main wraps them into one.
                    let st = self.stmt()?;
                    top_stmts.push(st);
                    continue;
                }
            }
        }
        if !top_stmts.is_empty() {
            let has_main = items.iter().any(|it| matches!(it, Item::Fn(f) if f.name == "main" && f.receiver.is_none()));
            if has_main {
                let sp = top_stmts[0].span;
                return perr(
                    "this file has both fn main() and top-level statements; move the statements into main or remove main"
                        .to_string(),
                    &Token {
                        kind: TokKind::Eof,
                        line: sp.line,
                        col: sp.col,
                        start: sp.start,
                        end: sp.end,
                    },
                );
            }
            let main_decl = FnDecl {
                receiver: None,
                name: "main".to_string(),
                params: Vec::new(),
                ret: None,
                body: top_stmts,
                exported: false,
                span: Span::new(0, 0, 1, 1),
            };
            items.push(Item::Fn(main_decl));
        }
        Ok(Module { name: name.to_string(), items })
    }

    fn const_decl(&mut self, exported: bool, kw: Token) -> PResult<Item> {
        let (name, _) = self.ident_tok("constant name after 'let'")?;
        let ann = if self.eat(&Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Eq, "'='")?;
        let value = self.expr()?;
        self.finish_line()?;
        Ok(Item::Const { name, ann, value, exported, span: tok_span(&kw) })
    }

    fn extern_fn(&mut self) -> PResult<Item> {
        let kw = self.advance(); // extern
        self.expect(&Fn, "'fn' after 'extern'")?;
        let (name, _) = self.ident_tok("a C function name")?;
        self.expect(&LParen, "'('")?;
        let mut params = Vec::new();
        self.skip_comments();
        if !self.at(&RParen) {
            loop {
                self.skip_comments();
                params.push(self.param(None)?);
                self.skip_comments();
                if !self.eat(&Comma) {
                    break;
                }
            }
        }
        self.expect(&RParen, "')'")?;
        let ret = if self.eat(&Arrow) { Some(self.parse_type()?) } else { None };
        self.expect(&From, "'from' and a header name")?;
        let header = match self.peek_kind().clone() {
            TokKind::Str(h) => {
                self.advance();
                h
            }
            _ => return perr("expected a header string like \"math.h\"".to_string(), self.peek()),
        };
        self.finish_line()?;
        Ok(Item::ExternFn { name, params, ret, header, span: tok_span(&kw) })
    }

    fn fn_decl(&mut self, exported: bool) -> PResult<FnDecl> {
        let fn_tok = self.advance(); // fn
        let (first, _) = self.ident_tok("function name after 'fn'")?;
        let (receiver, name) = if self.eat(&Dot) {
            let (m, _) = self.ident_tok("method name after '.'")?;
            (Some(first), m)
        } else {
            (None, first)
        };
        self.expect(&LParen, "'('")?;
        let mut params = Vec::new();
        self.skip_comments();
        if !self.at(&RParen) {
            loop {
                self.skip_comments();
                params.push(self.param(receiver.as_deref())?);
                self.skip_comments();
                if !self.eat(&Comma) {
                    break;
                }
            }
        }
        self.expect(&RParen, "')'")?;
        let ret = if self.eat(&Arrow) { Some(self.parse_type()?) } else { None };
        let body = self.block()?;
        Ok(FnDecl {
            receiver,
            name,
            params,
            ret,
            body,
            exported,
            span: tok_span(&fn_tok),
        })
    }

    /// A function-expression parameter: name: type [= default]. The
    /// type may be omitted; it is inferred from the body.
    fn plain_param(&mut self) -> PResult<Param> {
        let (name, span) = self.ident_tok("parameter name")?;
        let ty = if self.eat(&Colon) {
            self.parse_type()?
        } else {
            Type::Named("__infer".to_string())
        };
        let default = if self.eat(&Eq) {
            Some(self.expr()?)
        } else {
            None
        };
        Ok(Param { name, ty, is_self: false, default, span })
    }

    /// A block, or a single inline `return expr` when ':' is followed on
    /// the same line (function expressions inside call parentheses).
    fn inline_or_block(&mut self) -> PResult<Block> {
        self.expect(&Colon, "':'")?;
        if self.at(&Newline) {
            let b = self.block_body()?;
            self.just_finished_fnblock = true;
            return Ok(b);
        }
        if self.at(&Return) {
            let t = self.advance();
            let value = if matches!(
                self.peek_kind(),
                Newline | Eof | RParen | Comma
            ) {
                None
            } else {
                Some(self.expr()?)
            };
            return Ok(vec![Stmt {
                kind: StmtKind::Return(value),
                span: tok_span(&t),
            }]);
        }
        // A bare expression is the body: its value is returned.
        let e = self.expr()?;
        Ok(vec![Stmt {
            kind: StmtKind::Expr(e.clone()),
            span: e.span,
        }])
    }

    /// Block content after ':' NEWLINE INDENT.
    fn block_body(&mut self) -> PResult<Block> {
        self.expect(&Newline, "a newline after ':'")?;
        self.expect(&Indent, "an indented block")?;
        let mut stmts = Vec::new();
        while !self.at(&Dedent) && !self.at(&Eof) {
            stmts.push(self.stmt()?);
        }
        self.eat(&Dedent);
        Ok(stmts)
    }

    fn param(&mut self, receiver: Option<&str>) -> PResult<Param> {
        let t = self.peek().clone();
        // 'self' as an unannotated first parameter of a method.
        if let TokKind::Ident(n) = &t.kind {
            if n == "self" {
                let next = self.toks.get(self.pos + 1).map(|x| &x.kind);
                let next_is_boundary = matches!(next, Some(Colon) | Some(Comma) | Some(RParen) | Some(Newline) | Some(Eof) | None);
                if let Some(recv) = receiver {
                    if next_is_boundary {
                        self.advance();
                        return Ok(Param {
                            name: "self".into(),
                            ty: Type::Named(recv.to_string()),
                            is_self: true,
                            default: None,
                            span: tok_span(&t),
                        });
                    }
                }
            }
        }
        let (name, span) = self.ident_tok("parameter name")?;
        // The type may be omitted; it is inferred from the body.
        let ty = if self.eat(&Colon) {
            self.parse_type()?
        } else {
            Type::Named("__infer".to_string())
        };
        let default = if self.eat(&Eq) {
            Some(self.expr()?)
        } else {
            None
        };
        Ok(Param { name, ty, is_self: false, default, span })
    }

    fn struct_decl(&mut self, exported: bool) -> PResult<StructDecl> {
        let kw = self.advance(); // struct
        let (name, _) = self.ident_tok("struct name")?;
        let mut fields = Vec::new();
        self.expect(&Colon, "':' after the struct name")?;
        self.expect(&Newline, "a newline")?;
        self.expect(&Indent, "an indented field list")?;
        while !self.at(&Dedent) && !self.at(&Eof) {
            if self.at_comment() {
                self.advance();
                self.expect_newline()?;
                continue;
            }
            let (fname, _) = self.ident_tok("field name")?;
            self.expect(&Colon, "':' then a field type")?;
            let ty = self.parse_type()?;
            let comment = match self.peek_kind().clone() {
                Comment(text) => {
                    self.advance();
                    Some(text)
                }
                _ => None,
            };
            self.expect_newline()?;
            fields.push(StructField { name: fname, ty, comment });
        }
        self.eat(&Dedent);
        Ok(StructDecl { name, fields, exported, span: tok_span(&kw) })
    }

    // --- statements ---

    fn block(&mut self) -> PResult<Block> {
        self.expect(&Colon, "':'")?;
        // Comments on the clause line flow into the block head.
        let mut head = Vec::new();
        while self.at_comment() {
            if let Comment(text) = self.peek_kind().clone() {
                self.advance();
                head.push(Stmt { kind: StmtKind::Comment(text.clone()), span: Span::new(0, 0, 0, 0) });
                self.expect_newline()?;
            }
        }
        self.expect(&Newline, "a newline after ':'")?;
        self.expect(&Indent, "an indented block")?;
        let mut stmts = head;
        while !self.at(&Dedent) && !self.at(&Eof) {
            stmts.push(self.stmt()?);
        }
        self.eat(&Dedent);
        Ok(stmts)
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        let t = self.peek().clone();
        match &t.kind {
            Comment(text) => {
                self.advance();
                self.expect_newline()?;
                return Ok(Stmt { kind: StmtKind::Comment(text.clone()), span: tok_span(&t) });
            }
            Let => {
                self.advance();
                let mut binders = vec![self.ident_tok("a name after 'let'")?.0];
                let tuple_ann = if self.eat(&Comma) {
                    // Two binders: the annotation is a tuple type.
                    let second_name = self.ident_tok("a second name after ','")?.0;
                    binders.push(second_name);
                    if self.eat(&Colon) {
                        if let Type::Tuple(a, b) = self.parse_type()? {
                            Some(Type::Tuple(a, b))
                        } else {
                            return perr(
                                "two binders need a tuple annotation like (int, int)"
                                    .to_string(),
                                self.peek(),
                            );
                        }
                    } else {
                        None
                    }
                } else {
                    if self.eat(&Colon) {
                        Some(self.parse_type()?)
                    } else {
                        None
                    }
                };
                self.expect(&Eq, "'=' and a value")?;
                let value = self.expr()?;
                self.finish_line()?;
                return Ok(Stmt {
                    kind: StmtKind::Let { binders, ann: tuple_ann, value },
                    span: tok_span(&t),
                });
            }
            Return => {
                self.advance();
                let value = if self.at(&Newline) || self.at(&Eof) || self.at_comment() {
                    None
                } else {
                    Some(self.expr()?)
                };
                self.finish_line()?;
                return Ok(Stmt { kind: StmtKind::Return(value), span: tok_span(&t) });
            }
            If => return self.if_stmt(),
            While => {
                self.advance();
                let cond = self.expr()?;
                let body = self.block()?;
                return Ok(Stmt {
                    kind: StmtKind::While { cond, body },
                    span: tok_span(&t),
                });
            }
            For => return self.for_stmt(),
            Try => {
                self.advance();
                let body = self.block()?;
                self.expect(&Except, "'except' after the try block")?;
                let binder = if let TokKind::Ident(_) = self.peek_kind() {
                    Some(self.advance().kind.ident_name().unwrap().to_string())
                } else {
                    None
                };
                let handler = self.block()?;
                return Ok(Stmt {
                    kind: StmtKind::Try { body, binder, handler },
                    span: tok_span(&t),
                });
            }
            Raise => {
                self.advance();
                let value = self.expr()?;
                self.finish_line()?;
                return Ok(Stmt { kind: StmtKind::Raise(value), span: tok_span(&t) });
            }
            Break => {
                self.advance();
                self.finish_line()?;
                return Ok(Stmt { kind: StmtKind::Break, span: tok_span(&t) });
            }
            Continue => {
                self.advance();
                self.finish_line()?;
                return Ok(Stmt { kind: StmtKind::Continue, span: tok_span(&t) });
            }
            Pass => {
                self.advance();
                self.finish_line()?;
                return Ok(Stmt { kind: StmtKind::Pass, span: tok_span(&t) });
            }
            Ident(n) if n == "asm" && matches!(self.toks.get(self.pos + 1).map(|x| &x.kind), Some(LParen)) => {
                self.advance(); // asm
                self.advance(); // (
                let code = match self.peek_kind().clone() {
                    TokKind::Str(code) => {
                        self.advance();
                        code
                    }
                    _ => return perr("asm takes one string of assembly".to_string(), self.peek()),
                };
                self.expect(&RParen, "')'")?;
                self.finish_line()?;
                return Ok(Stmt { kind: StmtKind::Asm(code), span: tok_span(&t) });
            }
            _ => {}
        }

        // Typed assignment: name: type = value (auto-declare with an
        // explicit type, the only way to annotate an empty collection).
        if let TokKind::Ident(_) = self.peek_kind() {
            if self.toks.get(self.pos + 1).map(|t| &t.kind) == Some(&Colon) {
                let (name, _) = self.ident_tok("a name")?;
                self.advance(); // :
                let ty = self.parse_type()?;
                self.expect(&Eq, "'='")?;
                let rhs = self.expr()?;
                self.finish_line()?;
                return Ok(Stmt {
                    kind: StmtKind::TypedAssign {
                        name,
                        ann: ty,
                        value: rhs,
                    },
                    span: tok_span(&t),
                });
            }
        }
        // Expression statement or assignment.
        let value = self.expr()?;
        // Tuple assignment: a, b = rhs
        if self.at(&Comma) {
            self.advance();
            let second = self.expr()?;
            self.expect(&Eq, "'=' for tuple assignment")?;
            let ta = self.lvalue_of(value)?;
            let tb = self.lvalue_of(second)?;
            let rhs = self.assignment_rhs()?;
            self.finish_line()?;
            return Ok(Stmt {
                kind: StmtKind::AssignMulti {
                    targets: vec![ta, tb],
                    value: rhs,
                },
                span: tok_span(&t),
            });
        }
        if self.at(&Eq) {
            self.advance();
            let target = self.lvalue_of(value)?;
            let rhs = self.assignment_rhs()?;
            self.finish_line()?;
            Ok(Stmt {
                kind: StmtKind::Assign { target, value: rhs },
                span: tok_span(&t),
            })
        } else {
            self.finish_line()?;
            Ok(Stmt { kind: StmtKind::Expr(value), span: tok_span(&t) })
        }
    }

    fn if_stmt(&mut self) -> PResult<Stmt> {
        let t = self.advance(); // if
        let cond = self.expr()?;
        let then = self.block()?;
        let mut elifs = Vec::new();
        let mut els = None;
        loop {
            match self.peek_kind() {
                Elif => {
                    self.advance();
                    let c = self.expr()?;
                    let b = self.block()?;
                    elifs.push((c, b));
                }
                Else => {
                    self.advance();
                    els = Some(self.block()?);
                    break;
                }
                _ => break,
            }
        }
        Ok(Stmt {
            kind: StmtKind::If { cond, then, elifs, els },
            span: tok_span(&t),
        })
    }

    fn for_stmt(&mut self) -> PResult<Stmt> {
        let t = self.advance(); // for
        let mut binders = vec![self.ident_tok("a loop variable after 'for'")?.0];
        if self.eat(&Comma) {
            binders.push(self.ident_tok("a second loop variable")?.0);
        }
        self.expect(&In, "'in'")?;
        let first = self.expr()?;
        let iter = match self.peek_kind() {
            DotDot => {
                self.advance();
                let end = self.expr()?;
                Iter::Range { start: first, end, inclusive: false }
            }
            DotDotEq => {
                self.advance();
                let end = self.expr()?;
                Iter::Range { start: first, end, inclusive: true }
            }
            _ => Iter::Expr(first),
        };
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::For { binders, iter, body }, span: tok_span(&t) })
    }

    /// The right side of an assignment: one expression, or a 2-tuple
    /// written without parentheses (for swaps).
    fn assignment_rhs(&mut self) -> PResult<Expr> {
        let first = self.expr()?;
        if self.at(&Comma) {
            let t = self.advance();
            let second = self.expr()?;
            return Ok(Expr {
                kind: ExprKind::TupleLit(Box::new(first), Box::new(second)),
                span: tok_span(&t),
            });
        }
        Ok(first)
    }

    fn lvalue_of(&mut self, e: Expr) -> PResult<LValue> {
        match e.kind {
            ExprKind::Ident(n) => Ok(LValue::Ident(n)),
            ExprKind::Index(recv, idx) => Ok(LValue::Index(Box::new(self.lvalue_of(*recv)?), *idx)),
            ExprKind::Field(recv, name) => Ok(LValue::Field(Box::new(self.lvalue_of(*recv)?), name)),
            other => {
                let tok = self.peek();
                perr(
                    format!("cannot assign to a {}", expr_kind_name(&other)),
                    tok,
                )
            }
        }
    }

    // --- expressions (Pratt) ---

    pub fn expr(&mut self) -> PResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut left = self.parse_and()?;
        while self.at(&Or) {
            let t = self.advance();
            let right = self.parse_and()?;
            left = Expr { kind: ExprKind::Or(Box::new(left), Box::new(right)), span: tok_span(&t) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut left = self.parse_not()?;
        while self.at(&And) {
            let t = self.advance();
            let right = self.parse_not()?;
            left = Expr { kind: ExprKind::And(Box::new(left), Box::new(right)), span: tok_span(&t) };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> PResult<Expr> {
        if self.at(&Not) {
            let t = self.advance();
            let inner = self.parse_not()?;
            return Ok(Expr { kind: ExprKind::Not(Box::new(inner)), span: tok_span(&t) });
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> PResult<Expr> {
        let left = self.parse_bit_or()?;
        let op = match self.peek_kind() {
            EqEq => CmpOp::Eq,
            NotEq => CmpOp::Ne,
            Lt => CmpOp::Lt,
            Gt => CmpOp::Gt,
            Le => CmpOp::Le,
            Ge => CmpOp::Ge,
            _ => return Ok(left),
        };
        let t = self.advance();
        let right = self.parse_add()?;
        // Chained comparisons are a parse error with a teaching message.
        if matches!(
            self.peek_kind(),
            EqEq | NotEq | Lt | Gt | Le | Ge
        ) {
            return perr(
                "chained comparisons are not supported; use and: (a < b) and (b < c)"
                    .to_string(),
                self.peek(),
            );
        }
        Ok(Expr {
            kind: ExprKind::Compare(op, Box::new(left), Box::new(right)),
            span: tok_span(&t),
        })
    }

    fn parse_bit_or(&mut self) -> PResult<Expr> {
        let mut left = self.parse_bit_xor()?;
        loop {
            let op = match self.peek_kind() {
                Pipe => BinOp::BitOr,
                _ => return Ok(left),
            };
            let t = self.advance();
            let right = self.parse_bit_xor()?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span: tok_span(&t),
            };
        }
    }

    fn parse_bit_xor(&mut self) -> PResult<Expr> {
        let mut left = self.parse_bit_and()?;
        loop {
            let op = match self.peek_kind() {
                Caret => BinOp::BitXor,
                _ => return Ok(left),
            };
            let t = self.advance();
            let right = self.parse_bit_and()?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span: tok_span(&t),
            };
        }
    }

    fn parse_bit_and(&mut self) -> PResult<Expr> {
        let mut left = self.parse_shift()?;
        loop {
            let op = match self.peek_kind() {
                Amp => BinOp::BitAnd,
                _ => return Ok(left),
            };
            let t = self.advance();
            let right = self.parse_shift()?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span: tok_span(&t),
            };
        }
    }

    fn parse_shift(&mut self) -> PResult<Expr> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek_kind() {
                Shl => BinOp::Shl,
                Shr => BinOp::Shr,
                _ => return Ok(left),
            };
            let t = self.advance();
            let right = self.parse_add()?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span: tok_span(&t),
            };
        }
    }

    fn parse_add(&mut self) -> PResult<Expr> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek_kind() {
                Plus => BinOp::Add,
                Minus => BinOp::Sub,
                _ => return Ok(left),
            };
            let t = self.advance();
            let right = self.parse_mul()?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span: tok_span(&t),
            };
        }
    }

    fn parse_mul(&mut self) -> PResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                Star => BinOp::Mul,
                Slash => BinOp::Div,
                Percent => BinOp::Mod,
                _ => return Ok(left),
            };
            let t = self.advance();
            let right = self.parse_unary()?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span: tok_span(&t),
            };
        }
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if self.at(&Minus) {
            let t = self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary(BinOp::Sub, Box::new(inner)),
                span: tok_span(&t),
            });
        }
        self.parse_pow()
    }

    /// power ::= postfix ["**" unary]; the right side recurses through
    /// unary so 2 ** -3 parses, and -2 ** 2 means -(2 ** 2).
    fn parse_pow(&mut self) -> PResult<Expr> {
        let base = self.parse_postfix()?;
        if self.at(&StarStar) {
            let t = self.advance();
            let right = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Binary(BinOp::Pow, Box::new(base), Box::new(right)),
                span: tok_span(&t),
            });
        }
        Ok(base)
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            match self.peek_kind().clone() {
                LParen => {
                    // The call node carries the '(' span so every postfix
                    // expression has a unique span for the type table.
                    let paren = self.advance();
                    let args = self.arg_list()?;
                    e = Expr {
                        kind: ExprKind::Call(Box::new(e), args),
                        span: tok_span(&paren),
                    };
                }
                Dot => {
                    let dot = self.advance();
                    let (name, _) = self.ident_tok("a name after '.'")?;
                    if self.at(&LParen) {
                        let paren = self.advance();
                        let args = self.arg_list()?;
                        e = Expr {
                            kind: ExprKind::Method(Box::new(e), name, args),
                            span: tok_span(&paren),
                        };
                    } else {
                        e = Expr {
                            kind: ExprKind::Field(Box::new(e), name),
                            span: tok_span(&dot),
                        };
                    }
                }
                LBracket => {
                    let bracket = self.advance();
                    // Slice [a:b] or index [i].
                    let start = if self.at(&Colon) {
                        None
                    } else {
                        Some(self.expr()?)
                    };
                    if self.eat(&Colon) {
                        let end = if self.at(&RBracket) {
                            None
                        } else {
                            Some(self.expr()?)
                        };
                        self.expect(&RBracket, "']'")?;
                        e = Expr {
                            kind: ExprKind::Slice(Box::new(e), start.map(Box::new), end.map(Box::new)),
                            span: tok_span(&bracket),
                        };
                    } else {
                        let idx = match start {
                            Some(i) => i,
                            None => {
                                return perr(
                                    "empty '[]'; use an index or a slice like xs[1:3]"
                                        .to_string(),
                                    self.peek(),
                                )
                            }
                        };
                        self.expect(&RBracket, "']'")?;
                        e = Expr {
                            kind: ExprKind::Index(Box::new(e), Box::new(idx)),
                            span: tok_span(&bracket),
                        };
                    }
                }
                _ => return Ok(e),
            }
        }
    }

    fn arg_list(&mut self) -> PResult<Vec<Expr>> {
        let mut args = Vec::new();
        self.skip_comments();
        if !self.at(&RParen) {
            loop {
                self.skip_comments();
                args.push(self.expr()?);
                self.skip_comments();
                if !self.eat(&Comma) {
                    break;
                }
            }
        }
        self.expect(&RParen, "')'")?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let t = self.peek().clone();
        match t.kind.clone() {
            Int(v) => {
                self.advance();
                Ok(Expr { kind: ExprKind::Int(v), span: tok_span(&t) })
            }
            Float(v) => {
                self.advance();
                Ok(Expr { kind: ExprKind::Float(v), span: tok_span(&t) })
            }
            Str(s) => {
                self.advance();
                Ok(Expr { kind: ExprKind::Str(s), span: tok_span(&t) })
            }
            FStr(parts) => {
                self.advance();
                let mut ast_parts = Vec::new();
                for p in parts {
                    match p {
                        FStrPart::Literal(l) => ast_parts.push(FStrPartAst::Lit(l)),
                        FStrPart::Expr(text, toks, rel) => {
                            let mut sub = Parser::new(&toks);
                            let mut e = sub.expr()?;
                            shift_spans(&mut e, t.start + rel + 1);
                            // The whole interior must be one expression.
                            if !matches!(sub.peek_kind(), Eof | Newline) {
                                return perr(
                                    format!(
                                        "f-string expression '{}' contains more than one expression",
                                        text
                                    ),
                                    &t,
                                );
                            }
                            ast_parts.push(FStrPartAst::E(e));
                        }
                    }
                }
                Ok(Expr { kind: ExprKind::FStr(ast_parts), span: tok_span(&t) })
            }
            True => {
                self.advance();
                Ok(Expr { kind: ExprKind::Bool(true), span: tok_span(&t) })
            }
            False => {
                self.advance();
                Ok(Expr { kind: ExprKind::Bool(false), span: tok_span(&t) })
            }
            Nil => {
                self.advance();
                Ok(Expr { kind: ExprKind::Nil, span: tok_span(&t) })
            }
            Fn => {
                // Function expression: fn(x: int) -> int: return x * 2
                self.advance(); // fn
                self.expect(&LParen, "'('")?;
                let mut params = Vec::new();
                self.skip_comments();
                if !self.at(&RParen) {
                    loop {
                        self.skip_comments();
                        params.push(self.plain_param()?);
                        self.skip_comments();
                        if !self.eat(&Comma) {
                            break;
                        }
                    }
                }
                self.expect(&RParen, "')'")?;
                let ret = if self.eat(&Arrow) { Some(self.parse_type()?) } else { None };
                let body = self.inline_or_block()?;
                Ok(Expr {
                    kind: ExprKind::FnExpr(Box::new(FndeclExpr { params, ret, body })),
                    span: tok_span(&t),
                })
            }
            Ident(name) => {
                // cast[T](expr)
                if name == "cast" && self.toks.get(self.pos + 1).map(|x| &x.kind) == Some(&LBracket) {
                    self.advance(); // cast
                    self.advance(); // [
                    let ty = self.parse_type()?;
                    self.expect(&RBracket, "']'")?;
                    self.expect(&LParen, "'(' after cast[T]")?;
                    let inner = self.expr()?;
                    self.expect(&RParen, "')'")?;
                    return Ok(Expr { kind: ExprKind::Cast(ty, Box::new(inner)), span: tok_span(&t) });
                }
                self.advance();
                Ok(Expr { kind: ExprKind::Ident(name), span: tok_span(&t) })
            }
            LParen => {
                self.advance();
                if self.at(&RParen) {
                    return perr("empty '()' is not a value; tuples hold exactly two values, like (1, 2)".to_string(), &t);
                }
                let first = self.expr()?;
                if self.at(&Comma) {
                    self.advance();
                    if self.at(&RParen) {
                        return perr(
                            "tuples hold exactly two values in Fang; (x,) is not allowed"
                                .to_string(),
                            self.peek(),
                        );
                    }
                    let second = self.expr()?;
                    if self.at(&Comma) {
                        return perr("tuples hold exactly two values in Fang".to_string(), self.peek());
                    }
                    self.expect(&RParen, "')'")?;
                    return Ok(Expr {
                        kind: ExprKind::TupleLit(Box::new(first), Box::new(second)),
                        span: tok_span(&t),
                    });
                }
                self.expect(&RParen, "')'")?;
                Ok(first)
            }
            LBracket => {
                self.advance();
                if self.at(&RBracket) {
                    self.advance();
                    return Ok(Expr { kind: ExprKind::ListLit(Vec::new()), span: tok_span(&t) });
                }
                let first = self.expr()?;
                // Comprehension: [item for x in xs if cond]
                if self.at(&For) {
                    self.advance();
                    let mut binders = vec![self.ident_tok("a loop variable after 'for'")?.0];
                    if self.eat(&Comma) {
                        binders.push(self.ident_tok("a second loop variable")?.0);
                    }
                    self.expect(&In, "'in'")?;
                    let first_iter = self.expr()?;
                    let iter = match self.peek_kind() {
                        DotDot | DotDotEq => {
                            let inclusive = matches!(self.peek_kind(), DotDotEq);
                            self.advance();
                            let end = self.expr()?;
                            Iter::Range { start: first_iter, end, inclusive }
                        }
                        _ => Iter::Expr(first_iter),
                    };
                    let cond = if self.eat(&If) {
                        Some(Box::new(self.expr()?))
                    } else {
                        None
                    };
                    self.expect(&RBracket, "']'")?;
                    return Ok(Expr {
                        kind: ExprKind::Comprehension {
                            item: Box::new(first),
                            binders,
                            iter: Box::new(iter),
                            cond,
                        },
                        span: tok_span(&t),
                    });
                }
                let mut items = vec![first];
                if !self.at(&RBracket) {
                    loop {
                        if !self.eat(&Comma) {
                            break;
                        }
                        if self.at(&RBracket) {
                            break; // trailing comma
                        }
                        items.push(self.expr()?);
                    }
                }
                self.expect(&RBracket, "']'")?;
                Ok(Expr { kind: ExprKind::ListLit(items), span: tok_span(&t) })
            }
            LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                if !self.at(&RBrace) {
                    loop {
                        let k = self.expr()?;
                        self.expect(&Colon, "':' between a key and value")?;
                        let v = self.expr()?;
                        pairs.push((k, v));
                        if !self.eat(&Comma) {
                            break;
                        }
                        if self.at(&RBrace) {
                            break; // trailing comma
                        }
                    }
                }
                self.expect(&RBrace, "'}'")?;
                Ok(Expr { kind: ExprKind::DictLit(pairs), span: tok_span(&t) })
            }
            other => perr(
                format!("expected an expression, found {}", describe(&other)),
                &t,
            ),
        }
    }

    // --- types ---

    pub fn parse_type(&mut self) -> PResult<Type> {
        // Tuple type: (A, B)
        if self.at(&LParen) {
            self.advance();
            let a = self.parse_type()?;
            self.expect(&Comma, "',' between the tuple element types")?;
            let b = self.parse_type()?;
            self.expect(&RParen, "')'")?;
            return Ok(Type::Tuple(Box::new(a), Box::new(b)));
        }
        // Function type: fn(A, B) -> C
        if self.at(&Fn) {
            self.advance();
            self.expect(&LParen, "'('")?;
            let mut params = Vec::new();
            if !self.at(&RParen) {
                loop {
                    params.push(self.parse_type()?);
                    if !self.eat(&Comma) {
                        break;
                    }
                }
            }
            self.expect(&RParen, "')'")?;
            let ret = if self.eat(&Arrow) {
                self.parse_type()?
            } else {
                Type::Named("void".to_string())
            };
            return Ok(Type::Fn(params, Box::new(ret)));
        }
        let (name, t) = self.ident_tok("a type")?;
        match name.as_str() {
            "int" => Ok(Type::Int),
            "float" => Ok(Type::Float),
            "bool" => Ok(Type::Bool),
            "str" => Ok(Type::Str),
            "ptr" => Ok(Type::Ptr),
            "list" => {
                self.expect(&LBracket, "'['")?;
                let elem = self.parse_type()?;
                self.expect(&RBracket, "']'")?;
                Ok(Type::List(Box::new(elem)))
            }
            "dict" => {
                self.expect(&LBracket, "'['")?;
                let k = self.parse_type()?;
                self.expect(&Comma, "',' between the key and value types")?;
                let v = self.parse_type()?;
                self.expect(&RBracket, "']'")?;
                Ok(Type::Dict(Box::new(k), Box::new(v)))
            }
            _ => {
                let _ = t;
                Ok(Type::Named(name))
            }
        }
    }
}

/// Move an f-string sub-expression's spans into the f-string token's byte
/// range so every expression in a module has a unique span start.
fn shift_spans(e: &mut Expr, off: u32) {
    e.span.start += off;
    e.span.end += off;
    match &mut e.kind {
        ExprKind::Unary(_, i) | ExprKind::Not(i) | ExprKind::Cast(_, i) => shift_spans(i, off),
        ExprKind::Binary(_, l, r) => {
            shift_spans(l, off);
            shift_spans(r, off);
        }
        ExprKind::And(l, r) | ExprKind::Or(l, r) | ExprKind::Compare(_, l, r) => {
            shift_spans(l, off);
            shift_spans(r, off);
        }
        ExprKind::Call(i, args) => {
            shift_spans(i, off);
            for arg in args {
                shift_spans(arg, off);
            }
        }
        ExprKind::Method(i, _, args) => {
            shift_spans(i, off);
            for arg in args {
                shift_spans(arg, off);
            }
        }
        ExprKind::Field(i, _) => shift_spans(i, off),
        ExprKind::Index(recv, idx) => {
            shift_spans(recv, off);
            shift_spans(idx, off);
        }
        ExprKind::Slice(recv, start, end) => {
            shift_spans(recv, off);
            if let Some(s) = start {
                shift_spans(s, off);
            }
            if let Some(x) = end {
                shift_spans(x, off);
            }
        }
        ExprKind::ListLit(args) => {
            for arg in args {
                shift_spans(arg, off);
            }
        }
        ExprKind::DictLit(pairs) => {
            for (k, v) in pairs {
                shift_spans(k, off);
                shift_spans(v, off);
            }
        }
        ExprKind::TupleLit(a, b) => {
            shift_spans(a, off);
            shift_spans(b, off);
        }
        ExprKind::FStr(parts) => {
            for part in parts {
                if let FStrPartAst::E(sub) = part {
                    shift_spans(sub, off);
                }
            }
        }
        _ => {}
    }
}

fn expr_kind_name(k: &ExprKind) -> &'static str {
    match k {
        ExprKind::Int(_) => "number",
        ExprKind::Float(_) => "number",
        ExprKind::Bool(_) => "bool",
        ExprKind::Nil => "nil",
        ExprKind::Str(_) => "string",
        ExprKind::FStr(_) => "f-string",
        ExprKind::Ident(_) => "name",
        _ => "expression",
    }
}

fn describe(k: &TokKind) -> String {
    match k {
        Amp => "'&'".into(),
        Pipe => "'|'".into(),
        Caret => "'^'".into(),
        Shl => "'<<'".into(),
        Shr => "'>>'".into(),
        Int(_) => "an integer".into(),
        Float(_) => "a float".into(),
        Str(s) => format!("string \"{}\"", s),
        FStr(_) => "an f-string".into(),
        Ident(n) => format!("'{}'", n),
        Comment(_) => "a comment".into(),
        Let => "'let'".into(),
        Fn => "'fn'".into(),
        If => "'if'".into(),
        Elif => "'elif'".into(),
        Else => "'else'".into(),
        For => "'for'".into(),
        In => "'in'".into(),
        While => "'while'".into(),
        Return => "'return'".into(),
        Break => "'break'".into(),
        Continue => "'continue'".into(),
        Pass => "'pass'".into(),
        Struct => "'struct'".into(),
        Extern => "'extern'".into(),
        True => "'true'".into(),
        False => "'false'".into(),
        Nil => "'nil'".into(),
        And => "'and'".into(),
        Or => "'or'".into(),
        Not => "'not'".into(),
        Raise => "'raise'".into(),
        Try => "'try'".into(),
        Except => "'except'".into(),
        Import => "'import'".into(),
        Export => "'export'".into(),
        From => "'from'".into(),
        Plus => "'+'".into(),
        Minus => "'-'".into(),
        Star => "'*'".into(),
        Slash => "'/'".into(),
        Percent => "'%'".into(),
        StarStar => "'**'".into(),
        Eq => "'='".into(),
        EqEq => "'=='".into(),
        NotEq => "'!='".into(),
        Lt => "'<'".into(),
        Gt => "'>'".into(),
        Le => "'<='".into(),
        Ge => "'>='".into(),
        LParen => "'('".into(),
        RParen => "')'".into(),
        LBracket => "'['".into(),
        RBracket => "']'".into(),
        LBrace => "'{'".into(),
        RBrace => "'}'".into(),
        Comma => "','".into(),
        Colon => "':'".into(),
        Dot => "'.'".into(),
        DotDot => "'..'".into(),
        DotDotEq => "'..='".into(),
        Arrow => "'->'".into(),
        Newline => "end of line".into(),
        Indent => "an indent".into(),
        Dedent => "a dedent".into(),
        Eof => "end of file".into(),
    }
}

impl TokKind {
    fn ident_name(&self) -> Option<&str> {
        match self {
            TokKind::Ident(n) => Some(n),
            _ => None,
        }
    }
}
