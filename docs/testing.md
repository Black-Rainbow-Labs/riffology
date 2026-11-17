# Testing

Every test in Riffology runs real code. The guitar model tests run the published model library on real guitar
recordings, and the app test runs the built program in a real window. This page lists the layers, how to run each,
and the rules a new test follows.

## The layers

| Layer | Command | What it covers | Time |
|---|---|---|---|
| 1. Unit | `cargo test -p riffology-core -p riffology-song -p riffology-fretboard -p riffology-game` | Pitches, placement, the note bus across threads, player state, charts, MIDI import, fretboard geometry, chart following | seconds |
| 2. Input, real model | `cargo test --release -p riffology-input` | The library loader, accuracy through the live path on GuitarSet, no notes from hum, no allocation on the audio thread | about a minute |
| 3. App, end to end | `cargo test --release -p riffology-app` | The built app in a window replays two fixtures; every note heard is drawn on the fretboard within a frame | about a minute |
| 4. Timing | `cargo test --release -p riffology-input --test latency -- --ignored --nocapture` | Riffology's added delay, on a quiet machine | about a minute |
| 5. Look at it | `pwsh tools/record-evidence.ps1` | A video of the release build for the pull request | 30 seconds |

`pwsh tools/validate.ps1` runs formatting, clippy, layers 1 to 3 and the documentation check. Run it before opening a
pull request.

## The tests that use the model

| Test | File | Fails when |
|---|---|---|
| The pinned library loads and names its model | `riffology-input/tests/loader.rs` | the library is missing, has another hash or another ABI version |
| A file with another hash is refused | same | the hash check stops working |
| Real time replay scores within 1.5 points of the tracker | `riffology-input/tests/replay_accuracy.rs` | anything in the live path loses or delays notes: onset F1 against the GuitarSet labels drops |
| Hum with nothing played starts no note | same | 60 s of 60 Hz hum produces a note on |
| Processing blocks allocates nothing | `riffology-input/tests/no_alloc.rs` | the audio thread allocates |
| Every note heard lights the fretboard within a frame | `riffology-app/tests/replay_e2e.rs` | a note on that sounded past the next frame was not drawn as a heard mark of its pitch, or the bus dropped an event |

These tests never skip. Without the library they fail with "Run tools/fetch-guitar-abi.ps1". A skipped test reports
success, and a suite that passes because it did nothing is worse than one that fails.

## Rules for a new test

1. **A test asserts a value.** "It did not crash" is not a test.
2. **A test can fail.** Revert the change and watch the test fail before you trust it. Say in the pull request that
   you did.
3. **Prefer the real path.** Drive the app with `--replay` and `--trace`, or the input with a fixture, over calling an
   inner function with made up data. Mock only the audio device, which the replay stands in for.
4. **A timing test is `#[ignore]` and prints its numbers.** Assertions on time hold only on a quiet machine. The
   numbers go in the pull request.
5. **A bug fix adds the test that would have caught it**, in the same pull request.
6. **Visual changes are checked in the release build**, through `tools/record-evidence.ps1`, and the video goes in the
   pull request.

## Fixtures

`tests/fixtures/guitar` holds three 10 second GuitarSet excerpts at 48 kHz with their note labels, and 60 seconds of
direct input hum. [tests/fixtures/README.md](../tests/fixtures/README.md) has the sources and licenses. A new fixture
needs labels (onset, offset and MIDI pitch per note) and a line in that file.

## Where to go next

[Pull requests](pull-requests.md) for what a pull request must show.
