struct Point:
  x: float
  y: float

fn Point.length(self) -> float:
  return math_sqrt(self.x * self.x + self.y * self.y)

fn math_sqrt(v: float) -> float:
  return v * 0.5

fn fib(n: int):
  if n < 2:
    return n
  return fib(n - 1) + fib(n - 2)

fn main():
  let p = Point(3.0, 4.0)
  print(f"len = {p.length()}")
  print(f"fib(10) = {fib(10)}")

  let xs: list[int] = []
  for i in 0..10:
    xs.push(i * i)
  let total = 0
  for v in xs:
    total = total + v
  print(f"sum of squares = {total}")
  print(f"xs[3] = {xs[3]}, last = {xs[-1]}")

  let d: dict[str, int] = {"one": 1, "two": 2}
  d.set("three", 3)
  for k, v in d.items():
    print(f"{k} -> {v}")
  print(f"has two: {d.contains("two")}")

  let s = "hello"
  let t = s + " " + "leo"
  print(t)
  print(f"slice = {t[0:5]}, upper count = {len(t)}")

  try:
    raise "boom"
  except e:
    print(f"caught: {e}")

  let n = 0
  while n < 3:
    n = n + 1
    if n == 2:
      continue
  print(f"n = {n}")
