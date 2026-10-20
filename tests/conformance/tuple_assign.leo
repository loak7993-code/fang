# swap with expressions on both sides
a = 1
b = 2
a, b = b + 10, a * 3
print(a, b)
# tuple-returning fn unpacked
pair = fn(): return (1, 2)
x, y = pair()
print(x, y)
