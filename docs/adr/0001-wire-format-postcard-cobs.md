---
status: accepted
---

# Wire format: postcard + COBS over UART

UART traffic from firmware to host carries `SerializableRef`/`Serializable` envelopes encoded with postcard's `to_allocvec_cobs`, which fuses serde serialization, COBS byte-stuffing, and a trailing `0x00` sentinel in a single call so the byte stream is self-framing with no out-of-band framing. The `use_cobs=false` path uses plain `to_allocvec` (no COBS, no `0x00`) as a deliberate unframed variant for non-streaming/recording use. This repo is encode-only; the decode side (split on `0x00`, COBS-decode, postcard-deserialize) lives in the sibling `mars-ranging-demo` repo — see [`docs/ecosystem.md`](../ecosystem.md) for the data flow and the forthcoming wire-format specification (#12) for byte-level detail.