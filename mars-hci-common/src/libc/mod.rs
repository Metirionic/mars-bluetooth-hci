//! Interface library for using rust with C.

#[cfg(feature = "libc-alloc")]
pub mod alloc;

#[cfg(feature = "libc-panic")]
pub mod panic;

pub mod serialize;

#[cfg(target_os = "android")]
#[safer_ffi::ffi_export]
pub extern "C" fn rust_eh_personality() {}
