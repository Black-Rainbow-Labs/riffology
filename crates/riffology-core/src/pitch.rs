//! MIDI pitches, note names and tunings. String 1 is the highest string, fret 0 is the open string, and a capo
//! raises every string. Ported from the guitar model of Chordical Sheet Music.

use serde::{Deserialize, Serialize};

/// The highest fret Riffology places a note on.
pub const MAX_FRET: i32 = 24;

/// The open string pitches of a fretted instrument, string 1 (the highest) first, and the capo fret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tuning {
    /// MIDI pitch of each open string, string 1 first.
    pub strings: Vec<i32>,
    /// The fret the capo is on, 0 for none.
    #[serde(default)]
    pub capo: i32,
    /// A name to show, for example "Standard".
    #[serde(default)]
    pub name: Option<String>,
}

/// Named tunings, string 1 first.
pub const NAMED_TUNINGS: [(&str, &[i32]); 7] = [
    ("Standard", &[64, 59, 55, 50, 45, 40]),
    ("Drop D", &[64, 59, 55, 50, 45, 38]),
    ("DADGAD", &[62, 57, 55, 50, 45, 38]),
    ("Open G", &[62, 59, 55, 50, 43, 38]),
    ("Open D", &[62, 57, 54, 50, 45, 38]),
    ("Half step down", &[63, 58, 54, 49, 44, 39]),
    ("Bass standard", &[43, 38, 33, 28]),
];

impl Tuning {
    /// Standard six string guitar tuning, E2 A2 D3 G3 B3 E4.
    pub fn standard() -> Tuning {
        Tuning::named("Standard").expect("standard tuning")
    }

    /// One of [`NAMED_TUNINGS`] by name, ignoring case.
    /// @param name - the tuning's name
    pub fn named(name: &str) -> Option<Tuning> {
        NAMED_TUNINGS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(n, s)| Tuning {
            strings: s.to_vec(),
            capo: 0,
            name: Some(n.to_string()),
        })
    }

    /// The number of strings.
    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    /// The lowest and highest pitch the tuning plays up to [`MAX_FRET`], capo included.
    pub fn range(&self) -> (i32, i32) {
        let low = self.strings.iter().min().copied().unwrap_or(40) + self.capo;
        let high = self.strings.iter().max().copied().unwrap_or(64) + self.capo + MAX_FRET;
        (low, high)
    }
}

/// The open pitch of a 1 based string, or None when the tuning has no such string.
/// @param tuning - the tuning
/// @param string - the string, 1 the highest
fn open_string(tuning: &Tuning, string: i32) -> Option<i32> {
    if string >= 1 {
        tuning.strings.get((string - 1) as usize).copied()
    } else {
        None
    }
}

/// The MIDI pitch a string and fret sound, capo included, or None for a string the tuning does not have.
/// @param tuning - the tuning
/// @param string - the string, 1 the highest
/// @param fret - the fret, 0 open
pub fn pitch_at_fret(tuning: &Tuning, string: i32, fret: i32) -> Option<i32> {
    open_string(tuning, string).map(|open| open + tuning.capo + fret)
}

/// The fret that sounds a pitch on a string. It may be negative or above the last fret; callers check the range.
/// @param tuning - the tuning
/// @param string - the string, 1 the highest
/// @param midi - the pitch
pub fn fret_for_pitch(tuning: &Tuning, string: i32, midi: i32) -> Option<i32> {
    open_string(tuning, string).map(|open| midi - open - tuning.capo)
}

/// Every string and fret that sounds a pitch with the fret between 0 and `max_fret`, string 1 first.
/// @param tuning - the tuning
/// @param midi - the pitch
/// @param max_fret - the highest fret to consider
pub fn positions_of(tuning: &Tuning, midi: i32, max_fret: i32) -> Vec<(i32, i32)> {
    (1..=tuning.strings.len() as i32)
        .filter_map(|s| fret_for_pitch(tuning, s, midi).filter(|f| (0..=max_fret).contains(f)).map(|f| (s, f)))
        .collect()
}

/// The position of a pitch with the lowest fret, or None when no string reaches it.
/// @param tuning - the tuning
/// @param midi - the pitch
/// @param max_fret - the highest fret to consider
pub fn lowest_position(tuning: &Tuning, midi: i32, max_fret: i32) -> Option<(i32, i32)> {
    positions_of(tuning, midi, max_fret).into_iter().min_by_key(|(_, f)| *f)
}

/// The name of a MIDI pitch with sharps and its octave, for example 40 is "E2" and 61 is "C#4".
/// @param midi - the pitch
pub fn note_name(midi: i32) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[midi.rem_euclid(12) as usize], midi.div_euclid(12) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_and_frets_sound_the_pitches_a_guitarist_expects() {
        let t = Tuning::standard();
        assert_eq!(pitch_at_fret(&t, 6, 0), Some(40));
        assert_eq!(pitch_at_fret(&t, 3, 5), Some(60));
        assert_eq!(pitch_at_fret(&t, 7, 0), None);
        assert_eq!(fret_for_pitch(&t, 1, 69), Some(5));
        let capo = Tuning { capo: 2, ..Tuning::standard() };
        assert_eq!(pitch_at_fret(&capo, 6, 0), Some(42));
    }

    #[test]
    fn a_pitch_has_every_position_and_one_lowest() {
        let t = Tuning::standard();
        assert_eq!(positions_of(&t, 64, 24), vec![(1, 0), (2, 5), (3, 9), (4, 14), (5, 19), (6, 24)]);
        assert_eq!(lowest_position(&t, 64, 24), Some((1, 0)));
        assert_eq!(lowest_position(&t, 30, 24), None);
    }

    #[test]
    fn names_and_ranges() {
        assert_eq!(note_name(40), "E2");
        assert_eq!(note_name(61), "C#4");
        assert_eq!(Tuning::standard().range(), (40, 88));
        assert_eq!(Tuning::named("drop d").unwrap().strings[5], 38);
    }
}
