//! The note bus: a single producer, single consumer lock free ring that carries note events from the audio thread
//! (the writer) to the game thread (the reader). Writing never blocks or allocates. A full ring drops the event and
//! counts it, and the count is shown in the app and asserted 0 by the tests.

use crate::event::NoteEvent;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Events the ring holds. At 60 frames a second the reader drains every 16 ms, and a guitar plays far fewer notes.
pub const BUS_CAPACITY: usize = 1024;

/// Counters both ends share.
#[derive(Debug, Default)]
struct Counters {
    written: AtomicU64,
    dropped: AtomicU64,
}

/// The audio thread's end.
pub struct BusWriter {
    ring: Producer<NoteEvent>,
    counters: Arc<Counters>,
    wake: Option<Box<dyn Fn() + Send>>,
}

/// The game thread's end.
pub struct BusReader {
    ring: Consumer<NoteEvent>,
    counters: Arc<Counters>,
}

/// Makes a connected writer and reader.
pub fn note_bus() -> (BusWriter, BusReader) {
    let (p, c) = RingBuffer::new(BUS_CAPACITY);
    let counters = Arc::new(Counters::default());
    (BusWriter { ring: p, counters: counters.clone(), wake: None }, BusReader { ring: c, counters })
}

impl BusWriter {
    /// Sets a function called after a write that added events, for example `egui::Context::request_repaint`, so the
    /// game thread draws the note without waiting for its next frame timer.
    /// @param wake - the function
    pub fn set_wake(&mut self, wake: Box<dyn Fn() + Send>) {
        self.wake = Some(wake);
    }

    /// Writes events. Never blocks or allocates. An event that does not fit is dropped and counted.
    /// @param events - the events, in order
    pub fn write(&mut self, events: &[NoteEvent]) {
        if events.is_empty() {
            return;
        }
        for e in events {
            if self.ring.push(*e).is_ok() {
                self.counters.written.fetch_add(1, Ordering::Relaxed);
            } else {
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
        if let Some(w) = &self.wake {
            w();
        }
    }
}

impl BusReader {
    /// Moves every waiting event into `out`, in order.
    /// @param out - receives the events
    pub fn drain_into(&mut self, out: &mut Vec<NoteEvent>) {
        while let Ok(e) = self.ring.pop() {
            out.push(e);
        }
    }

    /// Events written since the bus was made.
    pub fn written(&self) -> u64 {
        self.counters.written.load(Ordering::Relaxed)
    }

    /// Events dropped because the ring was full.
    pub fn dropped(&self) -> u64 {
        self.counters.dropped.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Source;

    fn on(midi: u8) -> NoteEvent {
        NoteEvent { midi, velocity: 100, is_on: true, source: Source::Midi, onset_s: 0.0, heard_s: 0.0 }
    }

    #[test]
    fn events_arrive_in_order_across_threads() {
        let (mut w, mut r) = note_bus();
        let t = std::thread::spawn(move || {
            for i in 0..500u32 {
                w.write(&[on((i % 128) as u8)]);
            }
        });
        t.join().unwrap();
        let mut out = Vec::new();
        r.drain_into(&mut out);
        assert_eq!(out.len(), 500);
        assert!(out.iter().enumerate().all(|(i, e)| e.midi == (i % 128) as u8));
        assert_eq!((r.written(), r.dropped()), (500, 0));
    }

    #[test]
    fn a_full_ring_drops_and_counts() {
        let (mut w, r) = note_bus();
        let many = vec![on(60); BUS_CAPACITY + 10];
        w.write(&many);
        assert_eq!((r.written(), r.dropped()), (BUS_CAPACITY as u64, 10));
    }
}
