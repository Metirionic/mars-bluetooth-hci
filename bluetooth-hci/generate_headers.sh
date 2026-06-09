#!/bin/bash
set -eo pipefail

HERE=$(dirname "$0")

# Pass the target path to the generate_headers script.
cargo run --manifest-path $HERE/Cargo.toml --features headers --target-dir $HERE/target-headers --bin generate_bluetooth_hci_headers -- $1