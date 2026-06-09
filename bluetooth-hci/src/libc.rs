//! C interface module for serialization.
extern crate alloc;

use core::ffi::c_char;
use core::ffi::c_str::CStr;

use mars_hci_common::libc::serialize::SerializedData;
use postcard::{to_allocvec, to_allocvec_cobs};
use safer_ffi::ffi_export;
use serde::Serialize;

use crate::event::hci_le_cs::subevent_result::SubeventResultEvent;

/// Serializable wrapper for data that crosses the FFI boundary.
///
/// Used to wrap different data types into a single serializable unit,
/// so that the C/embedded side can serialize both subevent results and
/// log messages into a uniform byte stream (via [`postcard`]).
/// The receiver side deserializes this enum and dispatches accordingly.
///
/// The [`SubeventResultEvent`] variant is boxed to avoid placing the
/// large struct (~8 KB) on the stack during deserialization.
#[derive(Debug, Serialize, serde::Deserialize)]
#[cfg(feature = "std")]
pub enum Serializable<'d> {
    /// A CS subevent result from the HCI layer.
    SubeventResultEvent(Box<SubeventResultEvent>),
    /// A log message from the firmware, borrowed from the C string.
    #[serde(borrow)]
    LogMessage(&'d str),
}

/// Borrowing variant of [`Serializable`] for zero-allocation serialization.
///
/// Uses references instead of owned/boxed values so the C/embedded side
/// can serialize without cloning or heap-allocating the large event struct.
/// Produces an identical wire format to [`Serializable`].
#[derive(Serialize)]
pub enum SerializableRef<'d> {
    /// A CS subevent result from the HCI layer.
    SubeventResultEvent(&'d SubeventResultEvent),
    /// A log message from the firmware, borrowed from the C string.
    #[serde(borrow)]
    LogMessage(&'d str),
}

/// Serialize a subevent result event to [`SerializedData`].
#[ffi_export]
pub extern "C" fn serialize_subevent_result_event(
    p_event: &SubeventResultEvent,
    use_cobs: bool,
) -> SerializedData {
    let event = SerializableRef::SubeventResultEvent(p_event);

    if use_cobs {
        to_allocvec_cobs(&event).unwrap().into()
    } else {
        to_allocvec(&event).unwrap().into()
    }
}

/// Serialize a log message to [`SerializedData`].
///
/// # Safety
/// Dereferences the raw pointer for `p_log_message`. Make sure it is valid.
#[ffi_export]
pub unsafe extern "C" fn serialize_log_message(
    p_log_message: *const c_char,
    use_cobs: bool,
) -> SerializedData {
    let message = SerializableRef::LogMessage(
        unsafe { CStr::from_ptr(p_log_message) }
            .to_str()
            .unwrap_or_default(),
    );

    if use_cobs {
        to_allocvec_cobs(&message).unwrap().into()
    } else {
        to_allocvec(&message).unwrap().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_message_wire_format_matches() {
        let msg = "test message";
        let ref_bytes = postcard::to_allocvec(&SerializableRef::LogMessage(msg)).unwrap();
        let owned_bytes = postcard::to_allocvec(&Serializable::LogMessage(msg)).unwrap();
        assert_eq!(ref_bytes, owned_bytes);
    }

    #[test]
    fn log_message_roundtrip() {
        let msg = "hello from firmware";
        let bytes = postcard::to_allocvec(&SerializableRef::LogMessage(msg)).unwrap();
        let deserialized: Serializable = postcard::from_bytes(&bytes).unwrap();
        assert!(matches!(deserialized, Serializable::LogMessage(m) if m == msg));
    }

    #[test]
    fn log_message_cobs_roundtrip() {
        let msg = "cobs test";
        let mut bytes = postcard::to_allocvec_cobs(&SerializableRef::LogMessage(msg)).unwrap();
        let deserialized: Serializable = postcard::from_bytes_cobs(&mut bytes).unwrap();
        assert!(matches!(deserialized, Serializable::LogMessage(m) if m == msg));
    }
}
