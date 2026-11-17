# Riffology documentation

This folder holds the reference pages for Riffology. Each page can be read on its own. The tables list them in
reading order for a programmer joining the project: what it is, how to run it, how the parts fit, then each part,
then how to change it.

`node tools/doc-style/check.mjs` fails when a page in `docs/` is missing from this index.

## Start here

| | Page | What it covers |
|---|---|---|
| 1 | [Getting started](getting-started.md) | Installing, fetching the model library, running the app, the command line options |
| 2 | [Glossary](glossary.md) | Each audio, music and game term the pages use, in one sentence |
| 3 | [Architecture](architecture.md) | The crates, the threads, and the path a note takes from the guitar to the screen |

## The parts

| | Page | What it covers |
|---|---|---|
| 4 | [The note API](note-api.md) | `NoteEvent`, the audio clock, `PlayerState` and the `Game` trait: what the game engine reads |
| 5 | [The guitar model library](guitar-model.md) | How the model reaches Riffology as a binary, how it is checked, updated and licensed |
| 6 | [Audio input](audio-input.md) | Devices, channels, block sizes, ASIO, MIDI input and WAV replay |
| 7 | [The fretboard](fretboard.md) | Geometry, string placement, marks and how the widget draws them |
| 8 | [Chart format](chart-format.md) | The JSON chart a game plays against, and MIDI import |

## Measurements

| | Page | What it covers |
|---|---|---|
| 9 | [Latency](latency.md) | Every source of delay from pick to pixel, the budget, and what has been measured |

## Working on it

| | Page | What it covers |
|---|---|---|
| 10 | [Testing](testing.md) | The test layers, the end to end tests, and the rule that every test can fail |
| 11 | [Pull requests](pull-requests.md) | What a pull request must contain: tests, video evidence, numbers, documentation |
| 12 | [Writing style](writing-style.md) | How to write these pages, commit messages and pull request descriptions |
