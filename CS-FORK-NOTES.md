# CS Launcher Plus fork of Horizon Android

This branch (`cs-host`) is Craft Studio's maintained fork of
[Vasysik/horizon-android](https://github.com/Vasysik/horizon-android), itself an Android port of
[Horizon](https://github.com/horizon-rust/horizon) by Ludovic Marie and the Horizon contributors (MIT).

CS Launcher Plus embeds `libhorizon_android.so` as a second, independent game engine
("Horizon Engine · Minecraft Java 26.3"). The Rust launcher UI, account panel and catalog of
this repository are not shown inside CS Launcher Plus; CS Launcher Plus provides installation,
profiles, accounts, controls and settings and drives the engine through the `cs-host` feature.

## What this fork changes

| Area | Change |
|:--|:--|
| `crates/horizon-android/Cargo.toml` | cargo feature `cs-host` |
| `crates/horizon-client/src/android/cs_bridge.rs` | **new** — intent-driven launch (`cs_launch` JSON extra), JNI installer/queries for `com.craftstudio.cslauncher.engine.horizon.HorizonNative` |
| `crates/horizon-client/src/android_main.rs` | with `cs-host`, a `cs_launch` extra bypasses the Rust launcher and runs the game directly |
| `tools/android_build.py` | `--cs-host` flag |
| `.github/workflows/cs-engine.yml` | builds and publishes the engine artefacts consumed by the CS Launcher Plus build |

Without the `cs-host` feature, or without the `cs_launch` extra, the library behaves exactly as upstream.
Engine, renderer, installer, input and diagnostics code is unchanged; upstream is merged periodically.

## Licensing

Horizon is MIT licensed; see `LICENSE` and `THIRD_PARTY_NOTICES.md` (both shipped inside CS Launcher Plus).
Minecraft resources are never redistributed: the engine downloads them on the device from official
Mojang endpoints with pinned hashes, as upstream does. Horizon and this fork are not affiliated with or
endorsed by Mojang or Microsoft. Fabric, Forge and NeoForge mods do not run on this engine.
