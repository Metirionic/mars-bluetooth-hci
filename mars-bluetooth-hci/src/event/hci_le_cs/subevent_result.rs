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

/// An unsupported field (used as a placeholder).
#[derive(Debug)]
pub struct Unsupported {}

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
/// The `mode2` field is only valid when `kind` is [`ModeRoleSpecificInfoKind::Mode2`].
// FIXME: Add support for other kinds.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[derive_ReprC]
#[repr(C)]
pub struct ModeRoleSpecificInfo {
    /// The kind of mode- and role-specific information.
    pub kind: ModeRoleSpecificInfoKind,
    /// Mode2 data. Only valid when `kind` is [`ModeRoleSpecificInfoKind::Mode2`].
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

impl SubeventResultEvent {
    /// Push steps from a binary message into the subevent result event.
    fn push_steps(&mut self, message: &[u8]) -> Result<(), ParseError> {
        let mut step_index = 0;
        let mut step_byte_offset = if self.has_initial_meta { 16 } else { 9 };

        for _ in 0..self.step_count {
            let step_mode = message[step_byte_offset];
            let step_channel = message[1 + step_byte_offset];
            let step_data_length = message[2 + step_byte_offset] as usize;
            let step_data = &message[3 + step_byte_offset..3 + step_byte_offset + step_data_length];

            let mut antenna_path_byte_offset = 0;
            for antenna_path_index in 0..self.antenna_path_count {
                match step_mode {
                    step_mode::MODE_0 => {}
                    step_mode::MODE_2 => {
                        let mut mode2 = Mode2 {
                            antenna_permutation_index: step_data[antenna_path_byte_offset],
                            ..Default::default()
                        };
                        mode2.phase_correction_terms[antenna_path_index] = step_data
                            [antenna_path_byte_offset + 1..antenna_path_byte_offset + 4]
                            .try_into()
                            .unwrap();
                        mode2.quality_indicators[antenna_path_index] = step_data[antenna_path_byte_offset + 5].into();

                        let step = Step {
                            mode: step_mode,
                            channel: step_channel,
                            info: ModeRoleSpecificInfo {
                                kind: ModeRoleSpecificInfoKind::Mode2,
                                mode2,
                            },
                        };

                        self.steps[step_index] = step;
                        step_index += 1;

                        antenna_path_byte_offset += 5;
                    }
                    _ => {
                        return Err(ParseError::InvalidModeType(step_mode, 16 + step_byte_offset));
                    }
                };
            }

            step_byte_offset += 3 + step_data_length;
        }

        Ok(())
    }
}

/// Parse a subevent from its raw HCI bytes.
///
/// The parser populates every field decoded from the wire but leaves the
/// identity fields (`origin`, `local_mac`, `peer_mac`) at their defaults
/// ([`Origin::Unknown`] / `0`); the caller sets them from out-of-band context
/// — `origin` in the same way the file-reader helper does.
///
/// # Examples
///
/// ```
/// use mars_bluetooth_hci::event::hci_le_cs::subevent_result::{SubeventResultEvent, Origin};
/// use mars_bluetooth_hci::constants::le_subevent_code::CS_CONFIG_COMPLETE;
///
/// // Minimal HCI_LE_CS_Config_Complete frame: 16-byte header, zero steps reported.
/// let bytes = [
///     CS_CONFIG_COMPLETE, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
/// ];
///
/// let mut event = SubeventResultEvent::try_from(bytes.as_slice()).unwrap();
///
/// // The parser leaves the identity fields at their defaults by design.
/// assert!(matches!(event.origin, Origin::Unknown));
/// assert_eq!(event.local_mac, 0);
/// assert_eq!(event.peer_mac, 0);
///
/// // The caller fills them in from out-of-band context.
/// event.origin = Origin::Initiator;
/// event.local_mac = 0xAABB_CCDD_EEFF;
/// event.peer_mac = 0x1122_3344_5566;
///
/// assert!(matches!(event.origin, Origin::Initiator));
/// assert_eq!(event.local_mac, 0xAABB_CCDD_EEFF);
/// ```
impl TryFrom<&[u8]> for SubeventResultEvent {
    type Error = ParseError;

    fn try_from(message: &[u8]) -> Result<Self, Self::Error> {
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
                    return Err(Self::Error::ExceededMaxStepCount);
                }

                SubeventResultEvent {
                    origin: Origin::Unknown,
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
                    return Err(Self::Error::ExceededMaxStepCount);
                }

                SubeventResultEvent {
                    origin: Origin::Unknown,
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

#[cfg(test)]
mod tests {
    use super::PhaseCorrectionTerm;

    #[test]
    fn test_pct() {
        let bin = [0x48, 0x7B, 0x54];
        let pct = PhaseCorrectionTerm::try_from(bin.as_slice()).unwrap();

        println!("{:?}", pct);
    }
}
