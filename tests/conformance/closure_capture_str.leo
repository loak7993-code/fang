# string methods on closure-captured strings, reassign after capture
prefix = "data-"
tagger = fn(name): return prefix + name
print(tagger("x"))
prefix = "log-"
print(tagger("y"))
