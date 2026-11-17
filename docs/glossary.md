# Glossary

Each term the pages use, in one sentence. Terms are in alphabetical order.

| Term | Meaning |
|---|---|
| ABI | The binary interface of a library: the function names, their arguments and the layout of the structs they pass. The guitar model library has ABI version 1 |
| Audio clock | Seconds of input processed since the input stream started: samples counted divided by the sample rate. Every `NoteEvent` time is on it |
| Audio thread | The thread the audio driver calls with each block of input. It must never wait on a lock, allocate memory or touch a file |
| ASIO | A Windows audio driver interface from Steinberg with lower latency than WASAPI. Most audio interfaces ship an ASIO driver |
| Block | The samples the driver hands over in one call, 128 at 48 kHz (2.7 ms) when the driver allows it |
| Chart | The notes of a song with their times, strings and frets, which a game plays against ([Chart format](chart-format.md)) |
| Detection delay | `heard_s - onset_s` of a note: how long after the note started Riffology knew about it. Mostly the model's own delay |
| Fret | A position on the neck. Fret 0 is the open string. Each fret raises the pitch a semitone |
| Fixture | A recording kept in `tests/fixtures` with labels of the notes in it, which tests replay |
| Frame | One drawing of the window, about 16 ms at 60 Hz. Also, for the model, one 10 ms step of audio. Pages say which |
| GuitarSet | A dataset of guitar recordings with every note labeled (Xi, Bittner, Pauwels, Ye and Bello, ISMIR 2018). The fixtures come from it |
| Hop | One 10 ms step of the model: 160 samples at 16 kHz |
| MIDI note number | A pitch as a number: 60 is middle C (C4), 40 is the low E string (E2) |
| Model | The polyphonic guitar pitch model `r2-final-s2`, which hears pitches in audio |
| Note bus | The lock free queue that carries note events from the audio thread to the game thread |
| Onset | The moment a note starts |
| Onset F1 | The accuracy score for note starts: an onset counts as right when it is within 50 ms of a labeled onset of the same pitch. 1.0 is perfect |
| Placement | Choosing the string and fret of a heard pitch, since the model hears pitches and not strings |
| Polyphonic | Hearing several notes at once, as in a chord |
| Replay | Playing a WAV file through the input path in real time, as if a guitar were plugged in |
| Sample rate | Samples per second of audio, usually 48,000 |
| String | String 1 is the highest (high E), string 6 the lowest (low E), as in tab |
| Sync mode | The model runs inside each push on the calling thread, so every run gives the same notes. Tests use it |
| Trace | The JSON file `--trace` writes: every note event and every drawn mark, per frame |
| Tuning | The pitch of each open string and the capo fret |
| WASAPI | The Windows audio interface every device supports. Shared mode adds about 10 ms over ASIO |
| Worker mode | The model runs on its own thread, so the audio thread only copies samples in and notes out. Live input uses it |
