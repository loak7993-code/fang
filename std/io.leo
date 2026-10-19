extern fn leo_input_line() -> str from "leo_rt.h"

export fn input() -> str:
  return leo_input_line()

export fn read_line() -> str:
  return leo_input_line()
