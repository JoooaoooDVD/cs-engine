//! CS Engine built-in voice chat (proximity): the game half.
//!
//! Voice rides the game connection itself, as mod messages on the channel
//! `cs:voice` (`horizon_protocol::mod_net`), so it works wherever the world
//! does — the integrated server, Open to LAN, a published (e4mc) world, a CS
//! Engine dedicated server — with no voice server, account or port of its own.
//!
//! - A client (the window, host or guest alike) captures the microphone
//!   (`horizon_client::cs_voice::Mic`), encodes 20 ms frames and sends them
//!   while push-to-talk is held (or the voice is loud enough, in open mode).
//! - The server (the integrated server's game, or the dedicated one) hears
//!   them in `run_native_net`, stamps the speaker's uuid and position, and
//!   forwards them to every other CS client in the same dimension within range.
//! - A client that receives one feeds it to the mixer, where it plays from the
//!   speaker's position: quieter with distance, panned left or right.
//!
//! Settings live in `cs-voice.json` next to the CS Friends identity (shared
//! by every profile).
use super::{Game, Screen};
use horizon_client::cs_voice::{self, FRAME, FRAME_BYTES, Mic};
use horizon_client::{
    extras::Corner,
    extras_hud::Corners,
    hud::HudBatch,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use winit::keyboard::KeyCode;

/// The mod-message channel voice travels on.
pub const CHANNEL: &str = "cs:voice";
/// Serverbound: `[1][seq u16][frame…]`.
const TO_SERVER: u8 = 1;
/// Clientbound: `[2][uuid 16][x f32][y f32][z f32][range u16][seq u16][frame…]`.
const TO_CLIENT: u8 = 2;
/// Speaking (push-to-talk) is this key's mapping, `key.cs_voice`, default V.
pub const PTT_KEY: KeyCode = KeyCode::KeyV;
/// Open-mic mode sends while the frame's peak is above this (0 to 1).
const OPEN_MIC_THRESHOLD: f32 = 0.04;
/// …and for this long after it last was (so words don't clip).
const OPEN_MIC_HOLD: Duration = Duration::from_millis(400);
/// A microphone that would not open is tried again this often.
const MIC_RETRY: Duration = Duration::from_secs(4);

/// The player's voice settings.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct VoiceSettings {
    /// Voice chat on at all.
    pub enabled: bool,
    /// Push to talk (else open mic by loudness).
    pub ptt: bool,
    /// Others' voices, percent (0 to 200).
    pub volume: u32,
    /// Microphone gain, percent (50 to 300).
    pub mic: u32,
    /// How far a voice carries, in blocks (a host's setting; its world uses it).
    pub range: u32,
    /// Players muted, by lowercase name.
    pub muted: Vec<String>,
}
impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            ptt: true,
            volume: 100,
            mic: 100,
            range: 48,
            muted: Vec::new(),
        }
    }
}
fn settings_path() -> std::path::PathBuf {
    horizon_client::cs_friends::identity_dir().join("cs-voice.json")
}
impl VoiceSettings {
    fn load() -> Self {
        std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }
    fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(settings_path(), text);
        }
    }
    pub fn is_muted(&self, name: &str) -> bool {
        let name = name.to_lowercase();
        self.muted.iter().any(|m| *m == name)
    }
}

/// Everything voice chat keeps between frames.
pub struct CsVoice {
    pub settings: VoiceSettings,
    loaded: bool,
    mic: Option<Mic>,
    /// Why the microphone isn't open, for the Voice tab.
    pub mic_problem: Option<String>,
    mic_retry: Option<Instant>,
    /// Captured samples not yet a whole frame.
    pending: Vec<i16>,
    seq: u16,
    /// The player is sending right now.
    pub talking: bool,
    /// Open mic: when the voice was last loud enough.
    last_loud: Option<Instant>,
    /// The microphone's level (0 to 1) for the meter.
    pub level: f32,
    /// Who is heard, by uuid, and when last (for the HUD and the Voice tab).
    pub heard: HashMap<u128, Instant>,
    /// Server side: frames relayed this session (a diagnostic).
    pub relayed: u64,
    /// Server side: when the settings file was last read (the host's range).
    reloaded: Option<Instant>,
}
impl Default for CsVoice {
    fn default() -> Self {
        Self {
            settings: VoiceSettings::default(),
            loaded: false,
            mic: None,
            mic_problem: None,
            mic_retry: None,
            pending: Vec::new(),
            seq: 0,
            talking: false,
            last_loud: None,
            level: 0.0,
            heard: HashMap::new(),
            relayed: 0,
            reloaded: None,
        }
    }
}

/// A clientbound voice payload.
struct Incoming {
    who: u128,
    at: [f32; 3],
    range: f32,
    frames: Vec<[i16; FRAME]>,
}
fn parse_to_client(data: &[u8]) -> Option<Incoming> {
    if data.len() < 1 + 16 + 12 + 2 + 2 || data[0] != TO_CLIENT {
        return None;
    }
    let who = u128::from_be_bytes(data[1..17].try_into().ok()?);
    let f = |i: usize| f32::from_be_bytes(data[i..i + 4].try_into().unwrap_or([0; 4]));
    let at = [f(17), f(21), f(25)];
    let range = f32::from(u16::from_be_bytes([data[29], data[30]]));
    let frames = data[33..]
        .chunks(FRAME_BYTES)
        .filter_map(cs_voice::decode_frame)
        .collect();
    Some(Incoming {
        who,
        at,
        range,
        frames,
    })
}

impl Game<'_> {
    /// Voice can travel: in a world, on a connection that carries mod
    /// messages (a CS Engine / Horizon server, the integrated one included).
    fn cs_voice_available(&self) -> bool {
        self.title.is_none()
            && self
                .remote
                .as_ref()
                .is_some_and(|r| r.joined && r.reconfiguring.is_none() && r.mod_net)
    }
    pub(super) fn cs_voice_load(&mut self) {
        if !self.cs_voice.loaded {
            self.cs_voice.settings = VoiceSettings::load();
            self.cs_voice.loaded = true;
            let v = self.cs_voice.settings.volume as f32 / 100.0;
            self.audio.voice_volume(v);
        }
    }
    /// The settings changed (the Voice tab): kept, and applied.
    pub(super) fn cs_voice_save(&mut self) {
        self.cs_voice.settings.save();
        let v = self.cs_voice.settings.volume as f32 / 100.0;
        self.audio.voice_volume(v);
        if !self.cs_voice.settings.enabled {
            self.cs_voice.mic = None;
            self.cs_voice.talking = false;
            self.audio.voice_clear();
        }
    }
    /// Whether the player means to send right now.
    fn cs_voice_wants_to_send(&mut self, peak: f32) -> bool {
        if self.cs_voice.settings.ptt {
            // Push to talk: the key (or the CS control button bound to it),
            // while the game has the keys (not typing in chat).
            let held = self.keys.contains(&PTT_KEY);
            return held && matches!(self.screen, Screen::Playing | Screen::Friends | Screen::Paused);
        }
        let now = Instant::now();
        let v = &mut self.cs_voice;
        if peak >= OPEN_MIC_THRESHOLD {
            v.last_loud = Some(now);
        }
        v.last_loud
            .is_some_and(|t| now.duration_since(t) < OPEN_MIC_HOLD)
    }
    /// Once a frame, on the window's game: the microphone in, frames out,
    /// and who is heard.
    pub(super) fn cs_voice_frame(&mut self) {
        self.cs_voice_load();
        // Who is heard right now, for the HUD.
        let now = Instant::now();
        for id in self.audio.voice_speaking() {
            self.cs_voice.heard.insert(id, now);
        }
        self.cs_voice
            .heard
            .retain(|_, t| now.duration_since(*t) < Duration::from_secs(2));
        if !self.cs_voice.settings.enabled || !self.cs_voice_available() {
            if self.cs_voice.mic.is_some() {
                self.cs_voice.mic = None;
                self.cs_voice.pending.clear();
            }
            self.cs_voice.talking = false;
            self.cs_voice.level = 0.0;
            if self.title.is_some() {
                self.audio.voice_clear();
                self.cs_voice.heard.clear();
            }
            return;
        }
        // The microphone, opened when needed; without permission it fails,
        // so the host is asked for it once, and the open is tried again.
        if self.cs_voice.mic.is_none()
            && self
                .cs_voice
                .mic_retry
                .is_none_or(|t| t.elapsed() > MIC_RETRY)
        {
            self.cs_voice.mic_retry = Some(Instant::now());
            match Mic::open() {
                Ok(mic) => {
                    log::info!("CS voice: microphone open ({})", mic.device);
                    self.cs_voice.mic = Some(mic);
                    self.cs_voice.mic_problem = None;
                }
                Err(e) => {
                    horizon_client::android_runtime::host_request_mic();
                    self.cs_voice.mic_problem = Some(e);
                }
            }
        }
        // The host's microphone failed (permission denied, no device): try
        // again later, and say so in the Voice tab.
        if let Some(problem) = self.cs_voice.mic.as_ref().and_then(|m| m.problem()) {
            self.cs_voice.mic = None;
            self.cs_voice.pending.clear();
            self.cs_voice.talking = false;
            self.cs_voice.mic_problem = Some(problem);
            self.cs_voice.mic_retry = Some(Instant::now());
            return;
        }
        let Some(mic) = &self.cs_voice.mic else {
            return;
        };
        let gain = (self.cs_voice.settings.mic as f32 / 100.0).clamp(0.25, 4.0);
        let samples = mic.take(gain);
        self.cs_voice.level = mic.level().min(1.0);
        let peak = self.cs_voice.level;
        self.cs_voice.pending.extend_from_slice(&samples);
        let send = self.cs_voice_wants_to_send(peak);
        if !send {
            // Not talking: drop what was captured, keep the tail short.
            self.cs_voice.pending.clear();
            self.cs_voice.talking = false;
            return;
        }
        self.cs_voice.talking = true;
        // Whole frames go out, the rest waits (at most 5 frames pile up when
        // the game stalls: beyond that the oldest are dropped, not delayed).
        let mut payload = Vec::with_capacity(3 + FRAME_BYTES * 3);
        let mut frames = 0;
        while self.cs_voice.pending.len() >= FRAME {
            let frame: [i16; FRAME] = self.cs_voice.pending[..FRAME]
                .try_into()
                .expect("a whole frame");
            self.cs_voice.pending.drain(..FRAME);
            if frames == 0 {
                payload.push(TO_SERVER);
                payload.extend_from_slice(&self.cs_voice.seq.to_be_bytes());
                self.cs_voice.seq = self.cs_voice.seq.wrapping_add(1);
            }
            payload.extend_from_slice(&cs_voice::encode_frame(&frame));
            frames += 1;
            if frames >= 5 {
                self.cs_voice.pending.clear();
                break;
            }
        }
        if frames > 0
            && let Err(e) = self.native_net_send_to_server(CHANNEL, &payload)
        {
            log::debug!("CS voice: not sent: {e}");
        }
    }

    /// Server side: a client's frames, relayed to everyone else who can hear.
    /// `true` when the message was voice (and is not for the mods).
    pub(super) fn cs_voice_from_client(&mut self, player: u128, channel: &str, data: &[u8]) -> bool {
        if channel != CHANNEL {
            return false;
        }
        if data.len() < 3 + FRAME_BYTES || data[0] != TO_SERVER {
            return true;
        }
        // The host's range, from the file the window's game saves (this is
        // the server's game: another thread, or another process).
        if self
            .cs_voice
            .reloaded
            .is_none_or(|t| t.elapsed() > Duration::from_secs(10))
        {
            self.cs_voice.reloaded = Some(Instant::now());
            self.cs_voice.settings = VoiceSettings::load();
        }
        let seq = [data[1], data[2]];
        let frames = &data[3..];
        let Some(host) = self.lan.as_ref() else {
            return true;
        };
        let Some(speaker) = host.hub.guests.iter().find(|g| g.uuid == player && g.placed) else {
            return true;
        };
        let at = [
            speaker.position[0] as f32,
            (speaker.position[1] + 1.62) as f32,
            speaker.position[2] as f32,
        ];
        let dimension = speaker.dimension.clone();
        let range = self.cs_voice.settings.range.clamp(8, 256);
        let mut payload = Vec::with_capacity(33 + frames.len());
        payload.push(TO_CLIENT);
        payload.extend_from_slice(&player.to_be_bytes());
        for c in at {
            payload.extend_from_slice(&c.to_be_bytes());
        }
        payload.extend_from_slice(&(range as u16).to_be_bytes());
        payload.extend_from_slice(&seq);
        payload.extend_from_slice(frames);
        let reach = f64::from(range) + 8.0;
        let hearers: Vec<u128> = host
            .hub
            .guests
            .iter()
            .filter(|g| {
                g.placed
                    && g.uuid != player
                    && g.brand == "horizon"
                    && g.dimension == dimension
                    && {
                        let d: f64 = (0..3)
                            .map(|i| (g.position[i] - speaker.position[i]).powi(2))
                            .sum();
                        d <= reach * reach
                    }
            })
            .map(|g| g.uuid)
            .collect();
        for uuid in hearers {
            if self.native_net_send_to_player(uuid, CHANNEL, &payload).is_ok() {
                self.cs_voice.relayed += 1;
            }
        }
        true
    }

    /// Client side: a speaker's frames from the server, into the mixer.
    /// `true` when the message was voice.
    pub(super) fn cs_voice_from_server(&mut self, channel: &str, data: &[u8]) -> bool {
        if channel != CHANNEL {
            return false;
        }
        self.cs_voice_load();
        if !self.cs_voice.settings.enabled {
            return true;
        }
        let Some(in_) = parse_to_client(data) else {
            return true;
        };
        let own = self.remote.as_ref().map(|r| r.profile());
        if own == Some(in_.who) {
            return true;
        }
        let name = self
            .remote
            .as_ref()
            .and_then(|r| r.players.get(&in_.who).cloned())
            .unwrap_or_default();
        if !name.is_empty() && self.cs_voice.settings.is_muted(&name) {
            return true;
        }
        for frame in &in_.frames {
            self.audio.voice_push(in_.who, in_.at, in_.range, frame);
        }
        self.cs_voice.heard.insert(in_.who, Instant::now());
        true
    }

    /// The players in the world (other than us) for the Voice tab: name,
    /// muted, heard now.
    pub(super) fn cs_voice_players(&self) -> Vec<(String, bool, bool)> {
        let Some(r) = self.remote.as_ref() else {
            return Vec::new();
        };
        let own = r.profile();
        let mut out: Vec<(String, bool, bool)> = r
            .players
            .iter()
            .filter(|(u, _)| **u != own)
            .map(|(u, n)| {
                (
                    n.clone(),
                    self.cs_voice.settings.is_muted(n),
                    self.cs_voice.heard.contains_key(u),
                )
            })
            .collect();
        out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        out
    }
    pub(super) fn cs_voice_toggle_mute(&mut self, name: &str) {
        let key = name.to_lowercase();
        let m = &mut self.cs_voice.settings.muted;
        if let Some(i) = m.iter().position(|x| *x == key) {
            m.remove(i);
        } else {
            m.push(key);
        }
        self.cs_voice_save();
    }

    /// The HUD: who is speaking (top left, under the CS HUD modules), and the
    /// player's own microphone while talking.
    pub(super) fn draw_cs_voice_hud(&mut self, batch: &mut HudBatch, corners: &mut Corners) {
        if !self.cs_voice.settings.enabled || self.title.is_some() || self.screen != Screen::Playing {
            return;
        }
        let mut lines: Vec<(String, u32)> = Vec::new();
        if self.cs_voice.talking {
            let bars = (self.cs_voice.level * 6.0).round() as usize;
            lines.push((format!("● You {}", "|".repeat(bars.min(6))), 0xFF55_FF55));
        }
        if let Some(r) = self.remote.as_ref() {
            let own = r.profile();
            let mut names: Vec<&String> = self
                .cs_voice
                .heard
                .keys()
                .filter(|u| **u != own)
                .filter_map(|u| r.players.get(u))
                .collect();
            names.sort();
            for n in names.into_iter().take(6) {
                lines.push((format!("● {n}"), 0xFFE0_E0E0));
            }
        }
        if lines.is_empty() {
            return;
        }
        let atlas = &self.gui.atlas;
        let width = lines
            .iter()
            .map(|(t, _)| HudBatch::text_width(atlas, t))
            .fold(0.0f32, f32::max)
            + 8.0;
        let height = lines.len() as f32 * 10.0 + 4.0;
        let (x0, y0) = corners.place(Corner::TopLeft, width, height);
        batch.fill(atlas, x0, y0, width, height, 0x7800_0000);
        for (i, (t, color)) in lines.iter().enumerate() {
            batch.text(atlas, t, x0 + 4.0, y0 + 2.0 + i as f32 * 10.0, *color, true);
        }
    }
}
