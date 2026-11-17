//! Song charts. A [`Chart`] is what a game plays against: notes with a start time in seconds, a length, a pitch, and
//! the string and fret to play it on. Charts are saved as JSON (`docs/chart-format.md`) and can be imported from a
//! Standard MIDI File, where strings and frets are chosen with the same placement the fretboard uses.

pub mod midi_import;

use riffology_core::{place_on_strings, Tuning};
use serde::{Deserialize, Serialize};

pub use midi_import::{import_midi, MidiImportOptions};

/// Notes starting this close together are one chord when strings are chosen.
pub const CHORD_WINDOW_S: f64 = 0.03;

/// One note of a chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartNote {
    /// Start time in seconds from the start of the song.
    pub time_s: f64,
    /// How long it is held, in seconds.
    pub length_s: f64,
    /// MIDI pitch.
    pub midi: i32,
    /// String, 1 the highest. None until `assign_strings` runs or the source gives it.
    #[serde(default)]
    pub string: Option<i32>,
    /// Fret, 0 open.
    #[serde(default)]
    pub fret: Option<i32>,
}

/// A named part of a song, for practice loops and the game's progress bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub name: String,
    pub start_s: f64,
}

/// A song chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chart {
    /// The format version, 1.
    pub version: u32,
    pub title: String,
    #[serde(default)]
    pub artist: String,
    pub tuning: Tuning,
    /// The first tempo, for drawing beat lines. Note times are already in seconds.
    pub tempo_bpm: f64,
    /// Notes sorted by start time.
    pub notes: Vec<ChartNote>,
    #[serde(default)]
    pub sections: Vec<Section>,
}

impl Chart {
    /// An empty chart.
    /// @param title - the song title
    /// @param tuning - the tuning it is played in
    pub fn new(title: &str, tuning: Tuning) -> Chart {
        Chart {
            version: 1,
            title: title.to_string(),
            artist: String::new(),
            tuning,
            tempo_bpm: 120.0,
            notes: vec![],
            sections: vec![],
        }
    }

    /// Reads a chart from JSON text.
    /// @param text - the JSON
    pub fn from_json(text: &str) -> Result<Chart, String> {
        let mut c: Chart = serde_json::from_str(text).map_err(|e| format!("chart: {e}"))?;
        if c.version != 1 {
            return Err(format!("chart: version {} is not supported", c.version));
        }
        c.sort();
        Ok(c)
    }

    /// The chart as pretty JSON text.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a chart serializes")
    }

    /// Sorts the notes by time, then pitch.
    pub fn sort(&mut self) {
        self.notes.sort_by(|a, b| {
            a.time_s.partial_cmp(&b.time_s).unwrap_or(std::cmp::Ordering::Equal).then(a.midi.cmp(&b.midi))
        });
    }

    /// The song's length in seconds: the end of its last note.
    pub fn duration_s(&self) -> f64 {
        self.notes.iter().map(|n| n.time_s + n.length_s).fold(0.0, f64::max)
    }

    /// The notes that start in `[from_s, to_s)`.
    /// @param from_s - the start of the window
    /// @param to_s - the end of the window
    pub fn notes_between(&self, from_s: f64, to_s: f64) -> impl Iterator<Item = &ChartNote> {
        self.notes.iter().filter(move |n| n.time_s >= from_s && n.time_s < to_s)
    }

    /// The notes sounding at a moment.
    /// @param t - the moment in seconds
    pub fn sounding_at(&self, t: f64) -> impl Iterator<Item = &ChartNote> {
        self.notes.iter().filter(move |n| n.time_s <= t && t < n.time_s + n.length_s.max(0.05))
    }

    /// Chooses a string and fret for every note that has none: notes starting within `CHORD_WINDOW_S` are a chord and
    /// get different strings. Notes no string reaches keep no string.
    pub fn assign_strings(&mut self) {
        self.sort();
        let mut i = 0;
        while i < self.notes.len() {
            let start = self.notes[i].time_s;
            let end = self.notes[i..]
                .iter()
                .position(|n| n.time_s - start > CHORD_WINDOW_S)
                .map_or(self.notes.len(), |p| i + p);
            self.place_group(i, end);
            i = end;
        }
    }

    /// Places one chord's notes that have no string.
    /// @param from - first note index
    /// @param to - one past the last
    fn place_group(&mut self, from: usize, to: usize) {
        let taken: Vec<i32> = self.notes[from..to].iter().filter_map(|n| n.string).collect();
        let free: Vec<i32> = self.notes[from..to].iter().filter(|n| n.string.is_none()).map(|n| n.midi).collect();
        for lit in place_on_strings(&free, &self.tuning, &taken) {
            if let Some(n) = self.notes[from..to].iter_mut().find(|n| n.string.is_none() && n.midi == lit.midi) {
                n.string = Some(lit.string);
                n.fret = Some(lit.fret);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(t: f64, midi: i32) -> ChartNote {
        ChartNote { time_s: t, length_s: 0.5, midi, string: None, fret: None }
    }

    #[test]
    fn a_chart_round_trips_through_json() {
        let mut c = Chart::new("Test", Tuning::standard());
        c.notes = vec![note(1.0, 64), note(0.5, 40)];
        let back = Chart::from_json(&c.to_json()).unwrap();
        assert_eq!(back.notes[0].midi, 40, "loading sorts by time");
        assert_eq!(back.notes.len(), 2);
        assert!(Chart::from_json(&c.to_json().replace("\"version\": 1", "\"version\": 9")).is_err());
    }

    #[test]
    fn a_chord_gets_one_string_per_note() {
        let mut c = Chart::new("Chord", Tuning::standard());
        c.notes = vec![note(0.0, 48), note(0.01, 52), note(0.02, 55), note(0.0, 60), note(0.0, 64), note(1.0, 64)];
        c.assign_strings();
        let strings: Vec<_> = c.notes.iter().take(5).map(|n| (n.midi, n.string.unwrap(), n.fret.unwrap())).collect();
        assert_eq!(strings, vec![(48, 5, 3), (60, 2, 1), (64, 1, 0), (52, 4, 2), (55, 3, 0)]);
        assert_eq!(c.notes[5].string, Some(1));
        assert_eq!(c.sounding_at(0.2).count(), 5);
        assert_eq!(c.notes_between(0.5, 2.0).count(), 1);
    }
}
