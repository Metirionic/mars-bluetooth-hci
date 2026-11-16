---
status: accepted
---

# Versioned persisted-frame codec, separate from the UART wire path

CS subevent results need a storage-replay contract with explicit format selection
and strict validation. The Rust-only
[`persisted_frame`](../../mars-bluetooth-hci/src/event/hci_le_cs/persisted_frame.rs)
codec provides `encode` and `decode` in host builds (`std`+`alloc`+`libc`).

Persisted frames use the FFI serializer's non-COBS bytes. Callers store a
`FrameDescriptor` alongside each frame so replay can select a supported format
without guessing from the bytes. Decode rejects unsupported descriptors,
trailing bytes, truncated frames, and non-CS envelopes. Encode and decode both
validate event invariants.

Incompatible changes require a new declared version and an explicit migration
transition. During migration, retain the preceding decoder and its fixtures;
remove both once migration completes. Golden fixtures pin the current encoder
bytes and verify any retained migration decoder.

## Version 2

V2 uses a compact, versioned CS representation while preserving the C ABI.
The current codec supports only v2 and provides no v1 converter. Existing v1
recordings require the old implementation to decode before re-encoding with v2.
See [wire-format.md](../wire-format.md) for the layout, descriptors, and fixtures.

## Considered options

- **Use the general FFI enum as the storage contract.** Rejected: storage accepts
  only CS subevent results and needs explicit format selection.
- **Use only an in-band version.** Rejected for V1 to preserve its existing FFI
  bytes. V2 includes a CS payload version but retains the storage descriptor.
- **Compact the original fixed-array representation.** Deferred from V1 to V2
  because it changes the wire format.
