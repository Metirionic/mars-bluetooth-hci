//! Bluetooth HCI (host controller interface) message encoding and decoding for the device side.
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

/// Describes HCI events that can be parsed from different types of input.
pub mod event;

#[cfg(feature = "libc")]
pub mod libc;

/// Re-export of HCI and CS constants for convenience.
pub use event::hci_le_cs::constants;

/// Generate C headers.
#[cfg(feature = "headers")]
pub fn generate_headers(target_path: &String) -> std::io::Result<()> {
    safer_ffi::headers::builder().to_file(target_path)?.generate()
}