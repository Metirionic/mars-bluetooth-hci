//! Versioned persistence contract for decoded CS subevent-result frames.
//!
//! V1 frames are byte-compatible with the existing non-COBS HCI Postcard
//! representation: [`encode`](crate::event::hci_le_cs::persisted_frame::encode)
//! serializes through the same shared core as the FFI serializer's
//! `use_cobs = false` path, so a persisted frame is exactly an FFI
//! serialization envelope. The byte format is therefore jointly defined by the
//! declaration order of `libc::Serializable` and the serde derives of
//! [`SubeventResultEvent`](crate::event::hci_le_cs::subevent_result::SubeventResultEvent);
//! changing either changes the persisted bytes, is caught by the committed
//! golden fixtures, and requires a new declared version here. This module
//! deliberately does not expose the general FFI serialization enum as a
//! persistence contract.
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
use crate::libc::serialize_subevent_result_event_bytes;

/// Stable format name for persisted CS subevent-result frames.
pub const CS_SUBEVENT_FRAME_FORMAT: &str = "mars-hci-cs-subevent";

/// The first persisted CS subevent-result frame version.
///
/// Private on purpose: callers reference versions through
/// [`CURRENT_CS_SUBEVENT_FRAME_VERSION`] and the descriptor, while the decode
/// dispatch names the versions that actually have a decoder.
const V1_CS_SUBEVENT_FRAME_VERSION: u16 = 1;

/// Current persisted CS subevent-result frame version.
///
/// Bumping this constant requires a matching dispatch arm in [`decode`],
/// fixtures for the new version, and an explicit migration transition.
pub const CURRENT_CS_SUBEVENT_FRAME_VERSION: u16 = V1_CS_SUBEVENT_FRAME_VERSION;

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

/// Descriptor identifying the frame bytes that [`encode`] currently emits.
///
/// [`encode`] does not embed the descriptor or the version in the bytes: the
/// caller persists this descriptor alongside the encoded frame (for example,
/// in the storage record or file name) and passes the stored descriptor back
/// to [`decode`].
pub const CURRENT_CS_SUBEVENT_FRAME_DESCRIPTOR: FrameDescriptor<'static> =
    FrameDescriptor::new(CS_SUBEVENT_FRAME_FORMAT, CURRENT_CS_SUBEVENT_FRAME_VERSION);

/// Returns the descriptor identifying the frame bytes that [`encode`] emits.
///
/// See [`CURRENT_CS_SUBEVENT_FRAME_DESCRIPTOR`] for the out-of-band
/// persistence contract.
pub const fn current_frame_descriptor() -> FrameDescriptor<'static> {
    CURRENT_CS_SUBEVENT_FRAME_DESCRIPTOR
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
    #[error("could not encode persisted CS subevent frame: {0}")]
    Encode(#[source] postcard::Error),
    /// Postcard could not decode the declared frame representation.
    #[error("could not decode persisted CS subevent frame: {0}")]
    Decode(#[source] postcard::Error),
    /// The frame decoded but left unconsumed trailing bytes.
    #[error("persisted CS subevent frame has trailing bytes")]
    TrailingBytes,
    /// The decoded FFI transport value is not a CS subevent-result event.
    #[error("persisted CS subevent frame contains a different FFI value kind")]
    UnexpectedFrameKind,
}

/// Encodes one CS subevent-result event using the current persisted-frame
/// representation.
///
/// The bytes are identical to the FFI serializer's non-COBS output by
/// construction: both share one serialization core
/// (`crate::libc::serialize_subevent_result_event_bytes`). No descriptor or
/// version is embedded; persist the descriptor returned by
/// [`current_frame_descriptor`] alongside the bytes.
pub fn encode(event: &SubeventResultEvent) -> Result<Vec<u8>, FrameCodecError> {
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
/// must stay aligned with `libc::Serializable`'s; the golden fixtures and the
/// `malformed_or_non_subevent_frames_are_rejected` test catch any drift. Unlike
/// `Serializable`, the subevent variant is unboxed: postcard deserializes
/// `Box<T>` and `T` to identical bytes, and the unboxed form returns the large
/// event (~8 KB) without a heap allocation and without copying it back out.
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
/// part-written, or concatenated storage record, which is rejected instead of
/// silently decoding its first frame.
fn decode_v1(bytes: &[u8]) -> Result<SubeventResultEvent, FrameCodecError> {
    let (frame, trailing) = take_from_bytes(bytes).map_err(FrameCodecError::Decode)?;
    if !trailing.is_empty() {
        return Err(FrameCodecError::TrailingBytes);
    }
    match frame {
        PersistedWireFrame::SubeventResultEvent(event) => Ok(event),
        PersistedWireFrame::LogMessage(_) => Err(FrameCodecError::UnexpectedFrameKind),
    }
}

#[cfg(test)]
mod tests {
    use postcard::to_allocvec;

    use super::*;
    use crate::event::hci_le_cs::subevent_result::{Origin, test_messages};
    use crate::libc::SerializableRef;

    /// Committed-fixture root relative to this crate's manifest directory.
    const FIXTURE_ROOT: &str = "tests/fixtures/persisted-frames";

    fn representative_event() -> SubeventResultEvent {
        let message = test_messages::continue_event(
            0x01,
            0x05,
            0x01,
            &test_messages::mode1_basic_step_data(0x21, 0x12, 0x34),
        );
        SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator)
            .expect("representative event parses")
    }

    /// Returns the representative events whose encodings are the committed V1
    /// fixtures, one per CS Mode 0 through 3.
    ///
    /// Mode 3 combines the Mode 1 packet/timing section with the Mode 2 tone
    /// section in one step, the way the parser decodes it.
    fn representative_events_per_mode() -> Vec<(u8, SubeventResultEvent)> {
        let mode0 = SubeventResultEvent::try_from_with_origin(
            test_messages::continue_event(0x00, 0x05, 0x01, &test_messages::mode0_initiator_step_data(0x96, 0x00))
                .as_slice(),
            Origin::Unknown,
        )
        .expect("representative Mode 0 event parses");
        let mode1 = representative_event();
        let mode2 = SubeventResultEvent::try_from_with_origin(
            test_messages::continue_event(0x02, 0x05, 0x01, &test_messages::mode2_step_data()).as_slice(),
            Origin::Unknown,
        )
        .expect("representative Mode 2 event parses");
        let mode3_step_data = [
            test_messages::mode1_basic_step_data(0x21, 0x12, 0x34).as_slice(),
            test_messages::mode2_step_data().as_slice(),
        ]
        .concat();
        let mode3 = SubeventResultEvent::try_from_with_origin(
            test_messages::continue_event(0x03, 0x05, 0x01, &mode3_step_data).as_slice(),
            Origin::Initiator,
        )
        .expect("representative Mode 3 event parses");

        Vec::from([(0, mode0), (1, mode1), (2, mode2), (3, mode3)])
    }

    /// Returns the path of one committed fixture file.
    fn fixture_path(version: u16, mode: u8) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURE_ROOT)
            .join(format!("v{version}"))
            .join(format!("mode{mode}.postcard.hex"))
    }

    #[test]
    fn v1_encoding_matches_the_existing_hci_ffi_serializer() {
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
        assert!(matches!(
            decode(current_frame_descriptor(), &[0xFF]),
            Err(FrameCodecError::Decode(_))
        ));

        let log_message = to_allocvec(&SerializableRef::LogMessage("not a subevent")).expect("log serializes");
        assert!(matches!(
            decode(current_frame_descriptor(), &log_message),
            Err(FrameCodecError::UnexpectedFrameKind)
        ));

        let cobs_framed: Vec<u8> = crate::libc::serialize_subevent_result_event(&representative_event(), true).into();
        assert!(
            decode(current_frame_descriptor(), &cobs_framed).is_err(),
            "COBS-framed transport bytes must not decode as a persisted frame"
        );
    }

    #[test]
    fn representative_events_reproduce_the_committed_v1_fixtures() {
        for (mode, event) in representative_events_per_mode() {
            let expected = encode(&event).expect("representative event encodes");

            let hex =
                std::fs::read_to_string(fixture_path(V1_CS_SUBEVENT_FRAME_VERSION, mode)).expect("fixture is readable");
            let committed = hex::decode(hex.trim()).expect("fixture contains valid hexadecimal Postcard bytes");

            assert_eq!(
                expected, committed,
                "the Mode {mode} representative event reproduces the committed V1 fixture"
            );
        }
    }

    /// Regenerates the committed V1 fixtures from the representative events.
    ///
    /// Ignored because regenerating the compatibility fixtures is a
    /// deliberate, reviewed act: run
    /// `cargo test -p mars-bluetooth-hci regenerate_committed_v1_fixtures -- --ignored --nocapture`
    /// after a versioned representation change, then review the fixture diff
    /// as the compatibility record of that change.
    #[test]
    #[ignore = "regenerating fixtures is a deliberate, reviewed operation"]
    fn regenerate_committed_v1_fixtures() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURE_ROOT)
            .join(format!("v{}", V1_CS_SUBEVENT_FRAME_VERSION));
        std::fs::create_dir_all(&directory).expect("fixture directory is created");

        for (mode, event) in representative_events_per_mode() {
            let bytes = encode(&event).expect("representative event encodes");
            std::fs::write(
                directory.join(format!("mode{mode}.postcard.hex")),
                format!("{}\n", hex::encode(bytes)),
            )
            .expect("fixture is written");
        }
    }
}
