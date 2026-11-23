# Wire-format specification

For developers implementing or maintaining a decoder. This is the byte-level
contract for CS event and log serialization, including **version 3** of the CS
payload. The same raw bytes are used by the persisted-frame codec.

See [architecture.md](architecture.md) for library boundaries and
[ADR-0001](adr/0001-wire-format-postcard-cobs.md) for the encoding decision.

## The envelope

Each frame contains a postcard enum tag followed by its payload:

```text
[variant tag] [payload]
```

| Tag | Variant | Payload |
|---|---|---|
| `0x00` | `SubeventResultEvent` | `u16` version, then the event fields below |
| `0x01` | `LogMessage` | varint byte length, then UTF-8 bytes |

The tags are serde declaration-order indices encoded as `u32` varints.
`SerializableRef` (used by the C FFI) and the owning Rust `Serializable` enum
use identical bytes. There is no magic byte or outer length prefix.

### CS payload version

The CS version is **3** (`SUBEVENT_RESULT_FORMAT_VERSION`), so a raw CS frame
starts with `00 03`. Log messages have no version field.

## Encoding: postcard + COBS

[Postcard](https://docs.rs/postcard/1.1.3/) defines primitive encoding. The rules
used by these payloads are:

| Type | Encoding |
|---|---|
| `u8` / `i8` | One byte; signed values use two's complement. |
| `bool` | `0x00` or `0x01`. |
| Unsigned integers wider than 8 bits, including `usize` | Unsigned LEB128 varint. |
| Signed integers wider than 8 bits | Zig-zag, then varint. |
| `f32` / `f64` | Little-endian IEEE-754. |
| String | Varint byte length, then UTF-8 bytes. |
| Sequence | Varint element count, then elements. |
| Fixed array or tuple | Elements without a length prefix or end marker. |
| Derived struct | Fields in declaration order, without names, padding, or a header. |

`local_mac` and `peer_mac` are `u64` varints. They are not fixed six-byte MAC
addresses; zero takes one byte and a value with bit 47 set takes seven.

## Framing variants

| Serializer option | Output | Use |
|---|---|---|
| `use_cobs=true` | COBS-encoded envelope followed by `0x00` | UART stream |
| `use_cobs=false` | Raw postcard envelope | Storage or another transport with known frame boundaries |

COBS removes zero bytes from the encoded body, making the appended `0x00` an
unambiguous delimiter. Each serialize call produces one frame; firmware forms
the stream by transmitting successive frames. Raw frames have no delimiter and
require an externally known length.

The `new_dummy_data` helper is separate: its optional COBS encoding does not
append a delimiter.

## Stream model and decode contract

For each UART frame:

1. Collect bytes through a `0x00` delimiter. Ignore empty frames and retain a
   partial frame until its delimiter arrives.
2. COBS-decode the body and read the envelope tag.
3. For a CS event, require version 3, then decode the fields below. Reject more
   than 160 steps or a sequence length that differs from `step_count`.
4. Require `antenna_path_count <= 4` and no decoded payload for reserved step modes. For a log message, decode the length-prefixed UTF-8 string.

`postcard::from_bytes_cobs` combines COBS decoding and deserialization in Rust.
For CS events, call `SubeventResultEvent::validate_invariants()` afterwards:
serde checks version, step counts, and reserved-mode payloads but does not
check antenna count. The persisted-frame codec performs these checks and also
rejects trailing bytes and non-CS envelopes.

## Payloads

### SubeventResultEvent (tag 0x00)

The CS payload has 16 fields in wire order, independent of the C struct layout:

| # | Field | Type | Encoding note |
|---|---|---|---|
| 1 | `version` | `u16` | varint; must be 3 |
| 2 | `origin` | `Origin` (enum) | varint tag: Unknown = 0, Initiator = 1, Reflector = 2 |
| 3 | `local_mac` | `u64` | varint; **not** a fixed 6-byte MAC |
| 4 | `peer_mac` | `u64` | varint; **not** a fixed 6-byte MAC |
| 5 | `connection_handle` | `u16` | varint |
| 6 | `config_id` | `u8` | 1 byte |
| 7 | `has_config_id` | `bool` | 1 byte |
| 8 | `procedure_done_status` | `DoneStatus` (enum) | varint tag, declaration order |
| 9 | `procedure_abort_reason` | `ProcedureAbortReason` (enum) | varint tag, declaration order |
| 10 | `subevent_done_status` | `DoneStatus` (enum) | varint tag, declaration order |
| 11 | `subevent_abort_reason` | `SubeventAbortReason` (enum) | varint tag, declaration order |
| 12 | `antenna_path_count` | `usize` | varint; at most 4 |
| 13 | `step_count` | `usize` | varint; at most 160 |
| 14 | `steps` | sequence of `Step` | varint length, then exactly that many steps; length must equal `step_count` |
| 15 | `initial_meta` | `InitialMeta` (struct) | fields concatenated in declaration order |
| 16 | `has_initial_meta` | `bool` | 1 byte |

The step count appears twice: once as `step_count` and once as the sequence length.
Only `steps[..step_count]` is serialized. Unreported array slots are omitted and
restored to `Step::default()` on Rust deserialization. A zero-step event still has
both zero counts and the initial-metadata fields.

Each `Step` is encoded in this order:

1. `mode`: a declaration-order enum tag (0–3 for Modes 0–3, 4 for Reserved).
2. `channel`: one `u8`.
3. `has_data`: one boolean byte. If false, no payload follows.
4. If `has_data` is true, Mode 0 carries `mode0`, Mode 1 carries `mode1`,
   Mode 2 carries `mode2`, and Mode 3 carries `mode1` followed by `mode2`.
   Reserved modes cannot carry a payload.

Unselected `mode0`, `mode1`, and `mode2` fields are omitted and restored to defaults
on Rust deserialization. Within a selected payload, fields retain their derived
serde encoding:

- `Mode0Data`: `packet_quality`, `packet_received_signal_strength_indicator`,
  `packet_antenna`, `measured_freq_offset`. Both roles serialize the offset field,
  including the reflector's default zero.
- `Mode1Data`: `packet`, `timing`, `packet_phase_correction_terms`,
  `has_packet_phase_correction_terms`. The phase-correction fields are serialized
  even when their validity flag is false.
- `Mode2Data`: `antenna_permutation_index`, all five `phase_correction_terms`, all five
  `quality_indicators`, then all five `extension_slots`. Each array has no length prefix and is not
  shortened to `antenna_path_count + 1`.
- `initial_meta` is serialized even when `has_initial_meta` is false; `config_id`
  is serialized even when `has_config_id` is false. Validity flags control
  interpretation, not field presence.

For nested field order and types within these derived payloads, see
[`subevent_result.rs`](../mars-bluetooth-hci/src/event/hci_le_cs/subevent_result.rs),
[`event/mod.rs`](../mars-bluetooth-hci/src/event/mod.rs), and the generated
[C header](../mars-bluetooth-hci/mars_bluetooth_hci.h).
The header describes the C ABI: it includes all 160 step slots and all mode fields,
and has no CS payload version field. It does not describe the compact wire shape.
C padding and fixed integer widths do not apply on the wire; enum wire tags are
serde declaration-order indices rather than explicit C discriminants.

### LogMessage (tag 0x01)

After the tag, read a varint byte length and that many UTF-8 bytes. There is no
struct header or version. The Rust variant's borrowing attribute affects memory
ownership only.

## Decoder notes (non-obvious encoding facts)

### Enum tags are declaration order, not C-repr discriminants

Nested enum tags are postcard varints of their serde declaration-order indices.
Explicit `#[repr(u8)]` values in the C header do not determine wire tags. For example:

| `DoneStatus` | C value | Wire tag |
|---|---|---|
| `AllComplete` | `0x00` | `0x00` |
| `Partial` | `0x01` | `0x01` |
| `Aborted` | `0x0F` | `0x02` |
| `Reserved` | `0xFF` | `0x03` |

`StepMode::Reserved` encodes as 4 despite its `0xFF` C/HCI discriminant.
Unknown serialized tags are rejected.

## Versioning and compatibility

Version 3 replaces the payload-kind tag with `Step.has_data` and mode-selected
payloads. The C API/layout changes accordingly. Firmware, generated headers,
and decoders must use compatible releases. Envelope tags, log encoding, and
COBS framing are unchanged.

Persisted frames require an out-of-band `FrameDescriptor` with format name
`mars-hci-cs-subevent` and version **3**, stored alongside the raw non-COBS bytes.
Decode checks the descriptor before the in-band CS version. The current codec
rejects older versions and provides no converter; existing recordings require
the old implementation to decode before re-encoding with version 3.

[Current fixtures](../mars-bluetooth-hci/tests/fixtures/persisted-frames/v3/) pin
encoder bytes for Modes 0–3 and a config-complete frame.
Future incompatible changes require a new version and an explicit migration
transition ([ADR-0003](adr/0003-versioned-persisted-frame-codec.md)).

The separate timestamped recording container `mars-common::serde::BinaryData`
has its own `BINARY_FORMAT_VERSION`, currently 2. Its checked `deserialize`
method rejects v1 containers. This container version is not part of an
individual UART or persisted frame.

## Related documents

- [Architecture](architecture.md): API boundaries and data flow.
- [C integration](c-embedded-integration.md): building and calling the serializer.
- [Persistence decision](adr/0003-versioned-persisted-frame-codec.md): version selection and migration policy.
- [C header](../mars-bluetooth-hci/mars_bluetooth_hci.h): C types and declarations.
- [Postcard reference](https://docs.rs/postcard/1.1.3/): primitive encoding.
