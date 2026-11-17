//! The trace file (`--trace`): every note event the window received and every mark the fretboard drew, with the
//! frame it happened in. The end to end tests read it to check that what was played was lit, and when.

use riffology_core::NoteEvent;
use riffology_fretboard::FretMark;
use serde::{Deserialize, Serialize};

/// One frame that received events or changed what the fretboard draws.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceFrame {
    /// The frame number since the window opened.
    pub frame: u64,
    /// The audio clock when the frame was drawn.
    pub clock_s: f64,
    /// Events drained this frame.
    pub events: Vec<NoteEvent>,
    /// Every mark drawn this frame.
    pub marks: Vec<FretMark>,
}

/// The whole trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Trace {
    pub model_version: String,
    pub input: String,
    pub frames: Vec<TraceFrame>,
    /// Frames drawn in total.
    pub frame_count: u64,
    /// Events dropped on the note buses.
    pub dropped: u64,
    /// The largest time one audio block took in Riffology's input code, in milliseconds.
    pub max_block_ms: f64,
    /// When the replay's first sample was played, as Unix milliseconds.
    pub replay_started_unix_ms: Option<u128>,
}

/// Collects the trace while the window runs.
#[derive(Default)]
pub struct TraceRecorder {
    trace: Trace,
    last_marks: Vec<FretMark>,
}

impl TraceRecorder {
    /// Records a frame when it received events or drew different marks.
    /// @param frame - the frame number
    /// @param clock_s - the audio clock
    /// @param events - the events drained this frame
    /// @param marks - the marks drawn this frame
    pub fn frame(&mut self, frame: u64, clock_s: f64, events: &[NoteEvent], marks: &[FretMark]) {
        self.trace.frame_count = frame + 1;
        if events.is_empty() && marks == self.last_marks.as_slice() {
            return;
        }
        self.last_marks = marks.to_vec();
        self.trace.frames.push(TraceFrame { frame, clock_s, events: events.to_vec(), marks: marks.to_vec() });
    }

    /// Writes the trace as JSON.
    /// @param path - the file
    /// @param model_version - the model run name
    /// @param input - what the guitar input was
    /// @param dropped - bus drops
    /// @param max_block_ms - the largest block time
    /// @param replay_started_unix_ms - when the replay started
    pub fn write(
        &mut self,
        path: &std::path::Path,
        model_version: &str,
        input: &str,
        dropped: u64,
        max_block_ms: f64,
        replay_started_unix_ms: Option<u128>,
    ) -> std::io::Result<()> {
        self.trace.replay_started_unix_ms = replay_started_unix_ms;
        self.trace.model_version = model_version.to_string();
        self.trace.input = input.to_string();
        self.trace.dropped = dropped;
        self.trace.max_block_ms = max_block_ms;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string(&self.trace)?)
    }
}
