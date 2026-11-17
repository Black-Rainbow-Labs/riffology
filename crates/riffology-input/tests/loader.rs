//! The library loader: the pinned library loads and names its model; a file with another hash and a file that is
//! not a library are refused with a message that says what to do.

mod common;

use riffology_input::abi::{check_hash, pinned_sha256, sha256_of, AbiError, GuitarAbi};

#[test]
fn the_pinned_library_loads_and_names_its_model() {
    let abi = common::abi();
    assert_eq!(abi.model_version(), "r2-final-s2");
    let info: serde_json::Value = serde_json::from_str(abi.info_json()).unwrap();
    assert_eq!(info["abiVersion"], 1);
    assert_eq!(info["midiRange"], serde_json::json!([36, 90]));
    assert_eq!(sha256_of(abi.path()).unwrap(), pinned_sha256().unwrap());
}

#[test]
fn a_library_with_another_hash_is_refused() {
    let abi = common::abi();
    let err = check_hash(abi.path(), Some("00")).unwrap_err();
    assert!(matches!(err, AbiError::HashMismatch { .. }));
    assert!(err.to_string().contains("tools/fetch-guitar-abi.ps1"), "{err}");
}

#[test]
fn a_file_that_is_not_a_library_is_refused() {
    let path = std::env::temp_dir().join("riffology-not-a-library.dll");
    std::fs::write(&path, b"not a library").unwrap();
    let r = GuitarAbi::load_from(&path);
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(r, Err(AbiError::LoadFailed(_))), "{:?}", r.err());
}
