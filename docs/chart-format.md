# Chart format

A chart is the list of notes a game plays against. This page describes the JSON file, the Rust type, and MIDI
import.

## The file

```json
{
  "version": 1,
  "title": "E minor pentatonic",
  "artist": "",
  "tuning": { "strings": [64, 59, 55, 50, 45, 40], "capo": 0, "name": "Standard" },
  "tempo_bpm": 90.0,
  "notes": [
    { "time_s": 2.0, "length_s": 0.6, "midi": 40, "string": 6, "fret": 0 },
    { "time_s": 2.667, "length_s": 0.6, "midi": 43, "string": 6, "fret": 3 }
  ],
  "sections": [{ "name": "Up", "start_s": 2.0 }]
}
```

| Field | Meaning |
|---|---|
| `version` | 1. Loading refuses any other version |
| `tuning.strings` | MIDI pitch of each open string, string 1 (the highest) first |
| `tempo_bpm` | The first tempo, for drawing beat lines. Note times are already in seconds |
| `notes[].time_s` | Start, in seconds from the start of the song |
| `notes[].length_s` | How long the note is held |
| `notes[].midi` | The pitch |
| `notes[].string`, `notes[].fret` | Where it is played. Optional: `Chart::assign_strings` fills the missing ones |
| `sections` | Named start times, for practice loops and progress |

`songs/e-minor-pentatonic.json` is a complete example.

## In Rust

```rust
let chart = Chart::from_json(&std::fs::read_to_string("songs/e-minor-pentatonic.json")?)?;
for note in chart.notes_between(song_s, song_s + 3.0) { /* draw it on the highway */ }
let now: Vec<&ChartNote> = chart.sounding_at(song_s).collect();
```

`assign_strings` treats notes starting within 30 ms of each other as a chord and puts them on different strings,
with the same placement the fretboard uses ([The fretboard](fretboard.md)).

## MIDI import

```rust
let chart = import_midi(&bytes, "My song", Tuning::standard(), &MidiImportOptions { track: Some(1), channel: None })?;
```

Ticks become seconds through every tempo change in the file. Every channel except channel 10 (drums) is read unless
`channel` names one. A note on is paired with the next note off of the same pitch on the same channel. Strings and
frets are then assigned. Timecode MIDI files are refused.

`cargo run --release -p riffology-app -- --chart song.mid` imports a MIDI file the same way.

## What is not here yet

Guitar Pro and MusicXML import, which give the string and fret a transcriber chose. Both importers exist in
Chordical Sheet Music's Rust crates, and bringing them here is a decision about publishing that code.
