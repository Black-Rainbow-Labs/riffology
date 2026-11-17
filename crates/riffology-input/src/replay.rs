//! A WAV file played through the input pipeline in real time on its own thread, as if a guitar were plugged in. The
//! end to end tests use it, and so does anyone working without a guitar (`riffology --replay <file>`). It can also
//! play the file to the default output device so a recording of the window has the guitar's sound.

use crate::pipeline::InputPipeline;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How a file is replayed.
#[derive(Debug, Clone, Copy)]
pub struct ReplayOptions {
    /// Samples per block, as an audio driver would give them.
    pub block: usize,
    /// Pace the blocks in real time. False runs as fast as possible (use with a sync tracker for exact results).
    pub realtime: bool,
    /// Play the file to the default output device too.
    pub monitor: bool,
    /// Start again at the end until stopped.
    pub looped: bool,
    /// Wait this long before the first block, so a screen recording can start first.
    pub start_delay: Duration,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        ReplayOptions { block: 128, realtime: true, monitor: false, looped: false, start_delay: Duration::ZERO }
    }
}

/// A running replay.
pub struct Replay {
    stop: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    join: Option<JoinHandle<InputPipeline>>,
    started: Instant,
}

impl Replay {
    /// Starts replaying samples through a pipeline on a new thread.
    /// @param samples - mono samples at the pipeline's rate
    /// @param rate - their sample rate
    /// @param pipeline - the pipeline, moved to the replay thread
    /// @param opts - block size, pacing, monitoring and looping
    pub fn start(samples: Vec<f32>, rate: u32, pipeline: InputPipeline, opts: ReplayOptions) -> Replay {
        let stop = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let (s2, d2) = (stop.clone(), done.clone());
        let started = Instant::now() + opts.start_delay;
        let join = std::thread::Builder::new()
            .name("riffology-replay".into())
            .spawn(move || {
                if opts.realtime {
                    crate::priority::raise_current_thread();
                }
                sleep_until(started);
                let monitor = if opts.monitor { Monitor::open(&samples, rate) } else { None };
                let p = run(samples, rate, pipeline, opts, &s2, started);
                drop(monitor);
                d2.store(true, Ordering::Release);
                p
            })
            .expect("replay thread");
        Replay { stop, done, join: Some(join), started }
    }

    /// The moment the replay's audio clock started, for converting wall time to the clock.
    pub fn started(&self) -> Instant {
        self.started
    }

    /// Whether the file has finished.
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Stops the replay and gives the pipeline back.
    pub fn stop(mut self) -> InputPipeline {
        self.stop.store(true, Ordering::Release);
        self.join.take().expect("joined once").join().expect("replay thread panicked")
    }

    /// Waits for the file to end (it never ends when looped) and gives the pipeline back.
    pub fn wait(mut self) -> InputPipeline {
        self.join.take().expect("joined once").join().expect("replay thread panicked")
    }
}

/// The replay loop: one block at a time, each sent at the moment its last sample would have been recorded.
/// @param samples - the file
/// @param rate - its rate
/// @param pipeline - the pipeline
/// @param opts - the options
/// @param stop - set to stop early
/// @param started - the clock's start
fn run(
    samples: Vec<f32>,
    rate: u32,
    mut pipeline: InputPipeline,
    opts: ReplayOptions,
    stop: &AtomicBool,
    started: Instant,
) -> InputPipeline {
    let mut sent: u64 = 0;
    loop {
        for chunk in samples.chunks(opts.block.max(1)) {
            if stop.load(Ordering::Acquire) {
                return pipeline;
            }
            sent += chunk.len() as u64;
            if opts.realtime {
                sleep_until(started + Duration::from_secs_f64(sent as f64 / rate as f64));
            }
            pipeline.process(chunk);
        }
        if !opts.looped {
            return pipeline;
        }
    }
}

/// Waits until a moment: sleeps, then spins for the last half millisecond so blocks leave on time, as a driver's
/// callback would.
/// @param at - the moment
fn sleep_until(at: Instant) {
    loop {
        let now = Instant::now();
        if now >= at {
            return;
        }
        let left = at - now;
        if left > Duration::from_micros(700) {
            std::thread::sleep(left - Duration::from_micros(500));
        } else {
            std::hint::spin_loop();
        }
    }
}

/// The output stream that plays the replayed file.
struct Monitor {
    _stream: cpal::Stream,
}

impl Monitor {
    /// Opens the default output at the file's rate and plays the samples once. None when that is not possible; the
    /// replay goes on silently.
    /// @param samples - the file
    /// @param rate - its rate
    fn open(samples: &[f32], rate: u32) -> Option<Monitor> {
        let device = cpal::default_host().default_output_device()?;
        let channels = device.default_output_config().ok()?.channels();
        let config = cpal::StreamConfig { channels, sample_rate: rate, buffer_size: cpal::BufferSize::Default };
        let data: Arc<Vec<f32>> = Arc::new(samples.to_vec());
        let mut pos = 0usize;
        let stream = device
            .build_output_stream(
                config,
                move |out: &mut [f32], _| {
                    for frame in out.chunks_mut(channels as usize) {
                        let s = data.get(pos).copied().unwrap_or(0.0);
                        pos += 1;
                        frame.iter_mut().for_each(|x| *x = s);
                    }
                },
                |e| eprintln!("replay monitor: {e}"),
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(Monitor { _stream: stream })
    }
}
