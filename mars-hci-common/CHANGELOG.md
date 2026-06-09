# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

- - -
## [mars-hci-common@0.1.0] - 2026-06-09
#### Features
- Initial open-source release as `mars-hci-common` (previously `mars-common`)
- FFI serialization infrastructure (`SerializedData`, `drop_bin`, `new_dummy_data`)
- C allocator bridge (`libc-alloc` feature)
- C panic handler bridge (`libc-panic` feature)
- Logging dispatch macros (`log` / `defmt` dual-backend, no-op fallback)
- `BinaryData` struct with postcard serialization (`std` feature)
- TOML config helpers (`store_config` / `load_config`, `std` feature)