//! What the player is playing now. The game thread applies the note bus's events once per frame, and every view (the
//! fretboard, the game engine, the latency readout) reads the same `PlayerState`.

use crate::event::{NoteEvent, Source};
use crate::pitch::Tuning;
use crate::placement::{place_on_strings, settle_shown, LitFret, ShownFrets};
use std::collections::VecDeque;

/// How many recent note ons are kept for `started_since` and the delay statistics.
const RECENT: usize = 512;

/// A note that is sounding now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sounding {
    pub midi: u8,
    pub velocity: u8,
    pub source: Source,
    /// When it started, on the audio clock.
    pub onset_s: f64,
}

/// The player's notes: what sounds now and what started recently.
#[derive(Debug, Default)]
pub struct PlayerState {
    sounding: Vec<Sounding>,
    recent: VecDeque<NoteEvent>,
    shown: ShownFrets,
    latest_s: f64,
    total_ons: u64,
}

impl PlayerState {
    /// An empty state.
    pub fn new() -> PlayerState {
        PlayerState::default()
    }

    /// Applies events in order: a note on adds the note (replacing the same pitch from the same source), a note off
    /// removes it.
    /// @param events - events from the note bus
    pub fn apply(&mut self, events: &[NoteEvent]) {
        for e in events {
            self.latest_s = self.latest_s.max(e.heard_s);
            self.sounding.retain(|s| !(s.midi == e.midi && s.source == e.source));
            if e.is_on && e.velocity > 0 {
                self.sounding.push(Sounding {
                    midi: e.midi,
                    velocity: e.velocity,
                    source: e.source,
                    onset_s: e.onset_s,
                });
                self.total_ons += 1;
                if self.recent.len() == RECENT {
                    self.recent.pop_front();
                }
                self.recent.push_back(*e);
            }
        }
        if !events.is_empty() {
            self.sounding.sort_by_key(|s| s.midi);
        }
    }

    /// The notes sounding now, lowest first.
    pub fn sounding(&self) -> &[Sounding] {
        &self.sounding
    }

    /// Whether a pitch is sounding now, from any source.
    /// @param midi - the pitch
    pub fn is_sounding(&self, midi: u8) -> bool {
        self.sounding.iter().any(|s| s.midi == midi)
    }

    /// The note ons whose onset is at or after `t` seconds on the audio clock, oldest first.
    /// @param t - the time
    pub fn started_since(&self, t: f64) -> impl Iterator<Item = &NoteEvent> {
        self.recent.iter().filter(move |e| e.onset_s >= t)
    }

    /// The newest time any event was heard, on the audio clock.
    pub fn latest_s(&self) -> f64 {
        self.latest_s
    }

    /// Note ons applied since the state was made.
    pub fn total_note_ons(&self) -> u64 {
        self.total_ons
    }

    /// The detection delays of the recent note ons in seconds, oldest first.
    pub fn recent_delays(&self) -> Vec<f64> {
        self.recent.iter().map(|e| e.detection_delay_s()).collect()
    }

    /// The cells the fretboard lights for the sounding notes, placed on strings and settled so a strum does not jump.
    /// Call once per frame.
    /// @param tuning - the fretboard's tuning
    pub fn update_frets(&mut self, tuning: &Tuning) -> &[LitFret] {
        let midis: Vec<i32> = self.sounding.iter().map(|s| s.midi as i32).collect();
        let next = place_on_strings(&midis, tuning, &[]);
        if next != self.shown.shown || self.shown.pending.is_some() {
            self.shown = settle_shown(&self.shown, next);
        }
        &self.shown.shown
    }

    /// The cells shown after the last `update_frets`.
    pub fn frets(&self) -> &[LitFret] {
        &self.shown.shown
    }

    /// Ends every sounding note, for example when the input changes.
    pub fn clear(&mut self) {
        self.sounding.clear();
        self.shown = ShownFrets::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(midi: u8, on: bool, t: f64) -> NoteEvent {
        NoteEvent {
            midi,
            velocity: if on { 90 } else { 0 },
            is_on: on,
            source: Source::Guitar,
            onset_s: t,
            heard_s: t + 0.02,
        }
    }

    #[test]
    fn note_ons_sound_until_their_note_off() {
        let mut p = PlayerState::new();
        p.apply(&[ev(64, true, 1.0), ev(52, true, 1.0)]);
        assert_eq!(p.sounding().iter().map(|s| s.midi).collect::<Vec<_>>(), vec![52, 64]);
        p.apply(&[ev(64, false, 1.5)]);
        assert!(!p.is_sounding(64) && p.is_sounding(52));
        assert_eq!(p.started_since(0.5).count(), 2);
        assert_eq!(p.started_since(1.2).count(), 0);
        assert!((p.recent_delays()[0] - 0.02).abs() < 1e-9);
    }

    #[test]
    fn sounding_notes_light_frets() {
        let mut p = PlayerState::new();
        p.apply(&[ev(40, true, 0.0)]);
        assert_eq!(p.update_frets(&Tuning::standard()), &[LitFret { string: 6, fret: 0, midi: 40 }]);
        p.apply(&[ev(40, false, 0.4)]);
        assert!(p.update_frets(&Tuning::standard()).is_empty());
    }
}
