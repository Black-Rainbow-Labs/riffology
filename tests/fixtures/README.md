# Test fixtures

## guitar/

| File | What it is |
|---|---|
| `05_Rock1-130-A_solo.48k.wav` | GuitarSet player 05, rock solo in A at 130 BPM, first 10 seconds, microphone, mono 48 kHz |
| `05_Jazz1-130-D_solo.48k.wav` | GuitarSet player 05, jazz solo in D at 130 BPM, first 10 seconds |
| `05_Funk1-97-C_comp.48k.wav` | GuitarSet player 05, funk comping in C at 97 BPM, first 10 seconds (chords) |
| `*.labels.json` | The GuitarSet note annotations for each excerpt: `notes` is a list of `[onset seconds, offset seconds, MIDI pitch]` |
| `silence-hum.48k.wav` | 60 seconds with nothing played, made from the quietest 2 seconds of EGDB `audio_DI/1.wav` with a 60 Hz hum added |

GuitarSet: Q. Xi, R. Bittner, J. Pauwels, X. Ye and J. P. Bello, "GuitarSet: A Dataset for Guitar Transcription",
ISMIR 2018. Licensed CC BY 4.0. The excerpts and labels are the ones Chordical's MIDI Guitar tests use.

EGDB: the electric guitar dataset of Chen, Yang and others (ICASSP 2022), used for the hum fixture only.
