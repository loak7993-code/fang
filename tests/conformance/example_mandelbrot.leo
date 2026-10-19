# ASCII mandelbrot: floats, loops, and math together.
import math

fn main():
  let rows = 24
  let cols = 78
  for y in 0..rows:
    let line = ""
    for x in 0..cols:
      let cx = (float(x) * 3.5) / float(cols) - 2.5
      let cy = (float(y) * 2.0) / float(rows) - 1.0
      let zx = 0.0
      let zy = 0.0
      let i = 0
      while i < 40 and zx * zx + zy * zy < 4.0:
        let nx = zx * zx - zy * zy + cx
        zy = 2.0 * zx * zy + cy
        zx = nx
        i = i + 1
      if i >= 40:
        line = line + " "
      elif i > 20:
        line = line + "#"
      elif i > 10:
        line = line + "+"
      elif i > 5:
        line = line + "."
      else:
        line = line + " "
    print(line)
