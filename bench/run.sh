#!/usr/bin/env bash
# Benchmark suite: Leo vs Python vs C. Median of 5 runs, in milliseconds.
set -u
LEO="${LEO:-./target/release/leo}"
DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR/.."

timeit() {
  # median of 5 runs, seconds with millisecond precision
  local cmd="$1"
  local out=""
  for i in 1 2 3 4 5; do
    local t0=${EPOCHREALTIME/./}
    eval "$cmd" >/dev/null
    local t1=${EPOCHREALTIME/./}
    out="$out $((t1 - t0))"
  done
  echo "$out" | tr ' ' '\n' | grep -E '^[0-9]+$' | sort -n | sed -n '3p' | awk '{printf "%.3f", $1 / 1000000}'
}

run_leo() {
  local name="$1"
  local bin="/tmp/leo-bench-$name"
  "$LEO" build "bench/$name.leo" -o "$bin" --release >/dev/null 2>&1 || { echo "compile-failed"; return; }
  timeit "$bin"
}

run_python() {
  local name="$1"
  timeit "python3 bench/$name.py"
}

run_rust() {
  local name="$1"
  [ -f "bench/$name.rs" ] || { echo "n/a"; return; }
  rustc -O "bench/$name.rs" -o "/tmp/leo-bench-$name.rs.bin" 2>/dev/null || { echo "n/a"; return; }
  timeit "/tmp/leo-bench-$name.rs.bin"
}

run_cpp() {
  local name="$1"
  [ -f "bench/$name.cpp" ] || { echo "n/a"; return; }
  g++ -O2 -std=c++17 "bench/$name.cpp" -o "/tmp/leo-bench-$name.cpp.bin" 2>/dev/null || { echo "n/a"; return; }
  timeit "/tmp/leo-bench-$name.cpp.bin"
}

run_c() {
  local name="$1"
  [ -f "bench/$name.c" ] || { echo "n/a"; return; }
  cc -O2 -o "/tmp/leo-bench-$name.c.bin" "bench/$name.c" 2>/dev/null || { echo "n/a"; return; }
  timeit "/tmp/leo-bench-$name.c.bin"
}

printf "%-12s %10s %10s %10s %10s %10s\n" "bench" "leo(s)" "python(s)" "c(s)" "rust(s)" "cpp(s)"
for name in fib loopsum strbuild dictops sort; do
  printf "%-12s %10s %10s %10s %10s %10s\n" "$name" "$(run_leo $name)" "$(run_python $name)" "$(run_c $name)" "$(run_rust $name)" "$(run_cpp $name)"
done
