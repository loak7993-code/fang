//! Sema tests: inference, method sugar, struct methods, error paths.

use leo_core::{parse_source, sema};

fn check(src: &str) -> sema::Analysis {
    let m = parse_source("main", src).unwrap_or_else(|e| panic!("parse: {}", e));
    sema::analyze(vec![("main".to_string(), m)], "main")
}

fn assert_no_errors(a: &sema::Analysis, src: &str) {
    if !a.errors.is_empty() {
        panic!(
            "expected no errors, got: {}\nsource:\n{}",
            a.errors.iter().map(|e| format!("{}:{}", e.line, e.msg)).collect::<Vec<_>>().join("; "),
            src
        );
    }
}

fn assert_one_error(a: &sema::Analysis, needle: &str, src: &str) {
    if a.errors.is_empty() {
        panic!("expected an error containing '{}', got none\nsource:\n{}", needle, src);
    }
    let joined = a.errors.iter().map(|e| e.msg.clone()).collect::<Vec<_>>().join("; ");
    assert!(
        joined.contains(needle),
        "expected error containing '{}', got: {}\nsource:\n{}",
        needle,
        joined,
        src
    );
}

#[test]
fn fib_infers_int_everywhere() {
    let a = check("fn fib(n: int):\n  if n < 2:\n    return n\n  return fib(n - 1) + fib(n - 2)\n\nfn main():\n  print(fib(10))\n");
    assert_no_errors(&a, "");
    assert_eq!(a.fns["f_main_fib"].ret, sema::Ty::Int);
}

#[test]
fn let_annotation_for_empty_list() {
    let a = check("fn main():\n  let xs: list[int] = []\n  xs.push(3)\n  print(xs[0])\n");
    assert_no_errors(&a, "");
    assert!(a.insts.contains(&sema::Ty::List(Box::new(sema::Ty::Int))));
}

#[test]
fn empty_list_without_annotation_is_an_error() {
    let a = check("fn main():\n  let xs = []\n  pass\n");
    assert_one_error(&a, "cannot infer the element type", "");
}

#[test]
fn int_float_promotion() {
    let a = check("fn main():\n  let x = 1 + 2.5\n  let y = 7 / 2\n  let z = 7.0 / 2\n  print(x + y + z)\n");
    assert_no_errors(&a, "");
}

#[test]
fn int_division_truncates() {
    let a = check("fn main():\n  let y = 7 / 2\n  print(y)\n");
    assert_no_errors(&a, "");
    // 7 / 2 is int: verified by using it where an int is required.
    let a2 = check("fn take(n: int):\n  pass\n\nfn main():\n  take(7 / 2)\n");
    assert_no_errors(&a2, "");
}

#[test]
fn str_plus_int_is_an_fstring_error() {
    let a = check("fn main():\n  let x = \"a\" + 1\n");
    assert_one_error(&a, "f-string", "");
}

#[test]
fn struct_field_and_method() {
    let src = "struct Point:\n  x: float\n  y: float\n\nfn Point.length(self) -> float:\n  return self.x * self.x + self.y * self.y\n\nfn main():\n  let p = Point(1.0, 2.0)\n  print(p.length())\n";
    let a = check(src);
    assert_no_errors(&a, src);
    // The Point constructor call's inferred type is the struct.
    assert!(a
        .ty
        .values()
        .any(|t| *t == sema::Ty::Struct("s_main_Point".into())));
}

#[test]
fn unknown_name_did_you_mean() {
    let a = check("fn main():\n  prnt(\"hi\")\n");
    assert_one_error(&a, "did you mean", "");
}

#[test]
fn assign_to_undeclared_is_an_error() {
    let a = check("fn main():\n  x = 1\n");
    assert_one_error(&a, "not declared", "");
}

#[test]
fn retyping_a_variable_is_an_error() {
    let a = check("fn main():\n  let x = 1\n  x = 1.5\n");
    assert_one_error(&a, "is int", "");
}

#[test]
fn dict_items_two_binders() {
    let a = check("fn main():\n  let d: dict[str, int] = {\"a\": 1}\n  for k, v in d.items():\n    print(k)\n");
    assert_no_errors(&a, "");
    assert!(a
        .insts
        .contains(&sema::Ty::List(Box::new(sema::Ty::Tuple(
            Box::new(sema::Ty::Str),
            Box::new(sema::Ty::Int)
        )))));
}

#[test]
fn iterating_a_dict_yields_keys() {
    let a = check("fn main():\n  let d: dict[str, int] = {\"a\": 1}\n  for k in d:\n    print(k)\n");
    assert_no_errors(&a, "");
}

#[test]
fn return_type_mismatch() {
    let a = check("fn f(n: int):\n  if n > 0:\n    return 1\n  return 1.5\n\nfn main():\n  print(f(3))\n");
    assert_one_error(&a, "return type mismatch", "");
}

#[test]
fn declared_return_must_match() {
    let a = check("fn f(n: int) -> float:\n  return 1\n\nfn main():\n  print(f(3))\n");
    assert_one_error(&a, "declared return type", "");
}

#[test]
fn catch_binder_prints() {
    let a = check("fn main():\n  try:\n    raise \"boom\"\n  except e:\n    print(e)\n");
    assert_no_errors(&a, "");
}

#[test]
fn catch_binder_cannot_be_used_as_int() {
    let a = check("fn main():\n  try:\n    raise \"boom\"\n  except e:\n    print(e + 1)\n");
    assert_one_error(&a, "add", "");
}

#[test]
fn struct_by_value_copy() {
    let a = check("struct P:\n  x: int\n\nfn main():\n  let a = P(1)\n  let b = a\n  print(a == b)\n");
    assert_no_errors(&a, "");
}

#[test]
fn recursive_struct_rejected() {
    let a = check("struct A:\n  other: A\n\nfn main():\n  pass\n");
    assert_one_error(&a, "contains itself", "");
}

#[test]
fn method_call_on_int_to_str() {
    let a = check("fn main():\n  let s = 3.to_str()\n  print(s)\n");
    assert_no_errors(&a, "");
}

#[test]
fn chained_list_ops() {
    let a = check("fn main():\n  let xs: list[int] = [3, 1, 2]\n  xs.sort()\n  if xs.contains(2):\n    print(xs.index_of(1))\n");
    assert_no_errors(&a, "");
}

#[test]
fn range_bounds_must_be_int() {
    let a = check("fn main():\n  for i in 0..\"5\":\n    pass\n");
    assert_one_error(&a, "range bounds must be int", "");
}

#[test]
fn cast_ptr_int() {
    let a = check("fn main():\n  let p = cast[ptr](1)\n  let n = cast[int](p)\n  print(n)\n");
    assert_no_errors(&a, "");
}

#[test]
fn main_must_exist() {
    let a = check("fn helper():\n  pass\n");
    assert_one_error(&a, "fn main()", "");
}

#[test]
fn for_binder_types_recorded() {
    let a = check("fn main():\n  let xs: list[str] = [\"a\"]\n  for x in xs:\n    print(x)\n");
    assert_no_errors(&a, "");
    let any_for = a
        .for_binders
        .values()
        .any(|v| v.len() == 1 && v[0] == sema::Ty::Str);
    assert!(any_for);
}

#[test]
fn fstring_parts_must_be_printable() {
    let a = check("fn main():\n  let xs: list[int] = [1]\n  print(f\"x = {xs}\")\n");
    assert_one_error(&a, "f-string", "");
}
