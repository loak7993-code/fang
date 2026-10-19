extern fn leo_os_argc() -> int from "leo_rt.h"
extern fn leo_os_arg(i: int) -> str from "leo_rt.h"
extern fn leo_os_env(name: str) -> str from "leo_rt.h"
extern fn leo_os_getcwd() -> str from "leo_rt.h"
extern fn leo_os_system(cmd: str) -> int from "leo_rt.h"
extern fn leo_os_exit(code: int) from "leo_rt.h"

export fn args() -> list[str]:
  let out: list[str] = []
  let n = leo_os_argc()
  for i in 0..n:
    out.push(leo_os_arg(i))
  return out

export fn env(name: str) -> str:
  return leo_os_env(name)

export fn getcwd() -> str:
  return leo_os_getcwd()

export fn system(cmd: str) -> int:
  return leo_os_system(cmd)

export fn exit(code: int):
  leo_os_exit(code)
