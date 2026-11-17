# Instructions for coding agents

This file is for any coding agent working in this repository: Claude Code, Codex, Gemini, Cursor, or another. People
read it too. It says how the project is laid out, what a finished change looks like, and the rules that are not
obvious from the code. `CLAUDE.md` and `GEMINI.md` point here.

## The project

Riffology is a guitar game played with a real guitar. Read [docs/architecture.md](docs/architecture.md) before
changing anything in `crates/`. The crates, in dependency order:

| Crate | Owns |
|---|---|
| `riffology-core` | Pitches, tunings, string placement, `NoteEvent`, the note bus, `PlayerState`. No I/O |
| `riffology-input` | The guitar model library loader, audio input, MIDI input, WAV replay |
| `riffology-fretboard` | Fretboard geometry and the rux widget |
| `riffology-song` | Charts and MIDI import |
| `riffology-game` | The `Game` trait and example games |
| `riffology-app` | The window and the `riffology` binary |

## Before your first build

```powershell
pwsh tools/fetch-guitar-abi.ps1
```

Without the guitar model library the input and app tests fail on purpose. Do not make them skip.

## What a finished change looks like

1. **It works in the real app.** Run it: `cargo run --release -p riffology-app -- --replay <fixture>` for anything a
   guitar does, and look at the result in the release build with `pwsh tools/record-evidence.ps1`.
2. **It is tested on the real path.** See [docs/testing.md](docs/testing.md). Prefer a replay through the app or the
   input pipeline over a unit test with invented data. A new test must fail with your change reverted; check that it
   does.
3. **It is measured** when it touches `riffology-input`, `riffology-core` or the drawing code. Run the latency test on
   a quiet machine and report its line. "It should be fast" is not a measurement.
4. **The documentation says what the code now does**, in the same change. Run `node tools/doc-style/check.mjs`.
5. **`pwsh tools/validate.ps1` passes.**
6. **The pull request fills in every section of the template**, with the video under Evidence and the numbers under
   Performance. See [docs/pull-requests.md](docs/pull-requests.md).

## Rules that are not obvious from the code

- **The audio thread never allocates, locks, sleeps, logs or touches a file.** That is `InputPipeline::process`, the
  cpal callback in `audio.rs`, and anything they call. `tests/no_alloc.rs` enforces the allocation part.
- **Judge timing by `onset_s`, never by when a note arrived.** Arrival is 20 to 30 ms after the player started the
  note because the model needs that long to be sure.
- **The guitar model library is pinned by SHA-256** in `tools/guitar-abi.json`. Never commit the library, never weaken
  the hash check, and never set `RIFFOLOGY_GUITAR_ABI_UNCHECKED` in CI or in a test.
- **Never unload the library.** Its thread pool outlives every tracker. `GuitarAbi` keeps it loaded on purpose.
- **Colors come from the rux theme.** No `Color32::from_rgb` in drawing code. If the theme lacks a color, say so in
  the pull request.
- **The note API is a contract with the game engine.** A change to `NoteEvent`, `PlayerState`, `GameContext` or
  `Game` says so in the pull request's Overview and updates [docs/note-api.md](docs/note-api.md).
- **Pin every new crates.io dependency to an exact version** (`"=1.2.3"`) in the root `Cargo.toml`, and commit
  `Cargo.lock`.
- **Do not add the model, its weights or its training code** to this repository in any form.

## Code style

- A module comment at the top of every file says what the file is for.
- Every function has a doc comment with what it does and `@param` lines, and stays under about 50 lines. A long
  function becomes several named ones.
- Function parameters on one line.
- Prefer free functions that take their state as parameters over methods that reach into `self` for it.
- `cargo fmt` with the repository's `rustfmt.toml`, and `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Writing

Follow [docs/writing-style.md](docs/writing-style.md) in comments, documentation, commit messages and pull request
descriptions: short declarative sentences, plain words, the name of the thing every time, American spelling, and
no hyphenated invented terms. Never add a credit line for a tool or an agent to a commit or a file.

## Commands

| Command | What it does |
|---|---|
| `pwsh tools/fetch-guitar-abi.ps1` | Downloads and checks the guitar model library |
| `pwsh tools/validate.ps1` | fmt, clippy, tests with the real model, documentation check |
| `pwsh tools/record-evidence.ps1` | Records the release build to an MP4 for a pull request |
| `cargo test --release -p riffology-input --test latency -- --ignored --nocapture` | The added delay, on a quiet machine |
| `node tools/doc-style/check.mjs` | The documentation rules |
| `node --test .github/scripts/check-pull-request.test.mjs` | The pull request description check's own tests |
