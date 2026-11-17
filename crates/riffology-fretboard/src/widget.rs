//! The drawn fretboard: an inset rux panel holding the string names, the open string column, the nut, the fret
//! wires, the inlay dots, the strings, and a mark on every cell it is given. A heard note is a filled dot with its
//! name; an expected note is a ring; a judged note is a dot in the judgment's color. Every color comes from the rux
//! theme, so the board follows the light and dark themes.

use crate::geometry::{midi_of_cell, FretGeometry, FretMark, MarkKind, MARKER_FRETS};
use egui::{pos2, vec2, Color32, Rect, Stroke};
use riffology_core::{note_name, Tuning};
use rux::components::Panel;
use rux::{Rux, Style};

/// The width of the string name column.
const NAME_COLUMN: f32 = 30.0;
/// The height of the fret number row.
const NUMBER_ROW: f32 = 18.0;

/// A fretboard to draw.
pub struct Fretboard<'a> {
    tuning: &'a Tuning,
    marks: &'a [FretMark],
    frets: i32,
}

/// What a drawn frame of the fretboard shows, for tests and the trace.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FretboardOutput {
    /// Each mark that was drawn with the rectangle it was drawn in.
    pub drawn: Vec<(FretMark, Rect)>,
    /// The cell under the pointer when it was pressed this frame.
    pub pressed: Option<(i32, i32)>,
}

impl<'a> Fretboard<'a> {
    /// A board for a tuning with the marks to draw.
    /// @param tuning - the tuning
    /// @param marks - the marked cells
    pub fn new(tuning: &'a Tuning, marks: &'a [FretMark]) -> Fretboard<'a> {
        Fretboard { tuning, marks, frets: 15 }
    }

    /// The last fret shown, 12 to 24.
    /// @param frets - the fret
    pub fn frets(mut self, frets: i32) -> Fretboard<'a> {
        self.frets = frets.clamp(12, 24);
        self
    }

    /// The height the board wants for a given width.
    /// @param rux - the rux context
    pub fn preferred_height(&self, rux: &Rux<'_>) -> f32 {
        rux.z(30.0) * self.tuning.string_count() as f32 + rux.z(NUMBER_ROW) + rux.z(28.0)
    }

    /// Draws the board in a rectangle.
    /// @param rux - the rux context
    /// @param rect - where
    pub fn show(self, rux: &mut Rux<'_>, rect: Rect) -> FretboardOutput {
        let inside = Panel::new().show(rux, rect);
        let names = Rect::from_min_max(
            inside.min,
            pos2(inside.left() + rux.z(NAME_COLUMN), inside.bottom() - rux.z(NUMBER_ROW)),
        );
        let area = Rect::from_min_max(pos2(names.right(), inside.top()), pos2(inside.right(), names.bottom()));
        let g = FretGeometry::new(self.tuning, area, self.frets);
        draw_board(rux, &g, self.tuning);
        draw_fret_numbers(rux, &g, inside.bottom());
        draw_string_names(rux, &g, names, self.tuning);
        let drawn = self.marks.iter().filter_map(|m| g.cell(m.string, m.fret).map(|c| (*m, c))).collect::<Vec<_>>();
        for (m, cell) in &drawn {
            draw_mark(rux, &g, *m, *cell, self.tuning);
        }
        let resp = rux.ui.interact(area, rux.ui.id().with("fretboard"), egui::Sense::click());
        let pressed = if resp.clicked() { resp.interact_pointer_pos().and_then(|p| g.hit(p)) } else { None };
        FretboardOutput { drawn, pressed }
    }
}

/// The wires, the nut, the inlays and the strings.
/// @param rux - the rux context
/// @param g - the geometry
/// @param tuning - the tuning
fn draw_board(rux: &Rux<'_>, g: &FretGeometry, tuning: &Tuning) {
    let t = rux.theme();
    let p = rux.painter();
    let (top, bottom) = (g.area.top(), g.area.bottom());
    let mid = (g.string_y(g.strings as i32 / 2) + g.string_y(g.strings as i32 / 2 + 1)) / 2.0;
    for f in MARKER_FRETS.iter().copied().filter(|f| *f <= g.frets) {
        let x = (g.wire_x(f - 1) + g.wire_x(f)) / 2.0;
        let r = (g.fret_width().min(g.string_height()) * 0.16).max(2.0);
        if f % 12 == 0 {
            p.circle_filled(pos2(x, g.string_y(2) + g.string_height() / 2.0), r, t.ink.i200);
            p.circle_filled(pos2(x, g.string_y(g.strings as i32 - 2) + g.string_height() / 2.0), r, t.ink.i200);
        } else {
            p.circle_filled(pos2(x, mid), r, t.ink.i200);
        }
    }
    for f in 1..=g.frets {
        p.line_segment([pos2(g.wire_x(f), top), pos2(g.wire_x(f), bottom)], Stroke::new(rux.z(2.0), t.ink.i300));
    }
    p.line_segment([pos2(g.nut_x(), top), pos2(g.nut_x(), bottom)], Stroke::new(rux.z(5.0), t.ink.i700));
    for s in 1..=tuning.string_count() as i32 {
        let width = rux.z(1.0 + 0.35 * (s - 1) as f32);
        p.line_segment(
            [pos2(g.area.left(), g.string_y(s)), pos2(g.area.right(), g.string_y(s))],
            Stroke::new(width, t.ink.i500),
        );
    }
}

/// The fret numbers under the board.
/// @param rux - the rux context
/// @param g - the geometry
/// @param bottom - the bottom of the panel's inside
fn draw_fret_numbers(rux: &Rux<'_>, g: &FretGeometry, bottom: f32) {
    let ink = rux.theme().ink.i400;
    for f in 0..=g.frets {
        let Some(c) = g.cell(1, f) else { continue };
        let r = Rect::from_min_max(pos2(c.left(), bottom - rux.z(NUMBER_ROW)), pos2(c.right(), bottom));
        let galley = rux::text::layout(rux.painter(), rux.zs(Style::CAPTION), &f.to_string(), ink);
        rux::text::draw_centred(rux.painter(), r, galley, ink);
    }
}

/// The open string names left of the board.
/// @param rux - the rux context
/// @param g - the geometry
/// @param names - the name column
/// @param tuning - the tuning
fn draw_string_names(rux: &Rux<'_>, g: &FretGeometry, names: Rect, tuning: &Tuning) {
    let ink = rux.theme().ink.i500;
    for s in 1..=tuning.string_count() as i32 {
        let Some(midi) = midi_of_cell(tuning, s, 0) else { continue };
        let r = Rect::from_center_size(pos2(names.center().x, g.string_y(s)), vec2(names.width(), g.string_height()));
        let galley = rux::text::layout(rux.painter(), rux.zs(Style::NUMBER), &note_name(midi), ink);
        rux::text::draw_centred(rux.painter(), r, galley, ink);
    }
}

/// The color of a mark.
/// @param rux - the rux context
/// @param kind - the mark's meaning
fn mark_color(rux: &Rux<'_>, kind: MarkKind) -> Color32 {
    let t = rux.theme();
    match kind {
        MarkKind::Heard => t.accent.blue,
        MarkKind::Expected => t.accent.violet,
        MarkKind::Good => t.semantic.success,
        MarkKind::Near => t.semantic.warning,
        MarkKind::Bad => t.semantic.danger,
    }
}

/// One mark: a ring for an expected note, otherwise a dot with the note's name.
/// @param rux - the rux context
/// @param g - the geometry
/// @param m - the mark
/// @param cell - its cell
/// @param tuning - the tuning, to name the note
fn draw_mark(rux: &Rux<'_>, g: &FretGeometry, m: FretMark, cell: Rect, tuning: &Tuning) {
    let p = rux.painter();
    let color = mark_color(rux, m.kind);
    let c = pos2(cell.center().x, g.string_y(m.string));
    let r = (g.string_height() * 0.42).min(g.fret_width() * 0.45).max(rux.z(6.0));
    if m.kind == MarkKind::Expected {
        p.circle_stroke(c, r + rux.z(2.0), Stroke::new(rux.z(3.0), color));
        return;
    }
    p.circle_filled(c, r, color);
    if let Some(midi) = midi_of_cell(tuning, m.string, m.fret) {
        let ink = rux.theme().surface.s0;
        let galley = rux::text::layout(p, rux.zs(Style::CAPTION).at(r * 0.85), &note_name(midi), ink);
        rux::text::draw_centred(p, Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)), galley, ink);
    }
}
