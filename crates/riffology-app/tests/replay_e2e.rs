//! The whole app, end to end: the built `riffology` binary opens its window, replays a GuitarSet fixture through the
//! real model in real time, and writes its trace. Every note the model heard that was still sounding a frame later
//! must have been drawn on the fretboard as a heard note of that pitch, in the frame it arrived or the next one.
//! Nothing may be dropped on the note bus.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// The trace of one run of the app on a fixture.
/// @param name - the fixture
fn run_app(name: &str) -> Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wav = root.join(format!("tests/fixtures/guitar/{name}.48k.wav"));
    let out = root.join("target/e2e").join(format!("{name}.trace.json"));
    let _ = std::fs::remove_file(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_riffology"))
        .arg("--replay")
        .arg(&wav)
        .arg("--trace")
        .arg(&out)
        .arg("--background")
        .status()
        .expect("the app runs");
    assert!(status.success(), "the app exited with {status}");
    serde_json::from_str(&std::fs::read_to_string(&out).expect("the app wrote its trace")).unwrap()
}

/// The pitch of a cell in standard tuning.
/// @param string - 1 the highest
/// @param fret - 0 open
fn pitch(string: i64, fret: i64) -> i64 {
    [64, 59, 55, 50, 45, 40][(string - 1) as usize] + fret
}

/// The heard pitches drawn in the latest recorded frame at or before `frame`.
/// @param frames - the trace frames
/// @param frame - the frame number
fn heard_at(frames: &[Value], frame: u64) -> Vec<i64> {
    let f = frames.iter().rev().find(|f| f["frame"].as_u64().unwrap() <= frame);
    f.map(|f| {
        f["marks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["kind"] == "Heard")
            .map(|m| pitch(m["string"].as_i64().unwrap(), m["fret"].as_i64().unwrap()))
            .collect()
    })
    .unwrap_or_default()
}

/// Checks one fixture's run and returns (notes that had to light, notes that lit).
/// @param name - the fixture
fn check(name: &str) -> (usize, usize) {
    let t = run_app(name);
    assert_eq!(t["dropped"], 0);
    assert_eq!(t["model_version"], "r2-final-s2");
    let frames = t["frames"].as_array().unwrap().clone();
    let mut ons: Vec<(i64, u64)> = vec![];
    let mut offs: Vec<(i64, u64)> = vec![];
    for f in &frames {
        let k = f["frame"].as_u64().unwrap();
        for e in f["events"].as_array().unwrap() {
            let midi = e["midi"].as_i64().unwrap();
            if e["is_on"].as_bool().unwrap() {
                ons.push((midi, k))
            } else {
                offs.push((midi, k))
            }
        }
    }
    let (mut needed, mut lit) = (0, 0);
    for (midi, k) in &ons {
        let off = offs.iter().filter(|(m, ko)| m == midi && ko >= k).map(|(_, ko)| *ko).min().unwrap_or(u64::MAX);
        if off <= k + 1 || !(40..=88).contains(midi) {
            continue;
        }
        needed += 1;
        if heard_at(&frames, *k).contains(midi) || heard_at(&frames, k + 1).contains(midi) {
            lit += 1;
        } else {
            eprintln!("{name}: note {midi} arrived in frame {k} and was not drawn by frame {}", k + 1);
        }
    }
    eprintln!(
        "{name}: {} note ons, {needed} sounded past the next frame, {lit} lit, {} frames drawn",
        ons.len(),
        t["frame_count"]
    );
    (needed, lit)
}

#[test]
fn every_note_heard_lights_the_fretboard_within_a_frame() {
    let (mut needed, mut lit) = (0, 0);
    for name in ["05_Rock1-130-A_solo", "05_Jazz1-130-D_solo", "05_Funk1-97-C_comp"] {
        let (n, l) = check(name);
        assert!(n > 0, "{name}: no note sounded past the next frame");
        needed += n;
        lit += l;
    }
    // A software renderer (the CI runner has no GPU) draws about 5 frames a second, so fewer notes outlast a frame
    // there than on a GPU, where all three fixtures give about 100. The count still has to mean something.
    assert!(needed >= 15, "only {needed} notes sounded past the next frame");
    assert_eq!(lit, needed);
}
