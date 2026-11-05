//! CS HUD modules (CS Engine, Powered by Craft Studio): the FPS panel, the
//! keystrokes and the coordinates panel, built into the game and switched on
//! the Extras page ("CS HUD modules"). Each is off by default and draws in the
//! corner the player chose, stacked with the minimap and mods' panels through
//! [`Corners`]. The FPS and click counters are the ones `game/extras.rs` keeps
//! for every frame; the keys are the movement keys the game reads, so touch
//! controls and a keyboard both show.
use super::cs_hud_edit::{Module, Placement};
use super::{Game, Screen};
use horizon_client::{
    extras::Corner,
    extras_hud::{self, Corners},
    hud::{HudAtlas, HudBatch, rgba},
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
        self.cs_hud_rects.clear();
        // The HUD draws inside the safe margins (`inset_overlay`): the editor's hit boxes
        // and saved fractions are relative to that inner area.
        let (full_w, full_h) = self.gui_size();
        let margins = self.hud_safe_margins();
        let pad = |full: f32, margin: f32| {
            let margin = if margin.is_finite() { margin.clamp(0.0, 0.12) } else { 0.0 };
            (full * batch.scale * margin).round() / batch.scale
        };
        self.cs_hud_inset = (batch.width, batch.height, pad(full_w, margins[0]), pad(full_h, margins[1]));
        let e = &self.settings.extras;
        if !(e.fps || e.keystrokes || e.coords) {
            return;
        }
        let (fps, fps_corner, fps_lows) = (e.fps, e.fps_corner, e.fps_lows);
        let (keystrokes, keystrokes_corner, cps_shown) =
            (e.keystrokes, e.keystrokes_corner, e.keystrokes_cps);
        let (coords, coords_corner) = (e.coords, e.coords_corner);
        let debug = self.debug;
        let placed = |game: &mut Self, m: Module| -> Option<Placement> {
            game.cs_hud_layout().placed.get(&m).copied()
        };
        // A placed module draws into a scratch corner, then moves to its place.
        let visible = |corner: Corner, placement: Option<Placement>| {
            placement.is_some() || !(debug && corner == Corner::TopLeft)
        };
        let fps_place = placed(self, Module::Fps);
        if fps && visible(fps_corner, fps_place) {
            let (rate, min, low) = self.frame_reading();
            let start = batch.vertices.len();
            let mut scratch = scratch_corners();
            let (target, corner) = match fps_place {
                Some(_) => (&mut scratch, Corner::TopLeft),
                None => (&mut *corners, fps_corner),
            };
            draw_fps_panel(batch, &self.gui.atlas, target, corner, rate, min, low, fps_lows);
            self.cs_hud_place(batch, start, Module::Fps, fps_place);
        }
        let coords_place = placed(self, Module::Coords);
        if coords && visible(coords_corner, coords_place) {
            if let Some(p) = self.player.as_ref().map(|p| p.position) {
                let xyz = format!(
                    "XYZ: {} {} {}",
                    p.x.floor() as i64,
                    p.y.floor() as i64,
                    p.z.floor() as i64
                );
                let facing = format!("Facing: {}", facing(self.yaw));
                let atlas = &self.gui.atlas;
                let start = batch.vertices.len();
                let mut scratch = scratch_corners();
                let (target, corner) = match coords_place {
                    Some(_) => (&mut scratch, Corner::TopLeft),
                    None => (&mut *corners, coords_corner),
                };
                extras_hud::draw_lines(
                    batch,
                    atlas,
                    target,
                    corner,
                    &[(xyz.as_str(), 0xFFFFFF), (facing.as_str(), 0xAAAAAA)],
                );
                self.cs_hud_place(batch, start, Module::Coords, coords_place);
            }
        }
        let keys_place = placed(self, Module::Keystrokes);
        if keystrokes && visible(keystrokes_corner, keys_place) {
            let start = batch.vertices.len();
            let mut scratch = scratch_corners();
            match keys_place {
                Some(_) => self.draw_keystrokes(batch, &mut scratch, Corner::TopLeft, cps_shown),
                None => self.draw_keystrokes(batch, corners, keystrokes_corner, cps_shown),
            }
            self.cs_hud_place(batch, start, Module::Keystrokes, keys_place);
        }
    }

    /// Moves what a module drew since `start` to its saved place and scale (a corner module
    /// stays), and records its box for the HUD Editor (in the full HUD's coordinates).
    fn cs_hud_place(&mut self, batch: &mut HudBatch, start: usize, m: Module, placement: Option<Placement>) {
        let Some(bounds) = bounds(batch, start) else {
            return;
        };
        let (iw, ih, px, py) = self.cs_hud_inset;
        let mut rect = bounds;
        if let Some(p) = placement {
            let (tx, ty) = ((p.x * iw).round(), (p.y * ih).round());
            let (dx, dy) = (tx - bounds[0], ty - bounds[1]);
            for v in &mut batch.vertices[start..] {
                v.pos[0] += dx;
                v.pos[1] += dy;
            }
            batch.scale_since(start, tx, ty, p.scale, p.scale);
            // Kept inside the HUD after scaling (a bigger module near an edge moves in).
            let (sw, sh) = (bounds[2] * p.scale, bounds[3] * p.scale);
            let fix_x = (iw - (tx + sw)).min(0.0);
            let fix_y = (ih - (ty + sh)).min(0.0);
            if fix_x != 0.0 || fix_y != 0.0 {
                for v in &mut batch.vertices[start..] {
                    v.pos[0] += fix_x;
                    v.pos[1] += fix_y;
                }
            }
            rect = [tx + fix_x, ty + fix_y, sw, sh];
        }
        self.cs_hud_rects.push((m, [rect[0] + px, rect[1] + py, rect[2], rect[3]]));
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

/// A corner set a placed module draws into first (top-left, at the margin), before it moves.
/// The FPS panel — the one FPS read-out of CS Engine, the same in the HUD,
/// the HUD Editor and every corner: the rate large, "FPS" beside it, a
/// band along the top in the rate's colour (red under 25, amber under 50,
/// green from 50), and the slowest frame and 1 % low under it if asked.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_fps_panel(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    corners: &mut Corners,
    corner: Corner,
    rate: u32,
    min: u32,
    low: u32,
    lows: bool,
) {
    let color: u32 = match rate {
        0..=24 => 0xFF5555,
        25..=49 => 0xFFC84A,
        _ => 0x5EE08C,
    };
    let [_, r, g, b] = color.to_be_bytes();
    let band = rgba(r, g, b, 255);
    let ink = rgba(r, g, b, 255);
    let number = rate.to_string();
    let number_w = HudBatch::text_width(atlas, &number) * 2.0;
    let label_w = HudBatch::text_width(atlas, "FPS");
    let detail = format!("min {min} · 1% low {low}");
    let detail_w = if lows { HudBatch::text_width(atlas, &detail) } else { 0.0 };
    let w = (number_w + 3.0 + label_w).max(detail_w) + 10.0;
    let h = if lows { 32.0 } else { 23.0 };
    let (x, y) = corners.place(corner, w, h);
    batch.fill(atlas, x, y, w, h, BACKGROUND);
    batch.fill(atlas, x, y, w, 1.0, band);
    batch.fill(atlas, x, y + 1.0, w, 1.0, rgba(r, g, b, 90));
    // The number at twice the size: drawn, then scaled about its top left.
    let start = batch.vertices.len();
    batch.text(atlas, &number, x + 5.0, y + 5.0, ink, true);
    batch.scale_since(start, x + 5.0, y + 5.0, 2.0, 2.0);
    batch.text(atlas, "FPS", x + 5.0 + number_w + 3.0, y + 12.0, rgba(200, 200, 208, 255), true);
    if lows {
        batch.text(atlas, &detail, x + 5.0, y + 22.0, rgba(170, 170, 178, 255), true);
    }
}

fn scratch_corners() -> Corners {
    Corners::new(4096.0, 4096.0, 0.0)
}
/// The box `[x, y, w, h]` of the quads pushed since `start`, if any.
fn bounds(batch: &HudBatch, start: usize) -> Option<[f32; 4]> {
    let vertices = &batch.vertices[start..];
    if vertices.is_empty() {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for v in vertices {
        x0 = x0.min(v.pos[0]);
        y0 = y0.min(v.pos[1]);
        x1 = x1.max(v.pos[0]);
        y1 = y1.max(v.pos[1]);
    }
    Some([x0, y0, x1 - x0, y1 - y0])
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
