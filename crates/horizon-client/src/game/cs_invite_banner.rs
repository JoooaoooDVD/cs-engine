//! CS Friends invite banner: a friend's invite shows at the bottom of the screen (title,
//! pause menu, chat, in game) like Essential's, with Accept and Later. Accept joins the
//! world straight away (the current one closes first). While the pointer is captured
//! (playing), the banner only tells; the pause menu makes it clickable.
use super::{Game, Screen};
use horizon_client::{
    hud::{HudBatch, rgba},
    text::Text,
    ui,
};
use std::time::Duration;

/// How long an invite stays on screen.
const SHOWN_FOR: Duration = Duration::from_secs(90);
const WIDTH: f32 = 230.0;
const HEIGHT: f32 = 40.0;

struct Banner {
    id: i64,
    from: String,
    world: String,
    address: String,
}

impl Game<'_> {
    fn cs_invite_banner(&self) -> Option<Banner> {
        let client = self.cs_friends.as_ref()?;
        let invite = client
            .state
            .invites
            .iter()
            .filter(|i| i.received.elapsed() < SHOWN_FOR && !i.address.is_empty())
            .max_by_key(|i| i.received)?;
        Some(Banner {
            id: invite.id,
            from: invite.from.clone(),
            world: invite.world.clone(),
            address: invite.address.clone(),
        })
    }
    /// Whether the banner's buttons take clicks: the pointer is free and no screen that
    /// would be confused by a stray click (inventory, chat typing is fine) is up.
    fn cs_invite_banner_interactive(&self) -> bool {
        if self.cs_hud_edit.is_some() || self.link_prompt.is_some() {
            return false;
        }
        self.title.is_some()
            || matches!(
                self.screen,
                Screen::Paused | Screen::Chat | Screen::Friends
            )
    }
    /// The banner's box and its two buttons: (x, y, w, h), accept, later.
    fn cs_invite_banner_layout(size: (f32, f32)) -> ([f32; 4], [f32; 4], [f32; 4]) {
        let (w, h) = size;
        let x = ((w - WIDTH) / 2.0).floor();
        // Above the hotbar and the experience bar in game; the same spot on the title.
        let y = (h - HEIGHT - 48.0).floor();
        let accept = [x + WIDTH - 2.0 - 52.0 - 2.0 - 44.0, y + HEIGHT - 22.0, 52.0, 20.0];
        let later = [x + WIDTH - 2.0 - 44.0, y + HEIGHT - 22.0, 44.0, 20.0];
        ([x, y, WIDTH, HEIGHT], accept, later)
    }
    pub(super) fn draw_cs_invite_banner(&self, batch: &mut HudBatch) {
        let Some(banner) = self.cs_invite_banner() else {
            return;
        };
        let atlas = &self.gui.atlas;
        let (bx, accept, later) = Self::cs_invite_banner_layout((batch.width, batch.height));
        let [x, y, w, h] = bx;
        batch.fill(atlas, x, y, w, h, rgba(16, 16, 20, 230));
        batch.fill(atlas, x, y, w, 1.0, rgba(85, 255, 255, 255));
        let title = format!("{} invited you", banner.from);
        batch.text(atlas, &title, x + 6.0, y + 5.0, rgba(85, 255, 255, 255), true);
        let world = if banner.world.is_empty() {
            "Join their world".to_owned()
        } else {
            let mut name = banner.world.clone();
            if name.chars().count() > 22 {
                name = name.chars().take(21).collect::<String>() + "…";
            }
            format!("World: {name}")
        };
        batch.text(atlas, &world, x + 6.0, y + 16.0, rgba(200, 200, 200, 255), true);
        if self.cs_invite_banner_interactive() {
            let hover = |r: [f32; 4]| ui::hovered(self.cursor, r[0], r[1], r[2], r[3]);
            ui::button(
                batch,
                atlas,
                accept[0],
                accept[1],
                accept[2],
                accept[3],
                &Text::literal("Accept"),
                true,
                hover(accept),
            );
            ui::button(
                batch,
                atlas,
                later[0],
                later[1],
                later[2],
                later[3],
                &Text::literal("Later"),
                true,
                hover(later),
            );
        } else {
            let hint = "Pause to accept";
            let tw = HudBatch::text_width(atlas, hint);
            batch.text(atlas, hint, x + w - 6.0 - tw, y + h - 14.0, rgba(140, 140, 140, 255), true);
        }
    }
    /// Whether a press now would land on the banner's buttons (the event match's guard).
    pub(super) fn cs_invite_banner_hovered(&self) -> bool {
        if !self.cs_invite_banner_interactive() || self.cs_invite_banner().is_none() {
            return false;
        }
        let (_, accept, later) = Self::cs_invite_banner_layout(self.gui_size());
        let hit = |r: [f32; 4]| ui::hovered(self.cursor, r[0], r[1], r[2], r[3]);
        hit(accept) || hit(later)
    }
    /// A left press over the banner's buttons; `true` when the press was taken.
    pub(super) fn cs_invite_banner_click(&mut self) -> bool {
        if !self.cs_invite_banner_interactive() {
            return false;
        }
        let Some(banner) = self.cs_invite_banner() else {
            return false;
        };
        let (_, accept, later) = Self::cs_invite_banner_layout(self.gui_size());
        let hit = |r: [f32; 4]| ui::hovered(self.cursor, r[0], r[1], r[2], r[3]);
        if hit(accept) {
            self.audio.play_button_click();
            if let Some(client) = self.cs_friends.as_mut() {
                client.dismiss_invite(banner.id);
            }
            self.cs_join_from_banner(&banner.address);
            true
        } else if hit(later) {
            self.audio.play_button_click();
            if let Some(client) = self.cs_friends.as_mut() {
                client.dismiss_invite(banner.id);
            }
            true
        } else {
            false
        }
    }
}
