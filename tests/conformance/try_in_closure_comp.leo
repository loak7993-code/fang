risky = fn(n):
  try:
    if n < 0:
      raise "negative"
    return n
  except e:
    return 0

vals = [risky(x) for x in [5, -1, 3]]
print(vals)
