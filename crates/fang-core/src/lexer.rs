//! Lexer for Fang.
//!
//! Produces a flat token stream with indentation-sensitive block structure
//! encoded as Indent/Dedent tokens, Python-style. Tokens carry line, column,
//! and byte spans; every later phase reports errors through those spans.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum FStrPart {
    Literal(String),
    /// Raw expression text, its lexed token stream, and the byte offset of
    /// the '{' relative to the f-string token start. The parser uses it to
    /// give every part a unique span shift.
    Expr(String, Vec<Token>, u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    Int(i64),
    Float(f64),
    Str(String),
    FStr(Vec<FStrPart>),
    Ident(String),

    Let, Fn, If, Elif, Else, For, In, While, Return, Break, Continue, Pass,
    Struct, Extern, True, False, Nil, And, Or, Not, Raise, Try, Except,
    Import, Export, From,

    Plus, Minus, Star, Slash, Percent, StarStar, Amp, Pipe, Caret, Shl, Shr,
    Eq, EqEq, NotEq, Lt, Gt, Le, Ge,
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Comma, Colon, Dot, DotDot, DotDotEq, Arrow,
    /// Comment text (without the leading '#').
    Comment(String),
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokKind,
    pub line: u32,
    pub col: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub msg: String,
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (line {}, column {})", self.msg, self.line, self.col)
    }
}

pub type LexResult<T> = Result<T, LexError>;

fn err<T>(msg: impl Into<String>, line: u32, col: u32) -> LexResult<T> {
    Err(LexError { msg: msg.into(), line, col })
}

/// Keyword table.
fn keyword(s: &str) -> Option<TokKind> {
    Some(match s {
        "let" => TokKind::Let,
        "fn" => TokKind::Fn,
        "if" => TokKind::If,
        "elif" => TokKind::Elif,
        "else" => TokKind::Else,
        "for" => TokKind::For,
        "in" => TokKind::In,
        "while" => TokKind::While,
        "return" => TokKind::Return,
        "break" => TokKind::Break,
        "continue" => TokKind::Continue,
        "pass" => TokKind::Pass,
        "struct" => TokKind::Struct,
        "extern" => TokKind::Extern,
        "true" => TokKind::True,
        "false" => TokKind::False,
        "nil" => TokKind::Nil,
        "and" => TokKind::And,
        "or" => TokKind::Or,
        "not" => TokKind::Not,
        "raise" => TokKind::Raise,
        "try" => TokKind::Try,
        "except" => TokKind::Except,
        "import" => TokKind::Import,
        "export" => TokKind::Export,
        "from" => TokKind::From,
        _ => return None,
    })
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
    indents: Vec<u32>,
    tokens: Vec<Token>,
    /// Bracket nesting depth. Newlines and indentation are ignored inside.
    depth: u32,
    at_line_start: bool,
    /// Expression-only mode for f-string interiors: no newlines, no indents.
    expr_mode: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            indents: vec![0],
            tokens: Vec::new(),
            depth: 0,
            at_line_start: true,
            expr_mode: false,
        }
    }

    /// Lex a full source file.
    pub fn tokenize(mut self) -> LexResult<Vec<Token>> {
        while !self.at_eof() {
            if self.at_line_start && self.depth == 0 && !self.expr_mode {
                match self.handle_line_start()? {
                    LineStart::Skip => continue,
                    LineStart::Comment | LineStart::Code => {}
                }
            }
            self.at_line_start = false;
            let c = self.peek();
            match c {
                b' ' | b'\r' => self.advance(),
                b'\t' => {
                    return err("tab character; Fang uses 4 spaces for indentation", self.line, self.col)
                }
                b'\n' => {
                    if self.depth > 0 || self.expr_mode {
                        // Newlines inside brackets are not separators.
                        self.advance();
                    } else {
                        let (line, col, start, end) = self.here();
                        self.advance();
                        self.push(TokKind::Newline, line, col, start, end);
                        self.at_line_start = true;
                    }
                }
                b'#' => {
                    if self.expr_mode {
                        while !self.at_eof() && self.peek() != b'\n' {
                            self.advance();
                        }
                    } else {
                        let (line, col, start, _) = self.here();
                        let text_start = self.pos + 1;
                        while !self.at_eof() && self.peek() != b'\n' {
                            self.advance();
                        }
                        let raw = std::str::from_utf8(&self.src[text_start..self.pos]).unwrap_or("");
                        let text = raw.trim().to_string();
                        let end = self.pos as u32;
                        self.push_with(TokKind::Comment(text), line, col, start, end);
                    }
                }
                b'"' => self.lex_string(false)?,
                b'0'..=b'9' => self.lex_number()?,
                b'_' | b'a'..=b'z' | b'A'..=b'Z' => self.lex_ident_or_string()?,
                b'+' => self.simple(TokKind::Plus),
                b'-' => {
                    if self.peek2() == b'>' {
                        self.simple2(TokKind::Arrow);
                    } else {
                        self.simple(TokKind::Minus);
                    }
                }
                b'*' => {
                    if self.peek2() == b'*' {
                        self.simple2(TokKind::StarStar);
                    } else {
                        self.simple(TokKind::Star);
                    }
                }
                b'/' => self.simple(TokKind::Slash),
                b'&' => self.simple(TokKind::Amp),
                b'|' => self.simple(TokKind::Pipe),
                b'^' => self.simple(TokKind::Caret),
                b'%' => self.simple(TokKind::Percent),
                b'=' => {
                    if self.peek2() == b'=' {
                        self.simple2(TokKind::EqEq);
                    } else {
                        self.simple(TokKind::Eq);
                    }
                }
                b'!' => {
                    if self.peek2() == b'=' {
                        self.simple2(TokKind::NotEq);
                    } else {
                        return err("unexpected '!'; equality is '!='", self.line, self.col);
                    }
                }
                b'<' => {
                    if self.peek2() == b'=' {
                        self.simple2(TokKind::Le);
                    } else if self.peek2() == b'<' {
                        self.simple2(TokKind::Shl);
                    } else {
                        self.simple(TokKind::Lt);
                    }
                }
                b'>' => {
                    if self.peek2() == b'=' {
                        self.simple2(TokKind::Ge);
                    } else if self.peek2() == b'>' {
                        self.simple2(TokKind::Shr);
                    } else {
                        self.simple(TokKind::Gt);
                    }
                }
                b'(' => {
                    self.depth += 1;
                    self.simple(TokKind::LParen);
                }
                b')' => {
                    if self.depth == 0 {
                        return err("unmatched ')'", self.line, self.col);
                    }
                    self.depth -= 1;
                    self.simple(TokKind::RParen);
                }
                b'[' => {
                    self.depth += 1;
                    self.simple(TokKind::LBracket);
                }
                b']' => {
                    if self.depth == 0 {
                        return err("unmatched ']'", self.line, self.col);
                    }
                    self.depth -= 1;
                    self.simple(TokKind::RBracket);
                }
                b'{' => {
                    self.depth += 1;
                    self.simple(TokKind::LBrace);
                }
                b'}' => {
                    if self.depth == 0 {
                        return err("unmatched '}'", self.line, self.col);
                    }
                    self.depth -= 1;
                    self.simple(TokKind::RBrace);
                }
                b',' => self.simple(TokKind::Comma),
                b':' => self.simple(TokKind::Colon),
                b'.' => {
                    if self.peek2() == b'.' {
                        if self.peek3() == b'=' {
                            self.simple3(TokKind::DotDotEq);
                        } else {
                            self.simple2(TokKind::DotDot);
                        }
                    } else {
                        self.simple(TokKind::Dot);
                    }
                }
                other => {
                    return err(
                        format!("unexpected character '{}'", other as char),
                        self.line,
                        self.col,
                    )
                }
            }
        }

        // Terminate: ensure a final Newline unless the file is empty or the
        // last line was blank, then balance indents, then Eof.
        let last_is_newline = matches!(
            self.tokens.last().map(|t| &t.kind),
            Some(TokKind::Newline) | Some(TokKind::Indent) | Some(TokKind::Dedent)
        );
        let non_blank = self
            .tokens
            .iter()
            .any(|t| !matches!(t.kind, TokKind::Newline | TokKind::Indent | TokKind::Dedent));
        if non_blank && !last_is_newline {
            let (line, col, start, _) = self.here();
            self.push(TokKind::Newline, line, col, start, start);
        }
        let (line, col, start, _) = self.here();
        while self.indents.len() > 1 {
            self.indents.pop();
            self.push(TokKind::Dedent, line, col, start, start);
        }
        self.push(TokKind::Eof, line, col, start, start);
        Ok(self.tokens)
    }

    /// Handle the start of a line: measure indentation, emit Indent/Dedent.
    fn handle_line_start(&mut self) -> LexResult<LineStart> {
        let mut width = 0u32;
        loop {
            match self.peek() {
                b' ' => {
                    width += 1;
                    self.advance();
                }
                b'\t' => {
                    return err("tab in indentation; use 4 spaces", self.line, self.col)
                }
                b'\n' => {
                    self.advance();
                    return Ok(LineStart::Skip);
                }
                b'\r' => {
                    self.advance();
                }
                b'#' => {
                    // Comment-only line: comments never dedent. A comment
                    // deeper than the current level opens one, which the
                    // next code line either joins or closes.
                    let cur = *self.indents.last().unwrap();
                    if width > cur {
                        self.indents.push(width);
                        let (line, col, start, _) = self.here();
                        self.push(TokKind::Indent, line, col, start, start);
                    }
                    return Ok(LineStart::Comment);
                }
                _ => break,
            }
        }
        let cur = *self.indents.last().unwrap();
        let (line, col, start, _) = self.here();
        if width > cur {
            self.indents.push(width);
            self.push(TokKind::Indent, line, col, start, start);
        } else if width < cur {
            while *self.indents.last().unwrap() > width {
                self.indents.pop();
                let (line, col, start, _) = self.here();
                self.push(TokKind::Dedent, line, col, start, start);
            }
            if *self.indents.last().unwrap() != width {
                return err(
                    format!(
                        "indentation of {} spaces does not match any outer level",
                        width
                    ),
                    line,
                    col,
                );
            }
        }
        Ok(LineStart::Code)
    }

    fn lex_ident_or_string(&mut self) -> LexResult<()> {
        let start_pos = self.pos;
        let (line, col, start, _) = self.here();
        while matches!(self.peek(), b'_' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9') {
            self.advance();
        }
        let text = std::str::from_utf8(&self.src[start_pos..self.pos]).unwrap().to_string();
        // f-string prefix: identifier "f" immediately followed by a quote.
        if text == "f" && self.peek() == b'"' {
            self.lex_string(true)?;
            return Ok(());
        }
        let kind = match keyword(&text) {
            Some(k) => k,
            None => TokKind::Ident(text),
        };
        let end = self.pos as u32;
        self.push_with(kind, line, col, start, end);
        Ok(())
    }

    fn lex_number(&mut self) -> LexResult<()> {
        let start_pos = self.pos;
        let (line, col, start, _) = self.here();
        // Hex 0x.. and binary 0b.. literals.
        if self.peek() == b'0' && matches!(self.peek2(), b'x' | b'X' | b'b' | b'B') {
            let base = if matches!(self.peek2(), b'x' | b'X') { 16 } else { 2 };
            self.advance(); // 0
            self.advance(); // x/b
            let digits_start = self.pos;
            while self.peek().is_ascii_hexdigit() {
                self.advance();
            }
            let text = std::str::from_utf8(&self.src[digits_start..self.pos]).unwrap();
            if base == 2 && text.chars().any(|c| c > '1') {
                return err(
                    format!("invalid binary literal 0b{}", text),
                    line,
                    col,
                );
            }
            let v = i64::from_str_radix(text, base).map_err(|_| LexError {
                msg: format!("invalid base-{} literal '{}'", base, text),
                line,
                col,
            })?;
            let end = self.pos as u32;
            self.push_with(TokKind::Int(v), line, col, start, end);
            return Ok(());
        }
        while self.peek().is_ascii_digit() {
            self.advance();
        }
        let mut is_float = false;
        // Fraction: '.' followed by a digit. '..' is a range, so "0..9" stays int.
        if self.peek() == b'.' && self.peek2().is_ascii_digit() {
            is_float = true;
            self.advance(); // '.'
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }
        // Exponent: e/E [+-] digits.
        if matches!(self.peek(), b'e' | b'E') {
            let mut lookahead = self.pos + 1;
            if matches!(self.src.get(lookahead), Some(b'+') | Some(b'-')) {
                lookahead += 1;
            }
            if self.src.get(lookahead).is_some_and(|b| b.is_ascii_digit()) {
                is_float = true;
                self.advance(); // e
                if matches!(self.peek(), b'+' | b'-') {
                    self.advance();
                }
                while self.peek().is_ascii_digit() {
                    self.advance();
                }
            }
        }
        let text = std::str::from_utf8(&self.src[start_pos..self.pos]).unwrap();
        let kind = if is_float {
            TokKind::Float(text.parse::<f64>().map_err(|_| LexError {
                msg: format!("invalid float literal '{}'", text),
                line,
                col,
            })?)
        } else {
            match text.parse::<i64>() {
                Ok(v) => TokKind::Int(v),
                Err(_) => {
                    return err(
                        format!("integer literal '{}' does not fit in 64 bits", text),
                        line,
                        col,
                    )
                }
            }
        };
        let end = self.pos as u32;
        self.push_with(kind, line, col, start, end);
        Ok(())
    }

    fn lex_string(&mut self, fstring: bool) -> LexResult<()> {
        let (line, col, start, _) = self.here();
        self.advance(); // opening quote
        let mut value = String::new();
        let mut parts: Vec<FStrPart> = Vec::new();
        let mut lit_buf = String::new();
        loop {
            if self.at_eof() {
                return err("unterminated string", line, col);
            }
            let c = self.peek();
            match c {
                b'\n' => return err("unterminated string (newlines inside strings are not supported)", line, col),
                b'"' => {
                    self.advance();
                    break;
                }
                b'\\' => {
                    self.advance();
                    let e = self.peek();
                    let esc = match e {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'0' => '\0',
                        b'\\' => '\\',
                        b'"' => '"',
                        b'e' => '\x1b',
                        b'x' | b'u' => {
                            // \xHH hex escape (u accepts \uHHHH too)
                            let n = if e == b'x' { 2 } else { 4 };
                            self.advance();
                            let mut v: u32 = 0;
                            for _ in 0..n {
                                let h = self.peek();
                                let d = match h {
                                    b'0'..=b'9' => (h - b'0') as u32,
                                    b'a'..=b'f' => (h - b'a' + 10) as u32,
                                    b'A'..=b'F' => (h - b'A' + 10) as u32,
                                    _ => {
                                        return err(
                                            "hex escape needs hex digits".to_string(),
                                            self.line,
                                            self.col,
                                        )
                                    }
                                };
                                v = v * 16 + d;
                                self.advance();
                            }
                            let c = char::from_u32(v).unwrap_or('\u{fffd}');
                            value.push(c);
                            lit_buf.push(c);
                            continue;
                        }
                        _ => {
                            return err(
                                format!("unknown escape '\\{}'", e as char),
                                self.line,
                                self.col,
                            )
                        }
                    };
                    value.push(esc);
                    lit_buf.push(esc);
                    self.advance();
                }
                b'{' if fstring => {
                    if self.peek2() == b'{' {
                        self.advance();
                        self.advance();
                        value.push('{');
                        lit_buf.push('{');
                        continue;
                    }
                    // Expression part: collect until the matching close brace,
                    // respecting nesting and embedded strings.
                    let expr_rel = (self.pos - start as usize) as u32;
                    self.advance(); // '{'
                    let expr_start = self.pos;
                    let mut brace = 1u32;
                    let mut in_str = false;
                    while brace > 0 {
                        if self.at_eof() {
                            return err("unterminated '{' in f-string", line, col);
                        }
                        let ch = self.peek();
                        if in_str {
                            if ch == b'\\' {
                                self.advance();
                            } else if ch == b'"' {
                                in_str = false;
                            }
                        } else {
                            match ch {
                                b'"' => in_str = true,
                                b'{' => brace += 1,
                                b'}' => brace -= 1,
                                _ => {}
                            }
                        }
                        self.advance();
                    }
                    let expr_end = self.pos - 1; // position of the closing '}'
                    let expr_text =
                        std::str::from_utf8(&self.src[expr_start..expr_end]).unwrap().to_string();
                    if !lit_buf.is_empty() {
                        parts.push(FStrPart::Literal(std::mem::take(&mut lit_buf)));
                    }
                    let toks = lex_expression(&expr_text).map_err(|mut e| {
                        e.line += line - 1;
                        e
                    })?;
                    parts.push(FStrPart::Expr(expr_text, toks, expr_rel));
                }
                b'}' if fstring => {
                    if self.peek2() == b'}' {
                        self.advance();
                        self.advance();
                        value.push('}');
                        lit_buf.push('}');
                        continue;
                    }
                    return err("single '}' inside an f-string; use '}}'", self.line, self.col);
                }
                _ => {
                    let ch = self.peek() as char;
                    // Multibyte safety: copy the raw byte for non-ASCII.
                    if ch.is_ascii() {
                        value.push(ch);
                        lit_buf.push(ch);
                        self.advance();
                    } else {
                        // Decode UTF-8 sequence.
                        let len = utf8_len(self.src[self.pos]);
                        let s = std::str::from_utf8(&self.src[self.pos..self.pos + len])
                            .map_err(|_| LexError {
                                msg: "invalid UTF-8 in string".into(),
                                line: self.line,
                                col: self.col,
                            })?;
                        value.push_str(s);
                        lit_buf.push_str(s);
                        for _ in 0..len {
                            self.advance();
                        }
                    }
                }
            }
        }
        let end = self.pos as u32;
        if fstring {
            if !lit_buf.is_empty() {
                parts.push(FStrPart::Literal(lit_buf));
            }
            self.push_with(TokKind::FStr(parts), line, col, start, end);
        } else {
            self.push_with(TokKind::Str(value), line, col, start, end);
        }
        Ok(())
    }

    // --- low-level helpers ---

    fn at_eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn peek(&self) -> u8 {
        self.src.get(self.pos).copied().unwrap_or(0)
    }

    fn peek2(&self) -> u8 {
        self.src.get(self.pos + 1).copied().unwrap_or(0)
    }

    fn peek3(&self) -> u8 {
        self.src.get(self.pos + 2).copied().unwrap_or(0)
    }

    fn advance(&mut self) {
        if self.peek() == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += 1;
    }

    fn here(&self) -> (u32, u32, u32, u32) {
        (self.line, self.col, self.pos as u32, self.pos as u32)
    }

    fn push(&mut self, kind: TokKind, line: u32, col: u32, start: u32, end: u32) {
        self.tokens.push(Token { kind, line, col, start, end });
    }

    fn push_with(&mut self, kind: TokKind, line: u32, col: u32, start: u32, end: u32) {
        self.tokens.push(Token { kind, line, col, start, end });
    }

    fn simple(&mut self, kind: TokKind) {
        let (line, col, start, _) = self.here();
        self.advance();
        let end = self.pos as u32;
        self.push_with(kind, line, col, start, end);
    }

    fn simple2(&mut self, kind: TokKind) {
        let (line, col, start, _) = self.here();
        self.advance();
        self.advance();
        let end = self.pos as u32;
        self.push_with(kind, line, col, start, end);
    }

    fn simple3(&mut self, kind: TokKind) {
        let (line, col, start, _) = self.here();
        self.advance();
        self.advance();
        self.advance();
        let end = self.pos as u32;
        self.push_with(kind, line, col, start, end);
    }
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

enum LineStart {
    /// Blank line: consumed entirely, no tokens.
    Skip,
    /// Comment-only line: position left on '#'; main loop emits the Comment.
    Comment,
    /// Code line: indentation processed, position on the first code byte.
    Code,
}

/// Lex the interior of an f-string expression: single expression, no
/// newlines, no indentation tokens.
pub fn lex_expression(src: &str) -> LexResult<Vec<Token>> {
    let mut lx = Lexer::new(src);
    lx.expr_mode = true;
    lx.at_line_start = false;
    let toks = lx.tokenize()?;
    Ok(toks
        .into_iter()
        .filter(|t| !matches!(t.kind, TokKind::Newline | TokKind::Comment(_)))
        .collect())
}

/// Convenience: lex a whole source string to token kinds only (tests).
pub fn tokenize_kinds(src: &str) -> LexResult<Vec<TokKind>> {
    Ok(Lexer::new(src).tokenize()?.into_iter().map(|t| t.kind).collect())
}
