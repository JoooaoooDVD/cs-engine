# Known issues

Every game release lists the bullets below under "Known issues" (tools/release_train.py). Keep them short and
written for players; remove a bullet in the pull request that fixes it.

- Shader packs have been removed for now. They will come back later, written again from scratch. Packs in your
  `shaderpacks` folder are left where they are.


## CS Engine: content on external storage (build 19)

CS Launcher Plus keeps profiles under `Android/data/<pkg>/files/instances/<name>`
(FUSE/sdcardfs), where `symlink()` fails with `EPERM`. The resource-pack overlay
(`horizon-client-render/src/packs.rs`) and the content-mod overlay
(`horizon-client/src/mod_content.rs`) are built from symlinks, so every pack reload
failed with *resource reload failed* and `.hmod` content never loaded. Since build 19
`cs_bridge` points `android_runtime::configure_overlay_dir` at internal storage
(`<home>/overlay/<profile>-<hash>/`), one folder per profile.

Native `.hnmod` mods (43 of the 45 registry projects) are Rust crates compiled into the
engine; the shipped CS Engine cannot load them at runtime. They are now listed in
Options > Mods with "(Native .hnmod - not supported)" instead of vanishing silently.
