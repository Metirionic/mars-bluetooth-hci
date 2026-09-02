use core::array::TryFromSliceError;

use safer_ffi::derive_ReprC;
use serde::{Deserialize, Serialize};

use crate::constants::{
    extension_slot, frequency_compensation, procedure_abort_reason, reference_power_level, subevent_abort_reason,
    tone_quality_indicator,
};

/// Channel-sounding specific HCI LE events.
pub mod hci_le_cs;
/// Parse error kinds.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// Slice could not be converted.
    #[error("slice could not be converted")]
    TryFromSliceError(#[from] TryFromSliceError),
    /// The subevent type is not supported.
    #[error("unsupported subevent type")]
    UnsupportedSubevent,
    /// The parsed mode type is not valid.
    #[error("invalid mode type `{0}` at byte `{1}`")]
    InvalidModeType(u8, usize),
    /// The parsed step data length does not match the expected mode-specific length.
    #[error("invalid step data length `{1}` for mode `{0}`, expected `{2}`")]
    InvalidStepDataLength(u8, usize, usize),
    /// The event origin is required for the selected mode but is unknown.
    #[error("origin is unknown for mode `{0}`")]
    UnknownOriginForMode(u8),
    /// The node's role is not "initiator".
    #[error("node's role is not initiator")]
    RoleNotInitiator,
    /// The reference power level is not applicable.
    #[error("reference power level not applicable")]
    ReferencePowerLevelNotApplicable,
    /// A reserved value was encountered - cannot be parsed.
    #[error("reserved value cannot be parsed")]
    ReservedValue,
    /// Exceeded maximum step count.
    #[error("exceeded maximum step count")]
    ExceededMaxStepCount,
    /// Exceeded the maximum antenna path count.
    #[error("exceeded maximum antenna path count")]
    ExceededMaxAntennaPathCount,
    /// The message ended before the subevent header or a step header it
    /// declares.
    #[error("message too short: `{0}` bytes")]
    TooShort(usize),
    /// A step header declared more data than the message carries.
    #[error("step `{index}` declares `{declared}` data bytes but only `{available}` follow the header")]
    TruncatedStepData {
        /// The zero-based index of the offending step slot.
        index: usize,
        /// The data length the step header declares.
        declared: usize,
        /// The bytes actually available after the step header.
        available: usize,
    },
}

/// The relative frequency error compensation.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[derive_ReprC]
#[repr(C)]
pub struct FrequencyCompensation {
    /// The compensation value in steps of 0.01 ppm.
    value: u16,
}

impl From<u16> for FrequencyCompensation {
    fn from(value: u16) -> Self {
        Self { value }
    }
}

impl TryFrom<FrequencyCompensation> for f32 {
    type Error = ParseError;

    fn try_from(value: FrequencyCompensation) -> Result<Self, Self::Error> {
        if value.value == frequency_compensation::ROLE_NOT_INITIATOR {
            return Err(ParseError::RoleNotInitiator);
        }

        Ok(sign_extend_15(value.value) as f32 / 100.0)
    }
}

/// Sign-extend a 15-bit two's-complement value to [`i16`].
///
/// The Channel Sounding frequency-offset fields — `Frequency_Compensation`
/// (subevent-result header) and Mode 0 `Measured_Freq_Offset` (step data) —
/// are 15-bit signed values in a 16-bit container: bit 15 is reserved and
/// bit 14 is the sign bit (Bluetooth Core Spec, Vol 4, Part E, §7.7.65.44).
/// The 16-bit value `0xC000` is a separate "not available" sentinel that the
/// caller must reject before calling this. Sign-extends from bit 14 — the
/// plain `as i16` cast (sign bit 15) is wrong for negative offsets.
pub(crate) fn sign_extend_15(raw: u16) -> i16 {
    let value = raw & 0x7FFF;
    if value & 0x4000 != 0 {
        (value | 0x8000) as i16
    } else {
        value as i16
    }
}

/// A reference power level in dBm.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[derive_ReprC]
#[repr(C)]
pub struct ReferencePowerLevel {
    /// The reference power level value in dBm, from -127 to 20.
    value: i8,
}

impl From<i8> for ReferencePowerLevel {
    fn from(value: i8) -> Self {
        Self { value }
    }
}

impl TryFrom<ReferencePowerLevel> for f32 {
    type Error = ParseError;

    fn try_from(value: ReferencePowerLevel) -> Result<Self, Self::Error> {
        if value.value == reference_power_level::NOT_APPLICABLE {
            return Err(ParseError::ReferencePowerLevelNotApplicable);
        }

        if (value.value > reference_power_level::MAX_DBM) || (value.value < reference_power_level::MIN_DBM) {
            return Err(ParseError::ReservedValue);
        }

        Ok(value.value as f32)
    }
}

/// Done status (shared by procedures and subevents).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[derive_ReprC]
#[repr(u8)]
pub enum DoneStatus {
    /// All procedures/subevents are complete.
    AllComplete = 0x00,
    /// Procedures/subevents are partially complete. More are going to follow.
    Partial = 0x01,
    /// The procedure/subevent was aborted.
    Aborted = 0x0F,
    /// Reserved status (catch-all for unknown values).
    Reserved = 0xFF,
}

impl From<u8> for DoneStatus {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::AllComplete,
            0x01 => Self::Partial,
            0x0F => Self::Aborted,
            _ => Self::Reserved,
        }
    }
}

/// Type alias for DoneStatus used in procedure context.
pub type ProcedureDoneStatus = DoneStatus;
/// Type alias for DoneStatus used in subevent context.
pub type SubeventDoneStatus = DoneStatus;

/// Information wrapper for done status and abort reason.
pub struct DoneInfo<D, A>(pub D, pub A);

/// Procedure information: done status and abort reason.
pub type ProcedureInfo = DoneInfo<DoneStatus, ProcedureAbortReason>;
/// Subevent information: done status and abort reason.
pub type SubeventInfo = DoneInfo<DoneStatus, SubeventAbortReason>;

impl From<(u8, u8)> for ProcedureInfo {
    fn from(value: (u8, u8)) -> Self {
        let (done_status, abort_reason) = value;
        let status = DoneStatus::from(done_status);
        let abort = match status {
            DoneStatus::Aborted => ProcedureAbortReason::from(abort_reason),
            _ => ProcedureAbortReason::NoAbort,
        };
        Self(status, abort)
    }
}

impl From<(u8, u8)> for SubeventInfo {
    fn from(value: (u8, u8)) -> Self {
        let (done_status, abort_reason) = value;
        let status = DoneStatus::from(done_status);
        let abort = match status {
            DoneStatus::Aborted => SubeventAbortReason::from(abort_reason),
            _ => SubeventAbortReason::NoAbort,
        };
        Self(status, abort)
    }
}

/// Reasons for aborting procedures.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[derive_ReprC]
#[repr(u8)]
pub enum ProcedureAbortReason {
    /// The procedure was not aborted.
    NoAbort,
    /// Abort caused by local host or a remote request.
    LocalHostOrRemoteRequest,
    /// Less than 15 channels where used for measurement.
    LessThan15Channels,
    /// The channel map update instant has passed.
    ChannelMapUpdateInstantPassed,
    /// Unspecified abort reason.
    Unspecified,
    /// Reserved abort reason.
    Reserved,
}

impl From<u8> for ProcedureAbortReason {
    fn from(value: u8) -> Self {
        match value & 0xF {
            procedure_abort_reason::NO_ABORT => Self::NoAbort,
            procedure_abort_reason::LOCAL_HOST_OR_REMOTE_REQUEST => Self::LocalHostOrRemoteRequest,
            procedure_abort_reason::LESS_THAN_15_CHANNELS => Self::LessThan15Channels,
            procedure_abort_reason::CHANNEL_MAP_UPDATE_INSTANT_PASSED => Self::ChannelMapUpdateInstantPassed,
            procedure_abort_reason::UNSPECIFIED => Self::Unspecified,
            _ => Self::Reserved,
        }
    }
}

/// Reasons for aborting subevents.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[derive_ReprC]
#[repr(u8)]
pub enum SubeventAbortReason {
    /// The subevent was not aborted.
    NoAbort,
    /// Abort caused by local host or a remote request.
    LocalHostOrRemoteRequest,
    /// No CS sync was received.
    NoCsSyncReceived,
    /// Scheduling conflict or limited resources.
    SchedulingConflictOrLimitedResources,
    /// Unspecified abort reason.
    Unspecified,
    /// Reserved abort reason.
    Reserved,
}

impl From<u8> for SubeventAbortReason {
    fn from(value: u8) -> Self {
        match (value >> 4) & 0xF {
            subevent_abort_reason::NO_ABORT => Self::NoAbort,
            subevent_abort_reason::LOCAL_HOST_OR_REMOTE_REQUEST => Self::LocalHostOrRemoteRequest,
            subevent_abort_reason::NO_CS_SYNC_RECEIVED => Self::NoCsSyncReceived,
            subevent_abort_reason::SCHEDULING_CONFLICT_OR_LIMITED_RESOURCES => {
                Self::SchedulingConflictOrLimitedResources
            }
            subevent_abort_reason::UNSPECIFIED => Self::Unspecified,
            _ => Self::Reserved,
        }
    }
}

/// An indicator of tone quality
///
/// Order of entries is important, as it is used for sorting.
#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(u8)]
pub enum ToneQualityIndicator {
    /// Reserved tone quality indicator.
    Reserved,
    /// Tone quality is unavailable.
    #[default]
    Unavailable,
    /// Tone quality is low.
    Low,
    /// Tone quality is medium.
    Medium,
    /// Tone quality is high.
    High,
}

impl From<u8> for ToneQualityIndicator {
    fn from(value: u8) -> Self {
        match value {
            tone_quality_indicator::HIGH => Self::High,
            tone_quality_indicator::MEDIUM => Self::Medium,
            tone_quality_indicator::LOW => Self::Low,
            tone_quality_indicator::UNAVAILABLE => Self::Unavailable,
            _ => Self::Reserved,
        }
    }
}

/// Possible types of extension slots.
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(u8)]
pub enum ExtensionSlot {
    /// No extension slot is present.
    #[default]
    NotPresent,
    /// An extension slot is present but not expected.
    NotExpectedPresent,
    /// An extension slot is present and expected.
    ExpectedPresent,
    /// Reserved slot type.
    Reserved,
}

impl From<u8> for ExtensionSlot {
    fn from(value: u8) -> Self {
        match value {
            extension_slot::NOT_PRESENT => Self::NotPresent,
            extension_slot::NOT_EXPECTED_PRESENT => Self::NotExpectedPresent,
            extension_slot::EXPECTED_PRESENT => Self::ExpectedPresent,
            _ => Self::Reserved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FrequencyCompensation, ParseError};

    #[test]
    fn test_frequency_compensation_positive_offset_converts_to_ppm() {
        // 150 LSB = 1.5 ppm (bit 14 clear → positive).
        let ppm: f32 = FrequencyCompensation::from(150).try_into().unwrap();
        assert_eq!(ppm, 1.5);
    }

    #[test]
    fn test_frequency_compensation_negative_offset_sign_extends() {
        // -100 ppm = 0x58F0 in 15-bit two's complement (bit 14 is the sign bit).
        let ppm: f32 = FrequencyCompensation::from(0x58F0).try_into().unwrap();
        assert_eq!(ppm, -100.0);
    }

    #[test]
    fn test_frequency_compensation_not_available_sentinel_is_error() {
        // 0xC000 marks the value as not available / role not initiator.
        let result: Result<f32, ParseError> = FrequencyCompensation::from(0xC000).try_into();
        assert!(matches!(result, Err(ParseError::RoleNotInitiator)));
    }
}
