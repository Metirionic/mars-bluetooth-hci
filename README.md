# Mars Bluetooth HCI

Bluetooth HCI event parsing library for [Bluetooth Channel Sounding](https://www.bluetooth.com/blog/bluetooth-channel-sounding/) (BLE CS).

## Crates

| Crate | Version | Description |
|-------|---------|-------------|
| [`mars-bluetooth-hci`](https://crates.io/crates/mars-bluetooth-hci) | [![crates.io](https://img.shields.io/crates/v/mars-bluetooth-hci.svg)](https://crates.io/crates/mars-bluetooth-hci) | HCI event parsing for BLE CS subevents |
| [`mars-common`](https://crates.io/crates/mars-common) | [![crates.io](https://img.shields.io/crates/v/mars-common.svg)](https://crates.io/crates/mars-common) | Shared FFI infrastructure and logging dispatch |

## Overview

`mars-bluetooth-hci` parses HCI event packets containing LE Channel Sounding measurement data from a Bluetooth controller. It supports:

- **HCI_LE_CS_Config_Complete** (`0x31`) — CS procedure configuration metadata
- **HCI_LE_CS_Subevent_Result_Continue** (`0x32`) — Continuation with measurement data

The parsed data includes Mode2 step data with phase correction terms (I/Q), tone quality indicators, extension slots, and antenna permutation tables per the Bluetooth CS specification (Vol 6, Part H).

## Feature Flags

### `mars-bluetooth-hci`

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enable standard library support |
| `alloc` | Yes | Enable heap allocation (via `postcard`) |
| `libc` | Yes | Enable C FFI serialization functions |
| `libc-alloc` | No | Bridge Rust allocator to C `malloc`/`free` |
| `libc-panic` | No | Bridge Rust panic handler to C callback |
| `headers` | No | Generate C headers via `safer-ffi` |

### `mars-common`

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enable standard library, `BinaryData`, TOML config helpers |
| `alloc` | No | Enable heap allocation |
| `libc` | Yes | Enable FFI serialization (`SerializedData`, `drop_bin`, `new_dummy_data`) |
| `libc-alloc` | No | C allocator bridge |
| `libc-panic` | No | C panic handler bridge |
| `log` | No | Dispatch logging macros to the `log` crate |
| `defmt` | No | Dispatch logging macros to the `defmt` crate |
| `headers` | No | Generate C headers via `safer-ffi` |

## C FFI

Both crates provide C interoperability via [`safer-ffi`](https://crates.io/crates/safer-ffi):

- `mars-bluetooth-hci` exports `serialize_subevent_result_event` and `serialize_log_message`
- `mars-common` exports `SerializedData`, `drop_bin`, and `new_dummy_data`

Pre-generated C headers are included at `mars-bluetooth-hci/mars_bluetooth_hci.h`. To regenerate them:

```bash
./mars-bluetooth-hci/generate_headers.sh <output_path>
```

A CMake config is provided at `mars-bluetooth-hci/mars-bluetooth-hci-rust-config.cmake` for embedding into C projects (including cross-compilation for ARM Cortex-M targets).

## `no_std` Support

Both crates support `no_std` environments. Disable default features and enable the required `libc*` features for your target:

```toml
[dependencies]
mars-bluetooth-hci = { version = "0.7", default-features = false, features = ["libc", "alloc", "libc-alloc", "libc-panic"] }
mars-common = { version = "0.1", default-features = false, features = ["libc", "libc-alloc", "libc-panic"] }
```

Cross-compile for embedded (e.g. `thumbv6m-none-eabi`) with `panic = "abort"` in your Cargo profile.

## Usage

```rust
use mars_bluetooth_hci::event::hci_le_cs::subevent_result::SubeventResultEvent;

let event = SubeventResultEvent::try_from(hci_bytes)?;
println!("Connection handle: {}", event.connection_handle);
println!("Step count: {}", event.step_count);
for step in &event.steps[..event.step_count] {
    println!("  Mode {} on channel {}", step.mode, step.channel);
}
```

## License

Licensed under the [MIT License](LICENSE).