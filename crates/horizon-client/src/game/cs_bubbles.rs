//! CS Engine chat bubbles: what a player in the same world just said floats
//! over their name tag for a few seconds, popping in and fading out, and CS
//! Launcher players (the player, their CS friends) wear a small CS badge in
//! front of their name.
use super::Game;
use horizon_client::text::Text;
use std::time::Instant;

/// How long a bubble stays, plus a little per character to read it.
const BASE_SECS: f32 = 5.0;
const PER_CHAR_SECS: f32 = 0.05;
const MAX_SECS: f32 = 11.0;
/// The pop-in and the fade-out, seconds.
pub(super) const POP_SECS: f32 = 0.24;
pub(super) const FADE_SECS: f32 = 0.45;
/// Characters per bubble line and lines per bubble.
const LINE_CHARS: usize = 30;
const MAX_LINES: usize = 3;

pub(super) struct CsBubble {
    text: String,
    since: Instant,
}

/// The bubble's lines (top first) and its age in seconds, for a name tag.
pub(super) struct BubbleView {
    pub lines: Vec<Text>,
    pub age: f32,
    pub life: f32,
}

impl BubbleView {
    /// The pop-in scale: a quick overshoot settling at 1.
    pub fn scale(&self) -> f32 {
        let p = (self.age / POP_SECS).clamp(0.0, 1.0);
        let q = p - 1.0;
        let back = 1.0 + 2.70158 * q * q * q + 1.70158 * q * q;
        0.35 + 0.65 * back
    }
    /// 0..=1 opacity: full until the fade, then out.
    pub fn opacity(&self) -> f32 {
        let remain = self.life - self.age;
        (remain / FADE_SECS).clamp(0.0, 1.0)
    }
    /// How far the bubble drifts up as it fades, in blocks.
    pub fn drift(&self) -> f32 {
        (1.0 - self.opacity()) * 0.18
    }
}

fn life_for(text: &str) -> f32 {
    (BASE_SECS + text.chars().count() as f32 * PER_CHAR_SECS).min(MAX_SECS)
}

/// Word wrap into at most `MAX_LINES` lines of about `LINE_CHARS` characters,
/// the last one ending in … when the message went on.
pub(super) fn wrap(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut truncated = false;
    'words: for word in text.split_whitespace() {
        let mut word = word.to_owned();
        loop {
            let fits = if line.is_empty() {
                word.chars().count() <= LINE_CHARS
            } else {
                line.chars().count() + 1 + word.chars().count() <= LINE_CHARS
            };
            if fits {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(&word);
                continue 'words;
            }
            if line.is_empty() {
                // A word longer than a line: split it.
                let head: String = word.chars().take(LINE_CHARS).collect();
                word = word.chars().skip(LINE_CHARS).collect();
                lines.push(head);
            } else {
                lines.push(std::mem::take(&mut line));
            }
            if lines.len() >= MAX_LINES {
                truncated = true;
                break 'words;
            }
        }
    }
    if !line.is_empty() {
        if lines.len() < MAX_LINES {
            lines.push(line);
        } else {
            truncated = true;
        }
    }
    if truncated && let Some(last) = lines.last_mut() {
        let mut s: String = last.chars().take(LINE_CHARS - 1).collect();
        s.push('…');
        *last = s;
    }
    lines
}

impl Game<'_> {
    /// A player said something: their bubble shows it (replacing the last).
    pub(super) fn cs_bubble_say(&mut self, name: &str, text: &str) {
        let text = text.trim();
        if text.is_empty() || text.starts_with('/') {
            return;
        }
        self.cs_bubbles.insert(
            name.to_owned(),
            CsBubble {
                text: text.chars().take(256).collect(),
                since: Instant::now(),
            },
        );
    }
    /// The live bubble over `name`, if any; expired ones are dropped as met.
    pub(super) fn cs_bubble_view(&self, name: &str) -> Option<BubbleView> {
        let b = self.cs_bubbles.get(name)?;
        let age = b.since.elapsed().as_secs_f32();
        let life = life_for(&b.text);
        if age >= life {
            return None;
        }
        Some(BubbleView {
            lines: wrap(&b.text).into_iter().map(Text::literal).collect(),
            age,
            life,
        })
    }
    /// Once a tick: forget bubbles that faded out.
    pub(super) fn cs_bubbles_tick(&mut self) {
        self.cs_bubbles
            .retain(|_, b| b.since.elapsed().as_secs_f32() < life_for(&b.text));
    }
    /// Whether `name` wears the CS badge: the player, or one of their CS friends.
    pub(super) fn cs_badge_for(&self, name: &str) -> bool {
        if name.eq_ignore_ascii_case(&self.own_profile_name()) {
            return true;
        }
        self.cs_friends.as_ref().is_some_and(|c| {
            c.state
                .friends
                .iter()
                .any(|f| f.name.eq_ignore_ascii_case(name))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_long_messages_into_three_lines_at_most() {
        assert_eq!(wrap("hi there"), vec!["hi there"]);
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";
        let lines = wrap(long);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| l.chars().count() <= LINE_CHARS));
        assert!(lines[2].ends_with('…'));
        // A single word longer than a line is split, not dropped.
        let word = "a".repeat(70);
        let lines = wrap(&word);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].len(), LINE_CHARS);
    }

    #[test]
    fn the_pop_settles_at_full_size_and_the_fade_ends_transparent() {
        let v = |age: f32| BubbleView { lines: Vec::new(), age, life: 5.0 };
        assert!(v(0.0).scale() < 0.5);
        assert!((v(POP_SECS).scale() - 1.0).abs() < 1e-4);
        assert!((v(1.0).opacity() - 1.0).abs() < 1e-6);
        assert!(v(4.999).opacity() < 0.01);
    }
}
