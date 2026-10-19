//! Lexer unit tests. Each test asserts the exact token-kind stream.

use leo_core::{tokenize_kinds, TokKind::*, Lexer};

fn kinds(src: &str) -> Vec<leo_core::TokKind> {
    tokenize_kinds(src).expect("lex error")
}

#[test]
fn hello_world() {
    let k = kinds("fn main():\n  print(\"hello world\")\n");
    assert_eq!(
        k,
        vec![
            Fn, Ident("main".into()), LParen, RParen, Colon, Newline,
            Indent, Ident("print".into()), LParen, Str("hello world".into()), RParen, Newline,
            Dedent, Eof
        ]
    );
}

#[test]
fn numbers_int_float_exponent() {
    let k = kinds("let a = 42\nlet b = 3.25\nlet c = 1e10\nlet d = 2.5e-3\n");
    assert_eq!(
        k[..4],
        [Let, Ident("a".into()), Eq, Int(42)]
    );
    assert!(matches!(k[8], Float(f) if (f - 3.25).abs() < 1e-12));
    assert!(matches!(k[13], Float(f) if f == 1e10));
    assert!(matches!(k[18], Float(f) if (f - 2.5e-3).abs() < 1e-12));
}

#[test]
fn range_not_swallowed_by_float() {
    let k = kinds("for i in 0..10:\n  pass\n");
    assert_eq!(
        k[..7],
        [For, Ident("i".into()), In, Int(0), DotDot, Int(10), Colon]
    );
}

#[test]
fn inclusive_range() {
    let k = kinds("for i in 0..=9:\n  pass\n");
    assert_eq!(k[4], DotDotEq);
}

#[test]
fn operators() {
    let k = kinds("a = b + c * d - e / f % g ** h\n");
    assert_eq!(
        k[..15],
        [
            Ident("a".into()), Eq, Ident("b".into()), Plus, Ident("c".into()), Star,
            Ident("d".into()), Minus, Ident("e".into()), Slash, Ident("f".into()),
            Percent, Ident("g".into()), StarStar, Ident("h".into())
        ]
    );
}

#[test]
fn comparisons_and_arrow() {
    let k = kinds("a == b != c < d > e <= f >= g -> int\n");
    assert_eq!(
        k[..14],
        [
            Ident("a".into()), EqEq, Ident("b".into()), NotEq, Ident("c".into()),
            Lt, Ident("d".into()), Gt, Ident("e".into()), Le, Ident("f".into()),
            Ge, Ident("g".into()), Arrow
        ]
    );
    assert_eq!(k[14], Ident("int".into()));
}

#[test]
fn keywords() {
    let k = kinds("let fn if elif else for in while return break continue pass struct extern true false nil and or not raise try except import export\n");
    assert_eq!(
        k[..26],
        [
            Let, Fn, If, Elif, Else, For, In, While, Return, Break, Continue, Pass,
            Struct, Extern, True, False, Nil, And, Or, Not, Raise, Try, Except,
            Import, Export, Newline
        ]
    );
}

#[test]
fn strings_with_escapes() {
    let k = kinds("let s = \"a\\nb\\tc\\\\d\\\"e\"\n");
    assert_eq!(k[3], leo_core::TokKind::Str("a\nb\tc\\d\"e".into()));
}

#[test]
fn fstring_basic() {
    let k = kinds("let s = f\"hi {name}!\"\n");
    match &k[3] {
        leo_core::TokKind::FStr(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], leo_core::FStrPart::Literal("hi ".into()));
            assert!(matches!(&parts[1], leo_core::FStrPart::Expr(text, toks) if text == "name" && toks.len() == 2));
            assert_eq!(parts[2], leo_core::FStrPart::Literal("!".into()));
        }
        other => panic!("expected f-string, got {:?}", other),
    }
}

#[test]
fn fstring_escaped_braces() {
    let k = kinds("let s = f\"{{a}}\"\n");
    match &k[3] {
        leo_core::TokKind::FStr(parts) => {
            assert_eq!(parts.len(), 1);
            assert_eq!(parts[0], leo_core::FStrPart::Literal("{a}".into()));
        }
        other => panic!("expected f-string, got {:?}", other),
    }
}

#[test]
fn fstring_nested_expr_with_call() {
    let k = kinds("let s = f\"v={xs.len()}\"\n");
    match &k[3] {
        leo_core::TokKind::FStr(parts) => {
            assert!(matches!(&parts[1], leo_core::FStrPart::Expr(_, toks) if toks.len() >= 4));
        }
        other => panic!("expected f-string, got {:?}", other),
    }
}

#[test]
fn comments_are_tokens_and_blank_lines_skipped() {
    let k = kinds("# leading comment\n\nlet a = 1  # trailing\n\n# another\nlet b = 2\n");
    assert_eq!(
        k[..8],
        [
            Comment("leading comment".into()), Newline,
            Let, Ident("a".into()), Eq, Int(1), Comment("trailing".into()), Newline
        ]
    );
    // Blank lines produce no tokens; the next comment runs to the final let.
    assert_eq!(
        k[8..],
        [Comment("another".into()), Newline, Let, Ident("b".into()), Eq, Int(2), Newline, Eof]
    );
}

#[test]
fn newlines_suppressed_inside_brackets() {
    let k = kinds("let xs = [\n  1,\n  2,\n]\n");
    // No Newline tokens between the brackets.
    let inner: Vec<_> = k
        .iter()
        .take_while(|t| !matches!(t, leo_core::TokKind::Eof))
        .collect();
    let newlines = inner
        .iter()
        .filter(|t| matches!(t, leo_core::TokKind::Newline))
        .count();
    assert_eq!(newlines, 1); // only the final Newline after ']'
    assert!(k.contains(&leo_core::TokKind::LBracket));
    assert!(k.contains(&leo_core::TokKind::RBracket));
}

#[test]
fn nested_indent_dedent() {
    let k = kinds("fn f():\n  if a:\n    pass\n  b\n");
    assert_eq!(
        k.iter().filter(|t| matches!(t, leo_core::TokKind::Indent)).count(),
        2
    );
    assert_eq!(
        k.iter().filter(|t| matches!(t, leo_core::TokKind::Dedent)).count(),
        2
    );
}

#[test]
fn dedent_multiple_levels_at_once() {
    let k = kinds("if a:\n  if b:\n    pass\nx\n");
    // From depth 2 straight back to 0: two Dedents before x.
    let tail = &k[k.len() - 5..];
    assert_eq!(tail[0], leo_core::TokKind::Dedent);
    assert_eq!(tail[1], leo_core::TokKind::Dedent);
    assert_eq!(tail[2], Ident("x".into()));
}

#[test]
fn error_on_bad_indent_width() {
    let r = tokenize_kinds("if a:\n    pass\n   x\n");
    assert!(r.is_err());
}

#[test]
fn error_on_tab() {
    let r = tokenize_kinds("if a:\n\tpass\n");
    assert!(r.is_err());
}

#[test]
fn error_on_unterminated_string() {
    let r = tokenize_kinds("let s = \"oops\n");
    assert!(r.is_err());
}

#[test]
fn error_on_unknown_escape() {
    let r = tokenize_kinds("let s = \"a\\qb\"\n");
    assert!(r.is_err());
}

#[test]
fn error_on_unknown_char() {
    let r = tokenize_kinds("let a = 1 & 2\n");
    assert!(r.is_err());
}

#[test]
fn token_positions() {
    let toks = Lexer::new("let a = 1\n  let b = 2\n")
        .tokenize()
        .unwrap();
    let first = &toks[0];
    assert_eq!((first.line, first.col), (1, 1));
    let b_tok = toks.iter().find(|t| matches!(&t.kind, leo_core::TokKind::Ident(n) if n == "b")).unwrap();
    assert_eq!((b_tok.line, b_tok.col), (2, 7));
}

#[test]
fn no_trailing_newline_needed_at_eof() {
    let k = kinds("let a = 1");
    assert_eq!(k.last(), Some(&Eof));
    assert_eq!(k[k.len() - 2], Newline);
}
