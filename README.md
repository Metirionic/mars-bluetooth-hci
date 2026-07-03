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

## License

Licensed under the [MIT License](LICENSE).