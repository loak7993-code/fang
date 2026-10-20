#!/usr/bin/env bash
# Drive the full 18-lesson course end to end: every expected answer
# passes, wrong answers are rejected, progress never skips.
set -u
DIR="$(cd "$(dirname "$0")/.." && pwd)"
FANG="${FANG:-$DIR/target/release/fang}"
[ -x "$FANG" ] || { echo "build fang first"; exit 1; }

progress="$HOME/.fg/learn-progress"
mkdir -p "$(dirname "$progress")"
echo 0 > "$progress"

pass=0
fail=0

lesson() {
  local num="$1" answer="$2"
  local out
  out=$(printf '%s\nquit\n' "$answer" | "$FANG" learn 2>&1)
  if echo "$out" | grep -q "correct!"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL lesson $num:"
    echo "$out" | head -6
  fi
  cur=$(cat "$progress")
  if [ "$cur" != "$num" ]; then
    echo "FAIL lesson $num: progress at $cur, expected $num"
    fail=$((fail + 1))
  fi
}

# The course array runs basics(7), dictionaries, structs, strings,
# comprehensions, functions-as-values, errors, imports, extern C,
# closures, then the final guessing game.
lesson 1 'hello, fang'
lesson 2 'fang'
lesson 3 '7'
lesson 4 '20'
lesson 5 '4'
lesson 6 '1'
lesson 7 '2'
lesson 8 'nova'
lesson 9 '0.5'
lesson 10 '1'
lesson 11 'xs'
lesson 12 '2'
lesson 13 'the disk is full'
lesson 14 '9.0'
lesson 15 '4.0'
lesson 16 'x'
lesson 17 'correct!'

echo 0 > "$progress"
out=$(printf 'nope\nquit\n' | "$FANG" learn 2>&1)
if echo "$out" | grep -q "correct!"; then
  echo "FAIL: wrong answer accepted"
  fail=$((fail + 1))
else
  pass=$((pass + 1))
fi
[ "$(cat "$progress")" = "0" ] || { echo "FAIL: progress moved on wrong answer"; fail=$((fail + 1)); }

echo "learn course: $pass checks passed, $fail failed"
[ "$fail" -eq 0 ]
