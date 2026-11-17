//! Live audio input through cpal: WASAPI on Windows (ASIO with the `asio` feature), Core Audio on macOS. The stream
//! asks the driver for 128 sample blocks; the driver may give another size, and the readout shows what it gave. One
//! channel of the input is the guitar.

use crate::pipeline::{InputPipeline, PipelineStats};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The block size asked for.
pub const PREFERRED_BLOCK: u32 = 128;
/// The largest block the callback converts in one piece; a larger block is processed in pieces.
const SCRATCH: usize = 8192;

/// One input device as the picker lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct InputDevice {
    pub name: String,
    pub channels: u16,
    pub rate: u32,
}

/// The input devices of the default host.
pub fn list_input_devices() -> Vec<InputDevice> {
    let host = cpal::default_host();
    let Ok(devices) = host.input_devices() else { return vec![] };
    devices
        .filter_map(|d| {
            d.default_input_config().ok().map(|c| InputDevice {
                name: d.to_string(),
                channels: c.channels(),
                rate: c.sample_rate(),
            })
        })
        .collect()
}

/// The device and settings a stream is opened with.
#[derive(Debug, Clone, Default)]
pub struct InputChoice {
    /// The device name, or None for the default input.
    pub device: Option<String>,
    /// The channel that carries the guitar, 0 the first.
    pub channel: usize,
}

/// A running input stream.
pub struct AudioInput {
    _stream: cpal::Stream,
    pub device: InputDevice,
    pub stats: Arc<PipelineStats>,
    pub failed: Arc<AtomicBool>,
}

/// Finds the device a choice names.
/// @param choice - the choice
fn find_device(choice: &InputChoice) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    match &choice.device {
        None => host.default_input_device().ok_or_else(|| "there is no default audio input device".to_string()),
        Some(name) => host
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| &d.to_string() == name)
            .ok_or_else(|| format!("no audio input called {name}")),
    }
}

/// The rate and channel count of a device's default input format.
/// @param device - the device
pub fn device_rate(choice: &InputChoice) -> Result<(u32, u16), String> {
    let d = find_device(choice)?;
    let c = d.default_input_config().map_err(|e| e.to_string())?;
    Ok((c.sample_rate(), c.channels()))
}

impl AudioInput {
    /// Opens the input and starts the stream. The pipeline must have been made for the rate `device_rate` returns.
    /// @param choice - which device and channel
    /// @param pipeline - the pipeline, moved into the audio callback
    pub fn start(choice: &InputChoice, pipeline: InputPipeline) -> Result<AudioInput, String> {
        let device = find_device(choice)?;
        let supported = device.default_input_config().map_err(|e| e.to_string())?;
        let (rate, channels) = (supported.sample_rate(), supported.channels());
        let channel = choice.channel.min(channels as usize - 1);
        let stats = pipeline.stats();
        let failed = Arc::new(AtomicBool::new(false));
        let fixed = match supported.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } if (*min..=*max).contains(&PREFERRED_BLOCK) => {
                cpal::BufferSize::Fixed(PREFERRED_BLOCK)
            }
            _ => cpal::BufferSize::Default,
        };
        let config = cpal::StreamConfig { channels, sample_rate: rate, buffer_size: fixed };
        let stream = build(&device, config, supported.sample_format(), channel, pipeline, failed.clone())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(AudioInput {
            _stream: stream,
            device: InputDevice { name: device.to_string(), channels, rate },
            stats,
            failed,
        })
    }

    /// Whether the driver reported an error since the stream started (for example the device was unplugged).
    pub fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

/// Builds the stream for the device's sample format.
/// @param device - the device
/// @param config - the stream config
/// @param format - the sample format
/// @param channel - the guitar's channel
/// @param pipeline - the pipeline
/// @param failed - set by the error callback
fn build(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: cpal::SampleFormat,
    channel: usize,
    pipeline: InputPipeline,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, String> {
    let ch = config.channels as usize;
    let on_error = move |e: cpal::Error| {
        eprintln!("audio input: {e}");
        failed.store(true, Ordering::Relaxed);
    };
    let mut cb = Callback { pipeline, scratch: vec![0.0; SCRATCH], channel, channels: ch };
    let stream = match format {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(config, move |d: &[f32], _: &_| cb.run(d, |s| s), on_error, None)
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(config, move |d: &[i16], _: &_| cb.run(d, |s| s as f32 / 32768.0), on_error, None)
        }
        cpal::SampleFormat::I32 => device.build_input_stream(
            config,
            move |d: &[i32], _: &_| cb.run(d, |s| s as f32 / 2_147_483_648.0),
            on_error,
            None,
        ),
        other => return Err(format!("unsupported input sample format {other:?}")),
    };
    stream.map_err(|e| e.to_string())
}

/// The audio callback's state.
struct Callback {
    pipeline: InputPipeline,
    scratch: Vec<f32>,
    channel: usize,
    channels: usize,
}

impl Callback {
    /// Takes the guitar's channel out of the interleaved block and processes it. Never allocates.
    /// @param data - interleaved samples
    /// @param conv - converts one sample to f32
    fn run<T: Copy>(&mut self, data: &[T], conv: impl Fn(T) -> f32) {
        let frames = data.len() / self.channels.max(1);
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(self.scratch.len());
            for i in 0..n {
                self.scratch[i] = conv(data[(done + i) * self.channels + self.channel]);
            }
            self.pipeline.process(&self.scratch[..n]);
            done += n;
        }
    }
}
