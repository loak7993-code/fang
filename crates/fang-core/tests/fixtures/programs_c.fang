extern fn sqrt(x: float) -> float from "math.h"

fn dist(ax: float, ay: float, bx: float, by: float) -> float:
  dx = bx - ax
  dy = by - ay
  return sqrt(dx * dx + dy * dy)

export fn describe(n: int) -> str:
  return f"number {n} has distance {dist(0.0, 0.0, float(n), 1.0)}"
