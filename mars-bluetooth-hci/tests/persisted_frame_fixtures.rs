//! Golden fixtures for the versioned CS subevent persistence contract.
//!
//! Each `vN/modeN.postcard.hex` path declares the frame version and the Mode
//! carried by its raw, non-COBS Postcard bytes. The test discovers this layout
//! so a new wire version adds only its fixtures and codec support, not another
//! hand-maintained Rust fixture list.
//!
//! The retained version window mirrors the persisted-frame codec's dispatch
//! in both directions: every fixture must decode through the descriptor its
//! path declares, so a version directory may exist exactly while the codec
//! retains that version's decoder arm — and every retained decoder arm must
//! have its fixture directory (probed through decode itself, which rejects
//! unsupported versions before reading bytes). When an arm is removed after a
//! migration, the fixture directory is removed in the same change.
//!
//! The module is available when the `persisted-frame` feature is enabled —
//! part of the default build. Both this suite's gate and the module gate in
//! `mars-bluetooth-hci/src/event/hci_le_cs/mod.rs` reference the same
//! feature, so they cannot drift apart.
#![cfg(feature = "persisted-frame")]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mars_bluetooth_hci::event::hci_le_cs::persisted_frame::test_support::{FIXTURE_ROOT, REPRESENTATIVE_STEP_MODES};
use mars_bluetooth_hci::event::hci_le_cs::persisted_frame::{
    CS_SUBEVENT_FRAME_FORMAT, FrameCodecError, FrameDescriptor, current_frame_descriptor, decode, encode,
};
use mars_bluetooth_hci::event::hci_le_cs::subevent_result::{ModeRoleSpecificInfoKind, SubeventResultEvent};

/// The first persisted frame version; no earlier version directory is valid.
const FIRST_FRAME_VERSION: u16 = 1;

/// One fixture discovered from its version directory and mode filename.
#[derive(Debug)]
struct Fixture {
    path: PathBuf,
    descriptor: FrameDescriptor<'static>,
    expected_step_mode: u8,
}

/// Returns the absolute fixture-root path for this test invocation.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_ROOT)
}

/// Extracts the explicitly declared codec version from a `vN` directory.
fn directory_version(path: &Path) -> u16 {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture version directory is valid UTF-8");
    let version = name
        .strip_prefix('v')
        .expect("fixture version directory starts with `v`");
    let version: u16 = version
        .parse()
        .expect("fixture version directory has an integer version");

    assert_eq!(name, format!("v{version}"), "fixture version directory is canonical");
    version
}

/// Extracts the expected step mode from a `modeN.postcard.hex` fixture name.
fn fixture_mode(path: &Path) -> u8 {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture filename is valid UTF-8");
    let mode = name
        .strip_prefix("mode")
        .and_then(|name| name.strip_suffix(".postcard.hex"))
        .expect("fixture filename follows `modeN.postcard.hex`");
    let mode: u8 = mode.parse().expect("fixture filename has an integer mode");

    assert_eq!(
        name,
        format!("mode{mode}.postcard.hex"),
        "fixture filename is canonical"
    );
    assert!(
        REPRESENTATIVE_STEP_MODES.contains(&mode),
        "fixture has a supported step mode: {name}"
    );
    mode
}

/// Discovers the bounded set of retained current and migration-source fixtures.
///
/// The directory convention is deliberately validated here: a malformed or
/// incomplete fixture set must fail CI rather than silently lose coverage.
/// The retention bound mirrors the codec's decode dispatch: the current
/// version and, during a migration, its immediate source version, which the
/// codec keeps decodable through its retained decoder arm.
fn fixtures_at(root: &Path) -> Vec<Fixture> {
    let mut version_directories: Vec<_> = fs::read_dir(root)
        .expect("fixture root exists")
        .map(|entry| entry.expect("fixture root entry is readable"))
        .inspect(|entry| {
            assert!(
                entry.file_type().expect("fixture entry type is readable").is_dir(),
                "fixture-root entry is a version directory: {}",
                entry.path().display()
            );
        })
        .collect();
    version_directories.sort_by_key(|entry| entry.file_name());

    assert!(
        !version_directories.is_empty(),
        "at least one fixture version is committed"
    );

    let mut fixtures = Vec::new();
    let mut versions = BTreeSet::new();
    for version_directory in version_directories {
        let version = directory_version(&version_directory.path());
        versions.insert(version);
        let mut files: Vec<_> = fs::read_dir(version_directory.path())
            .expect("fixture version directory is readable")
            .map(|entry| entry.expect("fixture entry is readable"))
            .inspect(|entry| {
                assert!(
                    entry.file_type().expect("fixture entry type is readable").is_file(),
                    "fixture-version entry is a fixture file: {}",
                    entry.path().display()
                );
            })
            .collect();
        files.sort_by_key(|entry| entry.file_name());

        let mut found_modes = [false; REPRESENTATIVE_STEP_MODES.len()];
        for file in files {
            let mode = fixture_mode(&file.path());
            found_modes[usize::from(mode)] = true;
            fixtures.push(Fixture {
                path: file.path(),
                descriptor: FrameDescriptor::new(CS_SUBEVENT_FRAME_FORMAT, version),
                expected_step_mode: mode,
            });
        }

        assert!(
            found_modes.iter().all(|found| *found),
            "fixture version {version} contains one fixture for every Mode 0 through 3"
        );
    }

    let current_version = current_frame_descriptor().version();
    assert!(
        versions.contains(&current_version),
        "fixtures contain the current codec version {current_version}"
    );
    // The first persisted frame version has no migration source, so a `v0`
    // directory has never been valid.
    let source_version = current_version
        .checked_sub(1)
        .filter(|&version| version >= FIRST_FRAME_VERSION);
    for version in &versions {
        assert!(
            *version == current_version || Some(*version) == source_version,
            "fixture version {version} is the current version or its immediate migration source"
        );
    }

    // The converse direction: every version the codec still decodes must have
    // committed fixtures. Decode rejects unsupported versions before reading
    // the bytes, so probing it with empty bytes reports exactly the versions
    // with a retained decoder arm — the dispatch stays the single source of
    // truth.
    for version in FIRST_FRAME_VERSION..=current_version {
        let retained = !matches!(
            decode(FrameDescriptor::new(CS_SUBEVENT_FRAME_FORMAT, version), &[]),
            Err(FrameCodecError::UnsupportedVersion { .. })
        );
        assert!(
            !retained || versions.contains(&version),
            "retained codec version {version} has committed fixtures"
        );
    }

    fixtures
}

/// Discovers fixtures committed with this crate.
fn fixtures() -> Vec<Fixture> {
    fixtures_at(&fixture_root())
}

/// Decodes a fixture's reviewable hexadecimal representation into Postcard bytes.
fn fixture_bytes(fixture: &Fixture) -> Vec<u8> {
    let hex = fs::read_to_string(&fixture.path).expect("fixture is readable");
    hex::decode(hex.trim()).expect("fixture contains valid hexadecimal Postcard bytes")
}

/// Decodes one fixture using exactly the descriptor declared by its path.
fn decode_fixture(fixture: &Fixture) -> (Vec<u8>, SubeventResultEvent) {
    let bytes = fixture_bytes(fixture);
    let event = decode(fixture.descriptor, &bytes).expect("fixture decodes with its declared descriptor");
    (bytes, event)
}

/// Returns whether a step's payload kind is one of the kinds that carry the
/// given mode's data.
///
/// Guards against a degenerate fixture whose parse leaves the steps at their
/// defaults: `Step::default()` has mode 0 with the `Mode2` payload kind, which
/// would pass a Mode-0 fixture's mode assertion vacuously.
fn carries_its_modes_payload(kind: ModeRoleSpecificInfoKind, mode: u8) -> bool {
    match mode {
        0 => matches!(
            kind,
            ModeRoleSpecificInfoKind::Mode0Initiator | ModeRoleSpecificInfoKind::Mode0Reflector
        ),
        1 => matches!(
            kind,
            ModeRoleSpecificInfoKind::Mode1Initiator
                | ModeRoleSpecificInfoKind::Mode1InitiatorPbrRtt
                | ModeRoleSpecificInfoKind::Mode1Reflector
                | ModeRoleSpecificInfoKind::Mode1ReflectorPbrRtt
        ),
        2 => matches!(kind, ModeRoleSpecificInfoKind::Mode2),
        3 => matches!(
            kind,
            ModeRoleSpecificInfoKind::Mode3Initiator
                | ModeRoleSpecificInfoKind::Mode3InitiatorPbrRtt
                | ModeRoleSpecificInfoKind::Mode3Reflector
                | ModeRoleSpecificInfoKind::Mode3ReflectorPbrRtt
        ),
        mode => panic!("fixture mode {mode} is outside the expected step modes"),
    }
}

#[test]
/// Ensures every retained fixture remains readable through its declared codec.
fn every_fixture_decodes_with_its_declared_descriptor() {
    for fixture in fixtures() {
        let (_, event) = decode_fixture(&fixture);

        assert!(
            event.step_count > 0,
            "{} fixture carries at least one step",
            fixture.path.display()
        );
        assert_eq!(
            event.steps[0].mode,
            fixture.expected_step_mode,
            "{} fixture has its expected step mode",
            fixture.path.display()
        );
        assert!(
            carries_its_modes_payload(event.steps[0].info.kind, fixture.expected_step_mode),
            "{} fixture's first step carries its mode's payload kind",
            fixture.path.display()
        );
    }
}

#[test]
/// Locks current encoder output without requiring old migration fixtures to re-encode.
fn current_encoder_matches_current_descriptor_fixtures() {
    let current_descriptor = current_frame_descriptor();
    let current_fixtures: Vec<_> = fixtures()
        .into_iter()
        .filter(|fixture| fixture.descriptor == current_descriptor)
        .collect();

    for fixture in current_fixtures {
        let (bytes, event) = decode_fixture(&fixture);
        let encoded = encode(&event).expect("decoded fixture re-encodes");

        assert_eq!(
            encoded,
            bytes,
            "{} fixture matches the current encoder",
            fixture.path.display()
        );
    }
}

#[cfg(test)]
mod fixture_layout_tests {
    use super::*;

    /// Writes the minimal valid filenames required for fixture-layout tests.
    fn write_fixture_set(root: &Path, version: u16, modes: &[u8]) {
        let directory = root.join(format!("v{version}"));
        fs::create_dir(&directory).expect("fixture version directory is created");
        for mode in modes {
            fs::write(directory.join(format!("mode{mode}.postcard.hex")), "00\n").expect("fixture file is written");
        }
    }

    /// Uses the real current descriptor while constructing an invalid source version.
    fn current_version() -> u16 {
        current_frame_descriptor().version()
    }

    #[test]
    #[should_panic(expected = "fixtures contain the current codec version")]
    fn rejects_a_fixture_set_without_the_current_version() {
        let root = tempfile::tempdir().expect("temporary fixture root is created");
        write_fixture_set(root.path(), current_version() + 1, &REPRESENTATIVE_STEP_MODES);

        let _ = fixtures_at(root.path());
    }

    #[test]
    #[should_panic(expected = "immediate migration source")]
    fn rejects_a_non_immediate_migration_source_version() {
        let root = tempfile::tempdir().expect("temporary fixture root is created");
        write_fixture_set(root.path(), current_version(), &REPRESENTATIVE_STEP_MODES);
        write_fixture_set(root.path(), current_version() + 1, &REPRESENTATIVE_STEP_MODES);

        let _ = fixtures_at(root.path());
    }

    #[test]
    #[should_panic(expected = "immediate migration source")]
    fn rejects_a_version_zero_directory() {
        let root = tempfile::tempdir().expect("temporary fixture root is created");
        write_fixture_set(root.path(), current_version(), &REPRESENTATIVE_STEP_MODES);
        write_fixture_set(root.path(), 0, &REPRESENTATIVE_STEP_MODES);

        let _ = fixtures_at(root.path());
    }

    #[test]
    #[should_panic(expected = "fixture-root entry is a version directory")]
    fn rejects_a_stray_root_file() {
        let root = tempfile::tempdir().expect("temporary fixture root is created");
        fs::write(root.path().join("unexpected.txt"), "unexpected").expect("stray file is written");

        let _ = fixtures_at(root.path());
    }

    #[test]
    #[should_panic(expected = "fixture filename follows")]
    fn rejects_a_malformed_fixture_filename() {
        let root = tempfile::tempdir().expect("temporary fixture root is created");
        let directory = root.path().join(format!("v{}", current_version()));
        fs::create_dir(&directory).expect("fixture version directory is created");
        fs::write(directory.join("mode0.hex"), "00\n").expect("malformed fixture is written");

        let _ = fixtures_at(root.path());
    }

    #[test]
    #[should_panic(expected = "contains one fixture for every Mode")]
    fn rejects_an_incomplete_fixture_set() {
        let root = tempfile::tempdir().expect("temporary fixture root is created");
        write_fixture_set(root.path(), current_version(), &[0]);

        let _ = fixtures_at(root.path());
    }
}
