#!/usr/bin/env bash
# Drive `leo learn` end to end: every lesson's expected answer must
# pass, every wrong answer must be caught with output shown.
set -u
DIR="$(cd "$(dirname "$0")/.." && pwd)"
LEO="${LEO:-$DIR/target/release/leo}"
[ -x "$LEO" ] || { echo "build leo first"; exit 1; }

progress="$HOME/.leo/learn-progress"
mkdir -p "$(dirname "$progress")"
echo 0 > "$progress"

pass=0
fail=0

lesson() {
  local num="$1" answer="$2"
  # Feed the answer, then "quit" to exit.
  local out
  out=$(printf '%s\nquit\n' "$answer" | "$LEO" learn 2>&1)
  if echo "$out" | grep -q "correct!"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL lesson $num:"
    echo "$out" | head -6
  fi
  # The course advances progress itself; make sure we are at lesson num.
  cur=$(cat "$progress")
  if [ "$cur" != "$num" ]; then
    echo "FAIL lesson $num: progress at $cur, expected $num"
    fail=$((fail + 1))
  fi
}

lesson 1 'hello, leo'
lesson 2 'leo'
lesson 3 '7'
lesson 4 '20'
lesson 5 '4'
lesson 6 '1'
lesson 7 '2'
lesson 8 'correct!'

# A wrong answer must not advance progress.
echo 0 > "$progress"
out=$(printf 'wronganswer\nquit\n' | "$LEO" learn 2>&1)
if echo "$out" | grep -q "correct!"; then
  echo "FAIL: wrong answer accepted"
  fail=$((fail + 1))
else
  pass=$((pass + 1))
fi
cur=$(cat "$progress")
if [ "$cur" != "0" ]; then
  echo "FAIL: progress advanced on wrong answer ($cur)"
  fail=$((fail + 1))
fi

echo "learn: $pass checks passed, $fail failed"
[ "$fail" -eq 0 ]
