pub mod constants;

/// Versioned persistence contract for decoded CS subevent-result frames.
///
/// Available when the `persisted-frame` feature is enabled — part of the
/// default build, which enables `std`, `alloc`, and the HCI FFI serialization
/// surface. It is deliberately unavailable to the embedded no-std feature
/// set. The persistence decision is recorded in ADR-0003.
#[cfg(feature = "persisted-frame")]
pub mod persisted_frame;

pub mod subevent_result;

#[cfg(any(feature = "std", test))]
pub mod hci_file_reader;
