# Python bridge: use any Python library from Leo.

# Requires building with --python (leo run x.leo --python).

extern fn leo_py_import(name: ptr) -> ptr from "leo_py.h"

extern fn leo_py_attr(obj: ptr, name: ptr) -> ptr from "leo_py.h"

extern fn leo_py_eval(code: ptr) -> ptr from "leo_py.h"

extern fn leo_py_exec(code: ptr) from "leo_py.h"

extern fn leo_py_from_int(v: int) -> ptr from "leo_py.h"

extern fn leo_py_from_float(v: float) -> ptr from "leo_py.h"

extern fn leo_py_from_bool(v: bool) -> ptr from "leo_py.h"

extern fn leo_py_from_str(s: ptr, len: int) -> ptr from "leo_py.h"

extern fn leo_py_to_int(o: ptr) -> int from "leo_py.h"

extern fn leo_py_to_float(o: ptr) -> float from "leo_py.h"

extern fn leo_py_to_bool(o: ptr) -> bool from "leo_py.h"

extern fn leo_py_to_str(o: ptr) -> str from "leo_py.h"

extern fn leo_py_repr(o: ptr) -> str from "leo_py.h"

extern fn leo_py_none() -> ptr from "leo_py.h"

extern fn leo_py_is_none(o: ptr) -> bool from "leo_py.h"

extern fn leo_py_decref(o: ptr) from "leo_py.h"

extern fn leo_py_print(o: ptr) from "leo_py.h"

export fn import_module(name: str) -> ptr:
  return leo_py_import(name.to_cstr())

export fn attr(obj: ptr, name: str) -> ptr:
  return leo_py_attr(obj, name.to_cstr())
  # call with a list of python values

extern fn leo_py_call_list(f: ptr, args: ptr) -> ptr from "leo_py.h"

export fn call(f: ptr, args: list[ptr]) -> ptr:
  return leo_py_call_list(f, cast[ptr](args))

extern fn leo_py_call_list_method(obj: ptr, name: ptr, args: ptr) -> ptr from "leo_py.h"

export fn call_method(obj: ptr, name: str, args: list[ptr]) -> ptr:
  return leo_py_call_list_method(obj, name.to_cstr(), cast[ptr](args))

export fn eval(code: str) -> ptr:
  return leo_py_eval(code.to_cstr())

export fn exec(code: str):
  leo_py_exec(code.to_cstr())
  # Conversions

export fn to_py(v: int) -> ptr:
  return leo_py_from_int(v)

export fn to_py_f(v: float) -> ptr:
  return leo_py_from_float(v)

export fn to_py_b(v: bool) -> ptr:
  return leo_py_from_bool(v)

export fn to_py_s(v: str) -> ptr:
  return leo_py_from_str(v.to_cstr(), len(v))

export fn to_int(o: ptr) -> int:
  return leo_py_to_int(o)

export fn to_float(o: ptr) -> float:
  return leo_py_to_float(o)

export fn to_bool(o: ptr) -> bool:
  return leo_py_to_bool(o)

export fn to_str(o: ptr) -> str:
  return leo_py_to_str(o)

export fn repr(o: ptr) -> str:
  return leo_py_repr(o)

export fn none() -> ptr:
  return leo_py_none()

export fn is_none(o: ptr) -> bool:
  return leo_py_is_none(o)

export fn decref(o: ptr):
  leo_py_decref(o)

export fn print(o: ptr):
  leo_py_print(o)
