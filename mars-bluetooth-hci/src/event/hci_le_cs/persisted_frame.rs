//! Versioned persistence contract for decoded CS subevent-result frames.
//!
//! This module owns the format identity and version dispatch for frames stored
//! by MARS. It deliberately does not expose the general FFI serialization enum
//! as a persistence contract.
//!
//! An incompatible change to persisted bytes or their interpretation requires a
//! new declared version, compatibility fixtures, and an explicit migration
//! transition. Normal operation encodes the current version and replays only
//! the currently canonical version. A historical decoder is retained in the
//! HCI serialization implementation only while an explicit migration needs it.

extern crate alloc;

use alloc::vec::Vec;

use postcard::{take_from_bytes, to_allocvec};
use thiserror::Error;

use super::subevent_result::SubeventResultEvent;
use crate::event::ParseError;
use crate::libc::{Serializable, SerializableRef};

/// Stable format name for persisted CS subevent-result frames.
pub const CS_SUBEVENT_FRAME_FORMAT: &str = "mars-hci-cs-subevent";

/// Current persisted CS subevent-result frame version.
pub const CURRENT_CS_SUBEVENT_FRAME_VERSION: u16 = 1;

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

/// Descriptor emitted by the current CS subevent-result frame encoder.
pub const CURRENT_CS_SUBEVENT_FRAME_DESCRIPTOR: FrameDescriptor<'static> =
    FrameDescriptor::new(CS_SUBEVENT_FRAME_FORMAT, CURRENT_CS_SUBEVENT_FRAME_VERSION);

/// Returns the descriptor emitted by the current frame encoder.
pub const fn current_frame_descriptor() -> FrameDescriptor<'static> {
    CURRENT_CS_SUBEVENT_FRAME_DESCRIPTOR
}

/// Errors returned while selecting a persisted CS subevent-result frame codec.
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
    /// The frame's event exceeds the fixed step array or the antenna-path
    /// table, or a reported step's payload kind does not carry its mode's
    /// data; see [`SubeventResultEvent::validate_invariants`].
    #[error("persisted CS subevent frame decodes to an event that violates the subevent-result invariants")]
    InvalidEvent(#[source] ParseError),
    /// The decoded FFI transport value is not a CS subevent-result event.
    #[error("persisted CS subevent frame contains a different FFI value kind")]
    UnexpectedFrameKind,
}

/// Validates that a declared descriptor selects the current frame codec.
///
/// Future versions extend this function with explicit version dispatch. It
/// never guesses a codec from the encoded bytes.
pub fn validate_descriptor(descriptor: FrameDescriptor<'_>) -> Result<(), FrameCodecError> {
    if descriptor.format_name() != CS_SUBEVENT_FRAME_FORMAT {
        return Err(FrameCodecError::UnsupportedFormat);
    }
    if descriptor.version() != CURRENT_CS_SUBEVENT_FRAME_VERSION {
        return Err(FrameCodecError::UnsupportedVersion {
            version: descriptor.version(),
        });
    }
    Ok(())
}

/// Encodes one CS subevent-result event using the current persisted-frame
/// representation.
///
/// The event's counts and step payload kinds are validated first (see
/// [`SubeventResultEvent::validate_invariants`]), so every frame this codec
/// writes replays through [`decode`].
pub fn encode(event: &SubeventResultEvent) -> Result<Vec<u8>, FrameCodecError> {
    event.validate_invariants().map_err(FrameCodecError::InvalidEvent)?;
    to_allocvec(&SerializableRef::SubeventResultEvent(event)).map_err(FrameCodecError::Encode)
}

/// Decodes one CS subevent-result event using its declared persisted-frame
/// descriptor.
pub fn decode(descriptor: FrameDescriptor<'_>, bytes: &[u8]) -> Result<SubeventResultEvent, FrameCodecError> {
    validate_descriptor(descriptor)?;
    match descriptor.version() {
        CURRENT_CS_SUBEVENT_FRAME_VERSION => decode_v1(bytes),
        _ => unreachable!("descriptor validation permits only the current frame version"),
    }
}

/// Decodes the first persisted-frame representation through the current HCI
/// Postcard decoder.
///
/// The whole input must be one frame: leftover bytes mean a padded,
/// over-written, or concatenated storage record, which is rejected instead of
/// silently decoding its first frame. The decoded event must satisfy the
/// invariants of [`SubeventResultEvent::validate_invariants`].
fn decode_v1(bytes: &[u8]) -> Result<SubeventResultEvent, FrameCodecError> {
    let (frame, trailing) = take_from_bytes(bytes).map_err(FrameCodecError::Decode)?;
    if !trailing.is_empty() {
        return Err(FrameCodecError::TrailingBytes);
    }
    match frame {
        Serializable::SubeventResultEvent(event) => {
            event.validate_invariants().map_err(FrameCodecError::InvalidEvent)?;
            Ok(*event)
        }
        Serializable::LogMessage(_) => Err(FrameCodecError::UnexpectedFrameKind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ParseError;
    use crate::event::hci_le_cs::constants::le_subevent_code;
    use crate::event::hci_le_cs::subevent_result::{ModeRoleSpecificInfo, ModeRoleSpecificInfoKind, Origin, Step};

    fn representative_event() -> SubeventResultEvent {
        let message = [
            le_subevent_code::CS_SUBEVENT_RESULT_CONTINUE,
            0x01,
            0x00,
            0x07,
            0x00,
            0x00,
            0x00,
            0x01,
            0x01,
            0x01,
            0x05,
            0x06,
            0x21,
            0x80,
            0x34,
            0x12,
            0x34,
            0x02,
        ];
        SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator)
            .expect("representative event parses")
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
    fn v1_decoding_rejects_frames_with_trailing_bytes() {
        // Appended garbage (padding, over-write, or a second record) must not
        // silently decode as one valid event.
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
    fn encoding_rejects_events_with_broken_invariants() {
        let mut event = representative_event();
        event.step_count = 200;

        assert!(matches!(encode(&event), Err(FrameCodecError::InvalidEvent(_))));
    }

    #[test]
    fn encoding_rejects_fabricated_pre_migration_steps() {
        // A pre-PR build parsed a 0xFF slot by collapsing it, leaving a
        // fabricated default step (mode 0x00, kind Mode2) inside the step
        // window; replaying such a record must fail loudly instead of
        // returning fabricated measurement data.
        let mut event = representative_event();
        event.steps[1] = Step {
            mode: 0x00,
            channel: 0x05,
            info: ModeRoleSpecificInfo {
                kind: ModeRoleSpecificInfoKind::Mode2,
                ..Default::default()
            },
        };
        event.step_count = 2;

        assert!(matches!(
            encode(&event),
            Err(FrameCodecError::InvalidEvent(error))
                if matches!(error, ParseError::StepKindModeMismatch { index: 1, mode: 0x00, .. })
        ));
    }
}
