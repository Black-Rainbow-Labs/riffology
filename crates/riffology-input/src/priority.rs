//! Raises a thread to audio priority, the way an audio driver runs its callback thread: the "Pro Audio" MMCSS task on
//! Windows and the user interactive QoS class on macOS. On Windows the thread also opts out of power throttling, so
//! a hybrid CPU does not move it to an efficiency core. The WAV replay thread uses it so a replay is timed like live
//! input. Ported from Chordical's guitar tracker, which raises its model worker the same way.

/// Raises the calling thread's scheduling priority. Failure is ignored: the thread still runs at normal priority.
pub fn raise_current_thread() {
    platform::raise();
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::System::Threading::{
        AvSetMmThreadCharacteristicsW, GetCurrentThread, SetThreadInformation, ThreadPowerThrottling,
        THREAD_POWER_THROTTLING_CURRENT_VERSION, THREAD_POWER_THROTTLING_EXECUTION_SPEED,
        THREAD_POWER_THROTTLING_STATE,
    };

    /// Joins the calling thread to the MMCSS "Pro Audio" task and turns off execution speed throttling for it.
    pub fn raise() {
        let name: Vec<u16> = "Pro Audio".encode_utf16().chain(std::iter::once(0)).collect();
        let mut index = 0u32;
        let state = THREAD_POWER_THROTTLING_STATE {
            Version: THREAD_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: 0,
        };
        // SAFETY: `name` is a NUL terminated UTF-16 string that outlives the call; `index` is a valid out pointer;
        // `state` is a valid THREAD_POWER_THROTTLING_STATE of the size passed, for the calling thread's pseudo handle.
        unsafe {
            AvSetMmThreadCharacteristicsW(name.as_ptr(), &mut index);
            SetThreadInformation(
                GetCurrentThread(),
                ThreadPowerThrottling,
                &state as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<THREAD_POWER_THROTTLING_STATE>() as u32,
            );
        }
    }
}

#[cfg(target_vendor = "apple")]
mod platform {
    /// QOS_CLASS_USER_INTERACTIVE from <sys/qos.h>.
    const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;

    extern "C" {
        fn pthread_set_qos_class_self_np(class: u32, relative_priority: i32) -> i32;
    }

    /// Moves the calling thread to the user interactive QoS class.
    pub fn raise() {
        // SAFETY: a plain libSystem call on the calling thread with constant arguments.
        unsafe {
            pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
        }
    }
}

#[cfg(not(any(windows, target_vendor = "apple")))]
mod platform {
    /// Nothing to do on other platforms.
    pub fn raise() {}
}
