# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

- - -
## [bluetooth-hci@0.7.0] - 2026-06-09
#### Features
- Initial open-source release
- Parse HCI_LE_CS_Config_Complete and HCI_LE_CS_Subevent_Result_Continue events
- Mode2 step data with phase correction terms, quality indicators, extension slots
- Antenna permutation lookup tables per Bluetooth CS spec
- C FFI via safer-ffi (serialize subevent results and log messages)
- C static library output (staticlib crate type)
- Automatic C header generation (`headers` feature)
- CMake integration config for embedded builds
- `no_std` support