//! What the audio thread does with each input block: give it to the model, take the finished notes, stamp them on
//! the audio clock, and write them to the note bus. Live input and WAV replay both run this code, so a replay test
//! tests the path a guitar takes.
//!
//! `process` never blocks, locks or allocates. Settings reach it through atomics, and its counters leave through
//! atomics in [`PipelineStats`].

use crate::abi::CgNote;
use crate::tracker::{ModelTracker, TrackerSettings, TrackerStatus};
use riffology_core::{BusWriter, NoteEvent, Source};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Note events one drain call takes.
const DRAIN_CHUNK: usize = 256;
/// Blocks between telemetry reads.
const TELEMETRY_EVERY: u64 = 8;

/// Counters the audio thread writes and any thread reads.
#[derive(Debug, Default)]
pub struct PipelineStats {
    pub blocks: AtomicU64,
    pub samples: AtomicU64,
    pub note_ons: AtomicU64,
    /// Time the last block took in this code, in nanoseconds.
    pub last_block_ns: AtomicU64,
    /// Largest time one block took, in nanoseconds.
    pub max_block_ns: AtomicU64,
    /// `TrackerStatus` as 0 to 3.
    pub status: AtomicU32,
    /// Input RMS in dBFS, as f32 bits.
    pub input_rms_db: AtomicU32,
    /// 99th percentile model time per hop in milliseconds, as f32 bits.
    pub infer_ms_p99: AtomicU32,
    /// Input blocks the model's worker dropped.
    pub model_dropped_blocks: AtomicU32,
    /// The rate the pipeline runs at, as f32 bits.
    pub rate: AtomicU32,
    /// The block size the driver gives, in samples.
    pub block_size: AtomicU32,
    pending: AtomicBool,
    gain: AtomicU32,
    gate: AtomicU32,
    sensitivity: AtomicU32,
    reset: AtomicBool,
}

impl PipelineStats {
    /// The tracker status.
    pub fn status(&self) -> TrackerStatus {
        TrackerStatus::from_u8(self.status.load(Ordering::Relaxed) as u8)
    }

    /// An f32 stored as bits.
    /// @param a - the atomic
    pub fn f32_of(a: &AtomicU32) -> f32 {
        f32::from_bits(a.load(Ordering::Relaxed))
    }

    /// Asks the audio thread to apply new settings at its next block.
    /// @param s - the settings
    pub fn request_settings(&self, s: TrackerSettings) {
        self.gain.store(s.input_gain_db.to_bits(), Ordering::Relaxed);
        self.gate.store(s.gate_db.to_bits(), Ordering::Relaxed);
        self.sensitivity.store(s.sensitivity.to_bits(), Ordering::Relaxed);
        self.pending.store(true, Ordering::Release);
    }

    /// Asks the audio thread to end every held note at its next block.
    pub fn request_reset(&self) {
        self.reset.store(true, Ordering::Release);
    }
}

/// The per block work of one input.
pub struct InputPipeline {
    tracker: ModelTracker,
    bus: BusWriter,
    source: Source,
    samples: u64,
    rate: f64,
    scratch: Vec<CgNote>,
    events: Vec<NoteEvent>,
    stats: Arc<PipelineStats>,
}

impl InputPipeline {
    /// Wraps a tracker and the bus it writes to.
    /// @param tracker - the model tracker, made for this input's rate
    /// @param bus - the note bus writer
    /// @param source - `Guitar` for live input, `Replay` for a file
    pub fn new(tracker: ModelTracker, bus: BusWriter, source: Source) -> InputPipeline {
        let rate = tracker.rate();
        let stats = Arc::new(PipelineStats::default());
        stats.rate.store((rate as f32).to_bits(), Ordering::Relaxed);
        InputPipeline {
            tracker,
            bus,
            source,
            samples: 0,
            rate,
            scratch: vec![CgNote::default(); DRAIN_CHUNK],
            events: Vec::with_capacity(DRAIN_CHUNK * 4),
            stats,
        }
    }

    /// The counters, to read from another thread.
    pub fn stats(&self) -> Arc<PipelineStats> {
        self.stats.clone()
    }

    /// Seconds of input processed so far: the audio clock.
    pub fn clock_s(&self) -> f64 {
        self.samples as f64 / self.rate
    }

    /// Audio thread: processes one block of mono input.
    /// @param mono - the block's samples
    pub fn process(&mut self, mono: &[f32]) {
        let started = Instant::now();
        self.apply_requests();
        self.tracker.push(mono);
        self.samples += mono.len() as u64;
        let heard_s = self.clock_s();
        self.collect_events(heard_s);
        self.bus.write(&self.events);
        let blocks = self.stats.blocks.fetch_add(1, Ordering::Relaxed) + 1;
        if blocks.is_multiple_of(TELEMETRY_EVERY) {
            self.publish_telemetry();
        }
        self.record_timing(started, mono.len());
    }

    /// Applies settings and reset requests from other threads.
    fn apply_requests(&mut self) {
        if self.stats.pending.swap(false, Ordering::Acquire) {
            let s = &self.stats;
            self.tracker.set_settings(TrackerSettings {
                input_gain_db: PipelineStats::f32_of(&s.gain),
                gate_db: PipelineStats::f32_of(&s.gate),
                sensitivity: PipelineStats::f32_of(&s.sensitivity),
            });
        }
        if self.stats.reset.swap(false, Ordering::Acquire) {
            self.tracker.reset();
        }
    }

    /// Drains the tracker into `events`, stamped on the audio clock.
    /// @param heard_s - the audio clock at the end of this block
    fn collect_events(&mut self, heard_s: f64) {
        self.events.clear();
        loop {
            let n = self.tracker.drain(&mut self.scratch);
            for c in &self.scratch[..n] {
                let on = c.is_on == 1 && c.velocity > 0;
                let onset_s = if on { c.onset_s.min(heard_s) } else { heard_s };
                self.events.push(NoteEvent {
                    midi: c.note,
                    velocity: if on { c.velocity } else { 0 },
                    is_on: on,
                    source: self.source,
                    onset_s,
                    heard_s,
                });
            }
            if n < self.scratch.len() || self.events.len() + DRAIN_CHUNK > self.events.capacity() {
                break;
            }
        }
        let ons = self.events.iter().filter(|e| e.is_on).count() as u64;
        self.stats.note_ons.fetch_add(ons, Ordering::Relaxed);
    }

    /// Copies the tracker's telemetry into the shared counters.
    fn publish_telemetry(&mut self) {
        let t = self.tracker.telemetry();
        let s = &self.stats;
        s.status.store(t.status as u32, Ordering::Relaxed);
        s.input_rms_db.store(t.input_rms_db.to_bits(), Ordering::Relaxed);
        s.infer_ms_p99.store(t.infer_ms_p99.to_bits(), Ordering::Relaxed);
        s.model_dropped_blocks.store(t.dropped_blocks, Ordering::Relaxed);
    }

    /// Records how long this block took and its size.
    /// @param started - when `process` began
    /// @param len - the block's sample count
    fn record_timing(&self, started: Instant, len: usize) {
        let ns = started.elapsed().as_nanos() as u64;
        self.stats.last_block_ns.store(ns, Ordering::Relaxed);
        self.stats.max_block_ns.fetch_max(ns, Ordering::Relaxed);
        self.stats.samples.store(self.samples, Ordering::Relaxed);
        self.stats.block_size.store(len as u32, Ordering::Relaxed);
    }
}
