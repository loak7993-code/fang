extern fn leo_time_now() -> float from "leo_rt.h"
extern fn leo_time_clock() -> float from "leo_rt.h"
extern fn leo_time_sleep(seconds: float) from "leo_rt.h"

export fn now() -> float:
  return leo_time_now()

export fn clock() -> float:
  return leo_time_clock()

export fn sleep(seconds: float):
  leo_time_sleep(seconds)
