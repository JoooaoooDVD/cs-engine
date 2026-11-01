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

## Host input (CS control system → engine)

`cs_launch.host_input = true` puts the engine in host mode: input mode is forced to KeyboardMouse,
window touches are ignored, no touch HUD is drawn, `platform::capture_mouse` / IME requests only set
flags. CS Launcher's `HorizonBackend` queues events through these natives (`cs_bridge.rs`), drained
once per frame by `Game::android_host_input`:

| native | queue event | engine effect |
|---|---|---|
| `nativeHostKey(token, down)` | `HostEvent::Key` | `TouchControls::host_key` → `android_custom_events` (same path as Horizon custom buttons) |
| `nativeHostLook(dx, dy)` | `HostEvent::Look` | `MouseLook::on_move` while captured (camera) |
| `nativeHostCursor(x, y)` | `HostEvent::Cursor` | synthetic `WindowEvent::CursorMoved` |
| `nativeHostButton(b, down)` | `HostEvent::Button` | synthetic `WindowEvent::MouseInput` |
| `nativeHostScroll(x, y)` | `HostEvent::Scroll` | synthetic `WindowEvent::MouseWheel` (lines) |
| `nativeHostText(s)` | `HostEvent::Text` | `pad_osk_output(Output::Text)` (chat, signs, fields) |
| `nativeHostReleaseAll()` | `HostEvent::ReleaseAll` | releases held keys + buttons |
| `nativeHostPoll()` | — | bit 0 pointer captured, bit 1 keyboard wanted |

Key tokens are Minecraft key names (`key.keyboard.w`, `key.keyboard.left.shift`, …).
