# Audio input

This page covers the three inputs (a guitar through an audio interface, a WAV replay, a MIDI device), how each one
reaches the note bus, and the settings that change the delay.

## The three inputs

| Input | Code | Source in `NoteEvent` | Clock |
|---|---|---|---|
| Guitar | `riffology-input/src/audio.rs`, cpal | `Guitar` | input samples counted |
| WAV replay | `riffology-input/src/replay.rs` | `Replay` | the file's samples counted |
| MIDI | `riffology-input/src/midi.rs`, midir | `Midi` | seconds since the session started |

The guitar and the replay run the same `InputPipeline`, so a replay test tests the guitar's path. MIDI writes to its
own note bus and the window drains both.

## The guitar

```rust
let abi = GuitarAbi::load()?;
let choice = InputChoice { device: None, channel: 0 };
let (rate, _) = device_rate(&choice)?;
let tracker = ModelTracker::new(abi, rate as f64, false)?;   // worker mode
let (writer, reader) = note_bus();
let pipeline = InputPipeline::new(tracker, writer, Source::Guitar);
let input = AudioInput::start(&choice, pipeline)?;            // the stream runs until `input` is dropped
```

- The stream opens at the device's default rate. The model resamples to 16 kHz inside the library, so 44.1, 48 and
  96 kHz all work.
- It asks the driver for 128 sample blocks when the device's buffer range allows it. Many WASAPI devices give
  their own size; the readout's `block` shows what was given.
- One channel is the guitar: `--channel`, 0 the first.
- The callback accepts f32, 16 bit and 32 bit integer samples.

## Lower latency with ASIO

WASAPI shared mode adds about 10 ms over an ASIO driver. To build with ASIO:

```powershell
$env:CPAL_ASIO_DIR = "C:\path\to\asiosdk"      # the Steinberg ASIO SDK, unzipped
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
cargo run --release -p riffology-app --features asio
```

The SDK cannot be redistributed, so it is not in this repository. Selecting the ASIO host in the app is the next
step for the input picker.

## The tracker settings

`PipelineStats::request_settings` sets input gain, gate and sensitivity from any thread; the audio thread applies
them at its next block. `PipelineStats::request_reset` ends every held note. The ranges are in
[The guitar model library](guitar-model.md). The window has no controls for them yet.

## WAV replay

```powershell
cargo run --release -p riffology-app -- --replay my-riff.wav --monitor --loop
```

The replay thread raises itself to audio priority and sends each block at the moment its last sample would have
been recorded, so its timing matches a live input. `ReplayOptions { realtime: false, .. }` with a sync tracker runs
as fast as possible and gives the same notes every run, which the accuracy tests use. Any WAV works: 16 or 24 bit
PCM or 32 bit float, any rate, channel 0 is read.

## MIDI

```powershell
cargo run --release -p riffology-app -- --midi
cargo run --release -p riffology-app -- --midi "Fishman"
```

Note ons and note offs on every channel become `NoteEvent`s with `Source::Midi`. A note on with velocity 0 is a note
off. A MIDI guitar reports the string through its channel; Riffology does not read that yet.

## Where to go next

[Latency](latency.md) for what each input adds. [The note API](note-api.md) for what arrives at the game.
