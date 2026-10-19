extern fn leo_net_resolve(host: str, port: int) -> ptr from "leo_rt.h"
extern fn leo_tcp_send(sock: ptr, data: str) -> int from "leo_rt.h"
extern fn leo_tcp_recv_all(sock: ptr) -> str from "leo_rt.h"
extern fn leo_tcp_close(sock: ptr) from "leo_rt.h"

export fn tcp_connect(host: str, port: int) -> ptr:
  return leo_net_resolve(host, port)

export fn tcp_send(sock: ptr, data: str):
  let n = leo_tcp_send(sock, data)
  let _unused = n

export fn tcp_recv(sock: ptr) -> str:
  return leo_tcp_recv_all(sock)

export fn tcp_close(sock: ptr):
  leo_tcp_close(sock)

# http_get(url) fetches over plain HTTP/1.0. No TLS in v0.1.
export fn http_get(url: str) -> str:
  let rest = url
  if rest.starts_with("http://"):
    rest = rest.slice(7, len(rest))
  let slash = rest.find("/")
  let host = ""
  let path = "/"
  if slash < 0:
    host = rest
  else:
    host = rest.slice(0, slash)
    path = rest.slice(slash, len(rest))
  let sock = tcp_connect(host, 80)
  tcp_send(sock, "GET " + path + " HTTP/1.0\r\nHost: " + host + "\r\n\r\n")
  let response = tcp_recv(sock)
  tcp_close(sock)
  # Split headers from the body.
  let sep_pos = response.find("\r\n\r\n")
  if sep_pos < 0:
    return response
  return response.slice(4 + sep_pos, len(response))
