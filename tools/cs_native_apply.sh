#!/usr/bin/env bash
# CS Engine CI: compile the registry's native mods (cs-native-mods.txt) into the game.
# 1. horizon-mixins (pinned) is installed once, 2. the bundles are fetched and re-pinned,
# 3. `horizon-mixins apply` stages the modified workspace beside the checkout, 4. the staged
# source files are copied back over the checkout, which then builds as usual (caches intact).
set -euo pipefail
cd "$(dirname "$0")/.."
MIXINS_GIT="https://github.com/horizon-rust/horizon-mixins"
MIXINS_REV="${CS_MIXINS_REV:-4342a0bfbe71d4d14185528d64e496d6022f16a1}"
MODS="$(cd .. && pwd)/cs-native-mods"
STAGE="$(cd .. && pwd)/cs-engine-stage"
if ! grep -qv '^\s*#' cs-native-mods.txt 2>/dev/null || ! grep -qE '^[a-z0-9_]+' cs-native-mods.txt; then
  echo "cs_native_apply: no native mods listed"; exit 0
fi
BIN="$HOME/.cargo/bin/horizon-mixins"
if [ ! -x "$BIN" ] || [ "$(cat "$HOME/.cargo/.horizon-mixins-rev" 2>/dev/null)" != "$MIXINS_REV" ]; then
  cargo install --locked --git "$MIXINS_GIT" --rev "$MIXINS_REV" horizon-mixins-cli
  echo "$MIXINS_REV" > "$HOME/.cargo/.horizon-mixins-rev"
fi
python3 tools/cs_native_mods.py "$MODS"
# The tool snapshots the whole tree and refuses symlinks; the repo keeps one convenience link.
find . -path ./.git -prune -o -type l -print | while read -r l; do rm -f "$l"; done
git rev-parse HEAD > .horizon-revision
# The fork builds without --locked: the lock gains the fork's own dependencies here, then
# horizon-mixins (offline, locked metadata) sees a complete lock.
cargo fetch
cargo metadata --format-version 1 --offline --locked > /dev/null
rm -rf "$STAGE"
# Plan errors name the bundle at fault (`file`): that mod is set aside and the rest retried,
# so one registry mod that drifted from this source never blocks the build.
SKIPPED="$(dirname "$MODS")/cs-native-skipped"
rm -rf "$SKIPPED"; mkdir -p "$SKIPPED"
for attempt in $(seq 1 40); do
  if "$BIN" apply --workspace . --mods "$MODS" --out "$STAGE" > "$STAGE.json" 2> "$STAGE.err"; then
    break
  fi
  cat "$STAGE.err"
  BAD=$(python3 - "$STAGE.err" "$MODS" <<'PY'
import json, os, sys
raw = open(sys.argv[1]).read()
try:
    d = json.loads(raw)
except Exception:
    d = None
    start = raw.find("{")
    while start != -1 and d is None:
        try:
            d = json.loads(raw[start:raw.rfind("}") + 1])
        except Exception:
            start = raw.find("{", start + 1)
    if d is None:
        sys.exit(0)
mods = os.path.realpath(sys.argv[2])
mod_id = d.get("mod_id") or ""
if mod_id and os.path.isdir(os.path.join(mods, mod_id)):
    print(mod_id)
    sys.exit(0)
f = d.get("file") or ""
f = os.path.realpath(f) if f else ""
if f.startswith(mods + os.sep):
    print(f[len(mods) + 1:].split(os.sep)[0])
PY
)
  if [ -z "$BAD" ] || [ ! -d "$MODS/$BAD" ]; then
    echo "cs_native_apply: horizon-mixins failed and no bundle to blame"; exit 1
  fi
  echo "::warning::native mod $BAD skipped: $(python3 -c "import json,sys;print(json.load(open(sys.argv[1])).get('message',''))" "$STAGE.err" | head -c 300)"
  python3 -c "import json,sys;print(json.load(open(sys.argv[1])).get('message',''))" "$STAGE.err" > "$SKIPPED/$BAD.txt"
  mv "$MODS/$BAD" "$SKIPPED/$BAD"
  rm -rf "$STAGE"
done
test -d "$STAGE" || { echo "cs_native_apply: apply never succeeded"; exit 1; }
python3 - "$(dirname "$MODS")/cs-native-mods.json" "$SKIPPED" <<'PY'
import json, os, sys
path, skipped = sys.argv[1], sys.argv[2]
d = json.load(open(path))
for name in sorted(os.listdir(skipped)):
    if name.endswith('.txt'):
        mod = name[:-4]
        d["built"] = [m for m in d["built"] if m["id"] != mod]
        d["skipped"].append({"id": mod, "reason": open(os.path.join(skipped, name)).read().strip()[:300]})
json.dump(d, open(path, 'w'), indent=2)
print("cs_native_apply: built", [m["id"] for m in d["built"]])
print("cs_native_apply: skipped", [m["id"] for m in d["skipped"]])
PY
python3 - "$STAGE" <<'PY'
import json, os, shutil, sys
stage = sys.argv[1]
changed = 0
for dirpath, dirs, files in os.walk(stage):
    dirs[:] = [d for d in dirs if d not in ('.git', 'target', '.local')]
    for f in files:
        src = os.path.join(dirpath, f)
        rel = os.path.relpath(src, stage)
        dst = os.path.join('.', rel)
        if os.path.exists(dst) and open(src, 'rb').read() == open(dst, 'rb').read():
            continue
        os.makedirs(os.path.dirname(dst) or '.', exist_ok=True)
        shutil.copy2(src, dst)
        changed += 1
        print('cs_native_apply: staged', rel)
print(f'cs_native_apply: {changed} file(s) changed by the mixins')
PY
rm -rf "$STAGE"
cp "$(dirname "$MODS")/cs-native-mods.json" cs-native-mods.json
