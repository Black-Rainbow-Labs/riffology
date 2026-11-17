//! The audio thread's work never allocates. A counting allocator counts allocations made on this thread while it is
//! armed; two seconds of guitar audio go through `InputPipeline::process` with the model on its worker thread, the
//! way the audio callback calls it.

mod common;

use riffology_core::{note_bus, Source};
use riffology_input::{InputPipeline, ModelTracker};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
}
static COUNT: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, counting while this thread is armed.
struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ARMED.with(|a| a.get()) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

#[test]
fn processing_blocks_allocates_nothing() {
    let audio = common::audio("05_Rock1-130-A_solo");
    let tracker = ModelTracker::new(common::abi(), 48_000.0, false).unwrap();
    let (w, mut r) = note_bus();
    let mut p = InputPipeline::new(tracker, w, Source::Guitar);
    let blocks: Vec<&[f32]> = audio.chunks(128).collect();
    for b in &blocks[..50] {
        p.process(b);
    }
    ARMED.with(|a| a.set(true));
    for b in &blocks[50..800] {
        p.process(b);
        std::thread::sleep(std::time::Duration::from_micros(2_600));
    }
    ARMED.with(|a| a.set(false));
    let mut out = Vec::new();
    r.drain_into(&mut out);
    let n = COUNT.load(Ordering::Relaxed);
    eprintln!("{n} allocations in 750 blocks; {} events on the bus", out.len());
    assert!(!out.is_empty(), "the model heard nothing, so the test proved nothing");
    assert_eq!(n, 0);
}
