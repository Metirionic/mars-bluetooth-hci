# mars-common

[![crates.io](https://img.shields.io/crates/v/mars-common.svg)](https://crates.io/crates/mars-common)
[![docs.rs](https://docs.rs/mars-common/badge.svg)](https://docs.rs/mars-common)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)
![no_std](https://img.shields.io/badge/no__std-supported-orange)

Shared FFI infrastructure and logging dispatch for Mars crates.

Part of the [mars-bluetooth-hci](https://github.com/Metirionic/mars-bluetooth-hci) workspace. See also: [`mars-bluetooth-hci`](../mars-bluetooth-hci).

## Overview

`mars-common` provides the foundational building blocks shared across the Mars crate ecosystem:

- **`SerializedData`** — FFI-safe buffer (pointer + size + capacity) for passing serialized data across the C boundary
- **Logging macros** — Dual-backend dispatch to [`log`](https://crates.io/crates/log) or [`defmt`](https://crates.io/crates/defmt), with a no-op fallback for bare-metal targets
- **`BinaryData`** — Timestamped binary data container with [`postcard`](https://crates.io/crates/postcard) serialization (`std` feature)
- **TOML config helpers** — `store_config` / `load_config` for persisting configuration (`std` feature)
- **C allocator bridge** — `malloc`/`free`-backed global allocator (`libc-alloc` feature)
- **C panic handler bridge** — Forward panics to a C callback (`libc-panic` feature)

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enable standard library, `BinaryData`, TOML config helpers |
| `alloc` | No | Enable heap allocation (`postcard/alloc`) |
| `libc` | Yes | Enable FFI serialization (`SerializedData`, `drop_bin`, `new_dummy_data`) |
| `libc-alloc` | No | C allocator bridge (`malloc`/`free`) |
| `libc-panic` | No | C panic handler bridge |
| `log` | No | Dispatch logging macros to the `log` crate |
| `defmt` | No | Dispatch logging macros to the `defmt` crate |
| `headers` | No | Generate C headers via `safer-ffi` |

The `log` and `defmt` features are mutually exclusive — enabling both will produce a compile error.

## C FFI

C interoperability is provided via [`safer-ffi`](https://crates.io/crates/safer-ffi):

| Function | Description |
|----------|-------------|
| `new_dummy_data` | Allocate a byte buffer (optionally COBS-encoded) for testing |
| `drop_bin` | Free a `SerializedData` buffer previously allocated by Rust |

`SerializedData` is the shared FFI-safe struct used by both `mars-common` and [`mars-bluetooth-hci`](../mars-bluetooth-hci) to hand off serialized byte buffers across the C boundary.

## `no_std` Support

Disable default features and enable the required `libc*` features for your target:

```toml
[dependencies]
mars-common = { version = "0.1", default-features = false, features = ["libc", "libc-alloc", "libc-panic"] }
```

## License

Licensed under the [MIT License](../LICENSE).