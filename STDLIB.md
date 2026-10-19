# Leo standard library, version 0.1

Every function here is implemented in Leo (see `std/`) over a small C
runtime. Method sugar: functions whose first parameter is `str` are
callable as methods (`s.upper()` is `str.upper(s)`).

Builtins (`print`, `len`, `assert`, `float`, `int`) need no import.
Everything else: `import str`, `import math`, and so on. Module
functions are also callable as methods on receiver types.

## io

```leo
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

```leo
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

```leo
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
| `reverse()` | in place |
| `contains(v)` | `-> bool` |
| `index_of(v)` | `-> int`, -1 when missing |

## dict (methods on every dict)

```leo
fn main():
  let ages: dict[str, int] = {"panda": 30}
  print(ages.get("panda", 0))
```

| Method | Type |
|--------|------|
| `get(k, default)` | `-> V` |
| `set(k, v)` | insert or update |
| `keys()` | `-> list[K]` insertion order |
| `values()` | `-> list[V]` |
| `items()` | `-> list[(K, V)]` |
| `contains(k)` | `-> bool` |
| `remove(k)` | delete |

## math

```leo
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

```leo
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

```leo
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

Pure Leo, no runtime JSON code. Values are tagged structs; `list` and
`dict` results are read back with `as_list` and `as_dict`.

```leo
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

## net

Plain TCP and HTTP/1.0. No TLS in v0.1.

```leo
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
