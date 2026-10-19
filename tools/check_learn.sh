#!/usr/bin/env bash
# Every ```leo block in LEARN.md must run; every ```leo-expect-error block
# must fail to compile. The tutorial cannot rot.
set -u
DIR="$(cd "$(dirname "$0")/.." && pwd)"
LEO="${LEO:-$DIR/target/debug/leo}"
[ -x "$LEO" ] || LEO="$DIR/target/release/leo"
[ -x "$LEO" ] || { echo "build leo first: cargo build"; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

python3 - "$DIR/LEARN.md" "$tmp" << 'PYEOF'
import re
import sys

src = open(sys.argv[1]).read()
outdir = sys.argv[2]
prelude = "import json\nimport str\nimport math\n"
i = 0
for m in re.finditer(r"```(leo|leo-expect-error)\n(.*?)```", src, re.S):
    i += 1
    kind = "err" if m.group(1) == "leo-expect-error" else "run"
    with open(f"{outdir}/{i:03d}_{kind}.leo", "w") as f:
        f.write(prelude + m.group(2))
print(i)
PYEOF

ok=0
fail=0
for f in "$tmp"/*.leo; do
  kind=$(basename "$f" .leo)
  kind="${kind##*_}"
  if "$LEO" run "$f" >/dev/null 2>&1; then
    if [ "$kind" = "err" ]; then
      fail=$((fail + 1))
      echo "FAIL $(basename "$f"): expected a compile error, it ran"
    else
      ok=$((ok + 1))
    fi
  else
    if [ "$kind" = "err" ]; then
      ok=$((ok + 1))
    else
      fail=$((fail + 1))
      echo "FAIL $(basename "$f"):"
      "$LEO" run "$f" 2>&1 | head -4
    fi
  fi
done

echo "learn blocks: $ok ok, $fail failed"
[ "$fail" -eq 0 ]
