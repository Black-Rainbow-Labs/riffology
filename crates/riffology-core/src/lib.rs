//! Riffology's shared types. Nothing in this crate touches audio devices, files or the screen, so every part of the
//! game can use it and every rule in it is tested without hardware.
//!
//! - [`pitch`]: MIDI pitches, note names, tunings, and the string and fret that sound a pitch.
//! - [`placement`]: where on the neck a set of heard pitches is most likely played.
//! - [`event`]: [`NoteEvent`], one note on or note off from any input, with its times on the audio clock.
//! - [`bus`]: [`NoteBus`], the lock free queue that carries note events from the audio thread to the game thread.
//! - [`player`]: [`PlayerState`], what the player is playing now, which every view reads.

pub mod bus;
pub mod event;
pub mod pitch;
pub mod placement;
pub mod player;

pub use bus::{note_bus, BusReader, BusWriter, BUS_CAPACITY};
pub use event::{NoteEvent, Source};
pub use pitch::{fret_for_pitch, lowest_position, note_name, pitch_at_fret, positions_of, Tuning, MAX_FRET};
pub use placement::{place_on_strings, settle_shown, LitFret, ShownFrets};
pub use player::{PlayerState, Sounding};
