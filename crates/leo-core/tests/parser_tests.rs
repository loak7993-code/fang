//! Parser tests: AST shapes, error paths, and formatter round-trips.

use leo_core::{parse_source, Lexer, Parser};
use leo_core::ast::*;

fn parse(src: &str) -> Module {
    parse_source("test", src).unwrap_or_else(|e| panic!("parse error: {}", e))
}

#[test]
fn fn_decl_with_params_and_ret() {
    let m = parse("fn add(a: int, b: int) -> int:\n  return a + b\n");
    match &m.items[0] {
        Item::Fn(f) => {
            assert_eq!(f.name, "add");
            assert!(f.receiver.is_none());
            assert_eq!(f.params.len(), 2);
            assert_eq!(f.params[0].name, "a");
            assert_eq!(f.params[0].ty, Type::Int);
            assert_eq!(f.ret, Some(Type::Int));
            assert_eq!(f.body.len(), 1);
        }
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn method_decl() {
    let m = parse("struct P:\n  x: float\n\nfn P.get(self) -> float:\n  return self.x\n");
    match &m.items[1] {
        Item::Fn(f) => {
            assert_eq!(f.receiver.as_deref(), Some("P"));
            assert_eq!(f.name, "get");
            assert!(f.params[0].is_self);
        }
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn struct_with_field_comment() {
    let m = parse("struct P:\n  x: float  # horizontal\n  y: float\n");
    match &m.items[0] {
        Item::Struct(s) => {
            assert_eq!(s.name, "P");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].comment.as_deref(), Some("horizontal"));
            assert_eq!(s.fields[1].comment, None);
        }
        other => panic!("expected struct, got {:?}", other),
    }
}

#[test]
fn fstring_parses_nested_expression() {
    let m = parse("fn main():\n  print(f\"v={xs[0] + 1}\")\n");
    match &m.items[0] {
        Item::Fn(f) => match &f.body[0].kind {
            StmtKind::Expr(e) => match &e.kind {
                ExprKind::Call(_, args) => match &args[0].kind {
                    ExprKind::FStr(parts) => {
                        assert_eq!(parts.len(), 2);
                        match &parts[1] {
                            FStrPartAst::E(sub) => {
                                assert!(matches!(&sub.kind, ExprKind::Binary(BinOp::Add, _, _)));
                            }
                            other => panic!("expected expr part, got {:?}", other),
                        }
                    }
                    other => panic!("expected f-string, got {:?}", other),
                },
                other => panic!("expected call, got {:?}", other),
            },
            other => panic!("expected expr stmt, got {:?}", other),
        },
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn tuple_destructure_and_pair_loop() {
    let m = parse("fn main():\n  let a, b = pair()\n  for k, v in d.items():\n    pass\n");
    match &m.items[0] {
        Item::Fn(f) => {
            match &f.body[0].kind {
                StmtKind::Let { binders, .. } => assert_eq!(binders, &["a", "b"]),
                other => panic!("expected let, got {:?}", other),
            }
            match &f.body[1].kind {
                StmtKind::For { binders, .. } => assert_eq!(binders, &["k", "v"]),
                other => panic!("expected for, got {:?}", other),
            }
        }
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn for_range_exclusive_and_inclusive() {
    let m = parse("fn main():\n  for i in 0..10:\n    pass\n  for j in 0..=9:\n    pass\n");
    match &m.items[0] {
        Item::Fn(f) => {
            match &f.body[0].kind {
                StmtKind::For { iter: Iter::Range { inclusive, .. }, .. } => assert!(!inclusive),
                other => panic!("expected range, got {:?}", other),
            }
            match &f.body[1].kind {
                StmtKind::For { iter: Iter::Range { inclusive, .. }, .. } => assert!(inclusive),
                other => panic!("expected range, got {:?}", other),
            }
        }
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn slices_and_cast() {
    let m = parse("fn main():\n  a = xs[1:3]\n  b = xs[:2]\n  c = xs[2:]\n  d = cast[ptr](a)\n");
    match &m.items[0] {
        Item::Fn(f) => {
            assert!(matches!(&f.body[0].kind, StmtKind::Assign { .. }));
            match &f.body[3].kind {
                StmtKind::Assign { value, .. } => {
                    assert!(matches!(&value.kind, ExprKind::Cast(Type::Ptr, _)));
                }
                other => panic!("expected assign, got {:?}", other),
            }
        }
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn error_chained_comparison() {
    let r = parse_source("t", "fn main():\n  if a < b < c:\n    pass\n");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("chained comparisons"));
}

#[test]
fn error_one_tuple() {
    let r = parse_source("t", "fn main():\n  x = (1,)\n");
    assert!(r.is_err());
}

#[test]
fn error_three_tuple() {
    let r = parse_source("t", "fn main():\n  x = (1, 2, 3)\n");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("two values"));
}

#[test]
fn error_assignment_to_call() {
    let r = parse_source("t", "fn main():\n  f() = 1\n");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("cannot assign"));
}

#[test]
fn pow_is_right_associative_and_binds_over_unary_minus() {
    let m = parse("fn main():\n  x = -2 ** 3 ** 2\n");
    match &m.items[0] {
        Item::Fn(f) => match &f.body[0].kind {
            StmtKind::Assign { value, .. } => match &value.kind {
                ExprKind::Unary(BinOp::Sub, inner) => match &inner.kind {
                    ExprKind::Binary(BinOp::Pow, l, r) => {
                        assert!(matches!(&l.kind, ExprKind::Int(2)));
                        assert!(matches!(&r.kind, ExprKind::Binary(BinOp::Pow, _, _)));
                    }
                    other => panic!("expected pow, got {:?}", other),
                },
                other => panic!("expected unary, got {:?}", other),
            },
            other => panic!("expected assign, got {:?}", other),
        },
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn comment_statements_preserved() {
    let m = parse("# header\n\nfn main():\n  # lead\n  pass\n  pass  # trailing\n");
    assert!(matches!(&m.items[0], Item::Comment(t) if t == "header"));
    match &m.items[1] {
        Item::Fn(f) => {
            let kinds: Vec<String> = f
                .body
                .iter()
                .map(|s| format!("{:?}", s.kind))
                .collect();
            assert!(
                matches!(&f.body[0].kind, StmtKind::Comment(t) if t == "lead"),
                "body: {:#?}",
                kinds
            );
            assert!(matches!(&f.body[1].kind, StmtKind::Pass), "body: {:#?}", kinds);
            assert!(matches!(&f.body[2].kind, StmtKind::Pass), "body: {:#?}", kinds);
            // The trailing comment reflows to its own statement, last in the block.
            assert!(
                matches!(&f.body[3].kind, StmtKind::Comment(t) if t == "trailing"),
                "body: {:#?}",
                kinds
            );
        }
        other => panic!("expected fn, got {:?}", other),
    }
}

#[test]
fn all_fixture_files_parse() {
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |e| e == "leo") {
            let src = std::fs::read_to_string(&path).unwrap();
            parse_source("fixture", &src)
                .unwrap_or_else(|e| panic!("{} failed to parse: {}", path.display(), e));
        }
    }
}

// --- formatter ---

fn roundtrip(src: &str) {
    let m1 = parse(src);
    let f1 = leo_core::format_module(&m1);
    let m2 = parse(&f1);
    let f2 = leo_core::format_module(&m2);
    assert_eq!(f1, f2, "formatter is not a fixpoint for:\n{}", src);
}

#[test]
fn formatter_fixpoint_on_fixtures() {
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |e| e == "leo") {
            let src = std::fs::read_to_string(&path).unwrap();
            roundtrip(&src);
        }
    }
}

#[test]
fn formatter_canonical_output() {
    let src = "fn   main ( ) :\n    print( \"hi\" )\n";
    let m = parse(src);
    let out = leo_core::format_module(&m);
    assert_eq!(out, "fn main():\n  print(\"hi\")\n");
}

#[test]
fn formatter_parenthesizes_correctly() {
    let src = "fn main():\n  x = (a + b) * c\n  y = a + (b * c)\n  z = -(a ** 2)\n  w = not (a and b)\n";
    let m = parse(src);
    let out = leo_core::format_module(&m);
    assert!(out.contains("x = (a + b) * c"), "got: {}", out);
    assert!(out.contains("y = a + b * c"), "got: {}", out);
    assert!(out.contains("z = -a ** 2"), "got: {}", out);
    assert!(out.contains("w = not (a and b)"), "got: {}", out);
}

#[test]
fn formatter_preserves_elif_else_and_comments() {
    let src = "fn f(n: int) -> str:\n  # decide\n  if n == 0:\n    return \"zero\"\n  elif n == 1:\n    return \"one\"\n  else:\n    return \"many\"\n";
    let m = parse(src);
    let out = leo_core::format_module(&m);
    roundtrip(src);
    assert!(out.contains("# decide"));
    assert!(out.contains("elif n == 1:"));
    assert!(out.contains("else:"));
}
