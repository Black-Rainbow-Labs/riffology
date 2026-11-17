//! Where each string and fret is. String 1 (the highest) is the top row, as a player looking down at the neck sees
//! it in tab. Column 0 is the open string, left of the nut; columns 1 to `frets` are the frets, each the same width.

use egui::{pos2, Pos2, Rect};
use riffology_core::{pitch_at_fret, LitFret, Tuning};
use serde::{Deserialize, Serialize};

/// The frets with an inlay dot on a guitar neck. Fret 12 and 24 have two.
pub const MARKER_FRETS: [i32; 10] = [3, 5, 7, 9, 12, 15, 17, 19, 21, 24];

/// What a marked cell means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarkKind {
    /// A note the song plays now: drawn as a ring.
    Expected,
    /// A note the game judged right.
    Good,
    /// A note the game judged early, late or nearly right.
    Near,
    /// A note the game judged wrong.
    Bad,
    /// A note the player is playing: drawn as a filled dot with its name.
    Heard,
}

/// One marked cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FretMark {
    pub string: i32,
    pub fret: i32,
    pub kind: MarkKind,
}

impl FretMark {
    /// A heard mark for a lit fret.
    /// @param l - the lit fret
    pub fn heard(l: &LitFret) -> FretMark {
        FretMark { string: l.string, fret: l.fret, kind: MarkKind::Heard }
    }
}

/// The board's layout in a rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FretGeometry {
    /// The rectangle the strings and frets fill.
    pub area: Rect,
    pub strings: usize,
    /// The last fret shown.
    pub frets: i32,
    /// The open string column's width as a share of a fret's width.
    pub open_share: f32,
}

impl FretGeometry {
    /// The layout of a tuning's board in a rectangle.
    /// @param tuning - the tuning, whose string count gives the rows
    /// @param area - the rectangle
    /// @param frets - the last fret shown
    pub fn new(tuning: &Tuning, area: Rect, frets: i32) -> FretGeometry {
        FretGeometry { area, strings: tuning.string_count().max(1), frets: frets.max(1), open_share: 0.7 }
    }

    /// The width of one fret column.
    pub fn fret_width(&self) -> f32 {
        self.area.width() / (self.frets as f32 + self.open_share)
    }

    /// The height of one string row.
    pub fn string_height(&self) -> f32 {
        self.area.height() / self.strings as f32
    }

    /// The x of the nut.
    pub fn nut_x(&self) -> f32 {
        self.area.left() + self.fret_width() * self.open_share
    }

    /// The x of the fret wire at the right edge of fret `fret` (fret 0 is the nut).
    /// @param fret - the fret
    pub fn wire_x(&self, fret: i32) -> f32 {
        self.nut_x() + self.fret_width() * fret as f32
    }

    /// The y of a string's line.
    /// @param string - the string, 1 the highest
    pub fn string_y(&self, string: i32) -> f32 {
        self.area.top() + self.string_height() * (string as f32 - 0.5)
    }

    /// The rectangle of one cell, or None for a string or fret off the board.
    /// @param string - the string, 1 the highest
    /// @param fret - the fret, 0 open
    pub fn cell(&self, string: i32, fret: i32) -> Option<Rect> {
        if string < 1 || string as usize > self.strings || fret < 0 || fret > self.frets {
            return None;
        }
        let (left, right) =
            if fret == 0 { (self.area.left(), self.nut_x()) } else { (self.wire_x(fret - 1), self.wire_x(fret)) };
        let top = self.area.top() + self.string_height() * (string - 1) as f32;
        Some(Rect::from_min_max(pos2(left, top), pos2(right, top + self.string_height())))
    }

    /// The cell a point is on.
    /// @param p - the point
    pub fn hit(&self, p: Pos2) -> Option<(i32, i32)> {
        if !self.area.contains(p) {
            return None;
        }
        let string = (((p.y - self.area.top()) / self.string_height()) as i32 + 1).min(self.strings as i32);
        let fret = if p.x < self.nut_x() {
            0
        } else {
            (((p.x - self.nut_x()) / self.fret_width()) as i32 + 1).min(self.frets)
        };
        Some((string, fret))
    }
}

/// The sounding pitch of a cell under a tuning.
/// @param tuning - the tuning
/// @param string - the string
/// @param fret - the fret
pub fn midi_of_cell(tuning: &Tuning, string: i32, fret: i32) -> Option<i32> {
    pitch_at_fret(tuning, string, fret)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> FretGeometry {
        FretGeometry::new(&Tuning::standard(), Rect::from_min_max(pos2(0.0, 0.0), pos2(157.0, 60.0)), 15)
    }

    #[test]
    fn cells_are_where_strings_and_frets_cross() {
        let g = board();
        assert_eq!(g.fret_width(), 10.0);
        let open_low_e = g.cell(6, 0).unwrap();
        assert_eq!((open_low_e.left(), open_low_e.top(), open_low_e.width()), (0.0, 50.0, 7.0));
        let c = g.cell(1, 15).unwrap();
        assert_eq!((c.left(), c.right()), (147.0, 157.0));
        assert!(g.cell(7, 0).is_none() && g.cell(1, 16).is_none());
    }

    #[test]
    fn a_point_finds_its_cell_and_a_cell_its_pitch() {
        let g = board();
        assert_eq!(g.hit(g.cell(3, 5).unwrap().center()), Some((3, 5)));
        assert_eq!(g.hit(g.cell(2, 0).unwrap().center()), Some((2, 0)));
        assert_eq!(g.hit(pos2(500.0, 5.0)), None);
        assert_eq!(midi_of_cell(&Tuning::standard(), 3, 5), Some(60));
    }
}
