//! Android renderer and platform policy.
#[path = "graphics_startup.rs"]
pub mod graphics_startup;
use crate::android_gamepad::{Devices, InputMode};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(target_os = "android")]
static CACHE_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
#[cfg(target_os = "android")]
pub fn configure_cache_dir(path: std::path::PathBuf) {
    let _ = CACHE_DIR.set(path);
}
#[cfg(target_os = "android")]
pub(crate) fn cache_dir() -> Option<&'static std::path::Path> {
    CACHE_DIR.get().map(std::path::PathBuf::as_path)
}
/// CS Engine: where resource-pack and content-mod overlay roots are built. The game's
/// working directory (the profile) may live on external/FUSE storage, where `symlink()`
/// fails with EPERM and every pack reload failed; the launcher points this at internal
/// storage instead (`cs_bridge`). Unset: beside options.txt, as upstream.
static OVERLAY_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
pub fn configure_overlay_dir(path: std::path::PathBuf) {
    let _ = OVERLAY_DIR.set(path);
}
pub fn overlay_dir() -> Option<&'static std::path::Path> {
    OVERLAY_DIR.get().map(std::path::PathBuf::as_path)
}
static INPUT_MODE: std::sync::Mutex<(InputMode, InputMode)> =
    std::sync::Mutex::new((InputMode::Auto, InputMode::Touch));
static GAMEPADS: std::sync::LazyLock<std::sync::Mutex<Devices>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Devices::default()));
pub fn input_mode() -> InputMode {
    INPUT_MODE.lock().unwrap().0
}
pub fn active_input() -> InputMode {
    INPUT_MODE.lock().unwrap().1
}
pub fn configure_input(mode: InputMode) {
    let mut input = INPUT_MODE.lock().unwrap();
    if input.0 != mode {
        *input = (
            mode,
            if mode == InputMode::Auto {
                InputMode::Touch
            } else {
                mode
            },
        );
        clear_gamepads();
        clear_keypad();
    }
}
pub fn note_input(kind: InputMode) -> bool {
    let mut input = INPUT_MODE.lock().unwrap();
    if input.0 != InputMode::Auto && input.0 != kind {
        return false;
    }
    if input.1 == InputMode::Controller && kind != InputMode::Controller {
        clear_gamepads();
    }
    input.1 = kind;
    true
}
pub fn gamepad_event(id: i32, event: &winit::event::DeviceEvent) -> bool {
    use winit::event::{DeviceEvent, ElementState};
    let activity = {
        let mut devices = GAMEPADS.lock().unwrap();
        match *event {
            DeviceEvent::Motion { axis, value } => devices.axis(id, axis, value),
            DeviceEvent::Button { button, state } => {
                devices.key(id, button, state == ElementState::Pressed)
            }
            DeviceEvent::Removed => {
                devices.clear();
                false
            }
            _ => return false,
        }
    };
    if activity {
        note_input(InputMode::Controller);
    }
    true
}
pub fn retain_gamepads(ids: &[i32]) {
    GAMEPADS.lock().unwrap().retain(ids);
}
pub fn clear_gamepads() {
    GAMEPADS.lock().unwrap().clear();
}
pub fn active_gamepad_id() -> Option<i32> {
    (active_input() == InputMode::Controller)
        .then(|| GAMEPADS.lock().unwrap().active_id())
        .flatten()
}
pub fn gamepad_state() -> crate::gamepad::PadState {
    if active_input() == InputMode::Controller {
        GAMEPADS.lock().unwrap().poll()
    } else {
        crate::gamepad::PadState::default()
    }
}
static ATLAS_LIMIT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(16384);
/// The launcher opens the adapter before the resource worker builds the game atlas.
pub fn configure_atlas_limit(limit: u32) {
    ATLAS_LIMIT.store(limit, Ordering::Relaxed);
}
pub fn atlas_limit() -> u32 {
    ATLAS_LIMIT.load(Ordering::Relaxed)
}
static GPU_BATCHING: AtomicBool = AtomicBool::new(false);
static KEYPAD_ENABLED: AtomicBool = AtomicBool::new(false);
static KEYPAD: std::sync::LazyLock<std::sync::Mutex<crate::keypad::Keypad>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(crate::keypad::Keypad::default()));
pub fn configure_keypad_settings(settings: &crate::keypad::Settings) {
    KEYPAD.lock().unwrap().configure(settings);
    KEYPAD_ENABLED.store(settings.enabled, Ordering::Relaxed);
}
pub fn configure_keypad(enabled: bool) {
    KEYPAD_ENABLED.store(enabled, Ordering::Relaxed);
    clear_keypad();
}
pub fn keypad_enabled() -> bool {
    KEYPAD_ENABLED.load(Ordering::Relaxed)
}
pub fn keypad_key(key: winit::keyboard::KeyCode, down: bool) -> bool {
    keypad_enabled() && KEYPAD.lock().unwrap().key(key, down)
}
pub fn clear_keypad() {
    KEYPAD.lock().unwrap().clear();
}
pub fn keypad_state() -> crate::gamepad::PadState {
    KEYPAD.lock().unwrap().poll()
}
static OPEN_GL: AtomicBool = AtomicBool::new(cfg!(target_pointer_width = "32"));
pub fn configure_graphics(open_gl: bool) {
    OPEN_GL.store(open_gl, Ordering::Relaxed);
}
pub fn graphics_api() -> crate::graphics::GraphicsApi {
    if OPEN_GL.load(Ordering::Relaxed) {
        crate::graphics::GraphicsApi::OpenGl
    } else {
        crate::graphics::GraphicsApi::Vulkan
    }
}
pub fn configure_gpu(batching: bool) {
    GPU_BATCHING.store(batching, Ordering::Relaxed);
}
pub fn compatibility() -> bool {
    cfg!(target_os = "android") && !GPU_BATCHING.load(Ordering::Relaxed)
}

pub use crate::clipboard::{ClipboardHooks, clipboard_get, clipboard_set, install_clipboard};

static RETURN_TO_LAUNCHER: AtomicBool = AtomicBool::new(false);
pub fn return_to_launcher() {
    RETURN_TO_LAUNCHER.store(true, Ordering::Release);
}
pub fn take_launcher_return() -> bool {
    RETURN_TO_LAUNCHER.swap(false, Ordering::AcqRel)
}

#[cfg(test)]
mod input_tests {
    use super::*;
    #[test]
    fn explicit_modes_reject_other_devices_and_auto_follows_activity() {
        configure_input(InputMode::Touch);
        assert!(!note_input(InputMode::Controller));
        assert_eq!(active_input(), InputMode::Touch);
        gamepad_event(
            2,
            &winit::event::DeviceEvent::Button {
                button: 96,
                state: winit::event::ElementState::Pressed,
            },
        );
        assert_eq!(gamepad_state(), crate::gamepad::PadState::default());
        configure_input(InputMode::Controller);
        assert!(!note_input(InputMode::Touch));
        configure_input(InputMode::Auto);
        assert!(note_input(InputMode::KeyboardMouse));
        assert_eq!(active_input(), InputMode::KeyboardMouse);
        gamepad_event(
            2,
            &winit::event::DeviceEvent::Motion {
                axis: 0,
                value: 0.05,
            },
        );
        assert_eq!(active_input(), InputMode::KeyboardMouse);
        gamepad_event(
            2,
            &winit::event::DeviceEvent::Motion {
                axis: 0,
                value: 1.0,
            },
        );
        assert_eq!(active_input(), InputMode::Controller);
        assert!(note_input(InputMode::Touch));
        assert_eq!(gamepad_state(), crate::gamepad::PadState::default());
        clear_gamepads();
        configure_input(InputMode::Auto);
    }
}

// ---------------------------------------------------------------------------
// Host input: a launcher that owns the touch overlay (CS Launcher Plus) feeds
// the game mouse + keyboard events through this queue instead of the window.
// The game drains it once per frame (`Game::android_host_input`).
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum HostEvent {
    /// A touch began on the hotbar (window pixels): select that slot, as the engine's own
    /// touch controls do (`touch::hotbar_slot`).
    HotbarTap { x: f64, y: f64 },
    /// A key by its `key.keyboard.*` name (or `action:<keybind>`), pressed or released.
    Key { token: String, down: bool },
    /// Relative mouse motion in window pixels (camera when the pointer is captured).
    Look { dx: f64, dy: f64 },
    /// Absolute pointer position in window pixels (menus, inventory).
    Cursor { x: f64, y: f64 },
    /// Mouse button: 0 left, 1 right, 2 middle, 3 back, 4 forward.
    Button { button: u8, down: bool },
    /// Wheel lines: positive `y` scrolls up.
    Scroll { x: f32, y: f32 },
    /// Typed text (from the host's keyboard) for chat, signs, fields.
    Text(String),
    /// Release every key and button the host pressed (focus loss).
    ReleaseAll,
    /// The local player's own skin / cape changed in the host's Skin & Cape page: PNG paths
    /// (empty = none) and `slim`/`classic`. Applied only where no account/server texture wins.
    Skin { skin: String, model: String, cape: String },
}
static HOST_MODE: AtomicBool = AtomicBool::new(false);
static HOST_GRAB: AtomicBool = AtomicBool::new(false);
static HOST_KEYBOARD: AtomicBool = AtomicBool::new(false);
static HOST_EVENTS: std::sync::Mutex<Vec<(std::time::Instant, HostEvent)>> =
    std::sync::Mutex::new(Vec::new());
/// A click older than this when the game gets to it was meant for a screen that is gone
/// (a stall, the loading screen): it is dropped rather than replayed.
const HOST_CLICK_STALE: std::time::Duration = std::time::Duration::from_millis(400);
/// The launcher drives input; the engine draws no touch controls and ignores window touches.
pub fn configure_host(enabled: bool) {
    HOST_MODE.store(enabled, Ordering::Relaxed);
}
pub fn host_mode() -> bool {
    HOST_MODE.load(Ordering::Relaxed)
}
/// Input diagnostics (CS Engine): events the host handed over, events the game consumed, and the
/// pointer mapping in use. Read back into `cs-exit.json` so a device where taps vanish shows where.
static HOST_PUSHED: AtomicU64 = AtomicU64::new(0);
static HOST_PROCESSED: AtomicU64 = AtomicU64::new(0);
static HOST_GEOMETRY: std::sync::Mutex<Option<HostGeometry>> = std::sync::Mutex::new(None);
/// How host pointer coordinates (overlay window pixels) map onto the render target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostGeometry {
    /// Native window (surface) size in pixels.
    pub window: [u32; 2],
    /// Render target size (the window at the render scale).
    pub render: [u32; 2],
    /// Content rectangle origin inside the window (safe insets / aspect letterbox).
    pub origin: [u32; 2],
    /// Pixel scale from window content to render target.
    pub scale: [f64; 2],
}
pub fn host_set_geometry(geometry: HostGeometry) -> bool {
    let mut current = HOST_GEOMETRY.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if *current == Some(geometry) {
        return false;
    }
    *current = Some(geometry);
    true
}
pub fn host_geometry() -> Option<HostGeometry> {
    *HOST_GEOMETRY.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
pub fn host_note_processed(count: usize) {
    HOST_PROCESSED.fetch_add(count as u64, Ordering::Relaxed);
}
/// `(pushed by the host, processed by the game)`.
pub fn host_stats() -> (u64, u64) {
    (HOST_PUSHED.load(Ordering::Relaxed), HOST_PROCESSED.load(Ordering::Relaxed))
}
pub fn host_push(event: HostEvent) {
    HOST_PUSHED.fetch_add(1, Ordering::Relaxed);
    let mut events = HOST_EVENTS.lock().unwrap();
    let now = std::time::Instant::now();
    // Coalesce motion so a slow frame does not replay a long tail of deltas late.
    match (&event, events.last_mut()) {
        (HostEvent::Look { dx, dy }, Some((_, HostEvent::Look { dx: px, dy: py }))) => {
            *px += dx;
            *py += dy;
        }
        (HostEvent::Cursor { .. }, Some((at, last @ HostEvent::Cursor { .. }))) => {
            *last = event;
            *at = now;
        }
        _ => {
            if events.len() < 4096 {
                events.push((now, event));
            }
        }
    }
}
/// The queued host events. Clicks and hotbar taps that waited longer than
/// `HOST_CLICK_STALE` (the game was loading or stalled) are dropped: replaying
/// them would press whatever button now sits under that position.
pub fn host_take() -> Vec<HostEvent> {
    let now = std::time::Instant::now();
    std::mem::take(&mut *HOST_EVENTS.lock().unwrap())
        .into_iter()
        .filter(|(at, event)| {
            !(matches!(event, HostEvent::Button { down: true, .. } | HostEvent::HotbarTap { .. })
                && now.duration_since(*at) > HOST_CLICK_STALE)
        })
        .map(|(_, event)| event)
        .collect()
}
/// While the loading screen is up the game does not read input: everything but
/// the player's skin (sent once at launch) is thrown away so that it does not
/// land on the title screen's buttons when the game appears.
pub fn host_discard_input() {
    HOST_EVENTS
        .lock()
        .unwrap()
        .retain(|(_, event)| matches!(event, HostEvent::Skin { .. }));
}
/// Game wants the pointer captured (world view) rather than a visible cursor (menus).
pub fn host_set_grab(grabbed: bool) {
    HOST_GRAB.store(grabbed, Ordering::Relaxed);
}
pub fn host_grab() -> bool {
    HOST_GRAB.load(Ordering::Relaxed)
}
/// Where the hotbar was drawn last frame, in overlay-window pixels `[x, y, w, h]`
/// (None when no hotbar is on screen): the launcher's touch layer swallows touches
/// there and hands them over as `HostEvent::HotbarTap`.
static HOST_HOTBAR: std::sync::Mutex<Option<[f64; 4]>> = std::sync::Mutex::new(None);
pub fn host_set_hotbar(rect: Option<[f64; 4]>) {
    *HOST_HOTBAR.lock().unwrap_or_else(|e| e.into_inner()) = rect;
}
pub fn host_hotbar() -> Option<[f64; 4]> {
    *HOST_HOTBAR.lock().unwrap_or_else(|e| e.into_inner())
}
/// The world's public address (e4mc) while it is published, for the launcher's invites.
static HOST_PUBLISHED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
pub fn host_note_published(address: Option<&str>) {
    *HOST_PUBLISHED.lock().unwrap_or_else(|e| e.into_inner()) = address.map(str::to_owned);
}
pub fn host_published() -> Option<String> {
    HOST_PUBLISHED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}
/// CS Friends notices for the launcher's system notifications (`title\ntext`),
/// oldest first; the launcher polls them one at a time.
static HOST_NOTICES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
pub fn host_push_notice(title: &str, text: &str) {
    let mut notices = HOST_NOTICES.lock().unwrap_or_else(|e| e.into_inner());
    if notices.len() < 32 {
        notices.push(format!("{title}\n{text}"));
    }
}
pub fn host_take_notice() -> Option<String> {
    let mut notices = HOST_NOTICES.lock().unwrap_or_else(|e| e.into_inner());
    if notices.is_empty() {
        None
    } else {
        Some(notices.remove(0))
    }
}
/// The title's Customize button (CS Engine skin panel): the host opens its Skin &
/// Cape dialog once; `host_take_skin_request` hands the request over.
static HOST_SKIN_REQUEST: AtomicBool = AtomicBool::new(false);
pub fn host_request_skin() {
    HOST_SKIN_REQUEST.store(true, Ordering::Relaxed);
}
pub fn host_take_skin_request() -> bool {
    HOST_SKIN_REQUEST.swap(false, Ordering::Relaxed)
}
/// Game asked for a text editor (chat, sign, field): the host shows its keyboard.
pub fn host_set_keyboard(wanted: bool) {
    HOST_KEYBOARD.store(wanted, Ordering::Relaxed);
}
pub fn host_keyboard() -> bool {
    HOST_KEYBOARD.load(Ordering::Relaxed)
}
