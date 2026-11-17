# The note API

This page is for whoever builds the game engine. It describes what the engine reads every frame: the note events,
the clock they are on, the player state, and the `Game` trait that plugs the engine into the window.

## The Game trait

```rust
pub trait Game {
    fn name(&self) -> &str;
    fn update(&mut self, ctx: &GameContext<'_>);          // once per frame, after the frame's notes are applied
    fn marks(&self) -> Vec<FretMark>;                       // what the fretboard rings for this game
    fn draw(&mut self, rux: &mut rux::Rux<'_>, rect: egui::Rect, ctx: &GameContext<'_>);
}

pub struct GameContext<'a> {
    pub clock_s: f64,                // the audio clock now
    pub player: &'a PlayerState,     // what the player is playing
    pub tuning: &'a Tuning,          // the fretboard's tuning
}
```

The app calls `update` and then `draw` once per frame. `crates/riffology-game/src/chart_follow.rs` is a complete
small game in about 100 lines: it starts a song clock, rings the chart's current notes on the fretboard, and counts a
hit when the player starts the right pitch within 100 ms. Start from it.

## NoteEvent

```rust
pub struct NoteEvent {
    pub midi: u8,          // MIDI note number
    pub velocity: u8,      // 1 to 127 for a note on, 0 for a note off
    pub is_on: bool,
    pub source: Source,    // Guitar, Replay or Midi
    pub onset_s: f64,      // when the note started, audio clock
    pub heard_s: f64,      // when Riffology knew about it, audio clock
}
```

| Field | Use it for |
|---|---|
| `onset_s` | Judging timing. For the guitar it is the onset the model reports, which is earlier than the moment the model was sure |
| `heard_s` | Knowing when the event became available. `heard_s - onset_s` is the detection delay |
| `source` | Telling the guitar from a MIDI keyboard when both are open |

A note off has `onset_s == heard_s`. Note ends from the model are approximate.

## The audio clock

The clock is seconds of input processed: the pipeline's sample count divided by the sample rate. `GameContext::clock_s`
reads it every frame, and every `onset_s` and `heard_s` is on it. A song position computed as
`clock_s - song_start_s` is therefore directly comparable with `onset_s - song_start_s`.

To judge a note, compare the chart time with `onset_s`, never with the frame time. The frame a note arrives in is
about 20 to 30 ms after the player started it, because the model needs that long to be sure. Judging by arrival would
mark every note late.

The audio clock does not include the output latency of the song's backing track. When the engine plays audio, it
must add the output device's latency to the song position. A calibration screen that measures it belongs to the
engine.

## PlayerState

```rust
impl PlayerState {
    pub fn sounding(&self) -> &[Sounding];                               // notes sounding now, lowest first
    pub fn is_sounding(&self, midi: u8) -> bool;
    pub fn started_since(&self, t: f64) -> impl Iterator<Item = &NoteEvent>;  // note ons with onset_s >= t
    pub fn frets(&self) -> &[LitFret];                                  // where the sounding notes are drawn
    pub fn recent_delays(&self) -> Vec<f64>;                            // detection delays of recent note ons
    pub fn total_note_ons(&self) -> u64;
}
```

`started_since` keeps the last 512 note ons. A game that judges every frame only needs the last second.

## What the model can and cannot tell you

| It reports | It does not report |
|---|---|
| Pitches from MIDI 36 to 90 (C2 to F#6), several at once | The string or fret. Placement chooses one ([The fretboard](fretboard.md)) |
| Note starts, with an onset time | Pitch bends and vibrato. Notes are whole semitones |
| Velocity from the input peak | Hammer ons, pull offs and slides as such. They are missed more often than picked notes |
| Note ends, approximately | Exact note lengths |

Chords score about 6 points of onset F1 below single notes, and high gain distortion and direct input electric guitar
score below acoustic guitar. A game should be forgiving where the model is weak: judge chords by most of their
notes, and judge sustain loosely.

## Where to go next

[Chart format](chart-format.md) for the songs. [Latency](latency.md) for the numbers behind the clock.
