//! Accuracy through Riffology's whole input path. Each GuitarSet fixture is replayed in real time through the input
//! pipeline with the model on its worker thread, as live input runs, and the note ons on the bus are scored against
//! the GuitarSet labels. The same file through the model in sync mode (the tracker's own result) is the reference:
//! the real time path must be within 1.5 points of it. Also: no note from 60 seconds of hum with nothing played.

mod common;

use riffology_input::stats::onset_f1;

#[test]
fn real_time_replay_scores_within_one_and_a_half_points_of_the_tracker() {
    let (mut sum_rt, mut sum_sync) = (0.0, 0.0);
    for name in common::EXCERPTS {
        let audio = common::audio(name);
        let labels = common::labels(name);
        let sync = common::replay(&audio, true);
        let rt = common::replay(&audio, false);
        let (f_sync, f_rt) = (onset_f1(&labels, &common::onsets(&sync)), onset_f1(&labels, &common::onsets(&rt)));
        eprintln!(
            "{name}: tracker F1 {f_sync:.4}, Riffology real time F1 {f_rt:.4}, {} note ons, {} dropped",
            common::onsets(&rt).len(),
            rt.dropped
        );
        assert_eq!(rt.dropped, 0);
        sum_rt += f_rt;
        sum_sync += f_sync;
    }
    let n = common::EXCERPTS.len() as f64;
    eprintln!("mean tracker F1 {:.4}, mean Riffology F1 {:.4}", sum_sync / n, sum_rt / n);
    assert!(sum_sync / n > 0.85, "the tracker itself scored {:.4}", sum_sync / n);
    assert!((sum_rt - sum_sync).abs() / n <= 0.015);
}

#[test]
fn hum_with_nothing_played_starts_no_note() {
    let w = riffology_input::wav::read_wav(&common::fixtures().join("silence-hum.48k.wav"), 0).unwrap();
    let r = common::replay(&w.samples, true);
    let ons = common::onsets(&r).len();
    eprintln!("{ons} note ons in {:.0} s of hum", w.samples.len() as f64 / 48_000.0);
    assert_eq!(ons, 0);
}
