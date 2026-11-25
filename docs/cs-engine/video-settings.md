# CS Engine Android video settings

The Android video screen uses a dark performance-oriented layout. The first page
contains common controls; Advanced groups the complete controls into Render,
Quality, Display and Performance tabs. Desktop retains the upstream style.
Settings are opt-in; existing saved settings are not replaced on upgrade.

## Profiles

The preset button cycles through High FPS (8 chunks, 80% world scale), Low-end
(4 chunks, 70%), Balanced (12 chunks, 90%), Quality (16 chunks, 100%), Ultra
(24 chunks, 100%) and Custom. These values are starting points, not automatic
hardware detection or performance guarantees. Profiles also adjust simulation
distance, clouds, particles and other quality settings. Changing a profile
controlled option makes the selection Custom. Battery mode, FPS cap and overlay
are left to the player.

World resolution scale multiplies the separate CS Launcher render scale: for
example, a launcher scale of 75% and world scale of 80% render the world at
roughly 60% of display width and height. Lower scales can improve GPU load but
reduce image sharpness. The HUD uses the launcher's framebuffer resolution.

## Chunk loading and performance overlay

Zero (Auto) on the Android chunk-update and upload-budget sliders leaves the
existing adaptive mobile scheduler in control; it still applies time and byte
limits. Lower explicit values may smooth uploads but can delay terrain loading.
Battery mode also limits FPS and reduces worker threads. Overlay modes show FPS,
frame/CPU/GPU times, and a 1% low calculated from up to the 160 latest frames;
the graph mode additionally draws recent frame times. GPU timing may be missing
on some devices. Short sample windows are not a substitute for a benchmark.

## Testing on a phone

Use the same device, world/route, launcher scale, render distance and FPS cap
when comparing builds. Record average FPS, 1% low, frame-time spikes, load time,
battery and temperature after both a cold start and sustained play. Check touch,
landscape/portrait, settings persistence, Android lifecycle, multiplayer and
resource packs. Performance, heat and visual quality vary across devices.
