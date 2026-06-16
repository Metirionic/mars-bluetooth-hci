# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

- - -
## [mars-bluetooth-hci@0.8.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/0cd79b49b3775c7ad81eaf39c0f682914c1bd23e..mars-bluetooth-hci@0.8.0) - 2026-06-15

- - -

## [mars-bluetooth-hci@0.7.1](https://github.com/Metirionic/mars-bluetooth-hci/compare/120ce8f5b75875cd83ae6006bb8d3ed60aa4d686..mars-bluetooth-hci@0.7.1) - 2026-06-10
#### Bug Fixes
- add readme to crates - ([e8c40a1](https://github.com/Metirionic/mars-bluetooth-hci/commit/e8c40a12d21e16a5376f4de65f938320b9c0d9e1)) - Adrian Figueroa

- - -

## [mars-bluetooth-hci@0.7.0] - 2026-06-09
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