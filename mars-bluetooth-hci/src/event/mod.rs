use core::array::TryFromSliceError;

use safer_ffi::derive_ReprC;
use serde::{Deserialize, Serialize};

use crate::constants::{
    extension_slot, frequency_compensation, procedure_abort_reason, reference_power_level,
    subevent_abort_reason, tone_quality_indicator,
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

        Ok(value.value as i16 as f32 / 100.0)
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

        if (value.value > reference_power_level::MAX_DBM)
            || (value.value < reference_power_level::MIN_DBM)
        {
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
            procedure_abort_reason::CHANNEL_MAP_UPDATE_INSTANT_PASSED => {
                Self::ChannelMapUpdateInstantPassed
            }
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
