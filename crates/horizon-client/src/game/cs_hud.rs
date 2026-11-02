//! CS HUD modules (CS Engine, Powered by Craft Studio): the FPS counter, the
//! keystrokes and the coordinates panel, built into the game and switched on
//! the Extras page ("CS HUD modules"). Each is off by default and draws in the
//! corner the player chose, stacked with the minimap and mods' panels through
//! [`Corners`]. The FPS and click counters are the ones `game/extras.rs` keeps
//! for every frame; the keys are the movement keys the game reads, so touch
//! controls and a keyboard both show.
use super::{Game, Screen};
use horizon_client::{
    extras::Corner,
    extras_hud::{self, Corners},
    hud::{HudBatch, rgba},
};
use winit::keyboard::KeyCode;

/// A key box's side, the gap between boxes and the panel's padding, in GUI pixels.
const KEY: f32 = 18.0;
const GAP: f32 = 2.0;
const BACKGROUND: u32 = 0x7800_0000;
const PRESSED: u32 = 0xC8E6_E6E6;
const RELEASED: u32 = 0x7800_0000;
const TEXT: u32 = 0xFFFF_FFFF;
const TEXT_PRESSED: u32 = 0xFF1A_1A1A;

impl Game<'_> {
    /// The CS HUD modules, after the minimap and before mods' panels. `visible`
    /// says whether a corner may be drawn (the debug screen owns the top left).
    pub(super) fn draw_cs_hud(&mut self, batch: &mut HudBatch, corners: &mut Corners) {
        let e = &self.settings.extras;
        if !(e.fps || e.keystrokes || e.coords) {
            return;
        }
        let (fps, fps_corner, fps_lows) = (e.fps, e.fps_corner, e.fps_lows);
        let (keystrokes, keystrokes_corner, cps_shown) =
            (e.keystrokes, e.keystrokes_corner, e.keystrokes_cps);
        let (coords, coords_corner) = (e.coords, e.coords_corner);
        let debug = self.debug;
        let visible = |corner: Corner| !(debug && corner == Corner::TopLeft);
        if fps && visible(fps_corner) {
            let (rate, min, low) = self.frame_reading();
            let color: u32 = match rate {
                0..=24 => 0xFF5555,
                25..=49 => 0xFFFF55,
                _ => 0x55FF55,
            };
            let line = format!("FPS: {rate}");
            let lows = format!("Min {min} · 1% low {low}");
            let atlas = &self.gui.atlas;
            if fps_lows {
                extras_hud::draw_lines(
                    batch,
                    atlas,
                    corners,
                    fps_corner,
                    &[(line.as_str(), color), (lows.as_str(), 0xAAAAAA)],
                );
            } else {
                extras_hud::draw_lines(batch, atlas, corners, fps_corner, &[(line.as_str(), color)]);
            }
        }
        if coords && visible(coords_corner) {
            if let Some(p) = self.player.as_ref().map(|p| p.position) {
                let xyz = format!(
                    "XYZ: {} {} {}",
                    p.x.floor() as i64,
                    p.y.floor() as i64,
                    p.z.floor() as i64
                );
                let facing = format!("Facing: {}", facing(self.yaw));
                let atlas = &self.gui.atlas;
                extras_hud::draw_lines(
                    batch,
                    atlas,
                    corners,
                    coords_corner,
                    &[(xyz.as_str(), 0xFFFFFF), (facing.as_str(), 0xAAAAAA)],
                );
            }
        }
        if keystrokes && visible(keystrokes_corner) {
            self.draw_keystrokes(batch, corners, keystrokes_corner, cps_shown);
        }
    }

    /// The classic keystrokes layout: W over A S D, the mouse buttons (with
    /// their clicks per second) and the space bar under them.
    fn draw_keystrokes(
        &mut self,
        batch: &mut HudBatch,
        corners: &mut Corners,
        corner: Corner,
        cps_shown: bool,
    ) {
        let playing = self.screen == Screen::Playing && self.title.is_none();
        let stick = self.pad_move().filter(|_| playing);
        let held = |k: KeyCode| playing && self.keys.contains(&k);
        // The stick (touch joystick or controller) counts as the key it moves toward.
        let axis = |i: usize, sign: f32| stick.is_some_and(|s| s[i] * sign > 0.5);
        let w = held(KeyCode::KeyW) || held(KeyCode::ArrowUp) || axis(1, 1.0);
        let s = held(KeyCode::KeyS) || held(KeyCode::ArrowDown) || axis(1, -1.0);
        let a = held(KeyCode::KeyA) || held(KeyCode::ArrowLeft) || axis(0, -1.0);
        let d = held(KeyCode::KeyD) || held(KeyCode::ArrowRight) || axis(0, 1.0);
        let space = held(KeyCode::Space);
        let ((lcps, rcps), [lmb, rmb]) = self.mouse_reading();
        let (lmb, rmb) = (lmb && playing, rmb && playing);
        let atlas = &self.gui.atlas;

        let width = KEY * 3.0 + GAP * 2.0;
        let mouse_h = KEY;
        let space_h = 10.0;
        let height = KEY * 2.0 + mouse_h + space_h + GAP * 3.0;
        let (x0, y0) = corners.place(corner, width + GAP * 2.0, height + GAP * 2.0);
        batch.fill(atlas, x0, y0, width + GAP * 2.0, height + GAP * 2.0, BACKGROUND);
        let (x0, y0) = (x0 + GAP, y0 + GAP);

        let key = |batch: &mut HudBatch, x: f32, y: f32, w: f32, h: f32, label: &str, down: bool| {
            batch.fill(atlas, x, y, w, h, if down { PRESSED } else { RELEASED });
            let tw = HudBatch::text_width(atlas, label);
            let tx = (x + (w - tw) / 2.0).round();
            let ty = (y + (h - 8.0) / 2.0).round();
            let [_, r, g, b] = if down { TEXT_PRESSED } else { TEXT }.to_be_bytes();
            batch.text(atlas, label, tx, ty, rgba(r, g, b, 255), !down);
        };
        // Row 1: W
        key(batch, x0 + KEY + GAP, y0, KEY, KEY, "W", w);
        // Row 2: A S D
        let y1 = y0 + KEY + GAP;
        key(batch, x0, y1, KEY, KEY, "A", a);
        key(batch, x0 + KEY + GAP, y1, KEY, KEY, "S", s);
        key(batch, x0 + (KEY + GAP) * 2.0, y1, KEY, KEY, "D", d);
        // Row 3: the mouse buttons
        let y2 = y1 + KEY + GAP;
        let half = (width - GAP) / 2.0;
        let (ll, rl) = if cps_shown {
            (format!("LMB {lcps}"), format!("RMB {rcps}"))
        } else {
            ("LMB".to_owned(), "RMB".to_owned())
        };
        key(batch, x0, y2, half, mouse_h, &ll, lmb);
        key(batch, x0 + half + GAP, y2, half, mouse_h, &rl, rmb);
        // Row 4: the space bar
        let y3 = y2 + mouse_h + GAP;
        key(batch, x0, y3, width, space_h, "Space", space);
    }
}

/// `Direction.fromYRot` as the debug screen names it, with the axis it moves along.
fn facing(yaw: f32) -> &'static str {
    let yaw = yaw.rem_euclid(360.0);
    match ((yaw + 45.0) / 90.0).floor() as i32 % 4 {
        0 => "south (+Z)",
        1 => "west (-X)",
        2 => "north (-Z)",
        _ => "east (+X)",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn facing_matches_the_debug_screen() {
        assert_eq!(super::facing(0.0), "south (+Z)");
        assert_eq!(super::facing(90.0), "west (-X)");
        assert_eq!(super::facing(180.0), "north (-Z)");
        assert_eq!(super::facing(-90.0), "east (+X)");
        assert_eq!(super::facing(359.0), "south (+Z)");
    }
}
