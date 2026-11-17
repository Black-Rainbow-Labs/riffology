//! A safe handle on one tracker in the guitar model library. Make it off the audio thread; use it from one thread at
//! a time (the audio callback); drop it off the audio thread.

use crate::abi::{CgNote, CgTelemetry, CgTracker, GuitarAbi};
use std::os::raw::c_char;
use std::sync::Arc;

/// The tracker's shared settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerSettings {
    /// Gain before the model, -24 to +24 dB.
    pub input_gain_db: f32,
    /// Under this input level (dBFS, -80 to -20) no note starts.
    pub gate_db: f32,
    /// The note start threshold, 0.5 to 0.9. Lower starts notes earlier and finds more false notes.
    pub sensitivity: f32,
}

impl Default for TrackerSettings {
    fn default() -> Self {
        TrackerSettings { input_gain_db: 0.0, gate_db: -55.0, sensitivity: 0.7 }
    }
}

/// What the tracker reports about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerStatus {
    Off,
    Listening,
    NoInput,
    ModelFailed,
}

impl TrackerStatus {
    /// The status from the library's number.
    /// @param n - 0 to 3
    pub fn from_u8(n: u8) -> TrackerStatus {
        match n {
            1 => TrackerStatus::Listening,
            2 => TrackerStatus::NoInput,
            3 => TrackerStatus::ModelFailed,
            _ => TrackerStatus::Off,
        }
    }
}

/// One tracker.
pub struct ModelTracker {
    abi: Arc<GuitarAbi>,
    handle: *mut CgTracker,
    rate: f64,
}

// SAFETY: the library allows a handle to move between threads as long as one thread uses it at a time, which `&mut`
// on every call that touches the handle guarantees.
unsafe impl Send for ModelTracker {}

impl ModelTracker {
    /// Loads the model and starts a tracker.
    /// @param abi - the loaded library
    /// @param host_rate - the sample rate of the audio that will be pushed
    /// @param sync - true runs the model inside `push` (every run gives the same notes), false on a worker thread
    pub fn new(abi: Arc<GuitarAbi>, host_rate: f64, sync: bool) -> Result<ModelTracker, String> {
        let mut err = [0 as c_char; 512];
        // SAFETY: `err` holds `err.len()` bytes.
        let handle = unsafe { (abi.new)(host_rate, sync as u8, err.as_mut_ptr(), err.len()) };
        if handle.is_null() {
            // SAFETY: the library wrote a NUL terminated string into `err`, or left it zeroed.
            let msg = unsafe { std::ffi::CStr::from_ptr(err.as_ptr()) }.to_string_lossy().into_owned();
            return Err(if msg.is_empty() { "the guitar tracker did not start".to_string() } else { msg });
        }
        Ok(ModelTracker { abi, handle, rate: host_rate })
    }

    /// The sample rate the tracker was made for.
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Gives the tracker a block of mono input. Never blocks or allocates.
    /// @param mono - the samples
    pub fn push(&mut self, mono: &[f32]) {
        // SAFETY: the handle is live and `mono` is readable for its length.
        unsafe { (self.abi.push)(self.handle, mono.as_ptr(), mono.len()) };
    }

    /// Fills `out` with finished note events and returns how many. Events that do not fit wait for the next call.
    /// @param out - room for events
    pub fn drain(&mut self, out: &mut [CgNote]) -> usize {
        // SAFETY: the handle is live and `out` is writable for its length.
        unsafe { (self.abi.drain)(self.handle, out.as_mut_ptr(), out.len()) }
    }

    /// Sets gain, gate and sensitivity.
    /// @param s - the settings
    pub fn set_settings(&mut self, s: TrackerSettings) {
        // SAFETY: the handle is live.
        unsafe { (self.abi.settings)(self.handle, s.input_gain_db, s.gate_db, s.sensitivity) };
    }

    /// Ends every held note and clears the model state.
    pub fn reset(&mut self) {
        // SAFETY: the handle is live.
        unsafe { (self.abi.reset)(self.handle) };
    }

    /// The newest telemetry.
    pub fn telemetry(&mut self) -> CgTelemetry {
        let mut t = CgTelemetry::default();
        // SAFETY: the handle is live and `t` is writable.
        unsafe { (self.abi.telemetry)(self.handle, &mut t) };
        t
    }
}

impl Drop for ModelTracker {
    fn drop(&mut self) {
        // SAFETY: the handle came from `cg_tracker_new` and is freed once.
        unsafe { (self.abi.free)(self.handle) };
    }
}
