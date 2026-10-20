fn main():
  let d: dict[str, int] = {"a": 1, "b": 2}
  let flipped = [f"{k}={v}" for k, v in d.items()]
  print(flipped[0], flipped[1])
