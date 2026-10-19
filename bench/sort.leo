import math

fn main():
  let xs: list[int] = []
  for i in 0..1000000:
    xs.push(math.random_int(0, 1000000000))
  xs.sort()
  print(xs[0], xs[500000], xs[999999])
