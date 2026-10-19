import fs
import str

fn main():
  fs.write_file("/tmp/leo-notes.txt", "first line\nsecond line\n")
  let content = fs.read_file("/tmp/leo-notes.txt")
  let lines = content.split("\n")
  print(f"{len(lines)} line(s)")
  for line in lines:
    if len(line) > 0:
      print(f"- {line.upper()}")
  fs.remove("/tmp/leo-notes.txt")
