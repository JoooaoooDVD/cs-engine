//! The performance overlay (Horizon's Overlay setting): frames per second,
//! frame, CPU and GPU times, and a graph of recent frame times.
use crate::{
    graphics::Overlay,
    hud::{HudAtlas, HudBatch, rgba},
    text::Text,
};
use std::collections::VecDeque;

const HISTORY: usize = 160;

#[derive(Default)]
pub struct FrameGraph {
    /// Frame, CPU and GPU milliseconds of recent frames, oldest first.
    frames: VecDeque<(f32, f32, f32)>,
}
impl FrameGraph {
    pub fn record(&mut self, frame_ms: f32, cpu_ms: f32, gpu_ms: Option<f32>) {
        let gpu = gpu_ms.unwrap_or_else(|| self.frames.back().map_or(0.0, |f| f.2));
        if self.frames.len() == HISTORY {
            self.frames.pop_front();
        }
        self.frames.push_back((frame_ms, cpu_ms, gpu));
    }
    /// FPS at the 99th percentile frame time over up to 160 recent frames.
    /// Selection is linear and uses a fixed stack buffer, without per-frame heap allocation.
    fn one_percent_low(&self) -> f32 {
        let mut times = [0.0_f32; HISTORY];
        let mut n = 0;
        for &(ms, _, _) in &self.frames {
            if ms.is_finite() && ms > 0.0 {
                times[n] = ms;
                n += 1;
            }
        }
        if n == 0 {
            return 0.0;
        }
        let index = n - 1 - (n - 1) / 100;
        let (_, value, _) = times[..n].select_nth_unstable_by(index, f32::total_cmp);
        1000.0 / *value
    }
    pub fn draw(&self, mode: Overlay, batch: &mut HudBatch, atlas: &HudAtlas) {
        if mode == Overlay::Off || self.frames.is_empty() {
            return;
        }
        let n = self.frames.len() as f32;
        let avg = |f: fn(&(f32, f32, f32)) -> f32| self.frames.iter().map(f).sum::<f32>() / n;
        let (frame, cpu, gpu) = (avg(|f| f.0), avg(|f| f.1), avg(|f| f.2));
        let worst = self.frames.iter().map(|f| f.0).fold(0.0f32, f32::max);
        let low = self.one_percent_low();
        let lines = [
            format!("{:.0} fps  1% low {low:.0}", 1000.0 / frame.max(0.001)),
            format!("frame {frame:.1} ms  max {worst:.1}"),
            format!("cpu {cpu:.1}  gpu {gpu:.1} ms"),
        ];
        let width = 120.0f32.max(
            lines
                .iter()
                .map(|l| atlas.font.width(l))
                .fold(0.0, f32::max)
                + 6.0,
        );
        let x = batch.width - width - 2.0;
        let graph = if mode == Overlay::Graph { 42.0 } else { 0.0 };
        batch.fill(
            atlas,
            x,
            2.0,
            width,
            4.0 + lines.len() as f32 * 10.0 + graph,
            rgba(0, 0, 0, 144),
        );
        for (i, line) in lines.iter().enumerate() {
            let color = if i == 0 {
                rgba(255, 255, 255, 255)
            } else {
                rgba(200, 200, 200, 255)
            };
            batch.text_styled(
                atlas,
                &Text::literal(line.clone()),
                x + 3.0,
                4.0 + i as f32 * 10.0,
                color,
                true,
            );
        }
        if mode != Overlay::Graph {
            return;
        }
        // Bars of frame time on a 0..33 ms scale; lines at 60 and 30 fps.
        let (gy, gh) = (4.0 + lines.len() as f32 * 10.0 + 2.0, 36.0);
        let scale = gh / 33.3;
        let bar = (width - 6.0) / HISTORY as f32;
        for (i, &(ms, cpu, _)) in self.frames.iter().enumerate() {
            let height = (ms * scale).min(gh);
            let bx = x + 3.0 + i as f32 * bar;
            let color = if ms > 33.3 {
                rgba(240, 90, 80, 255)
            } else if ms > 16.7 {
                rgba(240, 200, 90, 255)
            } else {
                rgba(110, 210, 120, 255)
            };
            batch.fill(atlas, bx, gy + gh - height, bar.max(1.0), height, color);
            let cpu_height = (cpu * scale).min(height);
            batch.fill(
                atlas,
                bx,
                gy + gh - cpu_height,
                bar.max(1.0),
                cpu_height,
                rgba(80, 140, 230, 200),
            );
        }
        for target in [16.7, 33.3] {
            let ly = gy + gh - target * scale;
            batch.fill(
                atlas,
                x + 3.0,
                ly,
                width - 6.0,
                1.0,
                rgba(255, 255, 255, 90),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_uses_recent_finite_frame_times_and_bounded_history() {
        let mut graph = FrameGraph::default();
        assert_eq!(graph.one_percent_low(), 0.0);
        for i in 0..160 {
            graph.record(if i < 2 { 50.0 } else { 10.0 }, 1.0, None);
        }
        assert_eq!(graph.one_percent_low(), 20.0);
        graph.record(10.0, 1.0, None);
        assert_eq!(graph.one_percent_low(), 100.0);
        for _ in 0..160 {
            graph.record(f32::NAN, 1.0, None);
        }
        assert_eq!(graph.one_percent_low(), 0.0);
    }
}
