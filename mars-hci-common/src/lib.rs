//! Shared features for Mars HCI crates.
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

#[macro_use]
mod fmt;

#[cfg(feature = "libc")]
pub mod libc;

#[cfg(feature = "std")]
pub mod serde;
