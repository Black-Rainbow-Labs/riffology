//! Standard MIDI File import: the note ons and offs of one track (or every track) become chart notes, with ticks
//! turned into seconds through the file's tempo changes. Strings and frets are then chosen by `Chart::assign_strings`.

use crate::{Chart, ChartNote};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use riffology_core::Tuning;

/// What to import.
#[derive(Debug, Clone, Default)]
pub struct MidiImportOptions {
    /// The track to read, 0 the first; None reads every track.
    pub track: Option<usize>,
    /// A MIDI channel (0 to 15) to keep; None keeps every channel except 10 (index 9, drums).
    pub channel: Option<u8>,
}

/// One tempo change: from `tick` on, a quarter note lasts `us_per_quarter` microseconds.
#[derive(Debug, Clone, Copy)]
struct TempoChange {
    tick: u64,
    us_per_quarter: u32,
}

/// Imports a MIDI file into a chart with strings assigned.
/// @param bytes - the file contents
/// @param title - the chart title
/// @param tuning - the tuning to place notes in
/// @param opts - track and channel choice
pub fn import_midi(bytes: &[u8], title: &str, tuning: Tuning, opts: &MidiImportOptions) -> Result<Chart, String> {
    let smf = Smf::parse(bytes).map_err(|e| format!("MIDI file: {e}"))?;
    let tpq = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as u64,
        Timing::Timecode(..) => return Err("MIDI file: timecode timing is not supported".into()),
    };
    let tempos = tempo_map(&smf);
    let mut chart = Chart::new(title, tuning);
    chart.tempo_bpm = 60_000_000.0 / tempos[0].us_per_quarter as f64;
    for (i, track) in smf.tracks.iter().enumerate() {
        if opts.track.is_some_and(|t| t != i) {
            continue;
        }
        chart.notes.extend(track_notes(track, tpq, &tempos, opts.channel));
    }
    chart.assign_strings();
    Ok(chart)
}

/// The tempo changes of every track, sorted, with 120 BPM at tick 0 when the file sets none.
/// @param smf - the parsed file
fn tempo_map(smf: &Smf) -> Vec<TempoChange> {
    let mut out = vec![];
    for track in &smf.tracks {
        let mut tick = 0u64;
        for e in track {
            tick += e.delta.as_int() as u64;
            if let TrackEventKind::Meta(MetaMessage::Tempo(t)) = e.kind {
                out.push(TempoChange { tick, us_per_quarter: t.as_int() });
            }
        }
    }
    out.sort_by_key(|t| t.tick);
    if out.first().is_none_or(|t| t.tick > 0) {
        out.insert(0, TempoChange { tick: 0, us_per_quarter: 500_000 });
    }
    out
}

/// Seconds at a tick under the tempo map.
/// @param tick - the tick
/// @param tpq - ticks per quarter note
/// @param tempos - the tempo map
fn seconds_at(tick: u64, tpq: u64, tempos: &[TempoChange]) -> f64 {
    let mut s = 0.0;
    for (i, t) in tempos.iter().enumerate() {
        let end = tempos.get(i + 1).map_or(u64::MAX, |n| n.tick);
        if tick <= t.tick {
            break;
        }
        let span = tick.min(end) - t.tick;
        s += span as f64 * t.us_per_quarter as f64 / 1e6 / tpq as f64;
    }
    s
}

/// The notes of one track: each note on is paired with the next note off of the same pitch and channel.
/// @param track - the track's events
/// @param tpq - ticks per quarter note
/// @param tempos - the tempo map
/// @param channel - the channel to keep, or None for all but drums
fn track_notes(track: &[midly::TrackEvent], tpq: u64, tempos: &[TempoChange], channel: Option<u8>) -> Vec<ChartNote> {
    let mut open: Vec<(u8, u8, u64)> = vec![];
    let mut out = vec![];
    let mut tick = 0u64;
    for e in track {
        tick += e.delta.as_int() as u64;
        let TrackEventKind::Midi { channel: ch, message } = e.kind else { continue };
        let ch = ch.as_int();
        if channel.map_or(ch == 9, |c| c != ch) {
            continue;
        }
        let (key, on) = match message {
            MidiMessage::NoteOn { key, vel } => (key.as_int(), vel.as_int() > 0),
            MidiMessage::NoteOff { key, .. } => (key.as_int(), false),
            _ => continue,
        };
        if on {
            open.push((ch, key, tick));
        } else if let Some(i) = open.iter().position(|(c, k, _)| *c == ch && *k == key) {
            let (_, k, start) = open.remove(i);
            let t0 = seconds_at(start, tpq, tempos);
            out.push(ChartNote {
                time_s: t0,
                length_s: seconds_at(tick, tpq, tempos) - t0,
                midi: k as i32,
                string: None,
                fret: None,
            });
        }
    }
    out
}
