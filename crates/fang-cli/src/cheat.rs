//! `fang cheat`: the whole language on one screen.
//! `fang doc <topic>`: instant help for any construct or builtin.

use std::process::ExitCode;

pub const CHEAT: &str = "\
THE ENTIRE LANGUAGE
===================

print(x)                      show a value            print(\"hi\", 42)
name = value                  create/change a name    count = 0
x: list[int] = []             create with a type      d: dict[str, int] = {}
if cond: / elif: / else:      decide                  if x > 3:
while cond:                   loop while true         while x < 10:
for i in 0..10:               count (stops before 10)  for i in 0..=10:  (include 10)
for x in xs:                  loop over a list        for k, v in d.items():
break / continue / pass       control a loop          break
fn name(a, b):                function, no return     fn double(n): n * 2
                              keyword needed           (last expr returns)
fn T.m(self):                 method                  return exits early
struct Name:                  bundled fields          p = Point(3.0, 4.0)
p.field                       read a field            lists: xs[0]  xs[-1]  xs[1:3]
xs.push(v) / pop / sort       lists also: map, filter, reduce, contains
d[k] / d.get(k, def) / set    dicts also: keys, values, items, remove
f\"{name} is {n}\"            build text              \"a\" + \"b\"
[expr for x in xs if cond]   build a list            [k for k, v in d.items()]
fn(x): return x * 2           function value          xs.map(fn(x): return x * 2)
try: / except e: / raise      errors                  raise \"the disk is full\"
let / export                  optional declarations   export fn api(): ...
import math / import utils    modules                 math.sqrt(2.0)
extern fn sqrt(x: float)      call C                  from \"math.h\"
asm(\"nop\")                   inline assembly         cast[ptr](x), cast[int](p)
input(\"prompt\")              read keyboard           len(xs)  assert(cond)
float(x) / int(f) / str(x)    convert                 s.to_int()  s.upper()

RUNNING
=======
fang run app.fang               compile and run        fang build app.fang --release
fang learn                     the full course        fang doc <any word above>
fang repl                      try things live        fang test / fang add <git>

That is everything. The stdlib: import str, math, os, fs, time, json, net, server.";

pub fn cmd_cheat() -> ExitCode {
    println!("{}", CHEAT);
    ExitCode::SUCCESS
}

const DOC_ENTRIES: &[(&str, &str)] = &[
    ("print", "print(value, ...) shows values separated by spaces.\n  print(\"hi\", 42)      hi 42\nLists and dicts print directly: print([1, 2]) gives [1, 2]."),
    ("let", "let is optional. Assigning to a new name creates it.\n  x = 5\n  let y = 5     same thing\nA name keeps its type; a typed first assignment: d: dict[str, int] = {}"),
    ("if", "if cond: runs its block when cond is true. elif chains, else catches the rest.\n  if x > 3:\n    print(\"big\")\n  elif x == 3:\n    print(\"three\")\n  else:\n    print(\"small\")\nNonempty lists, dicts, and strings count as true: if xs:"),
    ("while", "while cond: repeats its block until cond is false.\n  while len(xs) > 0:\n    xs.pop()"),
    ("for", "for x in items: loops over a list, dict keys, or a range.\n  for i in 0..10:      0 through 9\n  for i in 0..=10:     0 through 10\n  for x in xs:\n  for k, v in d.items():   pairs"),
    ("range", "Ranges only appear in for loops: 0..10 excludes the end, 0..=10 includes it."),
    ("fn", "fn name(a, b): defines a function. Types optional; -> declares the return.\n  fn add(a, b):\n    return a + b\nThe last expression is the return value: fn double(n): n * 2\nRecursive functions need the return type: fn fib(n: int) -> int:\nA method takes self: fn Point.length(self) -> float:\nFunction types are annotations: fn(int, int) -> int"),
    ("struct", "struct bundles fields; call its name to build one.\n  struct Point:\n    x: float\n    y: float\n  p = Point(3.0, 4.0)\n  print(p.x)"),
    ("list", "A typed, growable array.\n  xs = [1, 2, 3]\n  xs.push(4)  xs.pop()  xs.sort()  xs.reverse()\n  xs[0]  xs[-1]  xs[1:3]  len(xs)\n  xs.map(f)  xs.filter(f)  xs.reduce(f, init)\nEmpty needs a type: xs: list[int] = []"),
    ("dict", "Maps str or int keys to values.\n  d = {\"a\": 1}\n  d[\"b\"] = 2\n  d.get(\"a\", 0)  d.set(\"k\", v)  d.remove(\"k\")\n  for k in d:  for k, v in d.items():\nEmpty needs a type: d: dict[str, int] = {}"),
    ("str", "f\"text {expr}\" builds text. String methods:\n  s.upper() s.lower() s.strip() s.split(\",\") s.replace(a, b)\n  s.find(x) s.starts_with(x) s.to_int() s.to_float()\n  \"-\".join(words)  str(42)  len(s)"),
    ("comprehension", "[expr for x in xs if cond] builds a list in one line.\n  squares = [x * x for x in xs if x > 2]\n  pairs = [f\"{k}={v}\" for k, v in d.items()]"),
    ("closure", "fn values close over the names around them (captured by value).\n  add5 = fn(x): return 5 + x\n  add5(1)\n  xs.map(fn(x): return x * 2)\n  xs.map(double)      named functions work too"),
    ("map", "list.map(f) applies f to every element, returning a new list.\n  xs.map(fn(x): return x * 2)"),
    ("filter", "list.filter(f) keeps elements where f returns true.\n  xs.filter(fn(x): return x % 2 == 0)"),
    ("reduce", "list.reduce(f, start) folds a list into one value.\n  total = xs.reduce(fn(acc, x): return acc + x, 0)"),
    ("try", "try runs risky code; except e catches a raise.\n  try:\n    raise \"disk full\"\n  except e:\n    print(f\"caught: {e}\")\nUncaught errors exit with code 101."),
    ("raise", "raise \"message\" throws an error; catch it with try/except."),
    ("import", "import math loads a module; call it as math.sqrt(2.0).\n  import utils                finds utils.fang beside your file\n  import utils from \"./d/u.fang\"\nStandard library: str, math, io, os, fs, time, json, net, py, server."),
    ("server", "HTTP server in pure Fang.\n  import server\n  fn handler(req: Request):\n    if req.path == \"/\":\n      return server.html(\"<h1>hi</h1>\")\n    return server.not_found()\n  server.serve(handler, 8080)\nHandler type: fn(Request) -> Response. Fields: method, path, body.\nBuilders: server.html/json/text/respond/not_found."),
    ("extern", "extern declares a C function and calls it natively.\n  extern fn sqrt(x: float) -> float from \"math.h\"\n  print(sqrt(2.0))"),
    ("asm", "asm(\"instruction\") emits inline assembly.\n  asm(\"nop\")"),
    ("cast", "cast converts pointers and integers.\n  p = cast[ptr](1)\n  n = cast[int](p)"),
    ("input", "input(\"prompt: \") reads one line from the keyboard as a str.\n  name = input(\"name: \")"),
    ("assert", "assert(cond) stops the program with an error if cond is false."),
    ("len", "len(s) counts a string, list, or dict: len([1, 2]) is 2."),
    ("export", "export fn, export struct, export let mark public names for importers."),
    ("repl", "fang repl: type expressions to evaluate them, definitions to keep them."),
    ("learn", "fang learn runs the interactive course from zero. fang learn list shows lessons."),
    ("test", "fang test runs every *_test.fang file under the current directory."),
    ("add", "fang add user/repo installs a package from git, pinned in fang.lock."),
];

pub fn cmd_doc(args: &[String]) -> ExitCode {
    let topic = match args.first() {
        Some(t) => t.to_lowercase(),
        None => {
            println!("usage: fang doc <topic>");
            println!("topics: ");
            let mut names: Vec<&str> = DOC_ENTRIES.iter().map(|(n, _)| *n).collect();
            names.sort();
            let line = names.join(", ");
            println!("  {}", line);
            return ExitCode::SUCCESS;
        }
    };
    for (name, body) in DOC_ENTRIES {
        if *name == topic {
            println!("{}", body);
            return ExitCode::SUCCESS;
        }
    }
    eprintln!("fang doc: no entry for '{}'. Try: fang doc", topic);
    ExitCode::from(1)
}
