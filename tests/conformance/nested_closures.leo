# 1. nested closures
fn make_adder(n):
  return fn(x): return n + x

add5 = make_adder(5)
add10 = make_adder(10)
print(add5(1), add10(1))
