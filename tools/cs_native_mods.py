#!/usr/bin/env python3
"""CS Engine: fetch the native mods of cs-native-mods.txt from the Horizon registry.

Each `.hnmod` (a zip: horizon-native.json + mixins/<id>/) is verified against the registry's
SHA-256, unpacked into `<out>/<id>/`, and its `[compatibility] horizon_revision` is re-pinned
to the commit being built (the mods use only the stable hook points; the launcher does the
same, launcher/src/native_mods.rs `repin`). The game's NATIVE_HOOKS_API must be at least each
mod's `hooks`.
"""
import hashlib, io, json, pathlib, re, subprocess, sys, urllib.request, zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
INDEX = "https://horizon-rust.github.io/horizon-mods/index.json"


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / ".local/cs-native-mods")
    ids = [l.strip() for l in (ROOT / "cs-native-mods.txt").read_text().splitlines()
           if l.strip() and not l.startswith("#")]
    if not ids:
        print("cs_native_mods: no mods listed")
        return
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    hooks_api = int(re.search(r"NATIVE_HOOKS_API: u32 = (\d+)",
                              (ROOT / "crates/horizon-client/src/game/native_hooks.rs").read_text()).group(1))
    index = json.load(urllib.request.urlopen(INDEX, timeout=60))
    projects = {p["id"]: p for p in index["projects"]}
    revoked = {r.get("id") for r in index.get("revoked", []) if isinstance(r, dict)}
    out.mkdir(parents=True, exist_ok=True)
    built = []
    for id_ in ids:
        p = projects.get(id_)
        if p is None or p["kind"] != "native-mod":
            sys.exit(f"cs_native_mods: {id_}: not a native mod in the registry")
        if id_ in revoked:
            sys.exit(f"cs_native_mods: {id_}: revoked")
        v = p["versions"][0]
        f = v["file"]
        data = urllib.request.urlopen(f["url"], timeout=120).read()
        if hashlib.sha256(data).hexdigest() != f["sha256"]:
            sys.exit(f"cs_native_mods: {id_}: sha256 mismatch")
        z = zipfile.ZipFile(io.BytesIO(data))
        manifest = json.loads(z.read("horizon-native.json"))
        hooks = manifest.get("hooks")
        if hooks is None:
            sys.exit(f"cs_native_mods: {id_}: pins an exact Horizon revision, not the hook points")
        if hooks > hooks_api:
            sys.exit(f"cs_native_mods: {id_}: needs hooks {hooks}, the game has {hooks_api}")
        dest = out / id_
        if dest.exists():
            import shutil
            shutil.rmtree(dest)
        for name in z.namelist():
            if name.startswith("/") or ".." in name.split("/"):
                sys.exit(f"cs_native_mods: {id_}: bad entry {name}")
        z.extractall(dest)
        for toml in dest.glob("mixins/*/horizon-mixin.toml"):
            text = toml.read_text()
            text = re.sub(r'(?m)^horizon_revision\s*=\s*"[0-9a-f]+"', f'horizon_revision = "{revision}"', text)
            toml.write_text(text)
        built.append({"id": id_, "version": v["version"], "name": p["name"], "hooks": hooks,
                      "license": p.get("license"), "source": p.get("source"), "sha256": f["sha256"]})
        print(f"cs_native_mods: {id_} {v['version']} (hooks {hooks})")
    (out / "cs-native-mods.json").write_text(json.dumps(built, indent=2))


if __name__ == "__main__":
    main()
