# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

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