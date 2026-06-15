# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

- - -
## [mars-common@0.2.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/8c7a1fc88ef3b6ef37b8d9cddd01b4825f528c23..mars-common@0.2.0) - 2026-06-15
#### Features
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) add version field to BinaryData file format - ([8c7a1fc](https://github.com/Metirionic/mars-bluetooth-hci/commit/8c7a1fc88ef3b6ef37b8d9cddd01b4825f528c23)) - Adrian Figueroa
#### Bug Fixes
- add missing docs - ([e1602ee](https://github.com/Metirionic/mars-bluetooth-hci/commit/e1602eed4a576b604331e0990490b785da5e0de8)) - Adrian Figueroa

- - -

## [mars-common@0.1.2](https://github.com/Metirionic/mars-bluetooth-hci/compare/e8c40a12d21e16a5376f4de65f938320b9c0d9e1..mars-common@0.1.2) - 2026-06-10
#### Bug Fixes
- add readme to crates - ([e8c40a1](https://github.com/Metirionic/mars-bluetooth-hci/commit/e8c40a12d21e16a5376f4de65f938320b9c0d9e1)) - Adrian Figueroa

- - -

## [mars-common@0.1.0] - 2026-06-09
#### Features
- Initial open-source release
- FFI serialization infrastructure (`SerializedData`, `drop_bin`, `new_dummy_data`)
- C allocator bridge (`libc-alloc` feature)
- C panic handler bridge (`libc-panic` feature)
- Logging dispatch macros (`log` / `defmt` dual-backend, no-op fallback)
- `BinaryData` struct with postcard serialization (`std` feature)
- TOML config helpers (`store_config` / `load_config`, `std` feature)