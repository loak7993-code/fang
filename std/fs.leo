extern fn leo_fs_read_file(path: str) -> str from "leo_rt.h"
extern fn leo_fs_write_file(path: str, data: str) from "leo_rt.h"
extern fn leo_fs_append_file(path: str, data: str) from "leo_rt.h"
extern fn leo_fs_exists(path: str) -> bool from "leo_rt.h"
extern fn leo_fs_list_dir(path: str) -> str from "leo_rt.h"
extern fn leo_fs_mkdir(path: str) from "leo_rt.h"
extern fn leo_fs_remove(path: str) from "leo_rt.h"

export fn read_file(path: str) -> str:
  return leo_fs_read_file(path)

export fn write_file(path: str, data: str):
  leo_fs_write_file(path, data)

export fn append_file(path: str, data: str):
  leo_fs_append_file(path, data)

export fn exists(path: str) -> bool:
  return leo_fs_exists(path)

export fn mkdir(path: str):
  leo_fs_mkdir(path)

export fn remove(path: str):
  leo_fs_remove(path)

export fn list_dir(path: str) -> list[str]:
  let raw = leo_fs_list_dir(path)
  if len(raw) == 0:
    let empty: list[str] = []
    return empty
  return raw.split("\n")
