fn main():
  let d: dict[int, int] = {}
  d.reserve(100000)
  for i in 0..1000000:
    d.set(i % 100000, i)
  let total = 0
  for i in 0..1000000:
    total = total + d.get(i % 100000, 0)
  print(total)
