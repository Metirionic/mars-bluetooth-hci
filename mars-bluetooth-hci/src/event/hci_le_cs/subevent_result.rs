//! Constructs subevent results from parsed inputs for further processing.
//!
//! Supports HCI_LE_CS_Config_Complete and HCI_LE_CS_Subevent_Result_Continue subevent codes.

use core::array::TryFromSliceError;
use core::result::Result;

use safer_ffi::derive_ReprC;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

pub use crate::event::hci_le_cs::constants::antenna_permutation;
pub use crate::event::hci_le_cs::constants::cs_params::{MAX_ANTENNA_PATH_COUNT, MAX_NUM_STEPS_REPORTED};
use crate::event::hci_le_cs::constants::{handle, le_subevent_code, step_data_len, step_mode};
use crate::event::{
    ExtensionSlot, FrequencyCompensation, ParseError, ProcedureAbortReason, ProcedureDoneStatus, ProcedureInfo,
    ReferencePowerLevel, SubeventAbortReason, SubeventDoneStatus, SubeventInfo, ToneQualityIndicator, sign_extend_15,
};

/// The phase correction term (PCT), composed of I and Q components.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct PhaseCorrectionTerm {
    /// The I component of the PCT.
    pub i: f32,
    /// The Q component of the PCT.
    pub q: f32,
}

impl TryFrom<&[u8]> for PhaseCorrectionTerm {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let i = u16::from_le_bytes(value[..2].try_into()?);
        let q = u16::from_le_bytes(value[1..3].try_into()?);

        let i = (i & 0x0FFF) << 4;
        let q = q & 0xFFF0;

        /// Normalizes I and Q to within [-1.0, 1.0).
        const NORMALIZATION_VALUE: f32 = 32768.0;

        Ok(PhaseCorrectionTerm {
            i: (i as i16 as f32) / NORMALIZATION_VALUE,
            q: (q as i16 as f32) / NORMALIZATION_VALUE,
        })
    }
}

/// Decoded contents of the packet quality byte.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct PacketQuality {
    /// Result of the access address check from the low nibble.
    pub access_address_check_result: u8,
    /// Payload bit error count from the high nibble.
    pub payload_bit_error_count: u8,
}

impl From<u8> for PacketQuality {
    fn from(value: u8) -> Self {
        Self {
            access_address_check_result: value & 0x0F,
            payload_bit_error_count: (value >> 4) & 0x0F,
        }
    }
}

/// Optional packet phase correction terms for enhanced packet-based ranging.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct PacketPhaseCorrectionTerms {
    /// First packet phase correction term.
    pub first_phase_correction_term: PhaseCorrectionTerm,
    /// Second packet phase correction term.
    pub second_phase_correction_term: PhaseCorrectionTerm,
}

impl TryFrom<&[u8]> for PacketPhaseCorrectionTerms {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            first_phase_correction_term: value[0..3].try_into()?,
            second_phase_correction_term: value[4..7].try_into()?,
        })
    }
}

/// Packet-level fields shared by time-based ranging modes.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct RoundTripTimePacketFields {
    /// Decoded packet quality fields.
    pub packet_quality: PacketQuality,
    /// Normalized attack detector metric for the packet.
    pub packet_normalized_attack_detector_metric: u8,
    /// Received signal strength indicator for the packet.
    pub packet_received_signal_strength_indicator: i8,
    /// Antenna used for the packet measurement.
    pub packet_antenna: u8,
}

impl TryFrom<&[u8]> for RoundTripTimePacketFields {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            packet_quality: value[0].into(),
            packet_normalized_attack_detector_metric: value[1],
            packet_received_signal_strength_indicator: value[2] as i8,
            packet_antenna: value[5],
        })
    }
}

/// Indicates which role-specific timing interpretation is valid.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(u8)]
pub enum RoundTripTimeRoleTimingKind {
    /// No role-specific timing has been assigned yet.
    #[default]
    Unavailable,
    /// Timing field `ToA_ToD_Initiator`.
    TimeOfArrivalTimeOfDepartureInitiator,
    /// Timing field `ToD_ToA_Reflector`.
    TimeOfDepartureTimeOfArrivalReflector,
}

/// Role-specific time delta for time-based ranging.
///
/// The raw role-specific timing value uses a time base of `0.5 ns` per least significant bit.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct RoundTripTimeRoleTiming {
    /// Indicates which role-specific timing field is valid.
    pub kind: RoundTripTimeRoleTimingKind,
    /// Signed value of the selected role-specific timing field.
    ///
    /// Stored with a time base of `0.5 ns` per least significant bit.
    pub role_specific_timing_value: i16,
}

/// Bluetooth sentinel for an unavailable Mode 1/3 time difference (`0x8000`).
const TIME_DIFFERENCE_NOT_AVAILABLE: i16 = i16::MIN;

impl RoundTripTimeRoleTiming {
    /// Convert the raw role-specific timing value to seconds.
    ///
    /// The raw HCI field uses 0.5 ns per least significant bit.
    /// Returns [`None`] when the timing kind or timing value marks the field as unavailable.
    /// This does not compute a one-way time of flight; initiator and reflector
    /// timing values still need to be paired by the processing layer.
    pub fn to_seconds(&self) -> Option<f32> {
        if matches!(self.kind, RoundTripTimeRoleTimingKind::Unavailable)
            || self.role_specific_timing_value == TIME_DIFFERENCE_NOT_AVAILABLE
        {
            None
        } else {
            Some(self.role_specific_timing_value as f32 * 0.5e-9)
        }
    }
}

/// Compact Mode 0 payload stored once per reported step.
///
/// Mode 0 steps exchange a known CS sync sequence to calibrate the frequency
/// offset between the initiator and reflector PLLs. The reflector role
/// carries no frequency offset; the initiator role adds the measured offset.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct Mode0Data {
    /// Decoded packet quality fields.
    pub packet_quality: PacketQuality,
    /// Received signal strength indicator for the packet.
    pub packet_received_signal_strength_indicator: i8,
    /// Antenna used for the packet measurement.
    pub packet_antenna: u8,
    /// Measured frequency offset, 0.01 ppm per least significant bit.
    ///
    /// 15-bit signed two's complement (bit 14 is the sign bit); the full 16-bit
    /// value `0xC000` marks the offset as not available. Present for the
    /// initiator role only; left at `0` for the reflector role, which carries
    /// no offset on the wire. Decode with [`Mode0Data::to_ppm`].
    pub measured_freq_offset: u16,
}

/// Bluetooth sentinel for an unavailable Mode 0 measured frequency offset (`0xC000`).
const MEASURED_FREQ_OFFSET_NOT_AVAILABLE: u16 = 0xC000;

impl Mode0Data {
    /// Convert the raw measured frequency offset to parts per million.
    ///
    /// The raw HCI field is a 15-bit signed two's-complement value with
    /// 0.01 ppm per least significant bit (valid range -100..+100 ppm);
    /// bit 15 is reserved. The full 16-bit value `0xC000` marks the offset
    /// as not available, in which case this returns [`None`]. Valid
    /// measurements are sign-extended from bit 14 and returned in ppm.
    pub fn to_ppm(&self) -> Option<f32> {
        if self.measured_freq_offset == MEASURED_FREQ_OFFSET_NOT_AVAILABLE {
            return None;
        }
        Some(sign_extend_15(self.measured_freq_offset) as f32 * 0.01)
    }
}

/// Compact Mode 1 payload stored once per reported step.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct Mode1Data {
    /// Shared packet-level fields.
    pub packet: RoundTripTimePacketFields,
    /// Role-specific RTT timing delta.
    pub timing: RoundTripTimeRoleTiming,
    /// Optional packet phase correction terms.
    pub packet_phase_correction_terms: PacketPhaseCorrectionTerms,
    /// If true, `packet_phase_correction_terms` is valid.
    pub has_packet_phase_correction_terms: bool,
}

/// Content of a Mode2 that is captured in a step.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct Mode2 {
    /// The selected antenna permutation index.
    pub antenna_permutation_index: u8,
    /// The phase correction terms for the antenna paths.
    pub phase_correction_terms: [PhaseCorrectionTerm; MAX_ANTENNA_PATH_COUNT + 1],
    /// The quality indicators for the antenna paths.
    pub quality_indicators: [ToneQualityIndicator; MAX_ANTENNA_PATH_COUNT + 1],
    /// The selected extension slots for the antenna paths.
    pub extension_slots: [ExtensionSlot; MAX_ANTENNA_PATH_COUNT + 1],
}

impl Mode2 {
    /// Look up the physical antenna index for a given logical path position.
    ///
    /// Uses the spec-defined antenna permutation tables (Bluetooth CS Vol 6, Part H,
    /// Tables 4.13–4.15).
    ///
    /// `n_ap` is the number of antenna paths (2, 3, or 4).
    /// `path_index` is the logical path position (0..n_ap).
    ///
    /// Returns the physical antenna index (0-based) assigned to that path.
    /// For out-of-range permutation indices or invalid `n_ap`, returns `path_index`
    /// (identity mapping).
    pub fn antenna_index(&self, n_ap: usize, path_index: usize) -> Result<usize, crate::constants::Error> {
        Ok(antenna_permutation::lookup(n_ap, self.antenna_permutation_index as usize)?[path_index])
    }
}

/// Discriminant for [`ModeRoleSpecificInfo`].
///
/// The parser populates Mode 0, Mode 1, Mode 2, and Mode 3 variants.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(u8)]
pub enum ModeRoleSpecificInfoKind {
    /// Mode 0, reflector role.
    Mode0Reflector,
    /// Mode 1, initiator role.
    Mode1Initiator,
    /// Mode 1, initiator role, with PBR and RTT measurements.
    Mode1InitiatorPbrRtt,
    /// Mode 1, reflector role.
    Mode1Reflector,
    /// Mode 1, reflector role, with PBR and RTT measurements.
    Mode1ReflectorPbrRtt,
    /// Mode 2 (see [`Mode2`]).
    #[default]
    Mode2,
    /// Mode 3, initiator role.
    Mode3Initiator,
    /// Mode 3, initiator role, with PBR and RTT measurements.
    Mode3InitiatorPbrRtt,
    /// Mode 3, reflector role.
    Mode3Reflector,
    /// Mode 3, reflector role, with PBR and RTT measurements.
    Mode3ReflectorPbrRtt,
    /// Mode 0, initiator role.
    ///
    /// Appended at the end to preserve the existing wire values of the
    /// variants above.
    Mode0Initiator,
    /// A step slot reported without valid data (Step_Mode `0xFF`).
    ///
    /// Appended at the end to preserve the existing wire values of the
    /// variants above. Old readers cannot deserialize frames that carry this
    /// kind — the same trade-off as every appended variant, governed by the
    /// persisted-frame version policy.
    Invalid,
}

impl ModeRoleSpecificInfoKind {
    /// Returns the CS step mode whose payload this kind carries.
    ///
    /// The kind-to-mode mapping lives here so consumers that need it (the
    /// fixture suite's representativeness check) share one compiler-checked
    /// definition instead of a parallel match.
    pub fn mode(self) -> u8 {
        match self {
            Self::Mode0Reflector | Self::Mode0Initiator => step_mode::MODE_0,
            Self::Mode1Initiator | Self::Mode1InitiatorPbrRtt | Self::Mode1Reflector | Self::Mode1ReflectorPbrRtt => {
                step_mode::MODE_1
            }
            Self::Mode2 => step_mode::MODE_2,
            Self::Mode3Initiator | Self::Mode3InitiatorPbrRtt | Self::Mode3Reflector | Self::Mode3ReflectorPbrRtt => {
                step_mode::MODE_3
            }
            Self::Invalid => step_mode::MODE_INVALID,
        }
    }
}

/// Mode- and role-specific information.
///
/// The payload fields are selected by `kind`.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct ModeRoleSpecificInfo {
    /// The kind of mode- and role-specific information.
    pub kind: ModeRoleSpecificInfoKind,
    /// Mode0 data. Valid when `kind`
    /// is [`ModeRoleSpecificInfoKind::Mode0Reflector`]
    /// or [`ModeRoleSpecificInfoKind::Mode0Initiator`].
    pub mode0: Mode0Data,
    /// Mode1 data. Valid when `kind`
    /// is [`ModeRoleSpecificInfoKind::Mode1Initiator`]
    /// or [`ModeRoleSpecificInfoKind::Mode1InitiatorPbrRtt`]
    /// or [`ModeRoleSpecificInfoKind::Mode1Reflector`]
    /// or [`ModeRoleSpecificInfoKind::Mode1ReflectorPbrRtt`],
    /// and also for Mode 3 variants (Mode 3 = Mode 1 + Mode 2 in the spec).
    pub mode1: Mode1Data,
    /// Mode2 data. Valid when `kind`
    ///  is [`ModeRoleSpecificInfoKind::Mode2`],
    /// and also for Mode 3 variants (the tone fields are populated alongside
    /// Mode 1 packet fields in `mode1`).
    pub mode2: Mode2,
}

/// Data that characterizes a step.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct Step {
    /// The type of mode.
    pub mode: u8,
    /// The selected frequency channel.
    pub channel: u8,
    /// Information that is specific to the mode and role of a node.
    pub info: ModeRoleSpecificInfo,
}

/// Metadata that is contained only in the initial HCI_LE_CS_Config_Complete subevent.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[derive_ReprC]
#[repr(C)]
pub struct InitialMeta {
    /// Starting ACL connection event counter for the results reported in the event.
    pub start_acl_conn_event_counter: u16,
    /// If true, the metadata contains a valid starting ACL connection event counter.
    pub has_start_acl_conn_event_counter: bool,
    /// CS procedure count since completion of the Channel Sounding Security Start procedure.
    pub procedure_counter: u16,
    /// Frequency compensation with a resolution of 0.01 ppm.
    pub frequency_compensation: FrequencyCompensation,
    /// The reference power level between -127 and 20 dBm.
    pub reference_power_level: ReferencePowerLevel,
}

/// The origin of data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(u8)]
pub enum Origin {
    /// Origin is unknown.
    Unknown,
    /// Data from an initiator. Carries MAC address identifier.
    Initiator,
    /// Data from a reflector. Carries MAC address identifier.
    Reflector,
}

/// Data that was collected from a "LE CS Subevent Result event" [7.7.65.44, p. 2446]
/// or a "LE CS Subevent Result Continue event" [7.7.65.45, p. 2459].
///
/// For the latter, the [`SubeventResultEvent::initial_meta`] is `None`.
///
/// See the `TryFrom<&[u8]>` implementation for a parse-from-bytes example.
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[derive_ReprC]
#[repr(C)]
pub struct SubeventResultEvent {
    /// The origin of the data (initiator or reflector).
    ///
    /// Left at [`Origin::Unknown`] by the parser; the caller sets this from
    /// out-of-band context (which node produced the bytes), as the file-reader
    /// helper does.
    pub origin: Origin,

    /// MAC address of the local node.
    ///
    /// Left at `0` by the parser; the caller sets this from out-of-band context.
    pub local_mac: u64,
    /// MAC address of the peer node.
    ///
    /// Left at `0` by the parser; the caller sets this from out-of-band context.
    pub peer_mac: u64,
    /// The connection handle between two nodes.
    pub connection_handle: u16,

    /// CS configuration identifier.
    pub config_id: u8,
    /// If true, has valid config ID.
    pub has_config_id: bool,

    /// State of the procedure.
    pub procedure_done_status: ProcedureDoneStatus,
    /// The procedure abort reason, if any.
    pub procedure_abort_reason: ProcedureAbortReason,

    /// State of the subevent.
    pub subevent_done_status: SubeventDoneStatus,
    /// The subevent abort reason, if any.
    pub subevent_abort_reason: SubeventAbortReason,

    /// The number of antenna paths.
    pub antenna_path_count: usize,

    /// The number of steps for the subevent.
    pub step_count: usize,
    /// The step data.
    #[serde_as(as = "[_; MAX_NUM_STEPS_REPORTED]")]
    pub steps: [Step; MAX_NUM_STEPS_REPORTED],

    /// Metadata that only the first subevent (not the "continue" variant) holds.
    pub initial_meta: InitialMeta,
    /// If true, initial metadata is available.
    pub has_initial_meta: bool,
}

/// Minimum subevent header length the parser indexes unconditionally:
/// subevent code, connection handle, and config id.
const MIN_HEADER_LEN: usize = 4;
/// Header length of a config-complete subevent, through the step counts.
const CONFIG_COMPLETE_HEADER_LEN: usize = 16;
/// Header length of a subevent-result continue before its steps.
const CONTINUE_HEADER_LEN: usize = 9;
/// Length of one step's header (mode, channel, and data length).
const STEP_HEADER_LEN: usize = 3;

/// Checks the step and antenna-path counts against the fixed arrays both
/// index.
///
/// Shared by the parser's header checks and by
/// [`SubeventResultEvent::validate_invariants`], so one place owns the bounds
/// and their error mapping.
fn check_counts(antenna_path_count: usize, step_count: usize) -> Result<(), ParseError> {
    if antenna_path_count > MAX_ANTENNA_PATH_COUNT {
        return Err(ParseError::ExceededMaxAntennaPathCount);
    }
    if step_count > MAX_NUM_STEPS_REPORTED {
        return Err(ParseError::ExceededMaxStepCount);
    }
    Ok(())
}

impl SubeventResultEvent {
    /// Parse a subevent result message with known origin information.
    pub fn try_from_with_origin(message: &[u8], origin: Origin) -> Result<Self, ParseError> {
        Self::parse_internal(message, origin)
    }

    /// Checks the bounds the parser guarantees for every event it produces.
    ///
    /// The parser rejects messages whose step or antenna-path counts exceed
    /// the fixed step array and the fixed antenna-path table, so no event it
    /// returns can break these bounds. Producers of events from stored bytes
    /// must re-validate so consumers indexing `steps[..step_count]` cannot
    /// panic on a corrupted or foreign-encoder frame.
    pub fn validate_invariants(&self) -> Result<(), ParseError> {
        check_counts(self.antenna_path_count, self.step_count)
    }

    /// Return the expected Mode 2 step payload length for an antenna path count.
    fn mode2_len(antenna_path_count: usize) -> usize {
        let tone_count = antenna_path_count + 1;

        step_data_len::ANTENNA_PERMUTATION_INDEX
            + step_data_len::TONE_PHASE_CORRECTION_TERM * tone_count
            + step_data_len::TONE_QUALITY_INDICATOR * tone_count
    }

    /// Return the expected basic Mode 3 step payload length for an antenna path count.
    fn mode3_len(antenna_path_count: usize) -> usize {
        step_data_len::MODE1 + Self::mode2_len(antenna_path_count)
    }

    /// Return the expected Mode 3 step payload length with packet phase correction terms.
    fn mode3_pbr_rtt_len(antenna_path_count: usize) -> usize {
        step_data_len::MODE1_PBR_RTT + Self::mode2_len(antenna_path_count)
    }

    /// Parse the role-specific RTT timing field shared by Mode 1 and Mode 3.
    fn parse_rtt_role_timing(step_data: &[u8], origin: Origin) -> Result<RoundTripTimeRoleTiming, ParseError> {
        let timing_kind = match origin {
            Origin::Initiator => RoundTripTimeRoleTimingKind::TimeOfArrivalTimeOfDepartureInitiator,
            Origin::Reflector => RoundTripTimeRoleTimingKind::TimeOfDepartureTimeOfArrivalReflector,
            Origin::Unknown => RoundTripTimeRoleTimingKind::Unavailable,
        };

        Ok(RoundTripTimeRoleTiming {
            kind: timing_kind,
            role_specific_timing_value: i16::from_le_bytes(step_data[3..5].try_into()?),
        })
    }

    /// Parse the Mode 2 tone fields from a byte slice.
    ///
    /// Shared by Mode 2 (full step payload) and Mode 3 (trailing tone portion
    /// after the Mode 1 prefix). Does not validate overall step length — the
    /// caller is responsible for slicing `step_data` to the correct tone bytes.
    fn parse_mode2_tones(step_data: &[u8], antenna_path_count: usize) -> Result<Mode2, ParseError> {
        let mut tones = Mode2 {
            antenna_permutation_index: step_data[0],
            ..Default::default()
        };

        let tone_count = antenna_path_count + 1;
        let tone_quality_offset =
            step_data_len::ANTENNA_PERMUTATION_INDEX + step_data_len::TONE_PHASE_CORRECTION_TERM * tone_count;

        for antenna_path_index in 0..tone_count {
            let phase_correction_offset = step_data_len::ANTENNA_PERMUTATION_INDEX
                + step_data_len::TONE_PHASE_CORRECTION_TERM * antenna_path_index;
            tones.phase_correction_terms[antenna_path_index] =
                step_data[phase_correction_offset..phase_correction_offset + 3].try_into()?;

            let tone_quality_byte = step_data[tone_quality_offset + antenna_path_index];
            tones.quality_indicators[antenna_path_index] = (tone_quality_byte & 0x0F).into();
            tones.extension_slots[antenna_path_index] = ((tone_quality_byte >> 4) & 0x0F).into();
        }

        Ok(tones)
    }

    /// Parse one grouped Mode 2 step payload.
    fn parse_mode2_step(step_data: &[u8], antenna_path_count: usize) -> Result<Mode2, ParseError> {
        let expected_step_data_length = Self::mode2_len(antenna_path_count);
        if step_data.len() != expected_step_data_length {
            return Err(ParseError::InvalidStepDataLength(
                step_mode::MODE_2,
                step_data.len(),
                expected_step_data_length,
            ));
        }

        Self::parse_mode2_tones(step_data, antenna_path_count)
    }

    /// Parse one Mode 0 step payload.
    ///
    /// Mode 0 step data is 3 bytes for the reflector role (packet quality,
    /// RSSI, antenna) and 5 bytes for the initiator role (plus the measured
    /// frequency offset). The step-data length is the authoritative role
    /// indicator; no `origin` is required.
    fn parse_mode0_step(step_data: &[u8]) -> Result<Mode0Data, ParseError> {
        if !matches!(
            step_data.len(),
            step_data_len::MODE0_REFLECTOR | step_data_len::MODE0_INITIATOR
        ) {
            return Err(ParseError::InvalidStepDataLength(
                step_mode::MODE_0,
                step_data.len(),
                step_data_len::MODE0_REFLECTOR,
            ));
        }

        Ok(Mode0Data {
            packet_quality: step_data[0].into(),
            packet_received_signal_strength_indicator: step_data[1] as i8,
            packet_antenna: step_data[2],
            measured_freq_offset: if step_data.len() == step_data_len::MODE0_INITIATOR {
                u16::from_le_bytes(step_data[3..5].try_into()?)
            } else {
                0
            },
        })
    }

    /// Parse one Mode 1 step payload.
    fn parse_mode1_step(step_data: &[u8], origin: Origin) -> Result<Mode1Data, ParseError> {
        let (has_packet_phase_correction_terms, expected_step_data_length) = match step_data.len() {
            step_data_len::MODE1 => (false, step_data_len::MODE1),
            step_data_len::MODE1_PBR_RTT => (true, step_data_len::MODE1_PBR_RTT),
            _ => {
                return Err(ParseError::InvalidStepDataLength(
                    step_mode::MODE_1,
                    step_data.len(),
                    step_data_len::MODE1,
                ));
            }
        };

        let packet_phase_correction_terms = if has_packet_phase_correction_terms {
            step_data[6..14].try_into()?
        } else {
            Default::default()
        };

        debug_assert_eq!(step_data.len(), expected_step_data_length);

        Ok(Mode1Data {
            packet: step_data.try_into()?,
            timing: Self::parse_rtt_role_timing(step_data, origin)?,
            packet_phase_correction_terms,
            has_packet_phase_correction_terms,
        })
    }

    /// Parse one Mode 3 step payload.
    ///
    /// Mode 3 is Mode 1 + Mode 2 in the spec, so the payload is a Mode 1 prefix
    /// followed by the Mode 2 tone fields. This function slices the payload at
    /// `tone_offset` and delegates to `parse_mode1_step` (for the prefix) and
    /// `parse_mode2_tones` (for the trailing tones), returning both so the caller
    /// can populate `info.mode1` and `info.mode2` on the same `Step`.
    fn parse_mode3_step(
        step_data: &[u8],
        origin: Origin,
        antenna_path_count: usize,
    ) -> Result<(Mode1Data, Mode2), ParseError> {
        let step_data_length = step_data.len();
        let basic_len = Self::mode3_len(antenna_path_count);
        let pbr_rtt_len = Self::mode3_pbr_rtt_len(antenna_path_count);

        let tone_offset = if step_data_length == basic_len {
            step_data_len::MODE1
        } else if step_data_length == pbr_rtt_len {
            step_data_len::MODE1_PBR_RTT
        } else {
            return Err(ParseError::InvalidStepDataLength(
                step_mode::MODE_3,
                step_data_length,
                basic_len,
            ));
        };

        let mode1 = Self::parse_mode1_step(&step_data[..tone_offset], origin)?;
        let mode2 = Self::parse_mode2_tones(&step_data[tone_offset..], antenna_path_count)?;
        Ok((mode1, mode2))
    }

    /// Push steps from a binary message into the subevent result event.
    fn push_steps(&mut self, message: &[u8]) -> Result<(), ParseError> {
        let mut step_index = 0;
        let mut step_byte_offset = if self.has_initial_meta {
            CONFIG_COMPLETE_HEADER_LEN
        } else {
            CONTINUE_HEADER_LEN
        };

        for _ in 0..self.step_count {
            let step_data_offset = STEP_HEADER_LEN + step_byte_offset;
            // The step header (mode, channel, length) runs through
            // `step_data_offset - 1`.
            if message.len() < step_data_offset {
                return Err(ParseError::TooShort(message.len()));
            }
            let step_mode = message[step_byte_offset];
            let step_channel = message[1 + step_byte_offset];
            let step_data_length = message[2 + step_byte_offset] as usize;

            let step_data_end = step_data_offset + step_data_length;
            if message.len() < step_data_end {
                // The header parsed but its declaration overruns the message:
                // report the corrupt declaration against what is actually
                // available, so it is not diagnosed as a truncated message.
                return Err(ParseError::TruncatedStepData {
                    index: step_index,
                    declared: step_data_length,
                    available: message.len() - step_data_offset,
                });
            }
            let step_data = &message[step_data_offset..step_data_end];

            match step_mode {
                step_mode::MODE_INVALID => {
                    // A 0xFF mode marks a reported step slot as carrying no
                    // valid data. Preserve the sentinel with its own payload
                    // kind instead of leaving a fabricated default behind:
                    // `steps[..step_count]` then holds exactly one entry per
                    // reported step whose kind never claims mode data, and
                    // consumers filter on the sentinel mode.
                    self.steps[step_index] = Step {
                        mode: step_mode,
                        channel: step_channel,
                        info: ModeRoleSpecificInfo {
                            kind: ModeRoleSpecificInfoKind::Invalid,
                            ..Default::default()
                        },
                    };
                    step_index += 1;
                }
                step_mode::MODE_0 => {
                    let mode0 = Self::parse_mode0_step(step_data)?;
                    self.steps[step_index] = Step {
                        mode: step_mode,
                        channel: step_channel,
                        info: ModeRoleSpecificInfo {
                            kind: if step_data.len() == step_data_len::MODE0_INITIATOR {
                                ModeRoleSpecificInfoKind::Mode0Initiator
                            } else {
                                ModeRoleSpecificInfoKind::Mode0Reflector
                            },
                            mode0,
                            ..Default::default()
                        },
                    };
                    step_index += 1;
                }
                step_mode::MODE_1 => {
                    if matches!(self.origin, Origin::Unknown) {
                        return Err(ParseError::UnknownOriginForMode(step_mode));
                    }
                    let mode1 = Self::parse_mode1_step(step_data, self.origin)?;
                    self.steps[step_index] = Step {
                        mode: step_mode,
                        channel: step_channel,
                        info: ModeRoleSpecificInfo {
                            kind: self.mode_1_selector(mode1.has_packet_phase_correction_terms),
                            mode1,
                            ..Default::default()
                        },
                    };
                    step_index += 1;
                }
                step_mode::MODE_2 => {
                    let mode2 = Self::parse_mode2_step(step_data, self.antenna_path_count)?;

                    let step = Step {
                        mode: step_mode,
                        channel: step_channel,
                        info: ModeRoleSpecificInfo {
                            kind: ModeRoleSpecificInfoKind::Mode2,
                            mode2,
                            ..Default::default()
                        },
                    };

                    self.steps[step_index] = step;
                    step_index += 1;
                }
                step_mode::MODE_3 => {
                    if matches!(self.origin, Origin::Unknown) {
                        return Err(ParseError::UnknownOriginForMode(step_mode));
                    }
                    // Mode 3 = Mode 1 + Mode 2; both siblings are populated.
                    let (mode1, mode2) = Self::parse_mode3_step(step_data, self.origin, self.antenna_path_count)?;
                    self.steps[step_index] = Step {
                        mode: step_mode,
                        channel: step_channel,
                        info: ModeRoleSpecificInfo {
                            kind: self.mode_3_selector(mode1.has_packet_phase_correction_terms),
                            mode1,
                            mode2,
                            ..Default::default()
                        },
                    };
                    step_index += 1;
                }
                _ => {
                    return Err(ParseError::InvalidModeType(step_mode, step_byte_offset));
                }
            }

            step_byte_offset += STEP_HEADER_LEN + step_data_length;
        }

        Ok(())
    }

    /// Select the Mode 1 role-specific payload kind for this event origin.
    fn mode_1_selector(&self, has_packet_phase_correction_terms: bool) -> ModeRoleSpecificInfoKind {
        match (self.origin, has_packet_phase_correction_terms) {
            (Origin::Initiator, false) => ModeRoleSpecificInfoKind::Mode1Initiator,
            (Origin::Initiator, true) => ModeRoleSpecificInfoKind::Mode1InitiatorPbrRtt,
            (Origin::Reflector, false) => ModeRoleSpecificInfoKind::Mode1Reflector,
            (Origin::Reflector, true) => ModeRoleSpecificInfoKind::Mode1ReflectorPbrRtt,
            (Origin::Unknown, false) | (Origin::Unknown, true) => unreachable!(),
        }
    }

    /// Select the Mode 3 role-specific payload kind for this event origin.
    fn mode_3_selector(&self, has_packet_phase_correction_terms: bool) -> ModeRoleSpecificInfoKind {
        match (self.origin, has_packet_phase_correction_terms) {
            (Origin::Initiator, false) => ModeRoleSpecificInfoKind::Mode3Initiator,
            (Origin::Initiator, true) => ModeRoleSpecificInfoKind::Mode3InitiatorPbrRtt,
            (Origin::Reflector, false) => ModeRoleSpecificInfoKind::Mode3Reflector,
            (Origin::Reflector, true) => ModeRoleSpecificInfoKind::Mode3ReflectorPbrRtt,
            (Origin::Unknown, false) | (Origin::Unknown, true) => unreachable!(),
        }
    }

    /// Parse a subevent result or continuation message with the supplied origin.
    ///
    /// Malformed input — boot noise, a truncated buffer, or a hostile frame —
    /// is rejected with an error instead of panicking: every message index is
    /// length-guarded before it is read.
    fn parse_internal(message: &[u8], origin: Origin) -> Result<Self, ParseError> {
        // The common header prefix (subevent code, connection handle, and the
        // config id for a non-CS-test handle) runs through the last byte of
        // `MIN_HEADER_LEN`.
        if message.len() < MIN_HEADER_LEN {
            return Err(ParseError::TooShort(message.len()));
        }
        let connection_handle = u16::from_le_bytes(message[1..3].try_into()?);
        let connection_handle_is_cs_test = connection_handle == handle::CS_TEST_CONNECTION_HANDLE;

        let (config_id, has_config_id) = if connection_handle_is_cs_test {
            (0, false)
        } else {
            (message[3], true)
        };

        let mut event = match message[0] {
            le_subevent_code::CS_CONFIG_COMPLETE => {
                // The config-complete header runs through index 15.
                if message.len() < CONFIG_COMPLETE_HEADER_LEN {
                    return Err(ParseError::TooShort(message.len()));
                }
                let (start_acl_conn_event_counter, has_start_acl_conn_event_counter) = if connection_handle_is_cs_test {
                    (0, false)
                } else {
                    (u16::from_le_bytes(message[4..6].try_into()?), true)
                };

                let procedure_counter = u16::from_le_bytes(message[6..8].try_into()?);
                let frequency_compensation =
                    FrequencyCompensation::from(u16::from_le_bytes(message[8..10].try_into()?));
                let reference_power_level = ReferencePowerLevel::from(message[10] as i8);

                let abort_reason = message[13];
                let procedure = ProcedureInfo::from((message[11], abort_reason));
                let subevent = SubeventInfo::from((message[12], abort_reason));

                let num_antenna_paths = message[14] as usize;
                let num_steps_reported = message[15] as usize;

                check_counts(num_antenna_paths, num_steps_reported)?;

                SubeventResultEvent {
                    origin,
                    local_mac: 0,
                    peer_mac: 0,
                    connection_handle,

                    config_id,
                    has_config_id,

                    initial_meta: InitialMeta {
                        start_acl_conn_event_counter,
                        has_start_acl_conn_event_counter,
                        procedure_counter,
                        frequency_compensation,
                        reference_power_level,
                    },
                    has_initial_meta: true,

                    procedure_done_status: procedure.0,
                    procedure_abort_reason: procedure.1,

                    subevent_done_status: subevent.0,
                    subevent_abort_reason: subevent.1,

                    antenna_path_count: num_antenna_paths,
                    step_count: num_steps_reported,
                    steps: core::array::from_fn(|_| Default::default()),
                }
            }
            le_subevent_code::CS_SUBEVENT_RESULT_CONTINUE => {
                // The continue header runs through index 8; steps follow from
                // `CONTINUE_HEADER_LEN` on.
                if message.len() < CONTINUE_HEADER_LEN {
                    return Err(ParseError::TooShort(message.len()));
                }
                let abort_reason = message[6];
                let procedure = ProcedureInfo::from((message[4], abort_reason));
                let subevent = SubeventInfo::from((message[5], abort_reason));

                let num_antenna_paths = message[7] as usize;
                let num_steps_reported = message[8] as usize;

                check_counts(num_antenna_paths, num_steps_reported)?;

                SubeventResultEvent {
                    origin,
                    local_mac: 0,
                    peer_mac: 0,
                    connection_handle,

                    config_id,
                    has_config_id,

                    initial_meta: Default::default(),
                    has_initial_meta: false,

                    procedure_done_status: procedure.0,
                    procedure_abort_reason: procedure.1,

                    subevent_done_status: subevent.0,
                    subevent_abort_reason: subevent.1,

                    antenna_path_count: num_antenna_paths,
                    step_count: num_steps_reported,
                    steps: core::array::from_fn(|_| Default::default()),
                }
            }
            _ => {
                return Err(ParseError::UnsupportedSubevent);
            }
        };

        event.push_steps(message)?;

        Ok(event)
    }
}

impl TryFrom<&[u8]> for SubeventResultEvent {
    type Error = ParseError;

    fn try_from(message: &[u8]) -> Result<Self, Self::Error> {
        Self::parse_internal(message, Origin::Unknown)
    }
}

/// Shared builders for representative subevent-result events.
///
/// Promoted out of the test module so every in-crate test module constructs
/// the same representative HCI events — the persisted-frame codec tests lock
/// the committed compatibility fixtures to these builders — instead of
/// hand-copying their bytes.
#[cfg(test)]
pub(crate) mod test_messages {
    use crate::event::hci_le_cs::constants::le_subevent_code;

    /// Builds the bytes of a `CS_CONFIG_COMPLETE` event carrying no steps and
    /// populated initial metadata.
    pub(crate) fn config_complete_event() -> [u8; 16] {
        [
            le_subevent_code::CS_CONFIG_COMPLETE,
            0x40,
            0x00, // connection handle
            0x07, // config id
            0x34,
            0x12, // start ACL connection event counter
            0x42,
            0x00, // procedure counter
            0xC8,
            0x00, // frequency compensation
            0x0A, // reference power level
            0x00, // procedure done status
            0x00, // subevent done status
            0x00, // abort reason
            0x01, // antenna path count
            0x00, // no steps reported
        ]
    }

    /// Builds the bytes of a `CS_SUBEVENT_RESULT_CONTINUE` event carrying one
    /// step.
    pub(crate) fn continue_event(step_mode: u8, channel: u8, antenna_path_count: u8, step_data: &[u8]) -> Vec<u8> {
        let mut message = vec![
            le_subevent_code::CS_SUBEVENT_RESULT_CONTINUE,
            0x01,
            0x00,
            0x07,
            0x00,
            0x00,
            0x00,
            antenna_path_count,
            0x01,
            step_mode,
            channel,
            step_data.len() as u8,
        ];
        message.extend_from_slice(step_data);
        message
    }

    /// Mode 1 step data with packet fields and role timing, without PCTs.
    pub(crate) fn mode1_basic_step_data(quality: u8, timing_lo: u8, timing_hi: u8) -> [u8; 6] {
        [quality, 0x80, 0x34, timing_lo, timing_hi, 0x02]
    }

    /// Mode 1 step data with packet fields, role timing, and packet PCTs.
    pub(crate) fn mode1_pbr_rtt_step_data(quality: u8, timing_lo: u8, timing_hi: u8) -> [u8; 14] {
        [
            quality, 0x80, 0x34, timing_lo, timing_hi, 0x02, 0x48, 0x7B, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    }

    /// Mode 2 step data carrying the per-antenna-path phase correction terms
    /// and quality indicators for a single antenna path.
    pub(crate) fn mode2_step_data() -> [u8; 9] {
        [0x09, 0x48, 0x7B, 0x54, 0x00, 0x00, 0x00, 0x21, 0x03]
    }

    /// Mode 0 reflector step data.
    pub(crate) fn mode0_reflector_step_data() -> [u8; 3] {
        [0xA2, 0x34, 0x02]
    }

    /// Mode 0 initiator step data with the measured frequency offset.
    pub(crate) fn mode0_initiator_step_data(freq_lo: u8, freq_hi: u8) -> [u8; 5] {
        [0xA2, 0x34, 0x02, freq_lo, freq_hi]
    }

    /// Mode 3 basic step data: the Mode 1 packet/timing section followed by the
    /// Mode 2 tone section, the way the parser splits it.
    pub(crate) fn mode3_basic_step_data() -> [u8; 15] {
        let mut step_data = [0u8; 15];
        step_data[..6].copy_from_slice(&mode1_basic_step_data(0x21, 0x12, 0x34));
        step_data[6..].copy_from_slice(&mode2_step_data());
        step_data
    }

    /// Mode 3 step data with packet phase correction terms: the Mode 1 PBR/RTT
    /// section followed by the Mode 2 tone section.
    pub(crate) fn mode3_pbr_rtt_step_data() -> [u8; 23] {
        let mut step_data = [0u8; 23];
        step_data[..14].copy_from_slice(&mode1_pbr_rtt_step_data(0x21, 0x12, 0x34));
        step_data[14..].copy_from_slice(&mode2_step_data());
        step_data
    }
}

#[cfg(test)]
mod tests {
    use super::test_messages::{
        config_complete_event, continue_event, mode0_initiator_step_data, mode0_reflector_step_data,
        mode1_basic_step_data, mode1_pbr_rtt_step_data, mode2_step_data, mode3_basic_step_data,
        mode3_pbr_rtt_step_data,
    };
    use super::{
        ModeRoleSpecificInfoKind, Origin, PhaseCorrectionTerm, RoundTripTimeRoleTimingKind, SubeventResultEvent,
    };
    use crate::event::hci_le_cs::constants::{le_subevent_code, step_mode};
    use crate::event::{ExtensionSlot, ParseError, ToneQualityIndicator};

    #[test]
    fn test_pct() {
        let bin = [0x48, 0x7B, 0x54];
        let pct = PhaseCorrectionTerm::try_from(bin.as_slice()).unwrap();

        println!("{:?}", pct);
    }

    #[test]
    fn test_grouped_mode2_step_stays_one_internal_step() {
        let message = continue_event(0x02, 0x05, 0x01, &mode2_step_data());

        let event = SubeventResultEvent::try_from(message.as_slice()).unwrap();

        assert_eq!(event.step_count, 1);
        assert!(matches!(event.steps[0].info.kind, ModeRoleSpecificInfoKind::Mode2));
        assert_eq!(event.steps[1].mode, 0);

        let mode2 = event.steps[0].info.mode2;
        assert_eq!(mode2.antenna_permutation_index, 9);
        let expected_pct0 = PhaseCorrectionTerm::try_from([0x48, 0x7B, 0x54].as_slice()).unwrap();
        let expected_pct1 = PhaseCorrectionTerm::try_from([0x00, 0x00, 0x00].as_slice()).unwrap();
        assert_eq!(mode2.phase_correction_terms[0].i, expected_pct0.i);
        assert_eq!(mode2.phase_correction_terms[0].q, expected_pct0.q);
        assert_eq!(mode2.phase_correction_terms[1].i, expected_pct1.i);
        assert_eq!(mode2.phase_correction_terms[1].q, expected_pct1.q);
        assert!(matches!(mode2.quality_indicators[0], ToneQualityIndicator::Medium));
        assert!(matches!(mode2.quality_indicators[1], ToneQualityIndicator::Unavailable));
        assert!(matches!(mode2.extension_slots[0], ExtensionSlot::ExpectedPresent));
        assert!(matches!(mode2.extension_slots[1], ExtensionSlot::NotPresent));
    }

    #[test]
    fn test_mode1_step_stays_one_internal_step() {
        let message = continue_event(0x01, 0x05, 0x01, &mode1_basic_step_data(0x21, 0x12, 0x34));

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();

        assert_eq!(event.step_count, 1);
        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode1Initiator
        ));
        assert_eq!(event.steps[1].mode, 0);

        let mode1 = event.steps[0].info.mode1;
        assert_eq!(mode1.packet.packet_quality.access_address_check_result, 1);
        assert_eq!(mode1.packet.packet_quality.payload_bit_error_count, 2);
        assert_eq!(mode1.packet.packet_normalized_attack_detector_metric, 0x80);
        assert_eq!(mode1.packet.packet_received_signal_strength_indicator, 0x34_i8);
        assert_eq!(mode1.packet.packet_antenna, 0x02);
        assert!(matches!(
            mode1.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfArrivalTimeOfDepartureInitiator
        ));
        assert_eq!(mode1.timing.role_specific_timing_value, 0x3412);
        assert_eq!(mode1.timing.to_seconds(), Some(0x3412_u16 as f32 * 0.5e-9));
        assert!(!mode1.has_packet_phase_correction_terms);
    }

    #[test]
    fn test_rtt_timing_not_available_sentinel_has_no_seconds_value() {
        let message = continue_event(0x01, 0x05, 0x01, &mode1_basic_step_data(0x21, 0x00, 0x80));

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();
        let timing = event.steps[0].info.mode1.timing;

        assert!(matches!(
            timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfArrivalTimeOfDepartureInitiator
        ));
        assert_eq!(timing.role_specific_timing_value, i16::MIN);
        assert_eq!(timing.to_seconds(), None);
    }

    #[test]
    fn test_rtt_unavailable_timing_kind_has_no_seconds_value() {
        let timing = super::RoundTripTimeRoleTiming::default();

        assert!(matches!(timing.kind, RoundTripTimeRoleTimingKind::Unavailable));
        assert_eq!(timing.to_seconds(), None);
    }

    #[test]
    fn test_mode1_reflector_kind_is_selected_during_parse() {
        let message = continue_event(0x01, 0x05, 0x01, &mode1_basic_step_data(0x21, 0x12, 0x34));

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Reflector).unwrap();

        assert!(matches!(event.origin, Origin::Reflector));
        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode1Reflector
        ));
        assert!(matches!(
            event.steps[0].info.mode1.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfDepartureTimeOfArrivalReflector
        ));
    }

    #[test]
    fn test_mode3_step_stays_one_internal_step() {
        // Mode 3 = Mode 1 + Mode 2: under the collapsed schema, Mode 1 fields
        // are on info.mode1 and tone fields are on info.mode2.
        let step_data = mode3_basic_step_data();
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();

        assert_eq!(event.step_count, 1);
        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3Initiator
        ));
        assert_eq!(event.steps[1].mode, 0);

        let mode1 = event.steps[0].info.mode1;
        let mode2 = event.steps[0].info.mode2;
        assert_eq!(mode1.packet.packet_quality.access_address_check_result, 1);
        assert_eq!(mode1.packet.packet_quality.payload_bit_error_count, 2);
        assert_eq!(mode1.packet.packet_antenna, 0x02);
        assert!(matches!(
            mode1.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfArrivalTimeOfDepartureInitiator
        ));
        assert_eq!(mode2.antenna_permutation_index, 9);
        let expected_pct = PhaseCorrectionTerm::try_from([0x48, 0x7B, 0x54].as_slice()).unwrap();
        assert_eq!(mode2.phase_correction_terms[0].i, expected_pct.i);
        assert_eq!(mode2.phase_correction_terms[0].q, expected_pct.q);
        assert!(matches!(mode2.quality_indicators[0], ToneQualityIndicator::Medium));
        assert!(matches!(mode2.extension_slots[0], ExtensionSlot::ExpectedPresent));
        assert!(!mode1.has_packet_phase_correction_terms);
    }

    #[test]
    fn test_mode0_reflector_step_is_parsed() {
        let message = continue_event(0x00, 0x05, 0x01, &mode0_reflector_step_data());

        let event = SubeventResultEvent::try_from(message.as_slice()).unwrap();

        assert_eq!(event.step_count, 1);
        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode0Reflector
        ));
        assert_eq!(event.steps[1].mode, 0);

        let mode0 = event.steps[0].info.mode0;
        assert_eq!(mode0.packet_quality.access_address_check_result, 0x02);
        assert_eq!(mode0.packet_quality.payload_bit_error_count, 0x0A);
        assert_eq!(mode0.packet_received_signal_strength_indicator, 0x34_i8);
        assert_eq!(mode0.packet_antenna, 0x02);
        assert_eq!(mode0.measured_freq_offset, 0);
        assert_eq!(mode0.to_ppm(), Some(0.0));
    }

    #[test]
    fn test_mode0_initiator_step_is_parsed() {
        // 150 LSB = 1.5 ppm (BlueZ doc example).
        let message = continue_event(0x00, 0x05, 0x01, &mode0_initiator_step_data(0x96, 0x00));

        let event = SubeventResultEvent::try_from(message.as_slice()).unwrap();

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode0Initiator
        ));

        let mode0 = event.steps[0].info.mode0;
        assert_eq!(mode0.measured_freq_offset, 150);
        assert_eq!(mode0.to_ppm(), Some(1.5));
    }

    #[test]
    fn test_mode0_to_ppm_sign_extends_negative_offset() {
        // -100 ppm = 0x58F0 in 15-bit two's complement (bit 14 is the sign bit).
        // The old mask-only decode read this as +227.68 ppm; sign-extend bit 14.
        let message = continue_event(0x00, 0x05, 0x01, &mode0_initiator_step_data(0xF0, 0x58));

        let event = SubeventResultEvent::try_from(message.as_slice()).unwrap();

        let mode0 = event.steps[0].info.mode0;
        assert_eq!(mode0.measured_freq_offset, 0x58F0);
        assert_eq!(mode0.to_ppm(), Some(-100.0));
    }

    #[test]
    fn test_mode0_to_ppm_not_available_sentinel_is_none() {
        // 0xC000 marks the measured frequency offset as not available.
        let message = continue_event(0x00, 0x05, 0x01, &mode0_initiator_step_data(0x00, 0xC0));

        let event = SubeventResultEvent::try_from(message.as_slice()).unwrap();

        let mode0 = event.steps[0].info.mode0;
        assert_eq!(mode0.measured_freq_offset, 0xC000);
        assert_eq!(mode0.to_ppm(), None);
    }

    #[test]
    fn test_mode0_unknown_origin_is_ok() {
        // Mode 0 has no origin requirement: the role comes from the
        // step-data length, so parsing with an unknown origin succeeds.
        let message = continue_event(0x00, 0x05, 0x01, &mode0_initiator_step_data(0x96, 0x00));

        let event = SubeventResultEvent::try_from(message.as_slice()).unwrap();

        assert!(matches!(event.origin, Origin::Unknown));
        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode0Initiator
        ));
    }

    #[test]
    fn test_mode0_wrong_length_is_rejected() {
        // 4 bytes is neither the 3-byte reflector nor the 5-byte initiator
        // payload, so it must be rejected.
        let bad_step_data = [0xA2, 0x34, 0x02, 0x00];
        let message = continue_event(0x00, 0x05, 0x01, &bad_step_data);

        let error = SubeventResultEvent::try_from(message.as_slice()).unwrap_err();
        assert!(matches!(error, ParseError::InvalidStepDataLength(0x00, 4, 3)));
    }

    #[test]
    fn test_packet_quality_nibbles_are_decoded_correctly() {
        let message = continue_event(0x01, 0x05, 0x01, &mode1_basic_step_data(0xA2, 0x12, 0x34));

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();
        let mode1 = event.steps[0].info.mode1;

        assert_eq!(mode1.packet.packet_quality.access_address_check_result, 0x02);
        assert_eq!(mode1.packet.packet_quality.payload_bit_error_count, 0x0A);
    }

    #[test]
    fn test_mode2_wrong_length_is_rejected() {
        let bad_step_data = [0x09, 0x48, 0x7B, 0x54, 0x00, 0x00, 0x00, 0x21];
        let message = continue_event(0x02, 0x05, 0x01, &bad_step_data);

        let error = SubeventResultEvent::try_from(message.as_slice()).unwrap_err();
        assert!(matches!(error, ParseError::InvalidStepDataLength(0x02, 8, 9)));
    }

    #[test]
    fn test_mode1_pbr_rtt_initiator_is_parsed() {
        let message = continue_event(0x01, 0x05, 0x01, &mode1_pbr_rtt_step_data(0x21, 0x12, 0x34));

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();
        let mode1 = event.steps[0].info.mode1;

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode1InitiatorPbrRtt
        ));
        assert!(mode1.has_packet_phase_correction_terms);
        let expected_pct = PhaseCorrectionTerm::try_from([0x48, 0x7B, 0x54].as_slice()).unwrap();
        assert_eq!(
            mode1.packet_phase_correction_terms.first_phase_correction_term.i,
            expected_pct.i
        );
        assert_eq!(
            mode1.packet_phase_correction_terms.first_phase_correction_term.q,
            expected_pct.q
        );
    }

    #[test]
    fn test_mode1_pbr_rtt_reflector_is_parsed() {
        let message = continue_event(0x01, 0x05, 0x01, &mode1_pbr_rtt_step_data(0x21, 0x12, 0x34));

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Reflector).unwrap();

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode1ReflectorPbrRtt
        ));
        assert!(matches!(
            event.steps[0].info.mode1.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfDepartureTimeOfArrivalReflector
        ));
    }

    #[test]
    fn test_mode3_reflector_basic_is_parsed() {
        // Mode 3 = Mode 1 + Mode 2: under the collapsed schema, Mode 1 fields
        // are on info.mode1 and tone fields are on info.mode2.
        let step_data = mode3_basic_step_data();
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Reflector).unwrap();

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3Reflector
        ));
        assert!(matches!(
            event.steps[0].info.mode1.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfDepartureTimeOfArrivalReflector
        ));
    }

    #[test]
    fn test_mode3_pbr_rtt_initiator_is_parsed() {
        // Mode 3 = Mode 1 + Mode 2: under the collapsed schema, Mode 1 fields
        // are on info.mode1 and tone fields are on info.mode2.
        let step_data = mode3_pbr_rtt_step_data();
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();
        let mode1 = event.steps[0].info.mode1;

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3InitiatorPbrRtt
        ));
        assert!(mode1.has_packet_phase_correction_terms);
    }

    #[test]
    fn test_mode3_pbr_rtt_reflector_is_parsed() {
        // Mode 3 = Mode 1 + Mode 2: under the collapsed schema, Mode 1 fields
        // are on info.mode1 and tone fields are on info.mode2.
        let step_data = mode3_pbr_rtt_step_data();
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Reflector).unwrap();

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3ReflectorPbrRtt
        ));
        assert!(matches!(
            event.steps[0].info.mode1.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfDepartureTimeOfArrivalReflector
        ));
    }

    #[test]
    fn test_mode1_wrong_length_is_rejected() {
        let bad_step_data = [0x21, 0x80, 0x34, 0x12, 0x34];
        let message = continue_event(0x01, 0x05, 0x01, &bad_step_data);

        let error = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap_err();
        assert!(matches!(error, ParseError::InvalidStepDataLength(0x01, 5, 6)));
    }

    #[test]
    fn test_mode3_wrong_length_is_rejected() {
        let mut bad_step_data = mode1_basic_step_data(0x21, 0x12, 0x34).to_vec();
        bad_step_data.extend_from_slice(&mode2_step_data()[..8]);
        let message = continue_event(0x03, 0x05, 0x01, &bad_step_data);

        let error = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap_err();
        assert!(matches!(error, ParseError::InvalidStepDataLength(0x03, 14, 15)));
    }

    #[test]
    fn test_mode1_unknown_origin_is_rejected() {
        let message = continue_event(0x01, 0x05, 0x01, &mode1_basic_step_data(0x21, 0x12, 0x34));

        let error = SubeventResultEvent::try_from(message.as_slice()).unwrap_err();
        assert!(matches!(error, ParseError::UnknownOriginForMode(0x01)));
    }

    #[test]
    fn test_mode3_unknown_origin_is_rejected() {
        let step_data = mode3_basic_step_data();
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let error = SubeventResultEvent::try_from(message.as_slice()).unwrap_err();
        assert!(matches!(error, ParseError::UnknownOriginForMode(0x03)));
    }

    #[test]
    fn test_invalid_mode_slots_preserve_the_sentinel() {
        // A 0xFF slot followed by a real Mode 2 step: the sentinel is
        // preserved with its own payload kind, so the step window maps 1:1 to
        // the reported slots and no step fabricates mode data.
        let mut message = vec![0x32u8, 0x01, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x02];
        message.extend_from_slice(&[0xFF, 0x05, 0x03, 0xAA, 0xBB, 0xCC]);
        message.extend_from_slice(&[0x02, 0x05, mode2_step_data().len() as u8]);
        message.extend_from_slice(&mode2_step_data());

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown).unwrap();

        assert_eq!(event.step_count, 2);
        let sentinel = &event.steps[0];
        assert_eq!(sentinel.mode, step_mode::MODE_INVALID);
        assert_eq!(sentinel.channel, 0x05);
        assert_eq!(sentinel.info.kind, ModeRoleSpecificInfoKind::Invalid);
        assert_eq!(sentinel.info.kind.mode(), sentinel.mode);
        let mode2 = &event.steps[1];
        assert_eq!(mode2.mode, step_mode::MODE_2);
        assert_eq!(mode2.info.kind, ModeRoleSpecificInfoKind::Mode2);
        assert_eq!(mode2.info.mode2.antenna_permutation_index, 9);
    }

    #[test]
    fn test_config_complete_header_fields_are_parsed() {
        let message = config_complete_event();

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown).unwrap();

        assert!(event.has_initial_meta);
        assert_eq!(event.step_count, 0);
        assert_eq!(event.antenna_path_count, 1);
        assert_eq!(event.config_id, 0x07);
        assert!(event.has_config_id);
        assert_eq!(event.initial_meta.start_acl_conn_event_counter, 0x1234);
        assert!(event.initial_meta.has_start_acl_conn_event_counter);
        assert_eq!(event.initial_meta.procedure_counter, 0x0042);
    }

    #[test]
    fn short_messages_are_rejected_without_panicking() {
        // Boot noise or a truncated buffer can produce short or empty messages;
        // every message index is length-guarded before it is read.
        for message in [
            &[] as &[u8],
            &[le_subevent_code::CS_SUBEVENT_RESULT_CONTINUE],
            &[0x11, 0x01],
            &[le_subevent_code::CS_SUBEVENT_RESULT_CONTINUE, 0x01, 0x00],
        ] {
            let error = SubeventResultEvent::try_from_with_origin(message, Origin::Initiator)
                .expect_err("a message below the common header prefix is rejected");
            assert!(matches!(error, ParseError::TooShort(_)));
        }

        // A continue header cut short (the header runs through index 8).
        let message = continue_event(0x01, 0x05, 0x01, &mode1_basic_step_data(0x21, 0x12, 0x34));
        let error = SubeventResultEvent::try_from_with_origin(&message[..8], Origin::Initiator)
            .expect_err("a truncated continue header is rejected");
        assert!(matches!(error, ParseError::TooShort(8)));

        // A config-complete header cut short (the header runs through index 15).
        let message = [le_subevent_code::CS_CONFIG_COMPLETE; 15];
        let error = SubeventResultEvent::try_from_with_origin(&message, Origin::Initiator)
            .expect_err("a truncated config-complete header is rejected");
        assert!(matches!(error, ParseError::TooShort(15)));

        // A config-complete message of 4..15 bytes with the CS-test handle:
        // `message[3]` is skipped, but the branch guard still fires.
        let message = [le_subevent_code::CS_CONFIG_COMPLETE, 0xFF, 0x0F, 0xAA];
        let error = SubeventResultEvent::try_from_with_origin(&message, Origin::Unknown)
            .expect_err("a truncated config-complete message is rejected");
        assert!(matches!(error, ParseError::TooShort(4)));

        // A multi-step message truncated between steps: the step-header guard
        // must fire before the second step's bytes are read.
        let mut message = continue_event(0x02, 0x05, 0x01, &mode2_step_data());
        message[8] = 2; // report two steps but ship only one

        let error = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown)
            .expect_err("a message truncated between steps is rejected");
        assert!(matches!(error, ParseError::TooShort(_)));
    }

    #[test]
    fn step_data_overruns_are_reported_against_the_declared_length() {
        // The step header claims more step data than the message carries: the
        // error names the declared length and what is actually available, not
        // a generic truncation of the whole message.
        let mut message = continue_event(0x02, 0x05, 0x01, &mode2_step_data());
        message[11] = 0xC8; // 200, beyond the bytes following the step header

        let error = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown)
            .expect_err("a step-data overrun is rejected");
        assert!(
            matches!(
                error,
                ParseError::TruncatedStepData {
                    index: 0,
                    declared: 200,
                    available: 9
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn antenna_path_counts_beyond_the_tone_tables_are_rejected() {
        // Mode 2 tone fields exist for at most `MAX_ANTENNA_PATH_COUNT`
        // antenna paths; the parser rejects an over-range count at the header
        // instead of indexing the fixed tone tables out of bounds.
        let message = continue_event(0x02, 0x05, 0x05, &mode2_step_data());

        let error = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown)
            .expect_err("an over-range antenna path count is rejected");
        assert!(matches!(error, ParseError::ExceededMaxAntennaPathCount));
    }

    #[test]
    fn invalid_step_modes_report_the_offending_message_index() {
        let mut message = continue_event(0x07, 0x05, 0x01, &mode2_step_data());

        let error = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown)
            .expect_err("an unknown step mode is rejected");
        assert!(matches!(error, ParseError::InvalidModeType(0x07, 9)), "{error:?}");

        message[9] = 0x02; // a valid mode keeps the same message parseable
        assert!(SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Unknown).is_ok());
    }
}
