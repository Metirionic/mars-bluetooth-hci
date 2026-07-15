# mars-bluetooth-hci

[![crates.io](https://img.shields.io/crates/v/mars-bluetooth-hci.svg)](https://crates.io/crates/mars-bluetooth-hci)
[![docs.rs](https://docs.rs/mars-bluetooth-hci/badge.svg)](https://docs.rs/mars-bluetooth-hci)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![no_std](https://img.shields.io/badge/no__std-supported-orange)

Bluetooth HCI event parsing library for [Bluetooth Channel Sounding](https://www.bluetooth.com/blog/bluetooth-channel-sounding/) (BLE CS).

Part of the [mars-bluetooth-hci](https://github.com/Metirionic/mars-bluetooth-hci) workspace. See also: [`mars-common`](../mars-common).

## Overview

`mars-bluetooth-hci` parses HCI event packets containing LE Channel Sounding measurement data from a Bluetooth controller. It supports:

- **HCI_LE_CS_Config_Complete** (`0x31`) — CS procedure configuration metadata
- **HCI_LE_CS_Subevent_Result_Continue** (`0x32`) — Continuation with measurement data

The parsed data includes Mode2 step data with phase correction terms (I/Q), tone quality indicators, extension slots, and antenna permutation tables per the Bluetooth CS specification (Vol 6, Part H).

### Key types

| Type | Description |
|------|-------------|
| [`SubeventResultEvent`] | Top-level parsed event with connection handle, steps, and metadata |
| [`Mode2`] | Mode2 step data: antenna permutation, phase correction, quality indicators |
| [`PhaseCorrectionTerm`] | I/Q components of a phase correction term |
| [`ToneQualityIndicator`] | High / Medium / Low / Unavailable quality rating |
| [`InitialMeta`] | Metadata from the first subevent (procedure counter, frequency compensation, reference power) |
| [`Origin`] | Initiator / Reflector / Unknown data origin |

[`SubeventResultEvent`]: src/event/hci_le_cs/subevent_result.rs
[`Mode2`]: src/event/hci_le_cs/subevent_result.rs
[`PhaseCorrectionTerm`]: src/event/hci_le_cs/subevent_result.rs
[`ToneQualityIndicator`]: src/event/mod.rs
[`InitialMeta`]: src/event/hci_le_cs/subevent_result.rs
[`Origin`]: src/event/hci_le_cs/subevent_result.rs

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | Yes | Enable standard library support |
| `alloc` | Yes | Enable heap allocation (via `postcard`) |
| `libc` | Yes | Enable C FFI serialization functions |
| `libc-alloc` | No | Bridge Rust allocator to C `malloc`/`free` |
| `libc-panic` | No | Bridge Rust panic handler to C callback |
| `headers` | No | Generate C headers via `safer-ffi` |

## C FFI

C interoperability is provided via [`safer-ffi`](https://crates.io/crates/safer-ffi):

- `serialize_subevent_result_event` — serialize a [`SubeventResultEvent`] to a byte buffer
- `serialize_log_message` — serialize a log string to a byte buffer

Both functions return [`SerializedData`](../mars-common) (pointer + size + capacity) and support optional COBS encoding.

Pre-generated C headers are included at [`mars_bluetooth_hci.h`](mars_bluetooth_hci.h). To regenerate them:

```bash
./generate_headers.sh <output_path>
```

A CMake config is provided at [`mars-bluetooth-hci-rust-config.cmake`](mars-bluetooth-hci-rust-config.cmake) for embedding into C projects, including cross-compilation for ARM Cortex-M targets.

For a full build/link + FFI call-pattern walkthrough, see [docs/c-embedded-integration.md](../docs/c-embedded-integration.md).

## `no_std` Support

Disable default features and enable the required `libc*` features for your target:

```toml
[dependencies]
mars-bluetooth-hci = { version = "0.9", default-features = false, features = ["libc", "alloc", "libc-alloc", "libc-panic"] }
```

Cross-compile for embedded (e.g. `thumbv6m-none-eabi`) with `panic = "abort"` in your Cargo profile.

## License

Licensed under the [MIT License](../LICENSE).
