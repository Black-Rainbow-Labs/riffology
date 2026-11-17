//! Where on the neck heard pitches are most likely played. The model hears pitches, not strings, so the fretboard
//! places them: highest pitch first, each on the free string where its fret is lowest. Ported from Chordical Sheet
//! Music's fretboard (`place_on_strings` and `settle_shown`).

use crate::pitch::{fret_for_pitch, Tuning, MAX_FRET};
use serde::{Deserialize, Serialize};

/// A lit cell: a string (1 the highest), a fret (0 open) and the pitch it sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LitFret {
    pub string: i32,
    pub fret: i32,
    pub midi: i32,
}

/// Places pitches on strings: highest pitch first, each on the free string where its fret is lowest. A pitch no free
/// string reaches is left out. The result is sorted lowest pitch first.
/// @param midis - the pitches
/// @param tuning - the tuning
/// @param taken - strings already used, for example by notes whose string a chart gives
pub fn place_on_strings(midis: &[i32], tuning: &Tuning, taken: &[i32]) -> Vec<LitFret> {
    let mut used: Vec<i32> = taken.to_vec();
    let mut sorted = midis.to_vec();
    sorted.sort_by(|a, b| b.cmp(a));
    sorted.dedup();
    let mut out = Vec::new();
    for midi in sorted {
        let best = (1..=tuning.string_count() as i32)
            .filter(|s| !used.contains(s))
            .filter_map(|s| {
                fret_for_pitch(tuning, s, midi).filter(|f| (0..=MAX_FRET).contains(f)).map(|f| LitFret {
                    string: s,
                    fret: f,
                    midi,
                })
            })
            .min_by_key(|l| l.fret);
        if let Some(b) = best {
            used.push(b.string);
            out.push(b);
        }
    }
    out.sort_by_key(|l| l.midi);
    out
}

/// What the fretboard shows, and a new strum waiting one model hop before it is shown.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShownFrets {
    pub shown: Vec<LitFret>,
    pub pending: Option<Vec<LitFret>>,
}

/// What the fretboard shows after an update. A set that shares no pitch with what is shown starts a new strum, and
/// its first update often has only some of its strings, which placed alone land somewhere else. So it waits one
/// update and the next set is shown, whatever it is. Silence is shown at once.
/// @param s - what was shown and what waits
/// @param next - the placed frets of the new update
pub fn settle_shown(s: &ShownFrets, next: Vec<LitFret>) -> ShownFrets {
    if next.is_empty() || s.pending.is_some() || s.shown.is_empty() {
        return ShownFrets { shown: next, pending: None };
    }
    let shared = next.iter().any(|f| s.shown.iter().any(|g| g.midi == f.midi));
    if shared {
        ShownFrets { shown: next, pending: None }
    } else {
        ShownFrets { shown: s.shown.clone(), pending: Some(next) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(frets: &[LitFret]) -> Vec<String> {
        frets.iter().map(|f| format!("{}-{}", f.string, f.fret)).collect()
    }

    #[test]
    fn an_open_c_chord_lands_on_its_usual_frets() {
        let frets = place_on_strings(&[48, 52, 55, 60, 64], &Tuning::standard(), &[]);
        assert_eq!(cells(&frets), ["5-3", "4-2", "3-0", "2-1", "1-0"]);
    }

    #[test]
    fn a_taken_string_is_skipped_and_an_unreachable_pitch_left_out() {
        let frets = place_on_strings(&[64], &Tuning::standard(), &[1]);
        assert_eq!(cells(&frets), ["2-5"]);
        assert!(place_on_strings(&[20], &Tuning::standard(), &[]).is_empty());
    }

    #[test]
    fn a_new_strum_waits_one_update_and_silence_is_shown_at_once() {
        let f = |string, fret, midi| LitFret { string, fret, midi };
        let mut s = settle_shown(&ShownFrets::default(), vec![f(5, 3, 48)]);
        assert_eq!(s.shown, vec![f(5, 3, 48)], "the first notes after silence show at once");
        s = settle_shown(&s, vec![f(3, 0, 55)]);
        assert_eq!(s.shown, vec![f(5, 3, 48)], "a set sharing no pitch waits");
        s = settle_shown(&s, vec![f(3, 0, 55), f(1, 0, 64)]);
        assert_eq!(s.shown.len(), 2, "the next update is shown");
        s = settle_shown(&s, vec![]);
        assert!(s.shown.is_empty());
    }
}
