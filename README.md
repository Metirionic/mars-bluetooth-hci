# Mars Bluetooth HCI

The open encoder, parser, and C-FFI bridge for the Metirionic Advanced
Ranging Stack (MARS) - this repository defines the authoritative Channel
Sounding wire format consumed by MARS firmware and the closed-source
evaluation GUI.

## Where it fits

The MARS Channel Sounding ecosystem spans three repositories: `mars-cs-nrf54l`
firmware is open, `mars-bluetooth-hci` is this open library, and
`mars-ranging-demo` is a public repository with a closed-source evaluation GUI.
MARS is separately licensed from those repositories. This library parses and
serializes Channel Sounding measurement data and defines the wire format
between firmware and GUI; it does not compute ranging or distance.

<!--
docs/ecosystem.md is the canonical, fully annotated data-flow source; keep this
landing diagram in sync.
-->

```mermaid
flowchart LR
    FW["<b>mars-cs-nrf54l</b><br/>firmware (open)"]
    LIB["<b>mars-bluetooth-hci</b><br/>this repo (open)"]
    APP["<b>mars-ranging-demo</b><br/>eval GUI (closed)"]
    FW -->|"serialize call"| LIB
    LIB -->|"COBS over UART"| APP
    classDef open fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px,color:#1b5e20
    classDef closed fill:#fce4ec,stroke:#c62828,stroke-width:2px,stroke-dasharray:6 4,color:#b71c1c
    class FW,LIB open
    class APP closed
```

For the full annotated ecosystem data flow, see
[docs/ecosystem.md](docs/ecosystem.md).

## License

Licensed under the [MIT License](LICENSE).
