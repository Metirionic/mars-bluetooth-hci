//! HCI and Channel Sounding constants defined by Bluetooth specifications.

/// Errors for HCI parsing and constant lookup.
#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    /// Antenna permutation lookup failed.
    #[error("antenna lookup failed for `n_ap={0}`, `permutation_index={1}`")]
    AntennaLookupFailed(usize, usize),
}

/// HCI packet types (H4 UART transport layer).
pub mod hci_packet_type {
    /// HCI command packet type.
    pub const COMMAND: u8 = 0x01;
    /// HCI ACL data packet type.
    pub const ACL: u8 = 0x02;
    /// HCI SCO data packet type.
    pub const SCO: u8 = 0x03;
    /// HCI event packet type.
    pub const EVENT: u8 = 0x04;
    /// HCI ISO data packet type.
    pub const ISO: u8 = 0x05;
}

/// HCI event codes.
pub mod event_code {
    /// LE Meta Event code.
    pub const LE_META: u8 = 0x3e;
}

/// LE Meta Event subevent codes.
pub mod le_subevent_code {
    /// HCI_LE_CS_Config_Complete subevent.
    pub const CS_CONFIG_COMPLETE: u8 = 0x31;
    /// HCI_LE_CS_Subevent_Result_Continue subevent.
    pub const CS_SUBEVENT_RESULT_CONTINUE: u8 = 0x32;
}

/// CS step modes.
pub mod step_mode {
    /// Step mode 0.
    pub const MODE_0: u8 = 0x00;
    /// Step mode 1.
    pub const MODE_1: u8 = 0x01;
    /// Step mode 2.
    pub const MODE_2: u8 = 0x02;
    /// Step mode 3.
    pub const MODE_3: u8 = 0x03;
}

/// CS procedure parameters.
pub mod cs_params {
    /// Maximum number of steps reported (per Bluetooth CS standard).
    pub const MAX_NUM_STEPS_REPORTED: usize = 160;
    /// Maximum number of antenna paths.
    pub const MAX_ANTENNA_PATH_COUNT: usize = 4;
}

/// Special HCI handle values.
pub mod handle {
    /// Connection handle for CS test mode.
    pub const CS_TEST_CONNECTION_HANDLE: u16 = 0x0FFF;
}

/// Frequency compensation special values.
pub mod frequency_compensation {
    /// Value indicating role is not initiator.
    pub const ROLE_NOT_INITIATOR: u16 = 0xC000;
}

/// Reference power level special values.
pub mod reference_power_level {
    /// Value indicating reference power level is not applicable.
    pub const NOT_APPLICABLE: i8 = 0x7f;
    /// Minimum valid reference power level (dBm).
    pub const MIN_DBM: i8 = -127;
    /// Maximum valid reference power level (dBm).
    pub const MAX_DBM: i8 = 20;
}

/// Procedure abort reason values.
pub mod procedure_abort_reason {
    /// Procedure was not aborted.
    pub const NO_ABORT: u8 = 0x00;
    /// Abort caused by local host or remote request.
    pub const LOCAL_HOST_OR_REMOTE_REQUEST: u8 = 0x01;
    /// Less than 15 channels used for measurement.
    pub const LESS_THAN_15_CHANNELS: u8 = 0x02;
    /// Channel map update instant has passed.
    pub const CHANNEL_MAP_UPDATE_INSTANT_PASSED: u8 = 0x03;
    /// Unspecified abort reason.
    pub const UNSPECIFIED: u8 = 0x0F;
}

/// Subevent abort reason values (nibble-shifted).
pub mod subevent_abort_reason {
    /// Subevent was not aborted.
    pub const NO_ABORT: u8 = 0x00;
    /// Abort caused by local host or remote request.
    pub const LOCAL_HOST_OR_REMOTE_REQUEST: u8 = 0x01;
    /// No CS sync was received.
    pub const NO_CS_SYNC_RECEIVED: u8 = 0x02;
    /// Scheduling conflict or limited resources.
    pub const SCHEDULING_CONFLICT_OR_LIMITED_RESOURCES: u8 = 0x03;
    /// Unspecified abort reason.
    pub const UNSPECIFIED: u8 = 0x0F;
}

/// Tone quality indicator values.
pub mod tone_quality_indicator {
    /// High quality.
    pub const HIGH: u8 = 0x00;
    /// Medium quality.
    pub const MEDIUM: u8 = 0x01;
    /// Low quality.
    pub const LOW: u8 = 0x02;
    /// Quality unavailable.
    pub const UNAVAILABLE: u8 = 0x03;
}

/// Extension slot types.
pub mod extension_slot {
    /// No extension slot present.
    pub const NOT_PRESENT: u8 = 0x00;
    /// Extension slot present but not expected.
    pub const NOT_EXPECTED_PRESENT: u8 = 0x01;
    /// Extension slot present and expected.
    pub const EXPECTED_PRESENT: u8 = 0x02;
}

/// Antenna path permutation tables per Bluetooth CS spec Vol 6, Part H, Tables 4.13–4.15.
///
/// Each table maps a permutation index to an array where `result[path_index]` is the
/// physical antenna (0-based) assigned to that logical path position.
/// Unused positions (beyond `n_ap`) are padded with the identity mapping.
pub mod antenna_permutation {
    use super::Error;
    use super::cs_params::MAX_ANTENNA_PATH_COUNT;

    /// N_AP = 1: 1 permutation.
    pub const TABLE_1: [usize; MAX_ANTENNA_PATH_COUNT] = [0, 1, 2, 3];

    /// N_AP = 2 (Table 4.13): 2 permutations.
    pub const TABLE_2: [[usize; MAX_ANTENNA_PATH_COUNT]; 2] = [[0, 1, 2, 3], [1, 0, 2, 3]];

    /// N_AP = 3 (Table 4.14): 6 permutations.
    pub const TABLE_3: [[usize; MAX_ANTENNA_PATH_COUNT]; 6] = [
        [0, 1, 2, 3],
        [1, 0, 2, 3],
        [0, 2, 1, 3],
        [2, 0, 1, 3],
        [2, 1, 0, 3],
        [1, 2, 0, 3],
    ];

    /// N_AP = 4 (Table 4.15): 24 permutations.
    pub const TABLE_4: [[usize; MAX_ANTENNA_PATH_COUNT]; 24] = [
        [0, 1, 2, 3],
        [1, 0, 2, 3],
        [0, 2, 1, 3],
        [2, 0, 1, 3],
        [2, 1, 0, 3],
        [1, 2, 0, 3],
        [0, 1, 3, 2],
        [1, 0, 3, 2],
        [0, 3, 1, 2],
        [3, 0, 1, 2],
        [3, 1, 0, 2],
        [1, 3, 0, 2],
        [0, 3, 2, 1],
        [3, 0, 2, 1],
        [0, 2, 3, 1],
        [2, 0, 3, 1],
        [2, 3, 0, 1],
        [3, 2, 0, 1],
        [3, 1, 2, 0],
        [1, 3, 2, 0],
        [3, 2, 1, 0],
        [2, 3, 1, 0],
        [2, 1, 3, 0],
        [1, 2, 3, 0],
    ];

    /// Number of valid permutations for each antenna count (N_AP!).
    pub const PERMUTATION_COUNT: [usize; MAX_ANTENNA_PATH_COUNT + 1] = [1, 1, 2, 6, 24];

    /// Look up the antenna permutation for a given antenna count and permutation index.
    ///
    /// Returns an array where `result[path_index]` is the physical antenna index
    /// (0-based) assigned to logical path position `path_index`.
    /// Only the first `n_ap` elements are meaningful.
    pub fn lookup(
        n_ap: usize,
        permutation_index: usize,
    ) -> Result<[usize; MAX_ANTENNA_PATH_COUNT], Error> {
        match n_ap {
            1 if permutation_index == 0 => Ok(TABLE_1),
            2 if permutation_index < 2 => Ok(TABLE_2[permutation_index]),
            3 if permutation_index < 6 => Ok(TABLE_3[permutation_index]),
            4 if permutation_index < 24 => Ok(TABLE_4[permutation_index]),
            _ => Err(Error::AntennaLookupFailed(n_ap, permutation_index)),
        }
    }
}
