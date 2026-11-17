//! A minimal game against a chart. The song clock starts at the first update and runs on the audio clock. Chart
//! notes sounding now ring on the fretboard. A chart note is a hit when the player starts the same pitch within
//! `HIT_WINDOW_S` of its time, judged on the note's onset so the model's detection delay does not count against the
//! player. This is an example of the API for the game engine; it is not the engine.

use crate::{Game, GameContext};
use egui::{pos2, Rect};
use riffology_core::note_name;
use riffology_fretboard::{FretMark, MarkKind};
use riffology_song::Chart;
use rux::Style;

/// How far from a chart note's time an onset still hits it, in seconds.
pub const HIT_WINDOW_S: f64 = 0.1;

/// The chart following game.
pub struct ChartFollow {
    chart: Chart,
    start_s: Option<f64>,
    song_s: f64,
    hit: Vec<bool>,
    judged: usize,
}

impl ChartFollow {
    /// A game for a chart. Every note needs a string and fret; call `Chart::assign_strings` first.
    /// @param chart - the chart
    pub fn new(chart: Chart) -> ChartFollow {
        let n = chart.notes.len();
        ChartFollow { chart, start_s: None, song_s: 0.0, hit: vec![false; n], judged: 0 }
    }

    /// Starts the song at a moment of the audio clock instead of at the first update.
    /// @param clock_s - the moment
    pub fn start_at(&mut self, clock_s: f64) {
        self.start_s = Some(clock_s);
    }

    /// The song position in seconds.
    pub fn song_s(&self) -> f64 {
        self.song_s
    }

    /// Hits and notes judged so far (a note is judged once the hit window after it has passed).
    pub fn score(&self) -> (usize, usize) {
        (self.hit[..self.judged].iter().filter(|h| **h).count(), self.judged)
    }

    /// Marks hits for every chart note whose window holds an onset of its pitch.
    /// @param ctx - the context
    /// @param start - the song's start on the audio clock
    fn judge(&mut self, ctx: &GameContext<'_>, start: f64) {
        for e in ctx.player.started_since(start - HIT_WINDOW_S) {
            let t = e.onset_s - start;
            for (i, n) in self.chart.notes.iter().enumerate() {
                if !self.hit[i] && n.midi == e.midi as i32 && (n.time_s - t).abs() <= HIT_WINDOW_S {
                    self.hit[i] = true;
                    break;
                }
            }
        }
        while self.judged < self.chart.notes.len() && self.chart.notes[self.judged].time_s + HIT_WINDOW_S < self.song_s
        {
            self.judged += 1;
        }
    }
}

impl Game for ChartFollow {
    fn name(&self) -> &str {
        &self.chart.title
    }

    fn update(&mut self, ctx: &GameContext<'_>) {
        let start = *self.start_s.get_or_insert(ctx.clock_s);
        self.song_s = (ctx.clock_s - start).max(0.0);
        self.judge(ctx, start);
    }

    fn marks(&self) -> Vec<FretMark> {
        self.chart
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.time_s <= self.song_s + 0.05 && self.song_s < n.time_s + n.length_s.max(0.15))
            .filter_map(|(i, n)| {
                Some(FretMark {
                    string: n.string?,
                    fret: n.fret?,
                    kind: if self.hit[i] { MarkKind::Good } else { MarkKind::Expected },
                })
            })
            .collect()
    }

    fn draw(&mut self, rux: &mut rux::Rux<'_>, rect: Rect, _ctx: &GameContext<'_>) {
        let t = rux.theme();
        let (hits, judged) = self.score();
        let next: Vec<String> = self
            .chart
            .notes_between(self.song_s, self.song_s + 2.0)
            .take(6)
            .map(|n| format!("{} ({:.1} s)", note_name(n.midi), n.time_s - self.song_s))
            .collect();
        let lines = [
            format!("{}  {:.1} s", self.chart.title, self.song_s),
            format!("Hits {hits} of {judged}"),
            format!("Next: {}", next.join(", ")),
        ];
        let mut y = rect.top();
        for (i, line) in lines.iter().enumerate() {
            let style = if i == 0 { Style::H2 } else { Style::BODY };
            let g = rux.text(style, line, t.ink.i900);
            rux.painter().galley(pos2(rect.left(), y), g.clone(), t.ink.i900);
            y += g.size().y + rux.z(8.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use riffology_core::{NoteEvent, PlayerState, Source, Tuning};
    use riffology_song::ChartNote;

    fn chart() -> Chart {
        let mut c = Chart::new("Two notes", Tuning::standard());
        c.notes = vec![
            ChartNote { time_s: 1.0, length_s: 0.5, midi: 40, string: None, fret: None },
            ChartNote { time_s: 2.0, length_s: 0.5, midi: 45, string: None, fret: None },
        ];
        c.assign_strings();
        c
    }

    fn on(midi: u8, onset_s: f64) -> NoteEvent {
        NoteEvent { midi, velocity: 90, is_on: true, source: Source::Guitar, onset_s, heard_s: onset_s + 0.05 }
    }

    #[test]
    fn an_onset_inside_the_window_hits_and_one_outside_misses() {
        let tuning = Tuning::standard();
        let mut player = PlayerState::new();
        let mut g = ChartFollow::new(chart());
        g.start_at(10.0);
        player.apply(&[on(40, 11.06), on(45, 12.3)]);
        g.update(&GameContext { clock_s: 12.5, player: &player, tuning: &tuning });
        assert_eq!(g.score(), (1, 2));
    }

    #[test]
    fn expected_notes_ring_while_they_sound() {
        let tuning = Tuning::standard();
        let player = PlayerState::new();
        let mut g = ChartFollow::new(chart());
        g.start_at(0.0);
        g.update(&GameContext { clock_s: 1.2, player: &player, tuning: &tuning });
        assert_eq!(g.marks(), vec![FretMark { string: 6, fret: 0, kind: MarkKind::Expected }]);
    }
}
