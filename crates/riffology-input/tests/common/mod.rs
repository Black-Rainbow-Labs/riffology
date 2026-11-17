//! Shared helpers for the tests that run the real guitar model library: the GuitarSet fixtures, their labels, and
//! replays through the input pipeline.
#![allow(dead_code)]

use riffology_core::{note_bus, NoteEvent, Source};
use riffology_input::replay::{Replay, ReplayOptions};
use riffology_input::{wav, GuitarAbi, InputPipeline, ModelTracker};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The GuitarSet excerpts in `tests/fixtures/guitar` (player 05, CC BY 4.0, see tests/fixtures/README.md).
pub const EXCERPTS: [&str; 3] = ["05_Rock1-130-A_solo", "05_Jazz1-130-D_solo", "05_Funk1-97-C_comp"];

/// The fixture folder.
pub fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/guitar")
}

/// The library, or a failure that says how to get it. These tests never skip: a missing library fails them.
pub fn abi() -> Arc<GuitarAbi> {
    GuitarAbi::load().unwrap_or_else(|e| panic!("{e}"))
}

/// A fixture's 48 kHz samples.
/// @param name - the excerpt
pub fn audio(name: &str) -> Vec<f32> {
    let w = wav::read_wav(&fixtures().join(format!("{name}.48k.wav")), 0).unwrap();
    assert_eq!(w.rate, 48_000);
    w.samples
}

/// A fixture's labeled notes as (onset seconds, midi).
/// @param name - the excerpt
pub fn labels(name: &str) -> Vec<(f64, f64)> {
    let text = std::fs::read_to_string(fixtures().join(format!("{name}.labels.json"))).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["notes"].as_array().unwrap().iter().map(|n| (n[0].as_f64().unwrap(), n[2].as_f64().unwrap())).collect()
}

/// Every event a replay produced, with the moment the reader received it in seconds after the replay started.
pub struct Received {
    pub events: Vec<(NoteEvent, f64)>,
    pub dropped: u64,
}

/// Replays samples through the pipeline and collects the bus on a reader thread polling every 0.25 ms.
/// @param samples - 48 kHz audio
/// @param sync - run the model inside each push (exact, as fast as possible) instead of on the worker in real time
pub fn replay(samples: &[f32], sync: bool) -> Received {
    let tracker = ModelTracker::new(abi(), 48_000.0, sync).unwrap();
    let (w, mut r) = note_bus();
    let pipeline = InputPipeline::new(tracker, w, Source::Replay);
    let opts = ReplayOptions { realtime: !sync, ..Default::default() };
    let replay = Replay::start(samples.to_vec(), 48_000, pipeline, opts);
    let started = replay.started();
    let stop = Arc::new(AtomicBool::new(false));
    let s2 = stop.clone();
    let reader = std::thread::spawn(move || {
        riffology_input::priority::raise_current_thread();
        let (mut out, mut buf) = (Vec::new(), Vec::new());
        loop {
            let done = s2.load(Ordering::Acquire);
            buf.clear();
            r.drain_into(&mut buf);
            let at = started.elapsed().as_secs_f64();
            out.extend(buf.iter().map(|e| (*e, at)));
            if done {
                return (out, r.dropped());
            }
            std::thread::sleep(Duration::from_micros(250));
        }
    });
    drop(replay.wait());
    if !sync {
        std::thread::sleep(Duration::from_millis(200));
    }
    let wait = Instant::now();
    stop.store(true, Ordering::Release);
    let (events, dropped) = reader.join().unwrap();
    assert!(wait.elapsed() < Duration::from_secs(2));
    Received { events, dropped }
}

/// The note ons of a replay as (onset seconds, midi).
/// @param r - the replay's events
pub fn onsets(r: &Received) -> Vec<(f64, f64)> {
    r.events.iter().filter(|(e, _)| e.is_on).map(|(e, _)| (e.onset_s, e.midi as f64)).collect()
}
