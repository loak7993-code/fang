fn add(a: int, b: int = 10) -> int:
  return a + b

fn double(x: int) -> int:
  return x * 2

fn main():
  # default arguments
  print(add(1))
  print(add(1, 2))

  # closure with capture
  let base = 100
  let addbase = fn(x: int) -> int: return base + x
  print(addbase(5))
  print(addbase(addbase(1)))

  # named function as a value
  let d = double
  print(d(21))

  # map / filter / reduce with lambdas
  let xs = [1, 2, 3, 4, 5, 6]
  let squares = xs.map(fn(x: int) -> int: return x * x)
  print(squares[0], squares[5])
  let evens = xs.filter(fn(x: int) -> bool: return x % 2 == 0)
  print(len(evens), evens[0], evens[1])
  let total = xs.reduce(fn(acc: int, x: int) -> int: return acc + x, 0)
  print(total)

  # map with a named function
  let doubled = xs.map(double)
  print(doubled[3])

  # comprehension
  let big = [x * 10 for x in xs if x > 3]
  print(len(big), big[0])
  let words = [f"n{x}" for x in xs]
  print(words[2])

  # swap
  let a = 1
  let b = 2
  a, b = b, a
  print(a, b)

  # comprehension over a range
  let zeros = [0 for i in 0..4]
  print(len(zeros))
