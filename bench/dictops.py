d = {}
for i in range(1000000):
    d[i % 100000] = i
total = 0
for i in range(1000000):
    total += d.get(i % 100000, 0)
print(total)
