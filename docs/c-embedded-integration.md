# C/embedded integration guide

For firmware engineers linking `libmars_bluetooth_hci.a` into a C/CMake project.
This guide covers the embedded build, required C callbacks, and buffer ownership.
See [architecture.md](architecture.md) for API boundaries and
[wire-format.md](wire-format.md) for the transmitted bytes.

## Prerequisites and target assumptions

You need Rust with your target installed, a C/CMake build, and a working heap.
The Rust target must match the firmware's architecture and ABI, including its
floating-point calling convention. Keep both workspace crates in the source
checkout: `mars-bluetooth-hci` depends on the sibling `mars-common` directory.
This example uses `thumbv6m-none-eabi`; choose the target for your hardware:

```bash
rustup target add thumbv6m-none-eabi
export CARGO_PROFILE_RELEASE_PANIC=abort
```

Run CMake and its build from an environment containing that variable. The
library requires abort-on-panic for embedded builds; this repository's release
profile does not set it.

## Building and linking the static library

Include the provided
[CMake module](../mars-bluetooth-hci/mars-bluetooth-hci-rust-config.cmake)
from your firmware project after obtaining the library sources:

```cmake
set(RUST_TARGET "thumbv6m-none-eabi")
set(HOST OFF)

include(${PATH_TO_mars-bluetooth-hci}/mars-bluetooth-hci/mars-bluetooth-hci-rust-config.cmake)

target_include_directories(${FW_TARGET} PRIVATE ${MARS_BT_HCI_RUST_INCLUDE_DIRECTORIES})
target_link_libraries(${FW_TARGET} PRIVATE mars_bt_hci_rust)
add_dependencies(${FW_TARGET} build_mars_bt_hci_rust)
```

Replace the source path and firmware target with your project's values. The module
builds the release library under `${CMAKE_BINARY_DIR}/cargo-target` and exposes it
as the imported target `mars_bt_hci_rust`. `HOST ON` uses default host features;
`HOST OFF` uses the embedded features below. If set, `MARS_BT_HCI_RUST_CONFIG_PATH`
is forwarded to Cargo as an environment variable.

### The embedded feature set

```text
--no-default-features --features libc,alloc,libc-panic,libc-alloc
```

| Feature | Purpose |
|---|---|
| `libc` | Enable the C FFI. |
| `alloc` | Enable allocated postcard output. |
| `libc-alloc` | Use the C `malloc`/`free` allocator bridge. |
| `libc-panic` | Use the C panic callback. |

The build uses `no_std`. See [Contributing](../CONTRIBUTING.md#3-no_std--embedded-cross-compile-testing)
for manual cross-compile checks.

## Embedded constraints you must provide

### Global allocator: `malloc` / `free`

Make `malloc(size_t)` and `free(void *)` available to the linker. An existing C
runtime allocator is sufficient; otherwise provide wrappers around your heap.
The Rust allocator bridge passes the requested size to `malloc` and relies on
it for alignment; it does not handle over-aligned allocations separately.

### Panic handler: `rust_panic_cb`

Provide a callback taking a NUL-terminated message:

```c
void rust_panic_cb(const char *msg)
{
    /* Log the message, then halt or reset according to firmware policy. */
    (void)msg;
    for (;;) { }
}
```

Serialization failures panic rather than returning an error code. The Rust panic
handler calls this callback and loops if it returns.

## The FFI call pattern

1. Construct `SubeventResultEvent_t` and fill its identity, status, count, and step
   fields. Set each step's `mode` and `has_data`, and populate the selected
   fields in `data` when a payload is present.
2. Call `serialize_subevent_result_event(&event, true)` for a UART frame.
3. Copy or transmit the returned `p_data[0..size]` bytes.
4. Call `drop_bin` exactly once after the bytes are no longer needed.

Before serialization, ensure `step_count <= 160`, `antenna_path_count <= 4`, and
every reported step with `has_data = true` has its mode-selected payload populated:
Mode 0 uses `data.mode0`, Mode 1 uses `data.mode1`, Mode 2 uses `data.mode2`, and
Mode 3 uses both `data.mode1` and `data.mode2`. Set `has_data = false` for a step
without a payload; its mode and channel are retained, but no payload is serialized.
Reserved modes must have `has_data = false`. Set validity flags such as
`has_config_id`, `has_initial_meta`, and `has_packet_phase_correction_terms` when
their fields contain valid data.
Zero-initialization alone does not produce a complete measurement. The FFI
serializer does not validate all these invariants or return validation errors.

Use a decoder compatible with the library release: the current CS wire payload
is v3 and is incompatible with v1 and v2. See
[wire compatibility](wire-format.md#versioning-and-compatibility).

### SerializedData ownership

The returned `SerializedData_t` contains `p_data`, `size`, and `capacity`. Preserve
these values until passing the buffer to `drop_bin`. Do not call C `free` on the
pointer or access it after `drop_bin`.

For asynchronous UART or DMA, either copy to a separate TX buffer before release
or retain the returned buffer until transmission completes. Use the returned
`size`; the C struct's size does not determine the serialized length.

## Minimal C example

This example serializes one synthetic Mode 2 step. Replace the values with actual
controller measurements. The event uses static storage to avoid a large stack
allocation; calls must be serialized because the example shares its buffers.

```c
#include "mars_bluetooth_hci.h"
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

/* Firmware implementation: consumes or copies all bytes before returning. */
extern void transmit_uart(const uint8_t *buf, size_t len);

/* Example capacity; send_subevent checks the actual serialized size. */
static uint8_t tx_buf[16 * 1024];
static SubeventResultEvent_t event;

static int send_subevent(const SubeventResultEvent_t *value)
{
    SerializedData_t out = serialize_subevent_result_event(value, true);
    if (out.size > sizeof(tx_buf)) {
        drop_bin(out);
        return -1;
    }

    size_t len = out.size;
    memcpy(tx_buf, out.p_data, len);
    drop_bin(out);
    transmit_uart(tx_buf, len);
    return 0;
}

int build_and_send_local_event(uint64_t local_mac, uint64_t peer_mac,
                               uint16_t conn_handle)
{
    memset(&event, 0, sizeof(event));
    event.origin = ORIGIN_INITIATOR;
    event.local_mac = local_mac;
    event.peer_mac = peer_mac;
    event.connection_handle = conn_handle;
    event.procedure_done_status = DONE_STATUS_ALL_COMPLETE;
    event.subevent_done_status = DONE_STATUS_ALL_COMPLETE;
    event.antenna_path_count = 1;
    event.step_count = 1;

    Step_t *step = &event.steps.idx[0];
    step->mode = STEP_MODE_MODE2;
    step->channel = 19;
    step->has_data = true;
    step->data.mode2.phase_correction_terms.idx[0].i = 1.0f;
    step->data.mode2.quality_indicators.idx[0] = TONE_QUALITY_INDICATOR_HIGH;
    return send_subevent(&event);
}
```

Choose TX capacity for your expected events and keep the overflow check. For
direct DMA, retain `out` and call `drop_bin(out)` from the completion path.

## Canonical consumer reference

The [nRF54L reference firmware](https://github.com/Metirionic/mars-cs-nrf54l) shows
measurement acquisition and serialization in a complete application:

- [Serializer](https://github.com/Metirionic/mars-cs-nrf54l/blob/main/initiator/src/serialize.c)
- [Build configuration and library release pin](https://github.com/Metirionic/mars-cs-nrf54l/blob/main/initiator/CMakeLists.txt)
- [Firmware architecture](https://github.com/Metirionic/mars-cs-nrf54l/blob/main/docs/architecture.md)

## When the header must be regenerated

Use the committed [C header](../mars-bluetooth-hci/mars_bluetooth_hci.h) from the
same library release. It is generated and must not be edited manually. After
changing public FFI types, regenerate and commit it:

```bash
./mars-bluetooth-hci/generate_headers.sh mars-bluetooth-hci/mars_bluetooth_hci.h
```

CI checks it against regenerated output. See
[Contributing](../CONTRIBUTING.md#1-the-generated-c-header-ci-header-regeneration-contract)
for the full workflow.
