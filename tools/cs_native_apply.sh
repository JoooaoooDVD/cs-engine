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
cargo fetch --locked
rm -rf "$STAGE"
"$BIN" apply --workspace . --mods "$MODS" --out "$STAGE" > "$STAGE.json"
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
cp "$MODS/cs-native-mods.json" cs-native-mods.json
