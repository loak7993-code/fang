# Fang standard library, version 0.1

Every function here is implemented in Fang (see `std/`) over a small C
runtime. Method sugar: functions whose first parameter is `str` are
callable as methods (`s.upper()` is `str.upper(s)`).

Builtins (`print`, `len`, `assert`, `float`, `int`) need no import.
Everything else: `import str`, `import math`, and so on. Module
functions are also callable as methods on receiver types.

## io

```fang
import io

fn main():
  let line = io.input()
  print(f"you said: {line}")
```

| Function | Type |
|----------|------|
| `input()` | `() -> str` reads one line from stdin |
| `read_line()` | `() -> str` same as input |

## str

```fang
import str

fn main():
  print("Hello".upper())
  print("a,b".split(",")[1])
```

| Function | Type |
|----------|------|
| `upper(s)` / `lower(s)` | `str -> str` |
| `strip(s)` | `str -> str` trims surrounding whitespace |
| `split(s, sep)` | `(str, str) -> list[str]` |
| `join(sep, items)` | `(str, list[str]) -> str` |
| `replace(s, old, new)` | `(str, str, str) -> str` |
| `contains(s, needle)` | `(str, str) -> bool` |
| `find(s, needle)` | `(str, str) -> int` -1 when missing |
| `find_from(s, start, needle)` | `(str, int, str) -> int` |
| `starts_with(s, prefix)` | `(str, str) -> bool` |
| `ends_with(s, suffix)` | `(str, str) -> bool` |
| `slice(s, start, end)` | `(str, int, int) -> str` |
| `to_int(s)` | `str -> int` panics on bad input |
| `to_float(s)` | `str -> float` panics on bad input |
| `to_cstr(s)` | `str -> ptr` NUL-terminated buffer |

## list (methods on every list)

```fang
fn main():
  let xs = [3, 1, 2]
  xs.push(4)
  print(xs.index_of(2), xs.contains(9))
```

| Method | Type |
|--------|------|
| `push(v)` | add to the end |
| `pop()` | remove and return the last value |
| `insert(i, v)` | insert at index |
| `remove_at(i)` | remove at index |
| `sort()` | ascending, int/float/str elements only |
| `map(fn)` | `-> list[R]` any function taking one element |
| `filter(fn)` | `-> list[T]` function returning bool |
| `reduce(fn, init)` | fold with `(acc, item) -> acc` |
| `reverse()` | in place |
| `contains(v)` | `-> bool` |
| `index_of(v)` | `-> int`, -1 when missing |

## dict (methods on every dict)

```fang
fn main():
  let ages: dict[str, int] = {"panda": 30}
  print(ages.get("panda", 0))
```

| Method | Type |
|--------|------|
| `get(k, default)` | `-> V` |
| `set(k, v)` | insert or update |
| `reserve(n)` | grow capacity to fit n entries without rehash churn |
| `keys()` | `-> list[K]` insertion order |
| `values()` | `-> list[V]` |
| `items()` | `-> list[(K, V)]` |
| `contains(k)` | `-> bool` |
| `remove(k)` | delete |

## math

```fang
import math

fn main():
  print(math.sqrt(2.0), math.PI)
```

| Function | Type |
|----------|------|
| `sqrt(x)` `floor(x)` `ceil(x)` `round(x)` | `float -> float` |
| `abs(x)` | `float -> float` |
| `sin cos tan log exp` | `float -> float` |
| `pow(base, e)` | `(float, float) -> float` |
| `min(a, b)` / `max(a, b)` | floats |
| `random()` | `-> float` in [0, 1) |
| `random_int(low, high)` | `-> int` inclusive |
| `PI`, `E` | constants |

## os

```fang
import os

fn main():
  print(os.getcwd())
  print(len(os.args()))
```

| Function | Type |
|----------|------|
| `args()` | `-> list[str]` program arguments |
| `env(name)` | `-> str` empty when unset |
| `getcwd()` | `-> str` |
| `system(cmd)` | `-> int` exit status |
| `exit(code)` | exits the process |

## fs

```fang
import fs

fn main():
  fs.write_file("/tmp/x.txt", "data")
  print(fs.read_file("/tmp/x.txt"))
```

| Function | Type |
|----------|------|
| `read_file(path)` | `-> str` panics when missing |
| `write_file(path, data)` | create or truncate |
| `append_file(path, data)` | append |
| `exists(path)` | `-> bool` |
| `list_dir(path)` | `-> list[str]` |
| `mkdir(path)` | create a directory |
| `remove(path)` | delete a file |

## time

| Function | Type |
|----------|------|
| `now()` | `-> float` unix epoch seconds |
| `clock()` | `-> float` monotonic seconds |
| `sleep(seconds)` | pause |

## json

Pure Fang, no runtime JSON code. Values are tagged structs; `list` and
`dict` results are read back with `as_list` and `as_dict`.

```fang
import json

fn main():
  let doc = json.parse("{\"port\": 8080, \"tags\": [\"a\"]}")
  print(json.get(doc, "port").i)
  let tags = json.as_list(json.get(doc, "tags"))
  print(len(tags))
  print(json.stringify(doc))
```

| Function | Type |
|----------|------|
| `parse(text)` | `-> JValue` |
| `stringify(v)` | `-> str` |
| `get(v, key)` | `-> JValue` dict field, nil when missing |
| `at(v, index)` | `-> JValue` list element |
| `as_list(v)` | `-> list[JValue]` |
| `as_dict(v)` | `-> dict[str, JValue]` |

A `JValue` has `kind` (one of `nil bool int float str list dict`),
`s`, `i`, `f`, `b`, and `ref`. `json.get` on a missing key returns a
`kind == "nil"` value, not an error.

## py (Python bridge)

Use any Python library from Fang. Requires building with `--python`
(needs the python3 dev package):

```
fang run app.fg --python
```

```fang
import py

fn main():
  let random = py.import_module("random")
  let randint = py.attr(random, "randint")
  let roll = py.call(randint, [py.to_py(1), py.to_py(6)])
  print(f"roll: {py.to_int(roll)}")
```

| Function | Type |
|----------|------|
| `import_module(name)` | `-> ptr` any Python module |
| `attr(obj, name)` | `-> ptr` attribute fetch |
| `call(fn, args)` | `args: list[ptr]`, returns `ptr` |
| `call_method(obj, name, args)` | method call |
| `eval(code)` / `exec(code)` | run Python source |
| `to_py(v)` / `to_py_f(v)` / `to_py_b(v)` / `to_py_s(v)` | Fang to Python |
| `to_int(o)` / `to_float(o)` / `to_bool(o)` / `to_str(o)` | Python to Fang |
| `repr(o)` | `-> str` Python repr |
| `none()`, `is_none(o)` | None handling |
| `decref(o)` | manual release; otherwise objects live for the process |

## net

Plain TCP and HTTP/1.0. No TLS in v0.1.

```fang
import net

fn main():
  let body = net.http_get("example.com/")
  print(len(body))
```

| Function | Type |
|----------|------|
| `tcp_connect(host, port)` | `-> ptr` |
| `tcp_send(sock, data)` | send a string |
| `tcp_recv(sock)` | `-> str` reads until close |
| `tcp_close(sock)` | close |
| `http_get(url)` | `-> str` response body |

## server

HTTP server in pure Fang. Accept loop, request parsing, response
serialization. Single-threaded: one request at a time, like Python's
`http.server`.

```fang
import server

fn handler(req: Request):
  if req.path == "/":
    return server.html("<h1>hello</h1>")
  elif req.path == "/api":
    return server.json('{"ok": true}')
  elif req.method == "POST" and req.path == "/echo":
    return server.text("you sent: " + req.body)
  return server.not_found()

server.serve(handler, 8080)
```

The handler is any function value of type `fn(Request) -> Response`;
function types are annotations now (`fn(A, B) -> C`).

`Request` fields: `method`, `path`, `body` (all `str`).
`Response` fields: `status`, `content_type`, `body`.

| Function | Type |
|----------|------|
| `serve(handler, port)` | accept loop, blocks forever |
| `respond(status, ctype, body)` | `-> Response` full control |
| `html(body)` | `-> Response` 200 text/html |
| `json(body)` | `-> Response` 200 application/json |
| `text(body)` | `-> Response` 200 text/plain |
| `not_found()` | `-> Response` 404 |

Verified with curl: GET and POST (Content-Length bodies) both round-trip.

## tui

Terminal UI in pure Fang: ANSI escape codes, a screen buffer, and
raw-mode key input. Works on any terminal (Linux, macOS, BSD, and
Windows 10+ terminals all speak ANSI).

```fang
import tui

tui.init()
s = tui.screen(40, 12)
tui.scr_box(s, 0, 0, 40, 12)
tui.scr_set_str(s, 2, 1, "hello from fang")
x = 2
while true:
  tui.scr_fill(s, 2, 3, 36, 1, " ")
  tui.scr_set(s, x, 3, "@")
  tui.scr_draw(s)
  k = tui.read_key(30)
  if k == tui.KEY_LEFT:
    x = x - 1
  elif k == tui.KEY_RIGHT:
    x = x + 1
  elif k == 113:  # q
    break
tui.done()
```

| Function | Type |
|----------|------|
| `init()` / `done()` | raw mode on / terminal restored |
| `screen(w, h)` | `-> Screen` cell buffer |
| `scr_set(s, x, y, ch)` / `scr_set_str(s, x, y, text)` | write cells |
| `scr_fill(s, x, y, w, h, ch)` / `scr_box(s, x, y, w, h)` | fill / box |
| `scr_draw(s)` | paint the whole buffer |
| `read_key(timeout_ms)` | `-> int` key or KEY_NONE |
| `fg(c)` / `bg(c)` / `bold()` / `reset()` / `style(f, b, is_bold)` | `-> str` ANSI |
| `goto(x, y)` / `clear()` / `hide_cursor()` / `show_cursor()` | cursor control |
| `write(s)` | raw output, no newline |

Key constants: `KEY_NONE, KEY_ESCAPE, KEY_UP, KEY_DOWN, KEY_LEFT,
KEY_RIGHT, KEY_HOME, KEY_END, KEY_DELETE, KEY_PAGE_UP, KEY_PAGE_DOWN`.
Colors: `BLACK..WHITE`, `DEFAULT`.

`examples/chip8.fg` is a complete CHIP-8 emulator built on this module:
64x32 display, all 35 opcodes, the original HEX keypad on 1234/qwer/
asdf/zxcv, and a `--frames N` headless mode for testing.
