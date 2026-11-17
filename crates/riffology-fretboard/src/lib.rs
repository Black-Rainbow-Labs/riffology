//! The fretboard. [`geometry`] says where every string and fret is in a rectangle and which cell a point is on, with
//! no drawing in it, so it is tested without a window. [`widget`] draws the board with egui and rux theme colors and
//! lights the cells it is given: the notes the player is playing, and the notes a game expects.
//!
//! Ported from Chordical Sheet Music's fretboard (`csm-ui/src/fretboard.rs` and the fretboard pane).

pub mod geometry;
pub mod widget;

pub use geometry::{FretGeometry, FretMark, MarkKind, MARKER_FRETS};
pub use widget::{Fretboard, FretboardOutput};
