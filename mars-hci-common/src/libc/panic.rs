//! Panic handler for use with C host applications.

use alloc::ffi::CString;
use alloc::format;
use core::ffi::c_char;
use core::panic::PanicInfo;

extern crate alloc;

unsafe extern "C" {
    fn rust_panic_cb(panic: *const c_char);
}

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    let panic_message = CString::new(format!("{}", panic)).unwrap();

    unsafe { rust_panic_cb(panic_message.as_ptr() as *const c_char) };

    loop {}
}