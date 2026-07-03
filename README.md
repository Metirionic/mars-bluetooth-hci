# MARS Bluetooth HCI

The open encoder, parser, and C-FFI bridge for the Metirionic Advanced Ranging Stack (MARS) — this repository defines the authoritative Channel Sounding wire format consumed by MARS firmware and the closed-source evaluation GUI.

## Where it fits

The Metirionic Channel Sounding product spans three repositories. [`mars-bluetooth-hci`](https://github.com/Metirionic/mars-bluetooth-hci) (this repo) and [`mars-cs-nrf54l`](https://github.com/Metirionic/mars-cs-nrf54l) (the nRF54L firmware) are open source under MIT; [`mars-ranging-demo`](https://github.com/Metirionic/mars-ranging-demo) is a public repo whose GUI decoder is closed-source. The Metirionic Advanced Ranging Stack (MARS) itself is a separately licensed product, not governed by these repositories. This library parses and serializes Channel Sounding measurement data; it does **not** compute ranging or distance.

<!-- The canonical, fully-annotated data flow lives in docs/ecosystem.md — keep this trimmed diagram in sync with that document. -->

```mermaid
flowchart LR
    FW["<b>mars-cs-nrf54l</b><br/>firmware (open)"]
    LIB["<b>mars-bluetooth-hci</b><br/>this repo (open)"]
    APP["<b>mars-ranging-demo</b><br/>eval GUI (closed)"]
    FW -->|"serialize call"| LIB
    FW -->|"COBS over UART"| APP
    classDef open fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px,color:#1b5e20
    classDef closed fill:#fce4ec,stroke:#c62828,stroke-width:2px,stroke-dasharray:6 4,color:#b71c1c
    class FW,LIB open
    class APP closed
```

For the full, annotated data flow (build-time `FetchContent` mechanics, the serialize call/return, and the UART transport), see [`docs/ecosystem.md`](docs/ecosystem.md).

## What's inside

![no_std](https://img.shields.io/badge/no__std-supported-orange) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

| Crate | Badges | Description |
|-------|--------|-------------|
| [`mars-bluetooth-hci`](mars-bluetooth-hci/README.md) | [![crates.io](https://img.shields.io/crates/v/mars-bluetooth-hci.svg)](https://crates.io/crates/mars-bluetooth-hci) [![docs.rs](https://docs.rs/mars-bluetooth-hci/badge.svg)](https://docs.rs/mars-bluetooth-hci) | Parses HCI LE CS subevent-result events (`0x31` config, `0x32` subevent-result) and serializes them over a C FFI. |
| [`mars-common`](mars-common/README.md) | [![crates.io](https://img.shields.io/crates/v/mars-common.svg)](https://crates.io/crates/mars-common) [![docs.rs](https://docs.rs/mars-common/badge.svg)](https://docs.rs/mars-common) | Shared FFI-safe `SerializedData` buffer, `drop_bin`, C allocator/panic bridges, and `log`/`defmt` logging dispatch. |

- `no_std` + embedded — bare-metal Cortex-M (e.g. `thumbv6m-none-eabi`), `panic = "abort"` compatible.
- Serialize-only FFI — serialization and memory management only cross the C boundary (no `parse_*`/`decode_*`/`deserialize_*`); the HCI parser is a Rust-API concern → `docs/adr/0002-serialize-only-ffi.md`.
- `postcard` + COBS wire format — self-framing, trailing-`0x00`-delimited; this repo is the authoritative spec → `docs/wire-format.md`, `docs/adr/0001-wire-format-postcard-cobs.md`.
- CMake / `FetchContent` — pre-generated C header + `mars-bluetooth-hci-rust-config.cmake`, including cross-compilation.

## License

Licensed under the [MIT License](LICENSE).