# Getting started

This page installs what Riffology needs, fetches the guitar model library, and runs the app with a guitar, with a
recording, and with MIDI.

## What you need

| Need | Why |
|---|---|
| Windows 10 or 11, 64 bit, or macOS on Apple silicon | The platforms the guitar model library is published for (see [The guitar model library](guitar-model.md)) |
| Rust 1.95 or newer (`rustup update`) | The workspace's `rust-version` |
| PowerShell 7 (`pwsh`) | The scripts in `tools/`. On macOS the fetch script also has a shell version |
| A GPU with DirectX 12, Vulkan or Metal | The window draws with wgpu |
| An audio interface with the guitar on an input | Live play. A recording works without one |
| ffmpeg on the PATH | Only for `tools/record-evidence.ps1` |

## Fetch the model library

On Windows:

```powershell
pwsh tools/fetch-guitar-abi.ps1
```

On macOS:

```sh
sh tools/fetch-guitar-abi.sh
```

The script downloads the zip for the platform, `chordical-guitar-abi-v1-windows-x64.zip` or
`chordical-guitar-abi-v1-macos-arm64.zip`, from the release named in `tools/guitar-abi.json`. It checks the zip's
SHA-256 and the library's SHA-256 against that file, and unpacks it into `vendor/guitar-abi/windows-x64/` or
`vendor/guitar-abi/macos-arm64/`. The shell version needs curl, unzip and node.
`vendor/guitar-abi/` is ignored by git. The app and the tests refuse a library whose hash differs.

## Run with a guitar

```powershell
cargo run --release -p riffology-app
cargo run --release -p riffology-app -- --device "Line 1/2 (Universal Audio Thunderbolt)" --channel 0
```

With no `--device` the app opens the system's default input. `--channel` picks the input the guitar is plugged into,
0 the first. The status line under the title names the device and says `Listening` once audio is arriving. The
readout at the bottom shows the block size the driver gave, the input level, and the detection delay of the notes
played so far.

## Run with a recording

```powershell
cargo run --release -p riffology-app -- --replay tests/fixtures/guitar/05_Rock1-130-A_solo.48k.wav --monitor
```

`--replay` plays the file through the same code path a guitar takes, in real time. `--monitor` plays it to the
speakers too. `--loop` repeats it.

## Run with MIDI or a chart

```powershell
cargo run --release -p riffology-app -- --midi                        # the first MIDI input, beside the guitar
cargo run --release -p riffology-app -- --chart songs/e-minor-pentatonic.json
```

A chart starts the chart following example game: expected notes ring on the fretboard and the hits are counted.

## Every option

| Option | What it does |
|---|---|
| `--replay <file.wav>` | Replay a WAV file in real time instead of the audio input |
| `--monitor` | Play the replayed file to the speakers too |
| `--loop` | Replay the file again and again |
| `--replay-delay <s>` | Wait this long before the replay starts. `tools/record-evidence.ps1` uses it |
| `--device <name>` | The audio input device. The default input when left out |
| `--channel <n>` | The input channel the guitar is on, 0 the first |
| `--midi [name]` | Also listen to a MIDI input: the first port, or the one whose name contains `name` |
| `--chart <file>` | Play against a `.json` chart or a `.mid` file |
| `--tuning <name>` | `Standard`, `Drop D`, `DADGAD`, `Open G`, `Open D`, `Half step down` |
| `--theme <name>` | `dark-neumorphic` (default) or `light-neumorphic` |
| `--trace <file.json>` | Write every note event and every drawn fretboard mark to a file |
| `--exit-after <s>` | Quit after this many seconds |
| `--background` | Open the window without taking the foreground |

## Where to go next

[Architecture](architecture.md) shows how the parts fit. [The note API](note-api.md) is what the game engine reads.
