---
status: accepted
---

# Serialize-only C FFI with a Rust-API-only HCI parser

The C FFI surface (generated via `safer-ffi` `#[ffi_export]` into `mars_bluetooth_hci.h`) exposes only serialization and memory-management for the HCI data path — `serialize_subevent_result_event`, `serialize_log_message`, `drop_bin`, `new_dummy_data` — and no `parse_*`/`decode_*`/`deserialize_*` symbol. Firmware constructs the `SubeventResultEvent` struct directly in C and uses this library only to serialize it into the wire format of ADR-0001, so the HCI parser (`impl TryFrom<&[u8]> for SubeventResultEvent`, plus `ParseError`) is a Rust-API-only concern, kept off the C surface to minimize it and retain the parser's memory safety and error handling in Rust. The struct shapes do cross the FFI (`#[derive_ReprC]`/`#[repr(C)]`, present in the header so C can construct them) — only the bytes→struct parsing behavior is Rust-only; see [`docs/ecosystem.md`](../ecosystem.md) for the WHAT and [`docs/architecture.md`](../architecture.md) for the two construction paths.

## Considered Options

- **Expose parsing/deserialization across the FFI.** Rejected: firmware builds the event struct directly in C (Path A) and never needs to parse raw HCI bytes, so exporting decode would enlarge the C surface and push parser error-handling and memory safety out of Rust for no consumer. The Rust-side raw-byte parser (Path B, `hci_file_reader`) remains std/test-gated and Rust-API-only.