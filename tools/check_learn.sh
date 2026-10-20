#!/usr/bin/env bash
# Every ```fang block in LEARN.md must run; every ```fang-expect-error block
# must fail to compile. The tutorial cannot rot.
set -u
DIR="$(cd "$(dirname "$0")/.." && pwd)"
FANG="${FANG:-$DIR/target/debug/fang}"
[ -x "$FANG" ] || FANG="$DIR/target/release/fang"
[ -x "$FANG" ] || { echo "build fang first: cargo build"; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

python3 - "$DIR/LEARN.md" "$tmp" << 'PYEOF'
import re
import sys

src = open(sys.argv[1]).read()
outdir = sys.argv[2]
i = 0
for m in re.finditer(r"```(fang|fang-expect-error)\n(.*?)```", src, re.S):
    i += 1
    kind = "err" if m.group(1) == "fang-expect-error" else "run"
    # Blocks that use the Python bridge need --python to link.
    needs_py = "import py" in m.group(2)
    with open(f"{outdir}/{i:03d}_{kind}{'_py' if needs_py else ''}.fg", "w") as f:
        prelude = "" if needs_py else "import json\nimport str\nimport math\n"
        f.write(prelude + m.group(2))
print(i)
PYEOF

ok=0
fail=0
for f in "$tmp"/*.fg; do
  base=$(basename "$f" .fg)
  kind="${base##*_}"
  if [[ "$kind" == "py" ]]; then
    kind="run"
    PYFLAG="--python"
  else
    PYFLAG=""
  fi
  if "$FANG" run "$f" $PYFLAG >/dev/null 2>&1; then
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
      "$FANG" run "$f" 2>&1 | head -4
    fi
  fi
done

echo "learn blocks: $ok ok, $fail failed"
[ "$fail" -eq 0 ]
