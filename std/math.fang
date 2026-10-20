extern fn sqrt(x: float) -> float from "math.h"
extern fn floor(x: float) -> float from "math.h"
extern fn ceil(x: float) -> float from "math.h"
extern fn round(x: float) -> float from "math.h"
extern fn fabs(x: float) -> float from "math.h"
extern fn sin(x: float) -> float from "math.h"
extern fn cos(x: float) -> float from "math.h"
extern fn tan(x: float) -> float from "math.h"
extern fn log(x: float) -> float from "math.h"
extern fn exp(x: float) -> float from "math.h"
extern fn pow(base: float, e: float) -> float from "math.h"
extern fn rand() -> int from "stdlib.h"

export let PI = 3.141592653589793
export let E = 2.718281828459045

export fn sqrt(x: float) -> float:
  return sqrt(x)

export fn floor(x: float) -> float:
  return floor(x)

export fn ceil(x: float) -> float:
  return ceil(x)

export fn round(x: float) -> float:
  return round(x)

export fn abs(x: float) -> float:
  return fabs(x)

export fn sin(x: float) -> float:
  return sin(x)

export fn cos(x: float) -> float:
  return cos(x)

export fn tan(x: float) -> float:
  return tan(x)

export fn log(x: float) -> float:
  return log(x)

export fn exp(x: float) -> float:
  return exp(x)

export fn pow(base: float, e: float) -> float:
  return pow(base, e)

export fn min(a: float, b: float) -> float:
  if a < b:
    return a
  return b

export fn max(a: float, b: float) -> float:
  if a > b:
    return a
  return b

export fn random() -> float:
  return rand() / 2147483647.0

export fn random_int(low: int, high: int) -> int:
  if high <= low:
    return low
  let span = high - low + 1
  return low + rand() % span
