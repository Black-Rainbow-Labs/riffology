//! The game interface. The app owns the inputs, the note bus, the player state and the fretboard; a [`Game`] reads
//! them through [`GameContext`] every frame, says which cells the fretboard should ring as expected, and draws its
//! own part of the window. The game engine (the note highway, scoring, song selection) is a `Game`.
//!
//! - [`FreePlay`]: no song. Lists what the player is playing and how long detection took.
//! - [`ChartFollow`]: a minimal example of a game against a chart: expected notes ring on the fretboard, and a note
//!   played within the hit window of a chart note counts as a hit. The engine replaces it; it is here to show the
//!   API in use and is tested.

mod chart_follow;
mod free_play;

pub use chart_follow::{ChartFollow, HIT_WINDOW_S};
pub use free_play::FreePlay;

use riffology_core::{PlayerState, Tuning};
use riffology_fretboard::FretMark;

/// What a game reads each frame.
pub struct GameContext<'a> {
    /// The audio clock in seconds: the same clock every `NoteEvent` time is on.
    pub clock_s: f64,
    /// What the player is playing.
    pub player: &'a PlayerState,
    /// The tuning the fretboard shows.
    pub tuning: &'a Tuning,
}

/// A game.
pub trait Game {
    /// The name shown in the game picker.
    fn name(&self) -> &str;

    /// Called once per frame after the player state has taken the frame's note events.
    /// @param ctx - the clock, the player state and the tuning
    fn update(&mut self, ctx: &GameContext<'_>);

    /// The cells the fretboard marks for this game (expected notes, judgments). Heard notes are added by the app.
    fn marks(&self) -> Vec<FretMark>;

    /// Draws the game's part of the window.
    /// @param rux - the rux context
    /// @param rect - the game's area
    /// @param ctx - the clock, the player state and the tuning
    fn draw(&mut self, rux: &mut rux::Rux<'_>, rect: egui::Rect, ctx: &GameContext<'_>);
}
