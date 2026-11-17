//! Riffology's inputs. Every input writes [`riffology_core::NoteEvent`]s to the note bus, so the rest of the game
//! does not know or care whether a note came from a guitar, a MIDI device or a replayed file.
//!
//! - [`abi`]: loads the guitar model library (`chordical_guitar_abi`), checks its ABI version and SHA-256.
//! - [`tracker`]: a safe handle on one tracker in that library.
//! - [`pipeline`]: what the audio thread does with each block: push to the model, drain notes, write the bus.
//! - [`audio`]: live audio input through cpal.
//! - [`replay`]: a WAV file played through the same pipeline in real time, for tests and for working without a
//!   guitar.
//! - [`midi`]: MIDI input through midir.
//! - [`wav`]: a small WAV reader.

pub mod abi;
pub mod audio;
pub mod midi;
pub mod pipeline;
pub mod priority;
pub mod replay;
pub mod stats;
pub mod tracker;
pub mod wav;

pub use abi::{AbiError, GuitarAbi};
pub use pipeline::{InputPipeline, PipelineStats};
pub use tracker::{ModelTracker, TrackerSettings};
