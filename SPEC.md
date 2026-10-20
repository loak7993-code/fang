# Fang language specification, version 0.1

This document is normative. Where the implementation and this document
disagree, file a bug.

## 1. Lexical structure

Source files are UTF-8. Indentation is spaces only; a tab anywhere in
indentation is an error. Blocks are 4 spaces by convention; the parser
accepts any consistent indent width.

- Comments run from `#` to end of line. Comments never affect
  indentation structure.
- Newlines separate statements. Newlines inside `()`, `[]`, or `{}`
  are not separators.
- Integer literals are i64 (64-bit, two's complement, wrapping).
- Float literals are f64: `1.5`, `1e10`, `2.5e-3`.
- String literals are double quoted with escapes
  `\\ \" \n \t \r \0`. Newlines inside string literals are not allowed.
- f-strings: `f"sum: {a + b}"`. Braces escape as `{{` and `}}`. Any
  printable expression may appear inside the braces.

Reserved words (25):

```
let fn if elif else for in while return break continue pass
struct extern true false nil and or not raise try except
import export from
```

`cast`, `self`, `float`, `int`, `print`, `len`, and `assert` are
contextual: they are ordinary names that the language recognizes in
specific positions.

## 2. Grammar

```ebnf
program     = { import | funcdef | structdef | constdecl | externfn } ;
import      = "import" ident [ "from" string ] ;
constdecl   = [ "export" ] "let" ident "=" constexpr ;
funcdef     = [ "export" ] "fn" qualident "(" [ params ] ")" [ "->" type ] block ;
qualident   = ident [ "." ident ] ;
params      = param { "," param } ;
param       = "self" | ident ":" type [ "=" expr ] ;
type        = "int" | "float" | "bool" | "str" | "ptr"
            | "list" "[" type "]" | "dict" "[" type "," type "]"
            | "(" type "," type ")"
            | ident ;
structdef   = [ "export" ] "struct" ident ":" newline indent { field } dedent ;
field       = ident ":" type newline ;
externfn    = "extern" "fn" ident "(" [ params ] ")" [ "->" type ]
              "from" string ;

block       = ":" newline indent { stmt } dedent ;
stmt        = letdecl | assign | "return" [ expr ]
            | ifstmt | whilestmt | forstmt | trystmt | raise
            | "break" | "continue" | "pass" | "asm" "(" string ")"
            | exprstmt ;
letdecl     = "let" binders [ ":" type ] "=" expr ;
binders     = ident [ "," ident ] ;
tupleassign = lvalue "," lvalue "=" rhs "," rhs ;
assign      = lvalue "=" expr ;
lvalue      = ident | lvalue "[" expr "]" | lvalue "." ident ;
ifstmt      = "if" expr block { "elif" expr block } [ "else" block ] ;
whilestmt   = "while" expr block ;
forstmt     = "for" binders "in" iterable block ;
iterable    = expr | expr ".." [ "=" ] expr ;
trystmt     = "try" block "except" [ ident ] block ;
raise       = "raise" expr ;

expr        = orexpr ;
orexpr      = andexpr { "or" andexpr } ;
andexpr     = notexpr { "and" notexpr } ;
notexpr     = "not" notexpr | comparison ;
comparison  = arith [ cmpop arith ] ;
cmpop       = "==" | "!=" | "<" | ">" | "<=" | ">=" ;
arith       = term { ("+" | "-") term } ;
term        = unary { ("*" | "/" | "%") unary } ;
unary       = "-" unary | power ;
power       = postfix [ "**" unary ] ;
postfix     = primary { "(" [ args ] ")" | "." ident | indexorslice } ;
indexorslice= "[" [ expr ] ":" [ expr ] "]" | "[" expr "]" ;
primary     = int | float | string | fstring | "true" | "false" | "nil"
            | ident | "(" expr ")" | tuplelit | listlit | dictlit
            | "cast" "[" type "]" "(" expr ")"
            | fnexpr | comprehension ;
fnexpr      = "fn" "(" [ params ] ")" [ "->" type ] ":" ( block | "return" expr ) ;
comprehension = "[" expr "for" binders "in" ( expr | expr ".." [ "=" ] expr )
                [ "if" expr ] "]" ;
tuplelit    = "(" expr "," expr ")" ;
listlit     = "[" [ expr { "," expr } ] "]" ;
dictlit     = "{" [ expr ":" expr { "," expr ":" expr } ] "}" ;
```

Chained comparisons (`a < b < c`) are a parse error; use `and`.
Tuples hold exactly two values.

## 3. Types

| Fang | Meaning | C representation |
|-----|---------|------------------|
| `int` | 64-bit signed integer, wrapping | `int64_t` |
| `float` | 64-bit IEEE float | `double` |
| `bool` | `true` / `false` | `bool` |
| `str` | immutable length-prefixed bytes | `fang_str*` |
| `ptr` | untyped pointer | `void*` |
| `list[T]` | growable array of T | stamped C struct |
| `dict[K, V]` | open-addressing map | stamped C struct |
| `(A, B)` | pair of values | stamped C struct |
| struct | nominal record, by value | stamped C struct |

### Rules

- Parameters carry type annotations. Locals, returns, and collection
  element types are inferred. Recursive functions must declare their
  return type.
- `int` and `float` mix in binary operators; the result is `float`.
- `int / int` truncates toward zero. Use `float(a) / b` for real
  division.
- Conditions must be `bool`. There is no truthiness.
- Dict keys are `str` or `int` in v0.1. Dict iteration is insertion
  ordered; `for k in d` yields keys, `for k, v in d.items()` pairs.
- Struct fields may be primitives, `str`, `ptr`, or other structs (no
  cycles). Collection-typed struct fields are not supported in v0.1.
- Structs are value types: assignment copies. `str`, `list`, and
  `dict` are reference types.
- Integer overflow wraps. Out-of-bounds indexing and division by
  zero stop the program with a panic (exit code 101).

## 4. Memory model

`str`, `list`, and `dict` are reference counted. String literals are
immortal. Deterministic release happens at function exit. Known
limitations in v0.1:

- Reference cycles leak.
- `raise` leaks the values held by the raising function's frame
  (the longjmp skips the release path).
- Locals live until function exit, not block exit.
- A value stored behind `cast[ptr]` is kept alive for the process
  lifetime and never released.

## 5. Errors

Two channels:

1. Panics: index out of bounds, division by zero, failed `assert`.
   Not catchable. Exit code 101.
2. `raise value` / `try` / `except e:`. The raised value is usually a
  `str`. The `except` binder has type "any": printable and
  re-raisable, nothing else.

## 6. Modules

- One file is one module. Module name is the file stem.
- `import math` resolves: embedded standard library, then a sibling
  `math.fang`, then `fang_packages/math/math.fang`.
- `import utils from "./dir/utils.fang"` loads an explicit path.
- `export` marks public functions, structs, and constants. Everything
  else is module-private.
- The standard library modules are always present in a build; their
  functions are reachable by method sugar (`s.upper()`) without
  import. Qualified use (`math.sqrt`) requires the import statement.
- Cyclic imports are a compile error.

## 7. C interop

```fang
extern fn sqrt(x: float) -> float from "math.h"
```

- One `extern` per C symbol. The header string is sanitized to
  letters, digits, `_ . / -`.
- Type mapping: `int` to `long long`, `float` to `double`, `bool` to
  `bool`, `str` to `fang_str*`, `ptr` to `void*`.
- Variadic C functions are rejected at compile time in v0.1.
- `cast[ptr](x)` / `cast[int](p)` convert between pointers and
  integers. `cast[T](p)` reinterprets a `ptr` back to an RC type.
- `s.to_cstr()` returns the NUL-terminated buffer behind a string for
  C APIs that take `const char*`.

## 8. Freestanding mode

`fang build --freestanding --target <triple>` compiles with
`-nostdlib -ffreestanding` and links no runtime. Available: `int`,
`float`, `bool`, `ptr`, structs, `asm("...")`, and `cast`. Not
available: `str`, collections, `print`, `raise`, `try`.

The entry wrapper is `void fang_kernel_main(void)`. Bring your own
crt0 and linker script.

## 9. Operator semantics

| Operator | Notes |
|----------|-------|
| `+ - * / %` | numeric; `+` also concatenates str and same-typed list |
| `**` | pow; int result truncates |
| `== != < > <= >=` | numeric (mixed promotes), str lexicographic, ptr identity, struct field-wise for primitive/str fields |
| `and or not` | bool only; `and`/`or` short-circuit |
| `..` / `..=` | range, valid only in `for` iterables |
| `xs[i]` | list/dict/str index; negative counts from the end |
| `xs[a:b]` | slice; missing bounds mean start/end |
| `-x` | numeric negation |

## 10. Function values and closures

`fn(...)` expressions create function values. A closure captures the
outer locals it reads by value at creation time (not by reference). Any
`fn` name also converts to a value. Calls through function values are
statically typed by the signature. `list.map`, `list.filter`, and
`list.reduce` take function values.

## 11. Comprehensions

`[expr for x in iter [if cond]]` builds a list. `iter` is a list, dict
keys, or a range. Two binders iterate pairs (`d.items()`).

## 12. Default parameters

Trailing parameters may declare defaults with `=`. Defaults are
evaluated at the call site, not at declaration.

## 13. Tuple assignment

`a, b = rhs` unpacks a 2-tuple into two lvalues; `a, b = b, a` swaps.

## 14. Builtins

| Name | Signature | Notes |
|------|-----------|-------|
| `print(args...)` | values, space separated, newline | int, float, bool, str, ptr |
| `len(x)` | `int` | str, list, dict |
| `assert(cond)` | panic on false | |
| `float(i)` | `int -> float` | |
| `int(f)` | `float -> int` | truncates |
