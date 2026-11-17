//! The inputs the window is listening to: the guitar (live audio or a replayed file) on one note bus and an optional
//! MIDI input on another. Both buses are drained every frame into the one player state.

use crate::args::Args;
use riffology_core::{note_bus, BusReader, NoteEvent, Source};
use riffology_input::audio::{device_rate, AudioInput, InputChoice};
use riffology_input::midi::MidiInput;
use riffology_input::replay::{Replay, ReplayOptions};
use riffology_input::{wav, GuitarAbi, InputPipeline, ModelTracker, PipelineStats};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

/// What keeps the guitar input running.
enum GuitarInput {
    Live(AudioInput),
    Replay(Replay),
}

/// The running inputs.
pub struct Session {
    guitar: Option<(GuitarInput, BusReader, Arc<PipelineStats>)>,
    midi: Option<(MidiInput, BusReader)>,
    /// Why the guitar input is not running, when it is not.
    pub guitar_error: Option<String>,
    pub midi_error: Option<String>,
    /// What the guitar input is: the device name or the replayed file.
    pub guitar_label: String,
    pub model_version: String,
    /// When the replay's first sample is played, as Unix milliseconds, for lining a recording up with the file.
    pub replay_started_unix_ms: Option<u128>,
    origin: Instant,
}

impl Session {
    /// Starts the inputs the arguments ask for.
    /// @param args - the options
    /// @param wake - called from the input threads when notes arrive, to redraw the window
    pub fn start(args: &Args, wake: impl Fn() + Send + Sync + Clone + 'static) -> Session {
        let origin = Instant::now();
        let mut s = Session {
            guitar: None,
            midi: None,
            guitar_error: None,
            midi_error: None,
            guitar_label: String::new(),
            model_version: String::new(),
            replay_started_unix_ms: None,
            origin,
        };
        if args.replay.is_some() {
            let now =
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
            s.replay_started_unix_ms = Some(now + (args.replay_delay * 1000.0) as u128);
        }
        match start_guitar(args, wake.clone()) {
            Ok((input, reader, stats, label, version)) => {
                s.guitar = Some((input, reader, stats));
                s.guitar_label = label;
                s.model_version = version;
            }
            Err(e) => s.guitar_error = Some(e),
        }
        if let Some(name) = &args.midi {
            let (mut w, r) = note_bus();
            w.set_wake(Box::new(wake.clone()));
            match MidiInput::open(if name.is_empty() { None } else { Some(name) }, w, origin) {
                Ok(m) => s.midi = Some((m, r)),
                Err(e) => s.midi_error = Some(e),
            }
        }
        s
    }

    /// The audio clock in seconds: samples the guitar input has processed, or wall time when there is none.
    pub fn clock_s(&self) -> f64 {
        match &self.guitar {
            Some((_, _, st)) => {
                st.samples.load(Ordering::Relaxed) as f64 / PipelineStats::f32_of(&st.rate).max(1.0) as f64
            }
            None => self.origin.elapsed().as_secs_f64(),
        }
    }

    /// Moves every waiting event from both buses into `out`.
    /// @param out - receives the events
    pub fn drain(&mut self, out: &mut Vec<NoteEvent>) {
        if let Some((_, r, _)) = &mut self.guitar {
            r.drain_into(out);
        }
        if let Some((_, r)) = &mut self.midi {
            r.drain_into(out);
        }
    }

    /// The guitar pipeline's counters.
    pub fn stats(&self) -> Option<&Arc<PipelineStats>> {
        self.guitar.as_ref().map(|g| &g.2)
    }

    /// Events dropped on the buses because a ring was full.
    pub fn dropped(&self) -> u64 {
        self.guitar.as_ref().map_or(0, |g| g.1.dropped()) + self.midi.as_ref().map_or(0, |m| m.1.dropped())
    }

    /// Whether the replayed file has ended.
    pub fn replay_done(&self) -> bool {
        matches!(&self.guitar, Some((GuitarInput::Replay(r), _, _)) if r.is_done())
    }

    /// Whether the live input reported a driver error.
    pub fn input_failed(&self) -> bool {
        matches!(&self.guitar, Some((GuitarInput::Live(a), _, _)) if a.has_failed())
    }

    /// The MIDI port name, when MIDI is open.
    pub fn midi_port(&self) -> Option<&str> {
        self.midi.as_ref().map(|m| m.0.port.as_str())
    }
}

type Started = (GuitarInput, BusReader, Arc<PipelineStats>, String, String);

/// Loads the model library and starts the replay or the live input.
/// @param args - the options
/// @param wake - the redraw function
fn start_guitar(args: &Args, wake: impl Fn() + Send + Sync + 'static) -> Result<Started, String> {
    let abi = GuitarAbi::load().map_err(|e| e.to_string())?;
    let version = abi.model_version();
    let (mut w, r) = note_bus();
    w.set_wake(Box::new(wake));
    if let Some(path) = &args.replay {
        let file = wav::read_wav(path, 0)?;
        let tracker = ModelTracker::new(abi, file.rate as f64, false)?;
        let p = InputPipeline::new(tracker, w, Source::Replay);
        let stats = p.stats();
        let replay = Replay::start(
            file.samples,
            file.rate,
            p,
            ReplayOptions {
                monitor: args.monitor,
                looped: args.looped,
                start_delay: std::time::Duration::from_secs_f64(args.replay_delay.max(0.0)),
                ..Default::default()
            },
        );
        let label =
            format!("Replaying {}", path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default());
        return Ok((GuitarInput::Replay(replay), r, stats, label, version));
    }
    let choice = InputChoice { device: args.device.clone(), channel: args.channel };
    let (rate, _) = device_rate(&choice)?;
    let tracker = ModelTracker::new(abi, rate as f64, false)?;
    let p = InputPipeline::new(tracker, w, Source::Guitar);
    let stats = p.stats();
    let input = AudioInput::start(&choice, p)?;
    let label = format!("{} (channel {}, {} Hz)", input.device.name, args.channel + 1, input.device.rate);
    Ok((GuitarInput::Live(input), r, stats, label, version))
}
