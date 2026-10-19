extern fn leo_str_upper(s: str) -> str from "leo_rt.h"
extern fn leo_str_lower(s: str) -> str from "leo_rt.h"
extern fn leo_str_strip(s: str) -> str from "leo_rt.h"
extern fn leo_str_replace(s: str, old: str, new: str) -> str from "leo_rt.h"
extern fn leo_str_starts_with(s: str, prefix: str) -> bool from "leo_rt.h"
extern fn leo_str_ends_with(s: str, suffix: str) -> bool from "leo_rt.h"
extern fn leo_str_find(s: str, start_at: int, needle: str) -> int from "leo_rt.h"
extern fn leo_str_slice(s: str, start: int, end: int) -> str from "leo_rt.h"
extern fn leo_str_to_i64(s: str) -> int from "leo_rt.h"
extern fn leo_str_to_f64(s: str) -> float from "leo_rt.h"

export fn upper(s: str) -> str:
  return leo_str_upper(s)

export fn lower(s: str) -> str:
  return leo_str_lower(s)

export fn strip(s: str) -> str:
  return leo_str_strip(s)

export fn replace(s: str, old: str, new: str) -> str:
  return leo_str_replace(s, old, new)

export fn starts_with(s: str, prefix: str) -> bool:
  return leo_str_starts_with(s, prefix)

export fn ends_with(s: str, suffix: str) -> bool:
  return leo_str_ends_with(s, suffix)

export fn contains(s: str, needle: str) -> bool:
  return leo_str_find(s, 0, needle) >= 0

export fn find(s: str, needle: str) -> int:
  return leo_str_find(s, 0, needle)

export fn find_from(s: str, start_at: int, needle: str) -> int:
  return leo_str_find(s, start_at, needle)

export fn to_int(s: str) -> int:
  return leo_str_to_i64(s)

export fn to_float(s: str) -> float:
  return leo_str_to_f64(s)

export fn split(s: str, sep: str) -> list[str]:
  if len(sep) == 0:
    raise "str.split separator must not be empty"
  let out: list[str] = []
  let start = 0
  let slen = len(s)
  let seplen = len(sep)
  while true:
    let at = leo_str_find(s, start, sep)
    if at < 0:
      out.push(leo_str_slice(s, start, slen))
      break
    out.push(leo_str_slice(s, start, at))
    start = at + seplen
  return out

export fn join(sep: str, items: list[str]) -> str:
  let out = ""
  let n = len(items)
  for i in 0..n:
    if i > 0:
      out = out + sep
    out = out + items[i]
  return out
