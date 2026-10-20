# Use Python libraries from Leo. Requires: leo run this.leo --python
import py

fn main():
  # Import any Python module
  let random = py.import_module("random")
  let json = py.import_module("json")

  # Call Python functions with converted values
  let seed_fn = py.attr(random, "seed")
  py.call(seed_fn, [py.to_py(42)])
  let roll = py.call_method(random, "randint", [py.to_py(1), py.to_py(6)])
  print(f"dice roll: {py.to_int(roll)}")

  # Python's json round-trip through Leo values
  let loads = py.attr(json, "loads")
  let payload = py.call(loads, [py.to_py_s("{\"lang\": \"leo\", \"fast\": true}")])
  print(py.repr(payload))

  # eval and exec
  let answer = py.eval("6 * 7")
  print(f"the answer: {py.to_int(answer)}")
  py.exec("import sys; print('python', sys.version.split()[0], 'inside leo')")
  # A real third-party library: numpy
  let np = py.import_module("numpy")
  let a = py.call_method(np, "arange", [py.to_py(5)])
  let b = py.call_method(a, "reshape", [py.to_py(5), py.to_py(1)])
  print(f"numpy shape: {py.repr(b)}")
  let no_args: list[ptr] = []
  let mean = py.call_method(a, "mean", no_args)
  print(f"numpy mean: {py.to_float(mean)}")
