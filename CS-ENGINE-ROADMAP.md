# CS Engine — source inspection & development roadmap

Fork: `JoooaoooDVD/horizon-android` (branch `main`, head d37bc03, feature `cs-host`) — upstream Horizon Android 0.7.7, MIT.
Host: CS Launcher Plus (`JoooaoooDVD/CSL2`, Horizon integration at a636d2b).
Goal: turn the fork, step by step and without a rewrite, into an independently maintained **CS Engine** (Minecraft Java 26.3 runtime by Craft Studio) with its own identity, options and features — while every current feature keeps working.

---

## 1. What the code base is

| Area | Size | What it is |
|---|---|---|
| Whole workspace | ~860 000 lines Rust, 37 crates, 3 767 unit tests | A from-scratch **Rust re-implementation of the Minecraft Java 26.3 client** (not a JVM, not a wrapper). Mojang assets are never shipped: the installer downloads them from Mojang and the engine reads them. |
| `horizon-client` | 422 k lines / 678 files | The game: `game.rs` (20 k, main loop + state), `title.rs` (title screen, world list), `menu.rs`, `save.rs`, mobs/AI, LAN/multiplayer, commands, advancements, `debug_overlay.rs` (F3), `android/` (platform: `cs_bridge.rs` ours, `catalog.rs` = `android-launcher.json` config, `launcher*.rs` = Horizon's own launcher UI we do **not** use, `native_ime.rs`, `performance.rs`, `diagnostics.rs`, `game_launch.rs`). |
| `horizon-client-render` | 120 k / 178 files | wgpu renderer: chunk mesher, LOD, shaders (`*.wgsl`), HUD (`hud*.rs`), `options_ui.rs` + `settings.rs` (vanilla Options screens), `pack_screen.rs`, `shader_pack_list.rs`, `touch.rs`/`touch/custom.rs` (Horizon touch controls, disabled in host mode), `android_runtime.rs` (host input channel, ours), `title_edition.rs` ("RUST EDITION" banner), `graphics_startup.rs` (Vulkan→GL recovery), `crash.rs`, `logging.rs`. |
| `horizon-world` | 174 k / 534 files | World gen (vanilla parity), chunk storage, Anvil I/O. |
| `horizon-core` | 20 k | Shared types, `release.rs` (`HORIZON_VERSION` / nightly labels), `VANILLA_VERSION = "26.3"`. |
| `horizon-protocol`, `horizon-server`, `horizon-relay`, `horizon-friends`, `horizon-lan` | ~33 k | Network protocol, integrated/dedicated server, relay + friends ("Join a Friend"). |
| `horizon-mods`, `horizon-mod-sdk`, `cargo-horizon-mod`, `horizon-shaders`, `horizon-shader-api` | ~21 k | `.hmod` native mod runtime (WASM-style sandboxed guests + native hooks `game/native_hooks.rs`), shader pack API. |
| `horizon-theme` | 1 k + fonts/icons | **Design system**: palette, type scale, radii, bundled fonts (Geist, Pixelify Sans, Bricolage Grotesque), icons. Used by `ui_theme.rs` for Horizon's own overlays (OSK, `horizon.txt` prefs) and by the Android launcher UI. |
| `horizon-prepare`, `horizon-assets`, `horizon-port-pack` | ~26 k | Asset/resource preparation (`engine-data.zip`), pack conversion. |
| `horizon-telemetry*`, `horizon-anticheat`, `horizon-bench`, `horizon-bots` | ~10 k | Optional: telemetry (off for us), anticheat for servers, benchmarks, bot clients. |
| `packaging/android`, `.github/workflows/{android-source,cs-engine}.yml`, `tools/android_build.py` | — | NDK/cargo build, our `cs-engine.yml` publishes `cs-engine-latest`. |
| `docs/` (~80 files), `ROADMAP.md`, `AGENTS.md` | — | Architecture notes, per-release notes, decompilation status, design system (`docs/design-system.md`). Valuable: keep reading these before touching a subsystem. |

**Key architectural facts**
- One process, one `winit` NativeActivity window, one `Game` state machine; everything (title, loading, in-game, menus, HUD, F3) is drawn by the engine's own immediate-mode batches (`HudBatch`, `ui.rs`, `screen_draw.rs`). There is no HTML/XML UI — every screen is Rust code → fully modifiable.
- Two UI layers exist: **vanilla-parity screens** (title, options, packs, pause, chat… in `title.rs`, `menu.rs`, `options_ui.rs`, `game/screens.rs`) and **Horizon-own overlays** built on `horizon-theme` (`ui_theme.rs`, `osk.rs`, launcher pages). CS identity goes into the second layer first, the first layer carefully.
- Configuration: `options.txt` (vanilla, untouched by design), `horizon.txt` (Horizon-only prefs, `ui_theme::preference`), `android-launcher.json` (`android/catalog.rs`: renderer, render scale, orientation, input host, device profile, perf recording), `horizon-touch.json`, `horizon-keypad.json`, `horizon-mods.json`. Adding an option = add a field in `catalog.rs`/`settings.rs` + UI row — a well-trodden path.
- Branding today: the string `"Horizon"` appears in ~143 files (mostly log messages/comments); user-visible spots are few: title screen version line `"Horizon 26.3 (Rust)"` (`title.rs:1905`), `title_edition.rs` ("RUST EDITION"), F3 header (`debug_overlay.rs`), crash/exit reports (`crash.rs`, `android/exit_reports.rs`), on-screen keyboard/overlays (`ui_theme.rs`), Android launcher pages (unused), library/process/file names, `horizon_core::release` version labels.
- Legal frame: MIT (rename/modify/ship allowed; keep copyright + license text in the app and `THIRD_PARTY_NOTICES.md`). Mojang assets are downloaded per user, never redistributed; "Minecraft" stays Minecraft; the "Not affiliated with Mojang or Microsoft" notice stays. Nothing may bypass Microsoft authentication/ownership.

---

## 2. What can be modified, and how risky it is

| Component | Change type | Risk | Where |
|---|---|---|---|
| Names, labels, version strings, file/lib/process names | Rebrand | Low (mechanical, test-covered) | `horizon-core/release.rs`, `title.rs`, `debug_overlay.rs`, `crash.rs`, `exit_reports.rs`, `packaging/android`, `cs-engine.yml`, CSL2 `HorizonEngine/HorizonLaunch` |
| Title screen (logo area, edition banner, footer line, buttons) | UI edit | Low–medium | `title.rs`, `title_edition.rs`, `title/onboarding.rs` |
| Loading / world-loading screens | UI edit | Low | `game/world_loading.rs`, `level_loading.rs`, `game/startup_menu.rs` |
| Pause / in-game menus, options pages | UI edit + new rows | Medium (vanilla parity expectations) | `game/screens.rs`, `menu.rs`, `options_ui.rs`, `settings.rs` |
| F3 debug overlay | UI edit + new lines | Low | `debug_overlay.rs`, `debug_entries.rs`, `perf_overlay.rs` |
| HUD (hotbar, crosshair, overlays, watermark) | UI edit | Low–medium | `hud*.rs`, `extras_hud.rs`, `hud_overlays.rs` |
| Design system (fonts, colors, radii) | Theme | Low | `horizon-theme` (swap fonts/icons/palette → CS look everywhere the theme is used) |
| Engine options (renderer, scale, FPS cap, FOV extras, perf toggles, device profiles) | New settings | Low–medium | `android/catalog.rs`, `settings.rs`, `options_ui.rs`, `device_runtime.rs`, `cs_bridge.rs` (pass from CS profile) |
| Controls (host input, keybinds, gamepad, gyro, keypad) | Extend | Medium | `android_runtime.rs`, `game/android_input.rs`, `game/pad.rs`, `device_gyro.rs`, CS side `HorizonBackend` |
| Horizon launcher UI, login, friends, telemetry | Remove / feature-gate | Low (we already bypass them) | `android/launcher*.rs`, `login.rs`, `friends_screen.rs`, `horizon-telemetry` — gate behind a Cargo feature off in CS builds |
| Mods (`.hmod`) runtime & SDK | Extend / rename SDK | Medium–high (ABI for mod authors) | `horizon-mods`, `horizon-mod-sdk`, `game/native_hooks.rs` |
| Shaders, post chain, LOD, performance | Deep | High (needs device testing per GPU) | `horizon-client-render` |
| World gen, protocol, mobs, gameplay | Deep | Very high — only for bug fixes / parity, never for identity | `horizon-world`, `horizon-protocol`, `mobs/` |

---

## 3. Maintenance strategy (how this stays "independently maintained" without dying)

1. **Three layers in one repo**
   - `cs/` crates and files that are 100 % ours (`cs-brand` crate: names, labels, palette, assets; `cs-host` bridge; CS screens/features).
   - **Thin hooks** in upstream code (`#[cfg(feature = "cs")]` or calls into `cs_brand::NAME`) — small, greppable, easy to re-apply.
   - Upstream code otherwise untouched → `git merge upstream/main` stays feasible for new Minecraft versions and fixes. Decide per upstream release whether to merge (`docs/android-upstream-review-*.md` shows upstream already works like this against Mojang releases).
2. **One identity source**: `crates/cs-brand/src/lib.rs` → `ENGINE_NAME = "CS Engine"`, `SHORT = "CS"`, `TAGLINE = "by Craft Studio"`, `VERSION`, `LIB_NAME`, file prefixes, palette, fonts, icons. Everything user-visible reads from here; `"Horizon"` only remains in credits/notices.
3. **Compatibility contract with CS Launcher**: `cs_launch` JSON + `HorizonNative` JNI (12 symbols) + install progress contract (CS-FORK-NOTES.md) are versioned; rename happens on both sides in the same release.
4. **Quality gates**: existing `cargo test` (3 767 tests) + `cargo clippy` in CI on every push (add to `cs-engine.yml`), device matrix for each milestone (your devices: Adreno + Mali at least), performance recording (`performance.rs`) before/after risky changes.
5. **Docs**: `docs/cs/` — architecture deltas, option reference, release notes per CS Engine version. Upstream `docs/` kept as reference.

---

## 4. Roadmap

### Milestone 0 — Foundation (1 sprint)
- `cs-brand` crate + `cs` Cargo feature; CI: tests + clippy; `docs/cs/architecture.md`; version scheme `CS Engine 1.0.0 (MC 26.3, core 0.7.7)`.
- Decide final name/tagline/logo (needed by M1).

### Milestone 1 — Identity, visible everywhere (1–2 sprints)
- Title screen: version line → `CS Engine 1.0 · Minecraft Java 26.3`, edition banner → "CS EDITION" (same `title_edition.rs` technique, glyphs ours), footer "Not affiliated…" kept, small Craft Studio mark; optional CS panorama/logo overlay (ours, not Mojang's).
- Loading / world-loading screens: CS progress style, "CS Engine is preparing…" lines.
- F3: header `CS Engine <ver> (Minecraft 26.3)`, renderer/profile line from CS profile.
- Pause menu footer, crash + exit reports, logs, window/task label, library `libcs_engine.so`, process `:csengine`, files `cs-*.json`, folders `.local/cs-engine/26.3` (with migration from the Horizon paths so nobody re-downloads).
- Theme: CS palette/fonts in `horizon-theme` → OSK and overlays look like CS Launcher.
- Horizon launcher UI/login/friends/telemetry compiled out in CS builds.
- Credits/About: MIT notice "Built on Horizon Android", `THIRD_PARTY_NOTICES`.

### Milestone 2 — Options & controls (2 sprints)
- New engine options (surfaced in CS profile editor and in-game Options → "CS Engine" page): FPS limit, VSync, render scale live slider, render/simulation distance presets, dynamic resolution, shadow/clouds/particles quality, chunk builder threads, texture mipmap/anisotropy, low-power mode, device profile override, performance HUD toggle, screenshot folder, log level.
- Controls: CS control system stays the one control system. Add: per-profile keybind map for CS buttons (custom actions beyond GLFW keys), gyro aim toggle + sensitivity, gamepad remap page in CS, in-game quick-edit of control layout, haptics.
- `cs_launch` v2: every option from the CS profile reaches the engine without files.

### Milestone 3 — CS features (ongoing, one feature per sprint)
- CS in-game menu (drawer) matching the Java engine: controls on/off, edit, keyboard, screenshots, performance panel, "report problem" (collect logs/exit report → CS crash tooling).
- Performance panel (frame time graph, GPU time per pass — `perf_overlay.rs` already has the data).
- Resource/shader pack manager pages inside the game with CS design; `.hmod` manager (enable/disable, info) + CS Mod SDK rename (`cargo-cs-mod`, with compatibility shim for existing `.hmod`).
- Virtual LAN integration (CS multiplayer) using the engine's LAN/relay code.
- Account UX: profile skins/capes via CS account bridge (never bypass MSA).

### Milestone 4 — Independence (continuous)
- Own release train (CS Engine x.y), changelogs, device compatibility list, crash analytics into CS Advanced Crash Auto Fix (classifier categories for engine crashes).
- Upstream sync policy: review each Horizon release (`docs/android-upstream-review-*`), cherry-pick fixes; when Mojang ships a new Minecraft version, evaluate upstream parity work vs. doing it ourselves.
- Long term: contribute general fixes back upstream (keeps merges cheap and is good citizenship under MIT).

---

## 5. Risks & rules
- **Never ship Mojang assets** or alter the installer's source of truth (official Mojang URLs + checksums). Branding is drawn by us, not copied from `minecraft.png`.
- Keep "Minecraft" as the game name, keep the non-affiliation notice, keep MIT notices.
- Deep renderer/world changes only with device testing and performance recordings; parity tests must stay green.
- One identity source (`cs-brand`), thin hooks — otherwise upstream merges become impossible within months.
- All APK/engine builds via GitHub Actions; no feature is "done" before a device test.

## 6. First sprint (concrete, after your go)
1. Name/tagline/logo decision from you.
2. Fork: `crates/cs-brand`, `cs` feature, CI tests+clippy, `docs/cs/architecture.md` (this document, trimmed).
3. Fork: title version line, edition banner, F3 header, loading lines, crash/exit report names → `cs-brand`.
4. Fork + CSL2: library/process/path rename with migration; CSL2 strings/classes "CS Engine"; credits notice.
5. CI build → your device test (title screen, F3, loading, crash report name, existing profile still plays without re-download).

---

## Sprint 1 report (2026-11-01) — CI passed, awaiting device test

**Fork `JoooaoooDVD/horizon-android` main:** `56bbcb17e` + `a066860f7` — workflow run 41332725316 SUCCESS (tests / type-check+clippy / build / publish). Release `cs-engine-latest` = **CS Engine build 6** (`libcs_engine-arm64-v8a.so`, BUILD.json `engine: cs-engine`, `cs_engine_version: 1.0.0`, core 0.7.7).

| Area | Change |
|---|---|
| `crates/cs-brand` (new) | Single source of identity: `NAME`, `TAGLINE`, `BRAND` (`cs-engine`), `LIB` (`cs_engine`), `VERSION` (`CS_ENGINE_VERSION`), `COMMIT`, `title_line()`, `full_version()`, `crash_report_title()` + unit test |
| Title screen (`title.rs`) | `CS Engine 1.0.0 · Minecraft Java 26.3` + `Powered by Craft Studio` line; "Not affiliated with Mojang or Microsoft" kept |
| Logo banner (`title_edition.rs`) | "JAVA EDITION" → **"CS EDITION"** (own C glyph, "EDITION" shifted to stay centred; resource-pack banners untouched); tests rewritten |
| F3 (`debug_keys.rs`) | `Minecraft 26.3 (26.3/cs-engine)` + `CS Engine 1.0.0 (core 0.7.7, <commit>)` |
| Crash report (`crash.rs`) | `---- CS Engine Crash Report ----`, `CS Engine Version`, `Engine Core Version`, saved/crashed lines |
| Library | `libhorizon_android.so` → **`libcs_engine.so`**; JNI symbols unchanged |
| CI (`cs-engine.yml`) | New host `tests` job (cs-brand, horizon-core, render identity tests) gates publish; env `CS_ENGINE_VERSION`/`CS_ENGINE_COMMIT`; release renamed; stale `libhorizon_android*` assets deleted |

**CSL2 `ffe75d6`** — run 41334355127 SUCCESS; APK verified (`lib/arm64-v8a/libcs_engine.so`, BUILD.json build 6).
Manifest `android.app.lib_name=cs_engine`, `HorizonNative.LIBRARY`, fetch script asset names, all EN + Hinglish strings "Horizon" → "CS Engine", home/picker labels, backend name, About credit "Horizon Android (MIT)". Internal ids (`engine=horizon`, class names, `<data>/horizon` home, `:horizon` process) unchanged → existing profiles/installs keep working without re-download.

**Not in Sprint 1:** `banner_create_horizon.webp` artwork (image, not text), internal file/dir names, loading-screen text (vanilla's, no Horizon text there).

**Device test checklist:** title shows `CS Engine 1.0.0 · Minecraft Java 26.3` + tagline; logo reads "CS EDITION" centred; F3 shows cs-engine lines; existing CS Engine profile launches without re-install; CS controls visible; crash report (if any) titled "CS Engine Crash Report".

---

## Round 70 — Input compatibility (Priority 1–3) and the extended plan

### Root causes found (source inspection, both repos)

| # | Symptom | Root cause | Fix |
|---|---|---|---|
| 1 | Taps on "Continue"/menu buttons do nothing on some devices | The CS overlay reports the cursor in **overlay-window pixels**; the engine fed them straight in as render-target pixels. Horizon's own touches go through winit's `native_pointer_position` = `(point − content origin) × render-scale`. Any profile with **Resolution scale < 100%** (or a safe-inset/aspect letterbox) therefore landed every tap short of its button — device-specific only because it depends on the chosen scale/cutout. | Engine: `host_map_pointer()` applies the same mapping to every `HostEvent::Cursor`; geometry logged once per change. Launcher: engine window laid out edge-to-edge/short-edges like the control window so both cover the same pixels. |
| 2 | After "Continue" the game sometimes stays un-captured / camera dead | The CS control window **holds Android window focus** (it owns touch and keyboard). When the focus callbacks arrive after the overlay attaches, the engine gets `Focused(false)`, which refuses pointer capture (`capture && !focused → return`) and drops look input. Timing differs per device/ROM. | Engine: in host mode `Focused(false)` is ignored (the host sends `ReleaseAll` on pause itself). |
| 3 | Mouse control button does nothing | `GameActivity.toggleMouse()` uses a static `weakCursor` set only in `GameActivity.onCreate`. `HorizonGameActivity` runs in its own process and never registered its cursor view → `toggleMouse` returned early, always. | `GameActivity.registerCursorView()` called by `HorizonGameActivity`; `PREF_VIRTUAL_MOUSE_START` honoured too. Same cursor, same touchpad processor, same toggle — no second mouse system. |

### Input diagnostics (no per-event logging)
- Engine: `cs-exit.json` now carries `host_input: {pushed, processed, geometry:{window, render, origin, scale}}` + `cs_engine` version; one log line `CS host input: window …, render …, content origin …, pointer scale …, focused …` whenever the mapping changes.
- Launcher: `<engine home>/cs-input-diagnostics.json` at pause/exit: device, Android, real display size/density, orientation, engine window / control window / input view sizes, `CallbackBridge.window*`, focus of both windows, grab state, cursor position/visibility, controls visible/path, backend counters (cursor/look/button/key/scroll/text sent, grab changes).
- Reading it: `sent_button > 0` but `processed` ≈ 0 → events lost between JNI and the game loop; `processed` ≈ `pushed` but no reaction → mapping (`geometry`) or focus; `sent_* == 0` → the CS overlay never saw the touch (window layering/focus on that device).

### Input compatibility requirements (standing)
- Every host pointer coordinate goes through the same transform as native touches (render scale, insets, aspect).
- Engine window and control window must cover identical screen pixels (both edge-to-edge, cutout short-edges).
- Engine behaviour in host mode must not depend on Android window focus.
- One cursor, one toggle, one touch processor: the CS control system's (`GameView`/`GameCursorView`), never a parallel engine mouse.
- Any device-specific report is first answered with the two diagnostics files, not a guess.
- Not "fixed" until confirmed on at least one device that previously failed.

### Extended feature plan (M2–M4, in small testable milestones)
**A. CS Engine settings (in-game section)** — FPS limiter, VSync, render resolution scaling, dynamic resolution, performance presets, low-end optimisation, render/simulation distance, graphics quality, particles, shadows, clouds, performance monitor, memory info. Per-profile where it makes sense (CSL2 instance JSON → `cs_launch` extras → engine config).
**B. Advanced touch controls** — responsiveness, custom mapping, mouse sensitivity, cursor speed, touch sensitivity, gyroscope, gamepad customisation, in-game layout editing, opacity/size, quick mouse↔gameplay switch. All on top of the existing CS control system.
**C. CS in-game menu** — controls, mouse mode, keyboard, performance, screenshots, engine settings, debug info, report problem. CS Launcher Plus design language.
**D. Performance** — frame pacing, CPU/GPU/memory, chunk rendering, texture memory, shaders, device GPU compatibility. Only measurable wins (`record_performance` traces before/after); never at the cost of stability.
**E. Customisation** — loading screens, pause menu, settings page, performance overlay, debug screen, branding, notifications. Gameplay stays recognisable.

### Development priorities (order of work)
1. Touchscreen + Continue button (done in code; device confirmation pending)
2. Mouse button + cursor visibility (done in code; device confirmation pending)
3. Input compatibility across devices (diagnostics shipped; collect from a failing device)
4. Remaining M1 branding (create-profile artwork, loading/pause/notification polish)
5. M2 settings + control enhancements
6. Further CS Engine features (C, D, E)

Rules: preserve working features; no unnecessary rewrites; keep CSL2 ↔ engine compatible; never break profiles; keep licence notices; GitHub Actions only; tests before publish; device-specific bugs are not "fixed" on CI green; lightweight ARM64 focus.
