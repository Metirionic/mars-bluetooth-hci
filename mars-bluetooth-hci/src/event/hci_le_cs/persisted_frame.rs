//! Versioned persistence contract for decoded CS subevent-result frames.
//!
//! V1 frames are byte-compatible with the existing non-COBS HCI Postcard
//! representation: [`encode`](crate::event::hci_le_cs::persisted_frame::encode)
//! serializes through the same shared core as the FFI serializer's
//! `use_cobs = false` path, so a persisted frame is exactly an FFI
//! serialization envelope. The byte format is therefore jointly defined by the
//! declaration order of `libc::SerializableRef` — the order the owning
//! `libc::Serializable` and the private decode mirror below match
//! byte-identically — and the serde derives of
//! [`SubeventResultEvent`](crate::event::hci_le_cs::subevent_result::SubeventResultEvent).
//! Any change that alters the representative frames' bytes is caught by the
//! committed golden fixtures, and the payload enums' variant tags are pinned
//! by test, so reordering or renumbering a payload enum is caught too.
//! Appending an enum variant keeps the representative fixtures byte-identical
//! and the old tags valid — but breaks readers of the new variant, so it is
//! still a wire change that only review against this module's version policy
//! catches. This module deliberately does not expose the general FFI
//! serialization enum as a persistence contract.
//!
//! The frame descriptor is persisted out of band: [`encode`](crate::event::hci_le_cs::persisted_frame::encode)
//! emits bare frame bytes with no in-band version, and the caller stores the
//! descriptor returned by
//! [`current_frame_descriptor`](crate::event::hci_le_cs::persisted_frame::current_frame_descriptor)
//! alongside the bytes it writes (for example, in the storage record or file
//! name). [`decode`](crate::event::hci_le_cs::persisted_frame::decode) never
//! guesses a codec from the bytes; it dispatches only on the declared
//! descriptor.
//!
//! Version policy: [`decode`](crate::event::hci_le_cs::persisted_frame::decode)
//! accepts exactly the versions that have an explicit arm in its dispatch —
//! normally only the current version. During a release migration, the
//! immediately preceding version's decoder and fixture directory are retained
//! here, and both are removed in the same change once the migration completes.
//! A historical decoder is retained in this module only while an explicit
//! migration needs it, never in the general HCI serialization implementation.
//! The decision is recorded in ADR-0003.
//!
//! Known V1 cost: the event's fixed step array is serialized in full, so every
//! frame is roughly 13 KB regardless of `step_count`. This is the price of
//! byte-compatibility with the FFI representation; shrinking the frames would
//! change the bytes and therefore requires a new declared version.

extern crate alloc;

use alloc::vec::Vec;

use postcard::take_from_bytes;
use serde::Deserialize;
use thiserror::Error;

use super::subevent_result::SubeventResultEvent;
use crate::event::ParseError;
use crate::libc::serialize_subevent_result_event_bytes;

/// Stable format name for persisted CS subevent-result frames.
pub const CS_SUBEVENT_FRAME_FORMAT: &str = "mars-hci-cs-subevent";

/// The first persisted CS subevent-result frame version.
///
/// Private on purpose: callers reference versions through
/// [`CURRENT_CS_SUBEVENT_FRAME_VERSION`], [`FIRST_CS_SUBEVENT_FRAME_VERSION`],
/// and the descriptor, while the decode dispatch names the versions that
/// actually have a decoder.
const V1_CS_SUBEVENT_FRAME_VERSION: u16 = 1;

/// The first persisted CS subevent-result frame version; no earlier frame
/// version exists.
pub const FIRST_CS_SUBEVENT_FRAME_VERSION: u16 = V1_CS_SUBEVENT_FRAME_VERSION;

/// Current persisted CS subevent-result frame version.
///
/// Bumping this constant requires a matching dispatch arm in [`decode`],
/// fixtures for the new version, and an explicit migration transition.
pub const CURRENT_CS_SUBEVENT_FRAME_VERSION: u16 = V1_CS_SUBEVENT_FRAME_VERSION;

/// The versions this build retains a decoder for, in dispatch order.
///
/// Declared beside the dispatch so an editor of its arms sees the list; the
/// `retained_versions_match_the_decode_dispatch` test asserts it cannot
/// disagree with the dispatch's behavior, which is what the fixture suite's
/// retention probe relies on.
#[cfg(test)]
const RETAINED_VERSIONS: [u16; 1] = [V1_CS_SUBEVENT_FRAME_VERSION];

/// Identifies one persisted CS subevent-result frame format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameDescriptor<'a> {
    /// Stable frame-format name.
    format_name: &'a str,
    /// Explicit frame-format version.
    version: u16,
}

impl<'a> FrameDescriptor<'a> {
    /// Creates a descriptor for a declared frame format.
    pub const fn new(format_name: &'a str, version: u16) -> Self {
        Self { format_name, version }
    }

    /// Returns the stable frame-format name.
    pub const fn format_name(self) -> &'a str {
        self.format_name
    }

    /// Returns the frame-format version.
    pub const fn version(self) -> u16 {
        self.version
    }
}

/// Returns the descriptor identifying the frame bytes that [`encode`] emits.
///
/// [`encode`] does not embed the descriptor or the version in the bytes: the
/// caller persists this descriptor alongside the encoded frame (for example,
/// in the storage record or file name) and passes the stored descriptor back
/// to [`decode`].
pub const fn current_frame_descriptor() -> FrameDescriptor<'static> {
    FrameDescriptor::new(CS_SUBEVENT_FRAME_FORMAT, CURRENT_CS_SUBEVENT_FRAME_VERSION)
}

/// Errors returned while encoding or decoding persisted CS subevent-result frames.
#[derive(Debug, Error)]
pub enum FrameCodecError {
    /// The declared frame-format name is not the CS subevent-result format.
    #[error("unsupported persisted CS subevent frame format")]
    UnsupportedFormat,
    /// The declared frame-format version has no available decoder.
    #[error("unsupported persisted CS subevent frame version: {version}")]
    UnsupportedVersion {
        /// Unsupported declared version.
        version: u16,
    },
    /// Postcard could not encode the current frame representation.
    #[error("could not encode persisted CS subevent frame")]
    Encode(#[source] postcard::Error),
    /// Postcard could not decode the declared frame representation.
    ///
    /// A truncated frame fails decode inside postcard and surfaces here.
    #[error("could not decode persisted CS subevent frame")]
    Decode(#[source] postcard::Error),
    /// The frame decoded but left unconsumed trailing bytes.
    #[error("persisted CS subevent frame has trailing bytes")]
    TrailingBytes,
    /// The frame's leading byte is not a V1 envelope tag: the bytes were
    /// written by a different envelope schema, or the record is corrupted —
    /// either way it is not a truncated or padded record of this one.
    #[error("persisted CS subevent frame has unknown envelope tag `{tag}`")]
    UnknownEnvelopeTag {
        /// The leading byte that is not a known envelope tag.
        tag: u8,
    },
    /// The frame's event exceeds the fixed step array or the antenna-path
    /// table; see [`SubeventResultEvent::validate_invariants`].
    #[error("persisted CS subevent frame decodes to an event that violates the subevent-result invariants")]
    InvalidEvent(#[source] ParseError),
    /// The decoded FFI transport value is not a CS subevent-result event.
    #[error("persisted CS subevent frame contains a different FFI value kind")]
    UnexpectedFrameKind,
}

/// Encodes one CS subevent-result event using the current persisted-frame
/// representation.
///
/// The event's counts are validated first (see
/// [`SubeventResultEvent::validate_invariants`]), so every frame this codec
/// writes replays through [`decode`]; unlike the FFI serializer, `encode`
/// rejects events that exceed the fixed arrays. The bytes are identical to
/// the FFI serializer's non-COBS output by construction: both share one
/// serialization core (`crate::libc::serialize_subevent_result_event_bytes`).
/// No descriptor or version is embedded; persist the descriptor returned by
/// [`current_frame_descriptor`] alongside the bytes.
pub fn encode(event: &SubeventResultEvent) -> Result<Vec<u8>, FrameCodecError> {
    event.validate_invariants().map_err(FrameCodecError::InvalidEvent)?;
    serialize_subevent_result_event_bytes(event).map_err(FrameCodecError::Encode)
}

/// Decodes one CS subevent-result event using its declared persisted-frame
/// descriptor.
///
/// Dispatch is explicit and version-pinned: a declared version decodes only
/// through the arm named for it, and every other version is rejected without
/// reading the bytes. During a release migration, the retained arm of the
/// immediately preceding version keeps that version's fixtures decodable;
/// when the arm is removed after the migration, the fixture directory is
/// removed in the same change.
pub fn decode(descriptor: FrameDescriptor<'_>, bytes: &[u8]) -> Result<SubeventResultEvent, FrameCodecError> {
    if descriptor.format_name() != CS_SUBEVENT_FRAME_FORMAT {
        return Err(FrameCodecError::UnsupportedFormat);
    }
    match descriptor.version() {
        V1_CS_SUBEVENT_FRAME_VERSION => decode_v1(bytes),
        version => Err(FrameCodecError::UnsupportedVersion { version }),
    }
}

/// Private wire mirror of `libc::Serializable` for decoding persisted frames.
///
/// Variant tags are serde declaration-order indices, so the variant order here
/// must stay aligned with `libc::Serializable`'s and `SerializableRef`'s; the
/// golden fixtures and the `malformed_or_non_subevent_frames_are_rejected` test
/// catch any drift. Unlike `Serializable`, the subevent variant is unboxed:
/// postcard deserializes `Box<T>` and `T` to identical bytes, and the unboxed
/// form returns the large event (~16 KB) without a heap allocation and without
/// copying it back out.
#[expect(
    clippy::large_enum_variant,
    reason = "boxing the subevent variant is exactly what this mirror avoids"
)]
#[derive(Deserialize)]
enum PersistedWireFrame<'d> {
    /// A CS subevent result (declaration index 0, as in `Serializable`).
    SubeventResultEvent(SubeventResultEvent),
    /// A firmware log message (declaration index 1, as in `Serializable`).
    ///
    /// The payload is deserialized so the variant stays wire-compatible with
    /// `Serializable::LogMessage` (a unit variant would change the bytes) and
    /// is then discarded — a log message is not a subevent-result frame, so
    /// decoding it fails with [`FrameCodecError::UnexpectedFrameKind`].
    #[expect(dead_code)]
    #[serde(borrow)]
    LogMessage(&'d str),
}

/// Decodes the first persisted-frame representation.
///
/// The whole input must be one frame: leftover bytes mean a padded,
/// over-written, or concatenated storage record, which is rejected instead of
/// silently decoding its first frame (a truncated record instead fails decode
/// inside postcard with [`FrameCodecError::Decode`]). The decoded event must
/// satisfy the count bounds of [`SubeventResultEvent::validate_invariants`];
/// deeper semantic consistency of the stored event is not re-derived.
fn decode_v1(bytes: &[u8]) -> Result<SubeventResultEvent, FrameCodecError> {
    // V1 knows exactly two envelope tags. Any other leading byte was written
    // by a different envelope schema or is corrupted — either way not a
    // truncated or padded record of this one. (The mirror below must stay
    // aligned with these tags.)
    if let Some(&tag) = bytes.first()
        && tag > 0x01
    {
        return Err(FrameCodecError::UnknownEnvelopeTag { tag });
    }
    let (frame, trailing) = take_from_bytes(bytes).map_err(FrameCodecError::Decode)?;
    if !trailing.is_empty() {
        return Err(FrameCodecError::TrailingBytes);
    }
    match frame {
        PersistedWireFrame::SubeventResultEvent(event) => {
            event.validate_invariants().map_err(FrameCodecError::InvalidEvent)?;
            Ok(event)
        }
        PersistedWireFrame::LogMessage(_) => Err(FrameCodecError::UnexpectedFrameKind),
    }
}

#[cfg(test)]
#[path = "../../../tests/support/fixture_layout.rs"]
mod fixture_support;

#[cfg(test)]
mod tests {
    use postcard::to_allocvec;

    use super::fixture_support::{
        CONFIG_COMPLETE_FIXTURE_FILE, REPRESENTATIVE_STEP_MODES, fixture_dir, fixture_file_name, read_fixture,
    };
    use super::*;
    use crate::event::hci_le_cs::subevent_result::{
        ModeRoleSpecificInfoKind, Origin, RoundTripTimeRoleTimingKind, test_messages,
    };
    use crate::event::{
        ExtensionSlot, ProcedureAbortReason, ProcedureDoneStatus, SubeventAbortReason, SubeventDoneStatus,
        ToneQualityIndicator,
    };
    use crate::libc::{Serializable, SerializableRef};

    /// Returns the representative Mode 1 event exercised by the codec tests.
    fn representative_event() -> SubeventResultEvent {
        representative_event_for_mode(1)
    }

    /// Returns the representative frames whose current-version encodings are
    /// the committed current-version fixtures, keyed by their fixture file
    /// names.
    fn representative_fixtures() -> Vec<(String, SubeventResultEvent)> {
        let mut fixtures: Vec<(String, SubeventResultEvent)> = REPRESENTATIVE_STEP_MODES
            .map(|mode| (fixture_file_name(mode), representative_event_for_mode(mode)))
            .to_vec();
        fixtures.push((
            CONFIG_COMPLETE_FIXTURE_FILE.to_string(),
            representative_config_complete_event(),
        ));
        fixtures
    }

    /// Builds the canonical representative event for one CS mode.
    ///
    /// Mode 0 and Mode 2 parse without a known origin; Mode 1 and Mode 3 need
    /// the initiator origin for their role-specific timing. The Mode 1 and
    /// Mode 3 representatives use the PBR/RTT step layout, whose packet phase
    /// correction terms are a superset of the basic layout's fields.
    fn representative_event_for_mode(mode: u8) -> SubeventResultEvent {
        let message = match mode {
            0 => test_messages::continue_event(0x00, 0x05, 0x01, &test_messages::mode0_initiator_step_data(0x96, 0x00)),
            1 => test_messages::continue_event(
                0x01,
                0x05,
                0x01,
                &test_messages::mode1_pbr_rtt_step_data(0x21, 0x12, 0x34),
            ),
            2 => test_messages::continue_event(0x02, 0x05, 0x01, &test_messages::mode2_step_data()),
            3 => test_messages::continue_event(0x03, 0x05, 0x01, &test_messages::mode3_pbr_rtt_step_data()),
            other => panic!("no representative event for Mode {other}"),
        };
        let origin = match mode {
            0 | 2 => Origin::Unknown,
            _ => Origin::Initiator,
        };
        SubeventResultEvent::try_from_with_origin(message.as_slice(), origin).expect("representative event parses")
    }

    /// Builds the canonical config-complete representative event, pinning the
    /// initial-metadata half of the wire format.
    fn representative_config_complete_event() -> SubeventResultEvent {
        let message = test_messages::config_complete_event();
        SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown)
            .expect("representative config-complete event parses")
    }

    /// Returns the path of one committed fixture file.
    fn fixture_path(version: u16, file_name: &str) -> std::path::PathBuf {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        fixture_dir(manifest, version).join(file_name)
    }

    /// Verifies the FFI serializer wrapper stays wired to the shared
    /// serialization core that [`encode`] also calls, so the FFI byte stream
    /// and the persisted byte stream remain identical.
    #[test]
    fn ffi_wrapper_stays_wired_to_the_shared_serialization_core() {
        let event = representative_event();

        let existing: Vec<u8> = crate::libc::serialize_subevent_result_event(&event, false).into();
        let persisted = encode(&event).expect("persisted serializer works");

        assert_eq!(persisted, existing);
    }

    #[test]
    fn v1_decoding_round_trips_the_current_hci_encoding() {
        let event = representative_event();
        let bytes = encode(&event).expect("persisted serializer works");

        let decoded = decode(current_frame_descriptor(), &bytes).expect("current descriptor decodes");

        assert_eq!(encode(&decoded).expect("decoded event re-encodes"), bytes);
    }

    #[test]
    fn v1_decoding_rejects_frames_with_trailing_bytes() {
        let event = representative_event();
        let mut bytes = encode(&event).expect("persisted serializer works");
        bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        assert!(matches!(
            decode(current_frame_descriptor(), &bytes),
            Err(FrameCodecError::TrailingBytes)
        ));
    }

    #[test]
    fn v1_decoding_rejects_concatenated_frames() {
        let event = representative_event();
        let mut bytes = encode(&event).expect("persisted serializer works");
        bytes.extend_from_slice(&encode(&event).expect("persisted serializer works"));

        assert!(matches!(
            decode(current_frame_descriptor(), &bytes),
            Err(FrameCodecError::TrailingBytes)
        ));
    }

    #[test]
    fn unknown_descriptor_values_are_rejected_without_decoding() {
        let event = representative_event();
        let bytes = encode(&event).expect("persisted serializer works");

        assert!(matches!(
            decode(
                FrameDescriptor::new("different-format", CURRENT_CS_SUBEVENT_FRAME_VERSION),
                &bytes
            ),
            Err(FrameCodecError::UnsupportedFormat)
        ));
        assert!(matches!(
            decode(
                FrameDescriptor::new(CS_SUBEVENT_FRAME_FORMAT, CURRENT_CS_SUBEVENT_FRAME_VERSION + 1),
                &bytes
            ),
            Err(FrameCodecError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn malformed_or_non_subevent_frames_are_rejected() {
        // A leading byte that is not a V1 envelope tag: written by a different
        // envelope schema, or a corrupted record — either way not a truncated
        // or padded V1 record. Tag 2 would be the first tag of a third
        // envelope variant, so the bound here tracks the mirror's variant
        // list.
        assert!(matches!(
            decode(current_frame_descriptor(), &[0xFF]),
            Err(FrameCodecError::UnknownEnvelopeTag { tag: 0xFF })
        ));
        assert!(matches!(
            decode(current_frame_descriptor(), &[0x02]),
            Err(FrameCodecError::UnknownEnvelopeTag { tag: 0x02 })
        ));

        let log_message = to_allocvec(&SerializableRef::LogMessage("not a subevent")).expect("log serializes");
        assert!(matches!(
            decode(current_frame_descriptor(), &log_message),
            Err(FrameCodecError::UnexpectedFrameKind)
        ));

        // A log record with corrupt payload bytes fails decode inside postcard
        // rather than surfacing as the wrong value kind.
        assert!(matches!(
            decode(current_frame_descriptor(), &[0x01, 0x02, 0xC3, 0x28]),
            Err(FrameCodecError::Decode(_))
        ));

        let cobs_framed: Vec<u8> = crate::libc::serialize_subevent_result_event(&representative_event(), true).into();
        assert!(
            decode(current_frame_descriptor(), &cobs_framed).is_err(),
            "COBS-framed transport bytes must not decode as a persisted frame"
        );
    }

    #[test]
    fn encoding_rejects_events_with_broken_invariants() {
        let mut event = representative_event();
        event.step_count = 200;

        assert!(matches!(encode(&event), Err(FrameCodecError::InvalidEvent(_))));
    }

    #[test]
    fn v1_frames_match_the_owned_boxed_ffi_envelope() {
        let event = representative_event();

        let boxed = to_allocvec(&Serializable::SubeventResultEvent(Box::new(event.clone())))
            .expect("owned envelope serializes");
        let persisted = encode(&event).expect("persisted serializer works");

        assert_eq!(persisted, boxed);
    }

    #[test]
    fn v1_decoding_rejects_events_with_broken_invariants() {
        let event = representative_event();

        // Step count past the fixed step array: splice the step-count varint
        // (envelope offset 12 in the representative frame) into the varint
        // for 1000.
        let mut bytes = encode(&event).expect("persisted serializer works");
        assert_eq!(
            bytes[12], 0x01,
            "the representative frame's step-count varint sits at envelope offset 12"
        );
        bytes.splice(12..13, [0xE8, 0x07]);
        assert!(matches!(
            decode(current_frame_descriptor(), &bytes),
            Err(FrameCodecError::InvalidEvent(error)) if matches!(error, ParseError::ExceededMaxStepCount)
        ));

        // Antenna-path count past the fixed tone table: patch the
        // antenna-path-count varint (envelope offset 11).
        let mut bytes = encode(&event).expect("persisted serializer works");
        assert_eq!(
            bytes[11], 0x01,
            "the representative frame's antenna-path-count varint sits at envelope offset 11"
        );
        bytes[11] = 0x05;
        assert!(matches!(
            decode(current_frame_descriptor(), &bytes),
            Err(FrameCodecError::InvalidEvent(error)) if matches!(error, ParseError::ExceededMaxAntennaPathCount)
        ));
    }

    #[test]
    fn v1_decoding_rejects_truncated_frames() {
        let event = representative_event();
        let bytes = encode(&event).expect("persisted serializer works");

        assert!(matches!(
            decode(current_frame_descriptor(), &bytes[..bytes.len() / 2]),
            Err(FrameCodecError::Decode(_))
        ));
    }

    #[test]
    fn retained_versions_match_the_decode_dispatch() {
        // Every listed version decodes — its decoder never reports
        // `UnsupportedVersion` — and every unlisted version up to the current
        // one is rejected by the dispatch. The fixture suite's retention probe
        // relies on exactly this reservation, so the two cannot drift.
        for version in FIRST_CS_SUBEVENT_FRAME_VERSION..=CURRENT_CS_SUBEVENT_FRAME_VERSION {
            let rejected = matches!(
                decode(FrameDescriptor::new(CS_SUBEVENT_FRAME_FORMAT, version), &[]),
                Err(FrameCodecError::UnsupportedVersion { .. })
            );
            assert_eq!(
                RETAINED_VERSIONS.contains(&version),
                !rejected,
                "version {version}: the retained list and the decode dispatch disagree"
            );
        }
    }

    /// Pins every payload enum's variant tags to their current wire values.
    ///
    /// A tag's wire value is its serde declaration index: reordering or
    /// renumbering a payload enum changes the persisted bytes and requires a
    /// new declared version. Appending a variant keeps these tags valid but
    /// is still a wire change — the fixtures and this test pin the old tags,
    /// not the policy that governs additions.
    #[test]
    fn payload_enum_variant_tags_are_pinned() {
        macro_rules! assert_tag {
            ($expected:expr, $variant:expr) => {
                assert_eq!(
                    to_allocvec(&$variant).expect("variant serializes")[0],
                    $expected
                );
            };
        }

        assert_tag!(0, Origin::Unknown);
        assert_tag!(1, Origin::Initiator);
        assert_tag!(2, Origin::Reflector);
        assert_tag!(0, ProcedureDoneStatus::AllComplete);
        assert_tag!(1, ProcedureDoneStatus::Partial);
        assert_tag!(2, ProcedureDoneStatus::Aborted);
        assert_tag!(3, ProcedureDoneStatus::Reserved);
        assert_tag!(0, ProcedureAbortReason::NoAbort);
        assert_tag!(1, ProcedureAbortReason::LocalHostOrRemoteRequest);
        assert_tag!(2, ProcedureAbortReason::LessThan15Channels);
        assert_tag!(3, ProcedureAbortReason::ChannelMapUpdateInstantPassed);
        assert_tag!(4, ProcedureAbortReason::Unspecified);
        assert_tag!(0, SubeventDoneStatus::AllComplete);
        assert_tag!(1, SubeventDoneStatus::Partial);
        assert_tag!(2, SubeventDoneStatus::Aborted);
        assert_tag!(3, SubeventDoneStatus::Reserved);
        assert_tag!(0, SubeventAbortReason::NoAbort);
        assert_tag!(1, SubeventAbortReason::LocalHostOrRemoteRequest);
        assert_tag!(2, SubeventAbortReason::NoCsSyncReceived);
        assert_tag!(3, SubeventAbortReason::SchedulingConflictOrLimitedResources);
        assert_tag!(4, SubeventAbortReason::Unspecified);
        assert_tag!(1, ToneQualityIndicator::Unavailable);
        assert_tag!(4, ToneQualityIndicator::High);
        assert_tag!(3, ToneQualityIndicator::Medium);
        assert_tag!(2, ToneQualityIndicator::Low);
        assert_tag!(0, ExtensionSlot::NotPresent);
        assert_tag!(1, ExtensionSlot::NotExpectedPresent);
        assert_tag!(2, ExtensionSlot::ExpectedPresent);
        assert_tag!(3, ExtensionSlot::Reserved);
        assert_tag!(0, ModeRoleSpecificInfoKind::Mode0Reflector);
        assert_tag!(1, ModeRoleSpecificInfoKind::Mode1Initiator);
        assert_tag!(2, ModeRoleSpecificInfoKind::Mode1InitiatorPbrRtt);
        assert_tag!(3, ModeRoleSpecificInfoKind::Mode1Reflector);
        assert_tag!(4, ModeRoleSpecificInfoKind::Mode1ReflectorPbrRtt);
        assert_tag!(5, ModeRoleSpecificInfoKind::Mode2);
        assert_tag!(6, ModeRoleSpecificInfoKind::Mode3Initiator);
        assert_tag!(7, ModeRoleSpecificInfoKind::Mode3InitiatorPbrRtt);
        assert_tag!(8, ModeRoleSpecificInfoKind::Mode3Reflector);
        assert_tag!(9, ModeRoleSpecificInfoKind::Mode3ReflectorPbrRtt);
        assert_tag!(10, ModeRoleSpecificInfoKind::Mode0Initiator);
        assert_tag!(0, RoundTripTimeRoleTimingKind::Unavailable);
        assert_tag!(1, RoundTripTimeRoleTimingKind::TimeOfArrivalTimeOfDepartureInitiator);
        assert_tag!(2, RoundTripTimeRoleTimingKind::TimeOfDepartureTimeOfArrivalReflector);
    }

    #[test]
    fn representative_frames_reproduce_the_committed_current_fixtures() {
        for (file_name, event) in representative_fixtures() {
            let expected = encode(&event).expect("representative event encodes");
            let committed = read_fixture(&fixture_path(CURRENT_CS_SUBEVENT_FRAME_VERSION, &file_name));

            assert_eq!(
                expected, committed,
                "the {file_name} representative event reproduces the committed current-version fixture"
            );
        }
    }

    /// Regenerates the committed current-version fixtures from the
    /// representative events.
    ///
    /// Ignored because regenerating the compatibility fixtures is a
    /// deliberate, reviewed act: run
    /// `cargo test -p mars-bluetooth-hci regenerate_committed_current_fixtures -- --ignored --nocapture`
    /// after a versioned representation change, then review the fixture diff
    /// as the compatibility record of that change. Retained migration-source
    /// fixture directories are never touched: they are removed together with
    /// their decoder arm once the migration completes (ADR-0003).
    #[test]
    #[ignore = "regenerating fixtures is a deliberate, reviewed operation"]
    fn regenerate_committed_current_fixtures() {
        let directory = fixture_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            CURRENT_CS_SUBEVENT_FRAME_VERSION,
        );
        std::fs::create_dir_all(&directory).expect("fixture directory is created");

        for (file_name, event) in representative_fixtures() {
            let bytes = encode(&event).expect("representative event encodes");
            std::fs::write(directory.join(&file_name), format!("{}\n", hex::encode(bytes)))
                .expect("fixture is written");
        }
    }
}
