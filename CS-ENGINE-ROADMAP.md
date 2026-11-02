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

---

## Round 71 — Expanded feature plan (CS Engine as a complete Android gaming engine)

Inspection first: what already exists and will be **reused, not duplicated**.

| Area | Already in the engine fork | Already in CS Launcher Plus | Gap CS Engine fills |
|---|---|---|---|
| Skins & capes | `skins.rs` (1367 l.): MSA profile textures (signed), legacy 64×32 → 64×64 (`process_legacy_skin`), cape layout 64×32 / 22×17 (`process_cape`), **offline skin/cape via `HORIZON_OFFLINE_SKIN`, `HORIZON_OFFLINE_SKIN_MODEL`, `HORIZON_OFFLINE_CAPE`** (read once at start); `modelPart_cape` option (vanilla cape visibility) | `skins/` package (2554 l.): `SkinLibrary` slots, `LocalSkinStore` (skins + **capes**, selection per account), `PlayerPreviewView` 3D 360°, `SkinEditorFragment`, `MinecraftSkinApi` (official upload/model change for MSA), `PremiumProfileLookup` | Hand the CS selection to the engine per launch; hot-switch without restart; per-profile cape; in-engine Skin & Cape page using the same store |
| Touch / controls | `touch.rs` (1784 l.) Horizon touch UI (unused in host mode), `device_gyro.rs`, `android_gamepad.rs`, `host_key/action:*` tokens | CS Control System (`ControlLayout`, editor, profiles, opacity/size, `GameView` touchpad, gamepad) | Gyro → host look, per-profile control files (already `cs_controls`), sensitivity/cursor speed settings surfaced in the CS drawer |
| Graphics / settings | `options_ui.rs`, `settings.rs`, `catalog.rs` (704 settings), `display.rs` render scale (25–100), `performance.rs`, `perf_overlay.rs`, `frame_scale.rs` (dynamic), `shader_catalog.rs` | Profile JSON (`horizonRenderScale`, renderer), `cs_launch` extras | Presets (Potato/Low/Balanced/High), FPS limiter, VSync, entity/particle/mipmap/texture controls, GPU compatibility profiles, low-end mode — as `cs_launch`→engine config, saved per profile |
| Packs / shaders | `packs.rs`, `pack_screen.rs` (vanilla pack screen), `shader_pack_list.rs`, `documents.rs` (Android document import) | Resource browser (Modrinth/CurseForge), per-profile folders | ZIP import from the CS drawer, compatibility detection (pack_format vs 26.3), priority ordering UI, shader config page |
| Worlds | `horizon-world` (174k l.), world select/create screens, `datapack.rs` | File manager, profile folders | Rename/icon/backup/restore/export/import/datapacks from the drawer; delete confirmation; quick switch |
| Performance | `performance.rs` (record_performance traces), `perf_overlay.rs`, F3 | Play-time page, device info | Performance Center page + history + automatic device profile recommendation |
| Screenshots | `gallery.rs` (MediaStore export), F2 screenshots | — | Gallery page, custom location, recording via Android `MediaProjection`/`MediaRecorder` in the launcher process (engine-side encode only if measurable) |
| Stability | `crash.rs`, `exit_reports.rs`, `diagnostics.rs`, `native_tombstone.rs`, Round 70 input diagnostics | Advanced Crash Auto Fix (Round 39), exit reports | GPU compatibility detection → safe graphics mode, recovery after failed launches, profile backup |

### Feature backlog (added to the roadmap, in the user's priority order)

1. **Touch & mouse compatibility** — Round 70 fixes shipped; device confirmation + diagnostics triage. *(gate for everything below)*
2. **Skin & Cape Management System (high priority)**
   - Skins: PNG import, 64×64 + legacy 64×32, Steve/Alex, 3D 360° preview, multiple slots, switch without reinstall, management inside CS Engine, CS account skin sync.
   - Capes: import, apply, 3D preview, slots, enable/disable, visibility, per-profile cape.
   - Accounts: MSA skin sync (official `MinecraftSkinApi` upload only — never bypass auth/ownership), premium skin fetch, offline/local custom skins, persistence across restarts.
   - Plan: **Sprint 2a** — `cs_launch` gains `skin{path,model}` + `cape{path}` from `LocalSkinStore`; engine `cs_bridge` maps them to the existing offline-skin/cape loaders for offline accounts and to a *local override* for MSA accounts (own player render + cape toggle), keeping server-side identity untouched. **Sprint 2b** — hot reload (`HostEvent::Skin`) and the in-engine Skin & Cape page (CS design) backed by the launcher store via a small JNI bridge. Tests: skin/cape decode + layout, launch JSON round-trip, override precedence.
3. **Advanced Controls** — custom layout editor (existing CS editor in engine host), button position/size/opacity, mouse sensitivity, virtual mouse mode, gyroscope (engine `device_gyro` → host look deltas), gamepad, custom key mapping (`HorizonKeyMap` + `action:*`), quick switching, per-profile configs.
4. **CS Engine Settings** — FPS limiter, VSync, resolution scaling, dynamic resolution, presets, low-end mode, render/simulation distance, graphics quality, particles, shadows, clouds, performance monitor, memory info; per-profile persistence.
5. **Performance Center** — FPS, frame time, RAM, CPU/GPU where available, render distance, chunk loading, temperature (thermal API where accessible), history, automatic device profile recommendation (GPU/RAM/SoC → preset).
6. **Resource Pack & Shader Manager** — in-game manager, ZIP import, enable/disable, preview/info, compatibility detection, shader import/config, priority.
7. **World Management** — list improvements, rename, icon, backup/restore, export/import, datapacks, info, delete confirmation, quick switching.
8. **CS Engine In-Game Drawer** — Skin & Cape, Controls, Mouse, Performance Center, Graphics, Resource Packs, Shader Packs, Screenshot Gallery, World Management, Engine Settings, Report a Problem. Mobile-first, CS Launcher Plus design. (Replaces the Round 65 minimal menu; built in the launcher overlay process so it shares the CS design system and never blocks the render thread.)
9. **Screenshot & Recording** — screenshots, gallery, custom location, recording where supported (MediaProjection), quality settings, FPS-impact indicator.
10. **Additional customisation** — HUD editor, hotbar, crosshair editor, FOV, HUD scaling, FPS counter style, coordinates/direction, custom performance overlay, toggleable CS watermark.
11. **Stability & compatibility (continuous)** — automatic crash collection, better diagnostics, GPU compatibility detection, device-specific config, safe graphics mode, recovery after failed launches, input diagnostics, profile backup/recovery.
12. **Advanced graphics (with 4)** — presets Potato/Low/Balanced/High, dynamic resolution, render-distance presets, entity distance, particles, texture quality, mipmaps, VSync, FPS limiter, GPU profiles, low-end mode.

### Per-feature rules (standing)
Inspect existing architecture → reuse → no duplicate implementations → low memory/CPU overhead → existing profiles and installs untouched → automated tests (host `cargo test` job + CSL2 JUnit) → GitHub Actions build → verified on real Android devices before "done".

### Next sprint
**Sprint 2a — Skin & Cape bridge** (after device confirmation of Round 70): launcher `LocalSkinStore` → `cs_launch` → engine render of own skin/cape for offline and MSA accounts, cape toggle, persistence; tests; CI; device test (Steve/Alex, legacy skin, cape on/off, switch skin between launches).

## Round 72 — Desktop-like virtual mouse, control-editor crash, in-game Skin & Cape (Sprint 2b)

Priority order from the brief: (1) virtual mouse behaves like a desktop cursor, (2) `InflateException`
in the control-button editor, (3) touch across devices (diagnostics from Round 70 stay in place),
(4) offline-account Skin & Cape management in game, (5) CS skin system ↔ engine bridge,
(6) hot switching + persistence, (7) rest of the roadmap.

### 1. Virtual mouse (reuses the CS cursor/touchpad — no second mouse system)
* Root cause: `GameActivity.toggleMouse()` returned silently while the engine had the pointer
  captured (world) and the cursor state was not owned by the engine activity, so the Mouse button
  looked dead and the cursor could vanish after a grab/release cycle.
* Fix: `GameActivity.setMouseToggleHandler(Runnable)` — the Java engine path is untouched;
  `HorizonGameActivity` owns a persistent `mMouseWanted` flag. While wanted the CS cursor view stays
  VISIBLE → `InGUIEventProcessor` runs in touchpad mode (drag moves the cursor with the mouse-speed
  preference, tap = left click at the cursor, scroll / right-click control buttons act at the cursor);
  the cursor is drawn whenever the engine is not capturing the pointer (menus, inventory, chat) and
  hides itself during capture (`GameCursorView.onGrabState`) — it reappears by itself when a menu
  opens, exactly like a desktop. Off = plain touch again (absolute taps). Toast tells the user when
  it was enabled inside the world. Menu item "Mouse: on/off" mirrors the control button;
  `mouse_wanted` added to `cs-input-diagnostics.json`.
* Engine: unchanged (host mode already uses `InputHost::KeyboardMouse`, absolute cursor via
  `nativeHostCursor`, grab over `nativeHostPoll` bit 0).

### 2. `InflateException layout/dialog_control_button_setting` (MaterialSwitch)
* Root cause: `HorizonGameActivity` is a `NativeActivity` with the plain framework fullscreen theme
  (needed by the engine surface); `ControlLayout.editControlButton()` inflates the editor with the
  overlay view's context → Material components require an AppCompat/Material theme → crash.
* Fix: the overlay root, every inflater, dialog and toast of the engine activity use
  `ContextThemeWrapper(this, R.style.AppTheme)` (the launcher's Material 3 theme). The manifest theme
  stays — only the view hierarchy gets the launcher theme.

### 3–6. Skin & Cape for CS Engine (engine fork d31db5cfc + launcher)
* Engine (`skins.rs`): offline skin/cape are now reloadable (`set_offline_skin(path, model)`,
  `set_offline_cape(path)`, `offline_generation()`); the env-var load Horizon's launcher used still
  works. `game/remote.rs` re-uploads the textures into the existing slots when the generation changes
  → **hot switch without rejoin**. New `HostEvent::Skin` + JNI `nativeHostSkin(skin, model, cape)`;
  `cs_launch` JSON gains `skin`, `skin_model`, `cape`. Unit test extended (generation bump, bad file
  keeps the previous skin, remove). CI checks the new JNI symbol.
* Launcher: `HorizonSkins` bridge — per-profile selection keys `csengine:<profile>` stored in the
  existing `SkinLibrary` / `LocalSkinStore` indices (fallback: the account's own selection), so it
  survives restarts and launcher updates. `HorizonLaunch` passes the files at start.
  `HorizonSkinDialog` (menu → "Account · Skin & Cape"): account header (name + Offline/Microsoft),
  3D `PlayerPreviewView`, saved skins with face thumbnails, Steve/Alex model, **Import PNG** (system
  picker; 64×64 / 64×32 validated), capes with front thumbnails + "None", Apply = save + hot switch.
* Account rules: offline/local accounts always render the selection. Microsoft accounts keep their
  official profile skin (the engine prefers the signed profile textures; only a missing cape is
  filled) — authentication and the official upload flow are untouched; the page says so.
* Not done on purpose: account switching inside a running game (session/ownership is decided at
  launch; switching would need a relaunch) — the launcher's account list remains the place for it.

### Device checklist for this round
1. Mouse button in world → toast; open inventory → cursor visible, drag moves, tap clicks, Mouse
   button again → touch. 2. Edit controls → tap a button → editor opens (no crash) and the switches
   work. 3. Menu → Account · Skin & Cape → import a PNG → Apply → press F5 (third person): new skin
   immediately; restart game → still there. 4. Microsoft account: official skin unchanged.

### Round 72b — Virtual mouse root cause: cursor teleported to the finger
* Root cause (launcher): `InGUIEventProcessor` only switches to touchpad semantics when it knows the
  cursor view (`setAbstractTouchpad`), and that was wired in `GameView.start()` — which only the Java
  engine calls (it creates the render surface). CS Engine never called it, so `touchpadDisplayed()`
  was always false and every touch was sent as an absolute cursor position. Fixed in
  `GameView.onFinishInflate()` (both engines, identical behaviour for Java).
* New `input/mouse/VirtualTouchpad` (pure Java, unit-tested): relative deltas only, finger lift keeps
  the cursor, quick still tap = left click at the cursor, quick still two-finger tap = right click,
  two moving fingers = scroll, finger-count changes re-anchor without motion, cancel emits nothing.
  `InGUIEventProcessor` routes all touchpad-mode events through it; the non-touchpad path is unchanged.
* Mouse on keeps the last cursor position (centred only when unset/off-screen).
* CI: `testFullDebugUnitTest --tests input.mouse.*` runs before the APK build (7 tests).
* Also available while the mouse is on: PRI/SEC/scroll control buttons act at the cursor; chat/
  keyboard never toggles mouse mode (keyboard flag only opens the IME).

### Round 72c — Keyboard Enter fix (engine) + mouse Sprint A (launcher)
* **Enter/Backspace from the Android keyboard did nothing in chat/commands.** Root cause (engine):
  host keys arrive as custom-key tokens → `mapped_key`/`key_pressed`, which only reach key mappings;
  the chat/sign/book/search fields read `WindowEvent::KeyboardInput` (not constructible from the host)
  or the on-screen keyboard route. Fix (`android_input.rs`, fork 0d738af61): while a text field has
  focus (`pad_typing()` or a focused title-screen field), host Enter/Backspace take the OSK route
  (`pad_osk_output`), and an IME that commits Enter as `\n` in the text is treated as Enter. All
  other keys unchanged.
* Launcher: auto cursor in menus (engine releases pointer and does not ask for the keyboard → cursor
  shows; world/typing → hides; independent of the manual Mouse button; preference
  `vmouse_auto_menus`), double-tap-and-hold drag, options tap-to-click / two-finger right click / drag,
  in-game "Mouse & touch settings" (cursor speed, size, the toggles — live + persisted). 9 unit tests.

### Rounds 73–76 — Touch/mouse iterations (launcher)
* Touchpad mode, auto-cursor-in-menus and direct-tap-in-menus were tried; the user's verdict in
  Round 79: cursor must behave **exactly like PojavLauncher**. The experimental modes and their
  settings were removed again (Round 79); `InGUIEventProcessor` is back to Pojav semantics.

### Rounds 77–78 — Friends without VPN: e4mc public address, server branding, hotbar tap (engine)
* Open to LAN publishes the world through e4mc (`xxxx.e4mc.link`, QUIC relay, cross-play with Java
  guests); IPv6-only / NAT64 (Jio) fixed (61945408d). `/publish` command and the e4mc section.
* `cs-server.json` (`{"motd": "...", "icon": "cs-server-icon.png"}`) + 64×64 PNG → MOTD and favicon
  in friends' server lists. Hotbar tap selects the slot on touch.

### Round 79 — CS Friends inside the engine (fork a4c90338b)
* Decision: the whole friends system lives **in-game** (CS Engine); the launcher has no Friends UI
  (Round 78's launcher page removed). The launcher only forwards engine notices as system
  notifications (`nativeHostNotice`) and writes the default server icon.
* `horizon-client::cs_friends`: worker-thread client for the Supabase RPCs (`cs_register`,
  `cs_sync` heartbeat+messages every 8 s / 3 s with chat open, `cs_add_friend`, `cs_respond`,
  `cs_remove_friend`, `cs_invite`, `cs_send`). Identity = random token in
  `<launcher home>/cs-friends-identity.json` (hashed server-side); friend code `CS-XXXXXX`.
* Screen (`cs_friends_screen` + `game/cs_friends_ui`): Friends button (pause menu icon, title
  button) → tabs **Friends** (your code + Copy, add by code, requests Accept/Decline, friends with
  status · Join when they host · Invite when you host · Chat · remove with confirm), **Chat**
  (one-to-one, unread counts, Enter sends), **Host** (Publish World / Copy Address / Invite All /
  Stop; MOTD field and icon choice CS Launcher / world icon / custom file — saved to
  `cs-server.json` and applied live via `lan::Shared::set_motd/set_favicon`).
* Invites/requests/messages → chat line `[Friends] …`, a system toast, and a launcher notification.
  Join from the title connects at once; in a world the world closes first, then connects.
* Text input works from the Android keyboard (OSK route), IME commit and hardware keys; Escape/Back
  steps out field → conversation → screen.
* Not done: group chat, voice, block list, friend nicknames; invites expire after 10 min server-side.
