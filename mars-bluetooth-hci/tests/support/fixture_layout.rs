//! Shared definition of the committed persisted-frame fixture layout.
//!
//! Included by the codec's unit tests (inside the crate) and by the
//! integration fixture suite (outside it) through `#[path]`, so both sides
//! use one definition of the fixture root, naming, and decoding instead of
//! parallel keep-in-sync copies. This file must stay free of crate-dependent
//! imports so it compiles in both contexts.

// Each compilation unit uses a subset of this module's items — the module is
// the shared owner, so unused-on-one-side helpers are expected.
#![allow(dead_code)]

/// Fixture-root path relative to the crate's manifest directory.
pub const FIXTURE_ROOT: &str = "tests/fixtures/persisted-frames";

/// Every retained codec version carries one representative fixture per step
/// mode in this list.
pub const REPRESENTATIVE_STEP_MODES: [u8; 4] = [0, 1, 2, 3];

/// The config-complete fixture pins the initial-metadata half of the wire
/// format, which carries no steps and hence has no mode file.
pub const CONFIG_COMPLETE_FIXTURE_FILE: &str = "config-complete.postcard.hex";

/// Returns the fixture version directory name for a frame version.
pub fn version_dir_name(version: u16) -> String {
    format!("v{version}")
}

/// Returns the fixture file name for one representative step mode.
pub fn fixture_file_name(step_mode: u8) -> String {
    format!("mode{step_mode}.postcard.hex")
}

/// Returns the fixture root directory below a crate's manifest directory.
pub fn fixture_root(manifest_dir: &std::path::Path) -> std::path::PathBuf {
    manifest_dir.join(FIXTURE_ROOT)
}

/// Returns one version's fixture directory below a crate's manifest directory.
pub fn fixture_dir(manifest_dir: &std::path::Path, version: u16) -> std::path::PathBuf {
    fixture_root(manifest_dir).join(version_dir_name(version))
}

/// Reads and decodes one committed fixture's hexadecimal representation.
pub fn read_fixture(path: &std::path::Path) -> Vec<u8> {
    let hex = std::fs::read_to_string(path).expect("fixture is readable");
    hex::decode(hex.trim()).expect("fixture contains valid hexadecimal Postcard bytes")
}
