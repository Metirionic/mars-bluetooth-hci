pub mod constants;

/// Versioned persistence contract for decoded CS subevent-result frames.
#[cfg(feature = "persisted-frame")]
pub mod persisted_frame;

pub mod subevent_result;

#[cfg(any(feature = "std", test))]
pub mod hci_file_reader;
