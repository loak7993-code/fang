//! leo-core: compiler library for the Leo programming language.
//!
//! Pipeline: lexer -> parser (AST) -> sema (typed AST) -> codegen (C).

pub mod ast;
pub mod fmt;
pub mod lexer;
pub mod parser;
pub mod sema;

pub use ast::*;
pub use fmt::format_module;
pub use lexer::{lex_expression, tokenize_kinds, FStrPart, LexError, LexResult, Lexer, TokKind, Token};
pub use parser::{ParseError, Parser};
pub use sema::{analyze, Analysis, CallTarget, ConstVal, DictOp, FnInfo, ListOp, SemaError, StructInfo, Ty};

/// Lex and parse a source string in one call.
pub fn parse_source(name: &str, src: &str) -> Result<Module, String> {
    let toks = Lexer::new(src).tokenize().map_err(|e| e.to_string())?;
    Parser::parse_module(name, &toks).map_err(|e| e.to_string())
}
