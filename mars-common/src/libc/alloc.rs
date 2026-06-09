//! Use allocation/deallocation functions from C for rust.
extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};

unsafe extern "C" {
    fn malloc(len: usize) -> *mut u8;
    fn free(item: *mut u8);
}

struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { malloc(layout.size()) as *mut _ }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { free(ptr as *mut _) };
    }
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator {};
