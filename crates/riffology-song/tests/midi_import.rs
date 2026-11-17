//! A MIDI file written here with a tempo change imports to notes at the right seconds, with strings chosen.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use riffology_core::Tuning;
use riffology_song::{import_midi, MidiImportOptions};

/// A note as (time, length, pitch, string, fret).
type Placed = (f64, f64, i32, Option<i32>, Option<i32>);

/// A track event.
fn ev(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
    TrackEvent { delta: u28::new(delta), kind }
}

/// A note message on channel 1.
fn note(delta: u32, key: u8, on: bool) -> TrackEvent<'static> {
    let message = if on {
        MidiMessage::NoteOn { key: u7::new(key), vel: u7::new(100) }
    } else {
        MidiMessage::NoteOff { key: u7::new(key), vel: u7::new(0) }
    };
    ev(delta, TrackEventKind::Midi { channel: u4::new(0), message })
}

#[test]
fn notes_land_at_their_seconds_through_a_tempo_change() {
    // 480 ticks a quarter. 120 BPM (0.5 s a quarter) for the first two quarters, then 60 BPM (1 s a quarter).
    let track = vec![
        ev(0, TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000)))),
        note(0, 40, true),
        note(480, 40, false),
        ev(480, TrackEventKind::Meta(MetaMessage::Tempo(u24::new(1_000_000)))),
        note(0, 64, true),
        note(0, 59, true),
        note(480, 64, false),
        note(0, 59, false),
        ev(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let smf = Smf { header: Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480))), tracks: vec![track] };
    let mut bytes = Vec::new();
    smf.write(&mut bytes).unwrap();

    let chart = import_midi(&bytes, "tempo", Tuning::standard(), &MidiImportOptions::default()).unwrap();
    let got: Vec<Placed> = chart.notes.iter().map(|n| (n.time_s, n.length_s, n.midi, n.string, n.fret)).collect();
    assert_eq!(
        got,
        vec![(0.0, 0.5, 40, Some(6), Some(0)), (1.0, 1.0, 59, Some(2), Some(0)), (1.0, 1.0, 64, Some(1), Some(0))]
    );
    assert_eq!(chart.tempo_bpm, 120.0);
    assert_eq!(chart.duration_s(), 2.0);
}
