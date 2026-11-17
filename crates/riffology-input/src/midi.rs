//! MIDI input through midir: a MIDI guitar, a keyboard, or a DAW sending MIDI. Note ons and offs on every channel go
//! to the note bus with `Source::Midi`. MIDI has no audio clock, so its times are seconds since `origin`, which the
//! app sets to the moment the audio input started.

use riffology_core::{BusWriter, NoteEvent, Source};
use std::time::Instant;

/// The MIDI input port names.
pub fn list_midi_inputs() -> Vec<String> {
    let Ok(m) = midir::MidiInput::new("riffology-list") else { return vec![] };
    m.ports().iter().filter_map(|p| m.port_name(p).ok()).collect()
}

/// A connected MIDI input.
pub struct MidiInput {
    _conn: midir::MidiInputConnection<()>,
    pub port: String,
}

/// Turns a MIDI message into a note event, or None for any other message.
/// @param msg - the message bytes
/// @param t - its time in seconds
pub fn note_event_of(msg: &[u8], t: f64) -> Option<NoteEvent> {
    if msg.len() < 3 {
        return None;
    }
    let (kind, note, vel) = (msg[0] & 0xF0, msg[1] & 0x7F, msg[2] & 0x7F);
    let on = kind == 0x90 && vel > 0;
    if kind != 0x90 && kind != 0x80 {
        return None;
    }
    Some(NoteEvent {
        midi: note,
        velocity: if on { vel } else { 0 },
        is_on: on,
        source: Source::Midi,
        onset_s: t,
        heard_s: t,
    })
}

impl MidiInput {
    /// Connects to the port whose name contains `name`, or the first port.
    /// @param name - part of the port name, or None
    /// @param bus - the bus writer, moved into the MIDI callback
    /// @param origin - the moment the clock counts from
    pub fn open(name: Option<&str>, mut bus: BusWriter, origin: Instant) -> Result<MidiInput, String> {
        let m = midir::MidiInput::new("riffology").map_err(|e| e.to_string())?;
        let ports = m.ports();
        let port = ports
            .iter()
            .find(|p| name.is_none_or(|n| m.port_name(p).map(|pn| pn.contains(n)).unwrap_or(false)))
            .cloned()
            .ok_or("no MIDI input port")?;
        let port_name = m.port_name(&port).unwrap_or_default();
        let conn = m
            .connect(
                &port,
                "riffology-in",
                move |_, msg, _| {
                    if let Some(e) = note_event_of(msg, origin.elapsed().as_secs_f64()) {
                        bus.write(&[e]);
                    }
                },
                (),
            )
            .map_err(|e| e.to_string())?;
        Ok(MidiInput { _conn: conn, port: port_name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_on_with_zero_velocity_is_a_note_off() {
        assert!(note_event_of(&[0x90, 60, 100], 1.0).unwrap().is_on);
        assert!(!note_event_of(&[0x91, 60, 0], 1.0).unwrap().is_on);
        assert!(!note_event_of(&[0x80, 60, 64], 1.0).unwrap().is_on);
        assert!(note_event_of(&[0xB0, 7, 100], 1.0).is_none());
    }
}
