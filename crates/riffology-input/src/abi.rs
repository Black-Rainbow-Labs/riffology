//! The guitar model library. `chordical_guitar_abi` is built from Chordical's MIDI Guitar tracker and published as a
//! zip on this repository's releases. `tools/fetch-guitar-abi.ps1` downloads it into `vendor/guitar-abi/<platform>/`.
//! This module finds the file, checks its SHA-256 against `tools/guitar-abi.json`, loads it, checks the ABI version
//! and resolves every function. The C declarations are in `chordical_guitar_abi.h` inside the zip.

use sha2::{Digest, Sha256};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The ABI version this code was written for.
pub const ABI_VERSION: u32 = 1;

/// The pinned library release, compiled in so a binary checks the file it was built against.
pub const PIN_JSON: &str = include_str!("../../../tools/guitar-abi.json");

/// One note event as the library writes it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CgNote {
    pub note: u8,
    pub velocity: u8,
    pub is_on: u8,
    pub reserved: u8,
    pub offset: u32,
    pub frame: u64,
    pub onset_s: f64,
}

/// The tracker's live values as the library writes them.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CgTelemetry {
    pub status: u8,
    pub gate_open: u8,
    pub reserved: [u8; 2],
    pub dropped_blocks: u32,
    pub active: [u64; 2],
    pub input_peak: f32,
    pub input_rms_db: f32,
    pub infer_ms: f32,
    pub infer_ms_max: f32,
    pub infer_ms_p99: f32,
    pub reserved2: u32,
    pub hops: u64,
}

/// The opaque tracker handle.
#[repr(C)]
pub struct CgTracker {
    _private: [u8; 0],
}

type FnVersion = unsafe extern "C" fn() -> u32;
type FnInfo = unsafe extern "C" fn() -> *const c_char;
type FnNew = unsafe extern "C" fn(f64, u8, *mut c_char, usize) -> *mut CgTracker;
type FnPush = unsafe extern "C" fn(*mut CgTracker, *const f32, usize) -> u8;
type FnDrain = unsafe extern "C" fn(*mut CgTracker, *mut CgNote, usize) -> usize;
type FnSettings = unsafe extern "C" fn(*mut CgTracker, f32, f32, f32) -> u8;
type FnReset = unsafe extern "C" fn(*mut CgTracker) -> u8;
type FnTelemetry = unsafe extern "C" fn(*mut CgTracker, *mut CgTelemetry) -> u8;
type FnFree = unsafe extern "C" fn(*mut CgTracker);

/// Why the library could not be used. Every message says what to do next.
#[derive(Debug, Clone, PartialEq)]
pub enum AbiError {
    NotFound(Vec<PathBuf>),
    HashMismatch { path: PathBuf, expected: String, found: String },
    LoadFailed(String),
    WrongVersion { expected: u32, found: u32 },
    MissingSymbol(String),
}

impl std::fmt::Display for AbiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AbiError::NotFound(paths) => write!(f, "Guitar model library not found (looked in {}). Run tools/fetch-guitar-abi.ps1 (or .sh).", paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")),
            AbiError::HashMismatch { path, expected, found } => write!(f, "Guitar model library {} has SHA-256 {found}, but tools/guitar-abi.json pins {expected}. Run tools/fetch-guitar-abi.ps1 again.", path.display()),
            AbiError::LoadFailed(e) => write!(f, "Guitar model library failed to load: {e}"),
            AbiError::WrongVersion { expected, found } => write!(f, "Guitar model library has ABI version {found}; this build of Riffology needs version {expected}."),
            AbiError::MissingSymbol(s) => write!(f, "Guitar model library has no function {s}."),
        }
    }
}

impl std::error::Error for AbiError {}

/// The loaded library and its functions. Shared with `Arc`; the library stays loaded while any tracker uses it.
pub struct GuitarAbi {
    pub(crate) new: FnNew,
    pub(crate) push: FnPush,
    pub(crate) drain: FnDrain,
    pub(crate) settings: FnSettings,
    pub(crate) reset: FnReset,
    pub(crate) telemetry: FnTelemetry,
    pub(crate) free: FnFree,
    info: String,
    path: PathBuf,
    /// Never unloaded: the model runtime keeps a thread pool alive inside the library after every tracker is freed,
    /// and unloading would unmap code those threads run.
    _lib: std::mem::ManuallyDrop<libloading::Library>,
}

/// The platform folder name used in `vendor/guitar-abi` and in the release zips.
pub fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows-x64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos-arm64"
    } else if cfg!(target_os = "macos") {
        "macos-x64"
    } else {
        "linux-x64"
    }
}

/// The library file name on this platform.
pub fn library_file_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "chordical_guitar_abi.dll"
    } else if cfg!(target_os = "macos") {
        "libchordical_guitar_abi.dylib"
    } else {
        "libchordical_guitar_abi.so"
    }
}

/// The SHA-256 `tools/guitar-abi.json` pins for this platform, or None when it pins none.
pub fn pinned_sha256() -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(PIN_JSON).ok()?;
    v["platforms"][platform()]["sha256"].as_str().map(str::to_string)
}

/// The places the library is looked for, in order: `RIFFOLOGY_GUITAR_ABI`, `vendor/guitar-abi/<platform>/` of this
/// checkout, and the folder of the running program.
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("RIFFOLOGY_GUITAR_ABI") {
        out.push(PathBuf::from(p));
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    out.push(repo.join("vendor/guitar-abi").join(platform()).join(library_file_name()));
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        out.push(dir.join(library_file_name()));
    }
    out
}

/// The SHA-256 of a file as lowercase hex.
/// @param path - the file
pub fn sha256_of(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect())
}

impl GuitarAbi {
    /// Finds, checks and loads the library from the first candidate path that exists.
    pub fn load() -> Result<Arc<GuitarAbi>, AbiError> {
        let paths = candidate_paths();
        let path = paths.iter().find(|p| p.is_file()).cloned().ok_or_else(|| AbiError::NotFound(paths.clone()))?;
        let unchecked = std::env::var("RIFFOLOGY_GUITAR_ABI_UNCHECKED").map(|v| v == "1").unwrap_or(false);
        if !unchecked {
            check_hash(&path, pinned_sha256().as_deref())?;
        }
        GuitarAbi::load_from(&path).map(Arc::new)
    }

    /// Loads a library file without the hash check and checks its ABI version.
    /// @param path - the library file
    pub fn load_from(path: &Path) -> Result<GuitarAbi, AbiError> {
        // SAFETY: loading a native library runs its initializers. The file was checked against the pinned hash by
        // `load`, or named on purpose by the caller.
        let lib = unsafe { libloading::Library::new(path) }.map_err(|e| AbiError::LoadFailed(e.to_string()))?;
        let version: FnVersion = symbol(&lib, "cg_abi_version")?;
        // SAFETY: `cg_abi_version` takes nothing and returns a u32 in every version of the ABI.
        let found = unsafe { version() };
        if found != ABI_VERSION {
            return Err(AbiError::WrongVersion { expected: ABI_VERSION, found });
        }
        let info_fn: FnInfo = symbol(&lib, "cg_model_info")?;
        // SAFETY: version 1 returns a NUL terminated string that lives as long as the process.
        let info = unsafe { std::ffi::CStr::from_ptr(info_fn()) }.to_string_lossy().into_owned();
        Ok(GuitarAbi {
            new: symbol(&lib, "cg_tracker_new")?,
            push: symbol(&lib, "cg_tracker_push")?,
            drain: symbol(&lib, "cg_tracker_drain")?,
            settings: symbol(&lib, "cg_tracker_set_settings")?,
            reset: symbol(&lib, "cg_tracker_reset")?,
            telemetry: symbol(&lib, "cg_tracker_telemetry")?,
            free: symbol(&lib, "cg_tracker_free")?,
            info,
            path: path.to_path_buf(),
            _lib: std::mem::ManuallyDrop::new(lib),
        })
    }

    /// The library's description of itself and its model, as JSON text.
    pub fn info_json(&self) -> &str {
        &self.info
    }

    /// The model run name, for example `r2-final-s2`.
    pub fn model_version(&self) -> String {
        serde_json::from_str::<serde_json::Value>(&self.info)
            .ok()
            .and_then(|v| v["modelVersion"].as_str().map(str::to_string))
            .unwrap_or_default()
    }

    /// The file the library was loaded from.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Checks a file's SHA-256 against the pinned value.
/// @param path - the file
/// @param expected - the pinned hash, or None when nothing is pinned for this platform
pub fn check_hash(path: &Path, expected: Option<&str>) -> Result<(), AbiError> {
    let found = sha256_of(path).map_err(|e| AbiError::LoadFailed(e.to_string()))?;
    match expected {
        Some(e) if e.eq_ignore_ascii_case(&found) => Ok(()),
        Some(e) => Err(AbiError::HashMismatch { path: path.to_path_buf(), expected: e.to_string(), found }),
        None => Err(AbiError::HashMismatch {
            path: path.to_path_buf(),
            expected: format!("nothing for {}", platform()),
            found,
        }),
    }
}

/// Resolves one function by name and copies the pointer out of the library borrow.
/// @param lib - the library
/// @param name - the exported name
fn symbol<T: Copy>(lib: &libloading::Library, name: &str) -> Result<T, AbiError> {
    // SAFETY: each name is resolved with the function type the header declares for it. The pointer is used only while
    // `GuitarAbi` holds the library.
    unsafe { lib.get::<T>(name.as_bytes()) }.map(|s| *s).map_err(|_| AbiError::MissingSymbol(name.to_string()))
}
