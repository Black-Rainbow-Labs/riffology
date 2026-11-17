//! One note on or note off from any input. Both times are seconds on the audio clock: input samples counted since
//! the input stream started, divided by its rate. A game judges timing with `onset_s`, so the detection delay of the
//! model is not held against the player; `heard_s - onset_s` is that delay.

use serde::{Deserialize, Serialize};

/// Where a note event came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    /// The guitar pitch model on live audio input.
    Guitar,
    /// The guitar pitch model on a WAV file replayed in real time.
    Replay,
    /// A MIDI input device (a MIDI guitar or a keyboard).
    Midi,
}

/// One note on or note off.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoteEvent {
    /// MIDI note number.
    pub midi: u8,
    /// 1 to 127 for a note on, 0 for a note off.
    pub velocity: u8,
    pub is_on: bool,
    pub source: Source,
    /// When the note started, in seconds on the audio clock. For the guitar model this is the onset the model
    /// reports, which is earlier than the moment the model was sure of it.
    pub onset_s: f64,
    /// When Riffology knew about the event, in seconds on the audio clock: the end of the input block whose drain
    /// returned it.
    pub heard_s: f64,
}

impl NoteEvent {
    /// The detection delay in seconds: how long after the note started Riffology knew about it.
    pub fn detection_delay_s(&self) -> f64 {
        (self.heard_s - self.onset_s).max(0.0)
    }
}
