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
use crate::event::hci_le_cs::constants::{handle, le_subevent_code, step_mode};
use crate::event::{
    ExtensionSlot, FrequencyCompensation, ParseError, ProcedureAbortReason, ProcedureDoneStatus, ProcedureInfo,
    ReferencePowerLevel, SubeventAbortReason, SubeventDoneStatus, SubeventInfo, ToneQualityIndicator,
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

/// Grouped tone fields shared by Mode 2 and Mode 3.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct ToneSection {
    /// The selected antenna permutation index.
    pub antenna_permutation_index: u8,
    /// The phase correction terms for the tone sequence.
    pub phase_correction_terms: [PhaseCorrectionTerm; MAX_ANTENNA_PATH_COUNT + 1],
    /// The quality indicators for the tone sequence.
    pub quality_indicators: [ToneQualityIndicator; MAX_ANTENNA_PATH_COUNT + 1],
    /// The selected extension slots for the tone sequence.
    pub extension_slots: [ExtensionSlot; MAX_ANTENNA_PATH_COUNT + 1],
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
// FIXME: `Mode2` currently duplicates the grouped tone fields from `ToneSection`.
// Keep this stable for downstream consumers for now, but consider collapsing the
// duplication once the parser, examples, and processing side are aligned.
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

impl From<ToneSection> for Mode2 {
    fn from(value: ToneSection) -> Self {
        Self {
            antenna_permutation_index: value.antenna_permutation_index,
            phase_correction_terms: value.phase_correction_terms,
            quality_indicators: value.quality_indicators,
            extension_slots: value.extension_slots,
        }
    }
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

/// Compact Mode 3 payload stored once per reported step.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct Mode3Data {
    /// Shared packet-level fields.
    pub packet: RoundTripTimePacketFields,
    /// Role-specific RTT timing delta.
    pub timing: RoundTripTimeRoleTiming,
    /// Grouped tone fields.
    pub tones: ToneSection,
    /// Optional packet phase correction terms.
    pub packet_phase_correction_terms: PacketPhaseCorrectionTerms,
    /// If true, `packet_phase_correction_terms` is valid.
    pub has_packet_phase_correction_terms: bool,
}

/// Discriminant for [`ModeRoleSpecificInfo`].
///
/// # Step-mode support limitation
///
/// Only [`ModeRoleSpecificInfoKind::Mode2`] is populated by the parser: [`Mode2`]
/// step data is decoded in [`SubeventResultEvent`]'s parse path. Mode 0 is
/// recognized but carries no step data (a no-op), and Mode 1 / Mode 3 step inputs
/// return [`ParseError::InvalidModeType`]. The Mode 1 and Mode 3 variants here
/// exist for ABI completeness so the C enum stays forward-compatible. Implementing
/// the remaining modes is tracked in
/// <https://github.com/Metironic/mars-bluetooth-hci/issues/9>.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(u8)]
pub enum ModeRoleSpecificInfoKind {
    /// Mode 0, reflector role. Recognized by the parser but carries no step data.
    Mode0Reflector,
    /// Mode 1, initiator role. Not populated by the parser.
    Mode1Initiator,
    /// Mode 1, initiator role, with PBR and RTT measurements. Not populated by the parser.
    Mode1InitiatorPbrRtt,
    /// Mode 1, reflector role. Not populated by the parser.
    Mode1Reflector,
    /// Mode 1, reflector role, with PBR and RTT measurements. Not populated by the parser.
    Mode1ReflectorPbrRtt,
    /// Mode 2. The only step mode populated by the parser (see [`Mode2`]).
    #[default]
    Mode2,
    /// Mode 3, initiator role. Not populated by the parser.
    Mode3Initiator,
    /// Mode 3, initiator role, with PBR and RTT measurements. Not populated by the parser.
    Mode3InitiatorPbrRtt,
    /// Mode 3, reflector role. Not populated by the parser.
    Mode3Reflector,
    /// Mode 3, reflector role, with PBR and RTT measurements. Not populated by the parser.
    Mode3ReflectorPbrRtt,
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
    /// Mode1 data. Valid when `kind`
    /// is [`ModeRoleSpecificInfoKind::Mode1Initiator`]
    /// or [`ModeRoleSpecificInfoKind::Mode1InitiatorPbrRtt`]
    /// or [`ModeRoleSpecificInfoKind::Mode1Reflector`]
    /// or [`ModeRoleSpecificInfoKind::Mode1ReflectorPbrRtt`].
    pub mode1: Mode1Data,
    /// Mode2 data. Only valid when `kind`
    ///  is [`ModeRoleSpecificInfoKind::Mode2`].
    pub mode2: Mode2,
    /// Mode3 data. Valid when `kind`
    /// is [`ModeRoleSpecificInfoKind::Mode3Initiator`]
    /// or [`ModeRoleSpecificInfoKind::Mode3InitiatorPbrRtt`]
    /// or [`ModeRoleSpecificInfoKind::Mode3Reflector`]
    /// or [`ModeRoleSpecificInfoKind::Mode3ReflectorPbrRtt`].
    pub mode3: Mode3Data,
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

impl SubeventResultEvent {
    /// Parse a subevent result message with known origin information.
    pub fn try_from_with_origin(message: &[u8], origin: Origin) -> Result<Self, ParseError> {
        Self::parse_internal(message, origin)
    }

    /// Length of a basic Mode 1 step payload.
    const MODE1_LEN: usize = 6;
    /// Length of a Mode 1 step payload with packet phase correction terms.
    const MODE1_PBR_RTT_LEN: usize = 14;

    /// Return the expected Mode 2 step payload length for an antenna path count.
    fn mode2_len(antenna_path_count: usize) -> usize {
        4 * antenna_path_count + 5
    }

    /// Return the expected basic Mode 3 step payload length for an antenna path count.
    fn mode3_len(antenna_path_count: usize) -> usize {
        Self::MODE1_LEN + Self::mode2_len(antenna_path_count)
    }

    /// Return the expected Mode 3 step payload length with packet phase correction terms.
    fn mode3_pbr_rtt_len(antenna_path_count: usize) -> usize {
        Self::MODE1_PBR_RTT_LEN + Self::mode2_len(antenna_path_count)
    }

    /// Parse the packet quality byte shared by packet-based ranging modes.
    fn parse_packet_quality(quality_byte: u8) -> PacketQuality {
        PacketQuality {
            access_address_check_result: quality_byte & 0x0F,
            payload_bit_error_count: (quality_byte >> 4) & 0x0F,
        }
    }

    /// Parse the packet-level fields shared by Mode 1 and Mode 3.
    fn parse_rtt_packet_fields(step_data: &[u8]) -> Result<RoundTripTimePacketFields, ParseError> {
        Ok(RoundTripTimePacketFields {
            packet_quality: Self::parse_packet_quality(step_data[0]),
            packet_normalized_attack_detector_metric: step_data[1],
            packet_received_signal_strength_indicator: step_data[2] as i8,
            packet_antenna: step_data[5],
        })
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

    /// Parse optional packet phase correction terms present in `PbrRtt` variants.
    fn parse_packet_phase_correction_terms(step_data: &[u8]) -> Result<PacketPhaseCorrectionTerms, ParseError> {
        Ok(PacketPhaseCorrectionTerms {
            first_phase_correction_term: step_data[0..3].try_into()?,
            second_phase_correction_term: step_data[4..7].try_into()?,
        })
    }

    /// Parse the grouped tone section shared by Mode 2 and Mode 3.
    fn parse_tone_section(
        step_data: &[u8],
        antenna_path_count: usize,
        offset: usize,
    ) -> Result<ToneSection, ParseError> {
        let mut tones = ToneSection {
            antenna_permutation_index: step_data[offset],
            ..Default::default()
        };

        let tone_count = antenna_path_count + 1;
        let tone_quality_offset = offset + 1 + 3 * tone_count;

        for antenna_path_index in 0..tone_count {
            let phase_correction_offset = offset + 1 + 3 * antenna_path_index;
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

        Ok(Self::parse_tone_section(step_data, antenna_path_count, 0)?.into())
    }

    /// Parse one Mode 1 step payload.
    fn parse_mode1_step(step_data: &[u8], origin: Origin) -> Result<Mode1Data, ParseError> {
        let (has_packet_phase_correction_terms, expected_step_data_length) = match step_data.len() {
            Self::MODE1_LEN => (false, Self::MODE1_LEN),
            Self::MODE1_PBR_RTT_LEN => (true, Self::MODE1_PBR_RTT_LEN),
            _ => {
                return Err(ParseError::InvalidStepDataLength(
                    step_mode::MODE_1,
                    step_data.len(),
                    Self::MODE1_LEN,
                ));
            }
        };

        let packet_phase_correction_terms = if has_packet_phase_correction_terms {
            Self::parse_packet_phase_correction_terms(&step_data[6..14])?
        } else {
            Default::default()
        };

        debug_assert_eq!(step_data.len(), expected_step_data_length);

        Ok(Mode1Data {
            packet: Self::parse_rtt_packet_fields(step_data)?,
            timing: Self::parse_rtt_role_timing(step_data, origin)?,
            packet_phase_correction_terms,
            has_packet_phase_correction_terms,
        })
    }

    /// Parse one Mode 3 step payload.
    fn parse_mode3_step(step_data: &[u8], origin: Origin, antenna_path_count: usize) -> Result<Mode3Data, ParseError> {
        let (has_packet_phase_correction_terms, expected_step_data_length, tone_offset) = match step_data.len() {
            len if len == Self::mode3_len(antenna_path_count) => (false, len, Self::MODE1_LEN),
            len if len == Self::mode3_pbr_rtt_len(antenna_path_count) => (true, len, Self::MODE1_PBR_RTT_LEN),
            _ => {
                return Err(ParseError::InvalidStepDataLength(
                    step_mode::MODE_3,
                    step_data.len(),
                    Self::mode3_len(antenna_path_count),
                ));
            }
        };

        let packet_phase_correction_terms = if has_packet_phase_correction_terms {
            Self::parse_packet_phase_correction_terms(&step_data[6..14])?
        } else {
            Default::default()
        };

        if step_data.len() != expected_step_data_length {
            return Err(ParseError::InvalidStepDataLength(
                step_mode::MODE_3,
                step_data.len(),
                expected_step_data_length,
            ));
        }

        Ok(Mode3Data {
            packet: Self::parse_rtt_packet_fields(step_data)?,
            timing: Self::parse_rtt_role_timing(step_data, origin)?,
            tones: Self::parse_tone_section(step_data, antenna_path_count, tone_offset)?,
            packet_phase_correction_terms,
            has_packet_phase_correction_terms,
        })
    }

    /// Push steps from a binary message into the subevent result event.
    fn push_steps(&mut self, message: &[u8]) -> Result<(), ParseError> {
        let mut step_index = 0;
        let mut step_byte_offset = if self.has_initial_meta { 16 } else { 9 };

        for _ in 0..self.step_count {
            let step_mode = message[step_byte_offset];
            let step_channel = message[1 + step_byte_offset];
            let step_data_length = message[2 + step_byte_offset] as usize;
            let step_data = &message[3 + step_byte_offset..3 + step_byte_offset + step_data_length];

            match step_mode {
                step_mode::MODE_0 => {}
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
                    let mode3 = Self::parse_mode3_step(step_data, self.origin, self.antenna_path_count)?;
                    self.steps[step_index] = Step {
                        mode: step_mode,
                        channel: step_channel,
                        info: ModeRoleSpecificInfo {
                            kind: self.mode_3_selector(mode3.has_packet_phase_correction_terms),
                            mode3,
                            ..Default::default()
                        },
                    };
                    step_index += 1;
                }
                _ => {
                    return Err(ParseError::InvalidModeType(step_mode, 16 + step_byte_offset));
                }
            }

            step_byte_offset += 3 + step_data_length;
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
    fn parse_internal(message: &[u8], origin: Origin) -> Result<Self, ParseError> {
        let connection_handle = u16::from_le_bytes(message[1..3].try_into()?);
        let connection_handle_is_cs_test = connection_handle == handle::CS_TEST_CONNECTION_HANDLE;

        let (config_id, has_config_id) = if connection_handle_is_cs_test {
            (0, false)
        } else {
            (message[3], true)
        };

        let mut event = match message[0] {
            le_subevent_code::CS_CONFIG_COMPLETE => {
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

                if num_steps_reported > MAX_NUM_STEPS_REPORTED {
                    return Err(ParseError::ExceededMaxStepCount);
                }

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
                let abort_reason = message[6];
                let procedure = ProcedureInfo::from((message[4], abort_reason));
                let subevent = SubeventInfo::from((message[5], abort_reason));

                let num_antenna_paths = message[7] as usize;
                let num_steps_reported = message[8] as usize;

                if num_steps_reported > MAX_NUM_STEPS_REPORTED {
                    return Err(ParseError::ExceededMaxStepCount);
                }

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

#[cfg(test)]
mod tests {
    use super::{
        ModeRoleSpecificInfoKind, Origin, PhaseCorrectionTerm, RoundTripTimeRoleTimingKind, SubeventResultEvent,
    };
    use crate::event::hci_le_cs::constants::le_subevent_code;
    use crate::event::{ExtensionSlot, ParseError, ToneQualityIndicator};

    fn continue_event(step_mode: u8, channel: u8, antenna_path_count: u8, step_data: &[u8]) -> Vec<u8> {
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

    fn mode1_basic_step_data(quality: u8, timing_lo: u8, timing_hi: u8) -> [u8; 6] {
        [quality, 0x80, 0x34, timing_lo, timing_hi, 0x02]
    }

    fn mode1_pbr_rtt_step_data(quality: u8, timing_lo: u8, timing_hi: u8) -> [u8; 14] {
        [
            quality, 0x80, 0x34, timing_lo, timing_hi, 0x02, 0x48, 0x7B, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    }

    fn mode2_step_data() -> [u8; 9] {
        [0x09, 0x48, 0x7B, 0x54, 0x00, 0x00, 0x00, 0x21, 0x03]
    }

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
        assert!(!mode1.has_packet_phase_correction_terms);
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
        let mut step_data = mode1_basic_step_data(0x21, 0x12, 0x34).to_vec();
        step_data.extend_from_slice(&mode2_step_data());
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();

        assert_eq!(event.step_count, 1);
        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3Initiator
        ));
        assert_eq!(event.steps[1].mode, 0);

        let mode3 = event.steps[0].info.mode3;
        assert_eq!(mode3.packet.packet_quality.access_address_check_result, 1);
        assert_eq!(mode3.packet.packet_quality.payload_bit_error_count, 2);
        assert_eq!(mode3.packet.packet_antenna, 0x02);
        assert!(matches!(
            mode3.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfArrivalTimeOfDepartureInitiator
        ));
        assert_eq!(mode3.tones.antenna_permutation_index, 9);
        let expected_pct = PhaseCorrectionTerm::try_from([0x48, 0x7B, 0x54].as_slice()).unwrap();
        assert_eq!(mode3.tones.phase_correction_terms[0].i, expected_pct.i);
        assert_eq!(mode3.tones.phase_correction_terms[0].q, expected_pct.q);
        assert!(matches!(
            mode3.tones.quality_indicators[0],
            ToneQualityIndicator::Medium
        ));
        assert!(matches!(mode3.tones.extension_slots[0], ExtensionSlot::ExpectedPresent));
        assert!(!mode3.has_packet_phase_correction_terms);
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
        let mut step_data = mode1_basic_step_data(0x21, 0x12, 0x34).to_vec();
        step_data.extend_from_slice(&mode2_step_data());
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Reflector).unwrap();

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3Reflector
        ));
        assert!(matches!(
            event.steps[0].info.mode3.timing.kind,
            RoundTripTimeRoleTimingKind::TimeOfDepartureTimeOfArrivalReflector
        ));
    }

    #[test]
    fn test_mode3_pbr_rtt_initiator_is_parsed() {
        let mut step_data = mode1_pbr_rtt_step_data(0x21, 0x12, 0x34).to_vec();
        step_data.extend_from_slice(&mode2_step_data());
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Initiator).unwrap();
        let mode3 = event.steps[0].info.mode3;

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3InitiatorPbrRtt
        ));
        assert!(mode3.has_packet_phase_correction_terms);
    }

    #[test]
    fn test_mode3_pbr_rtt_reflector_is_parsed() {
        let mut step_data = mode1_pbr_rtt_step_data(0x21, 0x12, 0x34).to_vec();
        step_data.extend_from_slice(&mode2_step_data());
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let event = SubeventResultEvent::try_from_with_origin(message.as_slice(), Origin::Reflector).unwrap();

        assert!(matches!(
            event.steps[0].info.kind,
            ModeRoleSpecificInfoKind::Mode3ReflectorPbrRtt
        ));
        assert!(matches!(
            event.steps[0].info.mode3.timing.kind,
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
        let mut step_data = mode1_basic_step_data(0x21, 0x12, 0x34).to_vec();
        step_data.extend_from_slice(&mode2_step_data());
        let message = continue_event(0x03, 0x05, 0x01, &step_data);

        let error = SubeventResultEvent::try_from(message.as_slice()).unwrap_err();
        assert!(matches!(error, ParseError::UnknownOriginForMode(0x03)));
    }
}
