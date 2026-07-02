# Mars Bluetooth HCI

The ubiquitous language for the Mars Bluetooth HCI library — the Channel Sounding domain terms and the library's encoder/decoder roles an agent needs to orient quickly. It defines vocabulary only and links to [docs/architecture.md](docs/architecture.md) and [docs/adr/](docs/adr) for the HOW and WHY.

## Language

### Channel Sounding domain

**MARS**:
The Metirionic Advanced Ranging Stack — the separately licensed ranging stack that this open library encodes, parses, and bridges to C FFI for (see [docs/ecosystem.md](docs/ecosystem.md)).
_Avoid_: Metirionic stack, the ranging stack, MARS stack

**Channel Sounding (CS)**:
The Bluetooth ranging technology this library targets, whose measurements are organized as procedures, subevents, and steps (see the [Bluetooth SIG overview](https://www.bluetooth.com/channel-sounding-tech-overview/) and [docs/ecosystem.md](docs/ecosystem.md)).
_Avoid_: ranging, distance measurement, CS measurement

**HCI event**:
A Bluetooth Host Controller Interface event — the input unit this library parses, specifically the LE CS subevent-result event (see [docs/architecture.md](docs/architecture.md)).
_Avoid_: HCI packet, BLE event, HCI message

**Procedure**:
The coarsest Channel Sounding measurement unit, comprising one or more subevents.
_Avoid_: measurement run, CS run, ranging procedure

**Subevent**:
A subdivision of a Channel Sounding procedure, comprising one or more steps; the unit whose result this library parses and serializes (see [docs/architecture.md](docs/architecture.md), [docs/wire-format.md](docs/wire-format.md)).
_Avoid_: sub-event, result event, sub-result

**Step**:
The atomic unit within a Channel Sounding subevent, carrying mode-specific data such as a Mode 2 step (see [docs/architecture.md](docs/architecture.md)).
_Avoid_: sample, tone step, measurement step

**Mode 2**:
The Channel Sounding step mode that carries per-antenna-path phase correction terms and quality indicators (see [docs/architecture.md](docs/architecture.md)).
_Avoid_: mode-2, CS mode 2, M2

**Phase Correction Term (PCT)**:
A per-step complex (I/Q) value carried in a Mode 2 step.
_Avoid_: phase term, correction term, I/Q term

### Library boundary

**Encoder**:
The open-source serialize side of the MARS data path; this repository (see [docs/ecosystem.md](docs/ecosystem.md); the wire-format decision is [ADR-0001](docs/adr/0001-wire-format-postcard-cobs.md), the FFI boundary decision is [ADR-0002](docs/adr/0002-serialize-only-ffi.md)).
_Avoid_: serializer, encode side, encoder library

**Decoder**:
The closed-source deserialize side of the MARS data path — the `mars-ranging-demo` GUI — which consumes the wire format this repository defines (see [docs/ecosystem.md](docs/ecosystem.md), [docs/wire-format.md](docs/wire-format.md)).
_Avoid_: deserializer, decode side, eval app, GUI

## Glossary gaps

The C/embedded integration guide uses a few integration-mechanics terms that are
not domain vocabulary and so are not defined above: **FFI**, **COBS**, `no_std`,
**staticlib**, and **CMake / `FetchContent`**. They are introduced in-context in
[docs/c-embedded-integration.md](docs/c-embedded-integration.md), which is the
authoritative integrator walkthrough.