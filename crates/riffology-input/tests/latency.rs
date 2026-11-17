//! The delay Riffology adds on top of the model's own decision. In sync mode the model decides each note inside the
//! push of a block, so the note's `heard_s` is the end of the block whose audio decided it. In real time the same
//! note reaches a reader on another thread later: after the worker thread runs the hop, after the next block's
//! drain, and through the note bus. That difference, per note, is what this measures.
//!
//! The targets are the ones Chordical's plugin test uses for the same path: median 6 ms, 95th percentile 10 ms.
//!
//! Run on a quiet machine: `cargo test --release -p riffology-input --test latency -- --ignored --nocapture`.

mod common;

use riffology_input::stats::percentile;

#[test]
#[ignore = "timing test: run on a quiet machine with --ignored"]
fn the_added_delay_is_under_six_ms_median_and_ten_ms_p95() {
    let (mut added, mut total) = (Vec::new(), 0);
    for name in common::EXCERPTS {
        let audio = common::audio(name);
        let sync = common::replay(&audio, true);
        let rt = common::replay(&audio, false);
        for (e, at) in rt.events.iter().filter(|(e, _)| e.is_on) {
            total += 1;
            if let Some((s, _)) =
                sync.events.iter().find(|(s, _)| s.is_on && s.midi == e.midi && (s.onset_s - e.onset_s).abs() < 1e-6)
            {
                added.push((at - s.heard_s) * 1000.0);
            }
        }
    }
    let pct = |p| percentile(&added, p).unwrap();
    let (median, p95, p99, max) = (pct(50.0), pct(95.0), pct(99.0), pct(100.0));
    eprintln!("added delay over {} of {total} notes: median {median:.2} ms, p95 {p95:.2} ms, p99 {p99:.2} ms, largest {max:.2} ms", added.len());
    assert!(added.len() as f64 >= 0.95 * total as f64, "only {} of {total} notes matched the sync run", added.len());
    assert!(median <= 6.0 && p95 <= 10.0);
}
