import random
xs = []
for i in range(1000000):
    xs.append(random.randint(0, 1000000000))
xs.sort()
print(xs[0], xs[500000], xs[999999])
