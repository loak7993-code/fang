import json

fn main():
  let v = json.parse("[1, 2, 3]")
  print(len(json.as_list(v)))
