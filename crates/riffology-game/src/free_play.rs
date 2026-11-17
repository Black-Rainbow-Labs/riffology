//! Free play: no song. Shows the notes sounding now and the last notes started, each with its detection delay.

use crate::{Game, GameContext};
use egui::{pos2, Rect};
use riffology_core::note_name;
use riffology_fretboard::FretMark;
use rux::Style;

/// How many recent notes the list shows.
const SHOWN: usize = 8;

/// The free play game.
#[derive(Default)]
pub struct FreePlay {
    lines: Vec<String>,
    now: String,
}

impl Game for FreePlay {
    fn name(&self) -> &str {
        "Free play"
    }

    fn update(&mut self, ctx: &GameContext<'_>) {
        let names: Vec<String> = ctx.player.sounding().iter().map(|s| note_name(s.midi as i32)).collect();
        self.now = if names.is_empty() {
            "Play something".to_string()
        } else {
            format!("You are playing {}", names.join(" "))
        };
        let since = ctx.clock_s - 30.0;
        let recent: Vec<_> = ctx.player.started_since(since).collect();
        self.lines = recent
            .iter()
            .rev()
            .take(SHOWN)
            .map(|e| {
                format!(
                    "{:>8.2} s   {:<4}  detected after {:>3.0} ms",
                    e.onset_s,
                    note_name(e.midi as i32),
                    e.detection_delay_s() * 1000.0
                )
            })
            .collect();
    }

    fn marks(&self) -> Vec<FretMark> {
        vec![]
    }

    fn draw(&mut self, rux: &mut rux::Rux<'_>, rect: Rect, _ctx: &GameContext<'_>) {
        let t = rux.theme();
        let head = rux.text(Style::H2, &self.now, t.ink.i900);
        let mut y = rect.top();
        rux.painter().galley(pos2(rect.left(), y), head.clone(), t.ink.i900);
        y += head.size().y + rux.z(12.0);
        for line in &self.lines {
            let g = rux.text(Style::NUMBER, line, t.ink.i500);
            rux.painter().galley(pos2(rect.left(), y), g.clone(), t.ink.i500);
            y += g.size().y + rux.z(4.0);
        }
    }
}
