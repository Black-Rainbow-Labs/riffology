//! The window. Each frame: drain the note buses into the player state, update the game, draw the status line, the
//! fretboard (heard notes and the game's marks), the game, and the latency readout, then write the frame to the trace.

use crate::args::Args;
use crate::session::Session;
use crate::trace::TraceRecorder;
use egui::{pos2, Rect};
use riffology_core::{NoteEvent, PlayerState, Tuning};
use riffology_fretboard::{FretMark, Fretboard};
use riffology_game::{ChartFollow, FreePlay, Game, GameContext};
use riffology_input::stats::percentile;
use riffology_input::PipelineStats;
use riffology_song::{import_midi, Chart, MidiImportOptions};
use rux::{Flex, RuxState, Style, Theme};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// The app state.
pub struct RiffologyApp {
    rux: RuxState,
    page: Page,
}

/// Everything the page draws and keeps between frames.
struct Page {
    args: Args,
    session: Session,
    player: PlayerState,
    tuning: Tuning,
    game: Box<dyn Game>,
    chart_error: Option<String>,
    events: Vec<NoteEvent>,
    trace: TraceRecorder,
    frame: u64,
    opened: Instant,
    closing: bool,
}

impl RiffologyApp {
    /// Starts the inputs and the game the arguments ask for.
    /// @param ctx - the egui context
    /// @param args - the options
    pub fn new(ctx: &egui::Context, args: Args) -> RiffologyApp {
        rux::text::install(ctx);
        let theme = args
            .theme
            .as_deref()
            .and_then(Theme::named)
            .unwrap_or_else(|| Theme::named("dark-neumorphic").expect("dark theme"));
        let wake_ctx = ctx.clone();
        let session = Session::start(&args, move || wake_ctx.request_repaint());
        let tuning = args.tuning.as_deref().and_then(Tuning::named).unwrap_or_else(Tuning::standard);
        let (game, chart_error) = make_game(&args, &tuning);
        let page = Page {
            args,
            session,
            player: PlayerState::new(),
            tuning,
            game,
            chart_error,
            events: Vec::with_capacity(1024),
            trace: TraceRecorder::default(),
            frame: 0,
            opened: Instant::now(),
            closing: false,
        };
        RiffologyApp { rux: RuxState::new(theme), page }
    }
}

/// The game for the arguments: a chart follow game when a chart is given and loads, free play otherwise.
/// @param args - the options
/// @param tuning - the tuning
fn make_game(args: &Args, tuning: &Tuning) -> (Box<dyn Game>, Option<String>) {
    let Some(path) = &args.chart else { return (Box::new(FreePlay::default()), None) };
    match load_chart(path, tuning) {
        Ok(c) => (Box::new(ChartFollow::new(c)), None),
        Err(e) => (Box::new(FreePlay::default()), Some(e)),
    }
}

/// Loads a JSON chart or imports a MIDI file.
/// @param path - the file
/// @param tuning - the tuning for MIDI imports
fn load_chart(path: &std::path::Path, tuning: &Tuning) -> Result<Chart, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let is_midi = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mid") || e.eq_ignore_ascii_case("midi"));
    let mut chart = if is_midi {
        import_midi(&bytes, &title, tuning.clone(), &MidiImportOptions::default())?
    } else {
        Chart::from_json(&String::from_utf8_lossy(&bytes))?
    };
    chart.assign_strings();
    Ok(chart)
}

impl Page {
    /// Takes the frame's events and updates the player state and the game.
    fn update(&mut self) {
        self.events.clear();
        self.session.drain(&mut self.events);
        self.player.apply(&self.events);
        self.player.update_frets(&self.tuning);
        let ctx = GameContext { clock_s: self.session.clock_s(), player: &self.player, tuning: &self.tuning };
        self.game.update(&ctx);
    }

    /// The marks the fretboard draws: the game's, then the heard notes on top.
    fn marks(&self) -> Vec<FretMark> {
        let mut m = self.game.marks();
        m.extend(self.player.frets().iter().map(FretMark::heard));
        m
    }

    /// One frame of the page.
    /// @param rux - the rux context
    /// @param area - the window
    fn draw(&mut self, rux: &mut rux::Rux<'_>, area: Rect) {
        rux.ground(area);
        let inner = area.shrink(rux.z(24.0));
        let marks = self.marks();
        let board = Fretboard::new(&self.tuning, &marks).frets(15);
        let [head, board_rect, game_rect, foot] = Flex::column(inner)
            .gap(rux.z(16.0))
            .fixed(rux.z(56.0))
            .fixed(board.preferred_height(rux))
            .fill(1.0)
            .fixed(rux.z(22.0))
            .solve();
        self.draw_head(rux, head);
        let out = board.show(rux, board_rect);
        let ctx = GameContext { clock_s: self.session.clock_s(), player: &self.player, tuning: &self.tuning };
        self.game.draw(rux, game_rect, &ctx);
        self.draw_readout(rux, foot);
        let drawn: Vec<FretMark> = out.drawn.iter().map(|(m, _)| *m).collect();
        self.trace.frame(self.frame, self.session.clock_s(), &self.events, &drawn);
    }

    /// The title and the status line.
    /// @param rux - the rux context
    /// @param rect - the header area
    fn draw_head(&self, rux: &mut rux::Rux<'_>, rect: Rect) {
        let t = rux.theme();
        let title = rux.text(Style::H1, "Riffology", t.ink.i900);
        rux.painter().galley(rect.left_top(), title.clone(), t.ink.i900);
        let status = self.status_line();
        let g = rux.text(Style::LABEL, &status, t.ink.i500);
        rux.painter().galley(pos2(rect.left(), rect.top() + title.size().y + rux.z(2.0)), g, t.ink.i500);
    }

    /// The status: what the guitar input is, the tracker's state, the model, MIDI, and any error.
    fn status_line(&self) -> String {
        let s = &self.session;
        let mut parts = vec![];
        match (&s.guitar_error, s.stats()) {
            (Some(e), _) => parts.push(e.clone()),
            (None, Some(st)) => parts.push(format!("{}: {:?}, model {}", s.guitar_label, st.status(), s.model_version)),
            _ => {}
        }
        if s.input_failed() {
            parts.push("the audio input stopped".into());
        }
        if let Some(p) = s.midi_port() {
            parts.push(format!("MIDI {p}"));
        }
        if let Some(e) = s.midi_error.as_ref().or(self.chart_error.as_ref()) {
            parts.push(e.clone());
        }
        parts.push(format!("Game: {}", self.game.name()));
        parts.join("   |   ")
    }

    /// The latency readout under the game.
    /// @param rux - the rux context
    /// @param rect - the footer area
    fn draw_readout(&self, rux: &mut rux::Rux<'_>, rect: Rect) {
        let t = rux.theme();
        let g = rux.text(Style::NUMBER, &self.readout(), t.ink.i400);
        rux::text::draw_left_centre(rux.painter(), rect.left_center(), g, t.ink.i400);
    }

    /// The readout text: detection delay, block time, model time, input level, block size and drops.
    fn readout(&self) -> String {
        let d: Vec<f64> = self.player.recent_delays().iter().map(|x| x * 1000.0).collect();
        let delay = match (percentile(&d, 50.0), percentile(&d, 99.0)) {
            (Some(m), Some(p)) => format!("detection {m:.0} ms median, {p:.0} ms p99"),
            _ => "detection: no notes yet".into(),
        };
        let Some(st) = self.session.stats() else { return delay };
        format!(
            "{delay}   block {} samples, input code {:.2} ms max   model {:.2} ms p99   input {:.0} dBFS   notes {}   dropped {}",
            st.block_size.load(Ordering::Relaxed),
            st.max_block_ns.load(Ordering::Relaxed) as f64 / 1e6,
            PipelineStats::f32_of(&st.infer_ms_p99),
            PipelineStats::f32_of(&st.input_rms_db),
            self.player.total_note_ons(),
            self.session.dropped() + st.model_dropped_blocks.load(Ordering::Relaxed) as u64,
        )
    }

    /// Whether the window should close now: `--exit-after` has passed, or the replay ended and nothing loops.
    fn should_close(&self) -> bool {
        let timed = self.args.exit_after.is_some_and(|s| self.opened.elapsed().as_secs_f64() >= s);
        timed || (self.args.exit_after.is_none() && self.args.trace.is_some() && self.session.replay_done())
    }

    /// Writes the trace file, once.
    fn finish(&mut self) {
        if let Some(path) = self.args.trace.clone() {
            let max_ms = self.session.stats().map_or(0.0, |s| s.max_block_ns.load(Ordering::Relaxed) as f64 / 1e6);
            let (version, label, dropped) =
                (self.session.model_version.clone(), self.session.guitar_label.clone(), self.session.dropped());
            if let Err(e) =
                self.trace.write(&path, &version, &label, dropped, max_ms, self.session.replay_started_unix_ms)
            {
                eprintln!("trace {}: {e}", path.display());
            }
        }
    }
}

impl eframe::App for RiffologyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.page.update();
        let area = ui.max_rect();
        rux::layer(ui, &self.rux, egui::Id::new("riffology"), area, |rux| self.page.draw(rux, area));
        self.rux.end_frame();
        self.page.frame += 1;
        if !self.page.closing && self.page.should_close() {
            self.page.closing = true;
            self.page.finish();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }

    fn on_exit(&mut self) {
        if !self.page.closing {
            self.page.closing = true;
            self.page.finish();
        }
    }
}
