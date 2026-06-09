//! Transports serialized data back and forth between rust and C.
extern crate alloc;

use core::ffi::{c_uchar, c_void};
use core::ptr::addr_of;

use safer_ffi::{derive_ReprC, ffi_export};

/// A representation of a serialized data buffer.
///
/// This is used for handing off memory from rust to a C application, and vice-versa.
/// If this was allocated from a rust vector, it has to be freed by rust.
#[derive_ReprC]
#[repr(C)]
pub struct SerializedData {
    /// A pointer to the buffer that holds serialized data.
    p_data: *mut c_uchar,
    /// The size of the buffer.
    size: usize,
    /// The maximum capacity of the buffer in byte.
    capacity: usize,
}

impl SerializedData {
    /// Create empty serialized data.
    pub fn empty() -> Self {
        Self {
            p_data: core::ptr::null_mut(),
            size: 0,
            capacity: 0,
        }
    }

    /// Determine, whether serialized data is empty.
    pub fn is_empty(&self) -> bool {
        self.p_data.is_null() || self.size == 0 || self.capacity == 0
    }
}

impl From<alloc::vec::Vec<u8>> for SerializedData {
    fn from(mut value: alloc::vec::Vec<u8>) -> Self {
        let data = SerializedData {
            size: value.len(),
            capacity: value.capacity(),
            p_data: value.as_mut_ptr(),
        };

        core::mem::forget(value);

        data
    }
}

impl From<SerializedData> for alloc::vec::Vec<u8> {
    fn from(value: SerializedData) -> Self {
        unsafe { alloc::vec::Vec::from_raw_parts(value.p_data, value.size, value.capacity) }
    }
}

impl From<&SerializedData> for &[u8] {
    fn from(value: &SerializedData) -> Self {
        unsafe { core::slice::from_raw_parts(value.p_data, value.size) }
    }
}

/// Get the raw pointer from a reference.
pub fn as_ptr<T>(reference: &mut T) -> *const c_void {
    addr_of!(*reference) as _
}

/// Get the mutable raw pointer from a reference.
pub fn as_mut_ptr<T>(reference: &mut T) -> *mut c_void {
    addr_of!(*reference) as _
}

/// Allocate `length` bytes in heap memory, and fill them with increasing u8 values.
///
/// If `use_cobs` is `true`, allocates at least two times `length` bytes and performs COBS encoding.
///
/// # Safety
/// Dereferences the raw pointer for `written_length`. Make sure it is valid.
#[ffi_export]
pub unsafe extern "C" fn new_dummy_data(length: usize, use_cobs: bool, written_length: *mut usize) -> *const u8 {
    let mut test_vec: alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity(length);
    for value in 0..length {
        test_vec.push(value as u8);
    }

    if use_cobs {
        test_vec = cobs::encode_vec(&test_vec);
    }

    unsafe {
        *written_length = test_vec.len();
    }

    let p = test_vec.as_ptr();
    core::mem::forget(test_vec);

    p
}

/// Drop previously allocated memory.
#[ffi_export]
pub extern "C" fn drop_bin(serialized: SerializedData) {
    if !serialized.is_empty() {
        let data: alloc::vec::Vec<u8> = serialized.into();
        drop(data);
    }
}
