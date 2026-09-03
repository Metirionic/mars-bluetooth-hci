# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

- - -
## [mars-bluetooth-hci@0.14.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/a9c94c52774728188a38c0a7c2e76a5cd6447f18..mars-bluetooth-hci@0.14.0) - 2026-09-03
#### Features
- add versioned CS subevent persistence frames - ([a9c94c5](https://github.com/Metirionic/mars-bluetooth-hci/commit/a9c94c52774728188a38c0a7c2e76a5cd6447f18)) - Johannes Guertler
#### Bug Fixes
- apply critical code-review findings to the persisted-frame codec - ([d568563](https://github.com/Metirionic/mars-bluetooth-hci/commit/d5685632ff1428cab4c53fd39ea316061e5c8568)) - Attila Römer, Claude Code
#### Documentation
- record the persisted-frame codec decision (ADR-0003) - ([1c0190a](https://github.com/Metirionic/mars-bluetooth-hci/commit/1c0190ab048fb937bd3c0fa3c0f5d47ac74b3136)) - Attila Römer, Claude Code
#### Refactoring
- make HCI persistence API part of the default build - ([850e62a](https://github.com/Metirionic/mars-bluetooth-hci/commit/850e62a92e3ea90627fc016bddb1e06dcdd7ea73)) - Johannes Guertler

- - -

## [mars-bluetooth-hci@0.13.2](https://github.com/Metirionic/mars-bluetooth-hci/compare/6912e46378d0fc754ec6c9683d84d7c9695b141d..mars-bluetooth-hci@0.13.2) - 2026-08-16
#### Bug Fixes
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) sign-extend Mode 0 and FrequencyCompensation offsets from bit 14 - ([6912e46](https://github.com/Metirionic/mars-bluetooth-hci/commit/6912e46378d0fc754ec6c9683d84d7c9695b141d)) - atti, Claude

- - -

## [mars-bluetooth-hci@0.13.1](https://github.com/Metirionic/mars-bluetooth-hci/compare/ebcf4134f7e2ebb839a34744d8687ea76edc88f0..mars-bluetooth-hci@0.13.1) - 2026-08-15
#### Bug Fixes
- Mode 0 reflector step data is 3 bytes, not 4 - ([ebcf413](https://github.com/Metirionic/mars-bluetooth-hci/commit/ebcf4134f7e2ebb839a34744d8687ea76edc88f0)) - atti, Claude
#### Documentation
- sync version literals to 0.13.0 - ([8b2bd50](https://github.com/Metirionic/mars-bluetooth-hci/commit/8b2bd50f47961be42ae6d20aa19d92b6b7f73399)) - atti, Claude

- - -

## [mars-bluetooth-hci@0.13.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/430f88b5c4ba82a103124696e7d18437f6b364ab..mars-bluetooth-hci@0.13.0) - 2026-08-14
#### Features
- populate Mode 0 steps from CS subevent results - ([430f88b](https://github.com/Metirionic/mars-bluetooth-hci/commit/430f88b5c4ba82a103124696e7d18437f6b364ab)) - atti, Claude
#### Documentation
- sync stale version literals to 0.12.0 - ([170be19](https://github.com/Metirionic/mars-bluetooth-hci/commit/170be19818f784e40c2839082e012620d8be5042)) - atti, Claude

- - -

## [mars-bluetooth-hci@0.12.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/ea0c23e7e12fa11fdd26127bfb42bad611392583..mars-bluetooth-hci@0.12.0) - 2026-07-21
#### Features
- add RTT conversion to seconds - ([ea0c23e](https://github.com/Metirionic/mars-bluetooth-hci/commit/ea0c23e7e12fa11fdd26127bfb42bad611392583)) - Johannes Guertler
#### Bug Fixes
- README, docs update - ([4d6faba](https://github.com/Metirionic/mars-bluetooth-hci/commit/4d6fabad658fee2cfae68a0361c7dfde92847f67)) - Johannes Guertler
#### Documentation
- updated stale comments and docs - ([5f8ba76](https://github.com/Metirionic/mars-bluetooth-hci/commit/5f8ba76c28ac2d55c0a7d34aec8d0cc05ef25525)) - Johannes Guertler

- - -

## [mars-bluetooth-hci@0.11.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/5382fc402e58a608a9528bd996f1e783f71fd9ec..mars-bluetooth-hci@0.11.0) - 2026-07-16
#### Features
- collapse modes - ([5382fc4](https://github.com/Metirionic/mars-bluetooth-hci/commit/5382fc402e58a608a9528bd996f1e783f71fd9ec)) - Adrian Figueroa
#### Bug Fixes
- readme snippet - ([ab6b03e](https://github.com/Metirionic/mars-bluetooth-hci/commit/ab6b03ef6d2affc57a81f535bb7147c5dc1dfce7)) - Adrian Figueroa
- formatting and clippy - ([a9d8be8](https://github.com/Metirionic/mars-bluetooth-hci/commit/a9d8be8865e4e15bca925da61369023e219bdc24)) - Adrian Figueroa

- - -

## [mars-bluetooth-hci@0.10.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/084b22f782e0d9fbb7b174de0beb61502b22a591..mars-bluetooth-hci@0.10.0) - 2026-07-15
#### Features
- invalid mode - ([084b22f](https://github.com/Metirionic/mars-bluetooth-hci/commit/084b22f782e0d9fbb7b174de0beb61502b22a591)) - Adrian Figueroa
#### Bug Fixes
- version strings in docs - ([ecad271](https://github.com/Metirionic/mars-bluetooth-hci/commit/ecad2714ab85769b4e9c6250a7318c1d82032d5e)) - Adrian Figueroa

- - -

## [mars-bluetooth-hci@0.9.0](https://github.com/Metirionic/mars-bluetooth-hci/compare/caf2398ee278187a702208d86861212660e01440..mars-bluetooth-hci@0.9.0) - 2026-07-14
#### Features
- add mode 1 and mode 3 subevent result support - ([e719422](https://github.com/Metirionic/mars-bluetooth-hci/commit/e719422f54cc17351c09b8fa9f88f0af652a5545)) - Johannes Guertler
#### Bug Fixes
- format pipeline - ([8499fa4](https://github.com/Metirionic/mars-bluetooth-hci/commit/8499fa4cdd72b425f7b27c81f1aa85236f5e524a)) - Johannes Guertler
- clippy pipeline - ([af11e3c](https://github.com/Metirionic/mars-bluetooth-hci/commit/af11e3cb02ffbd0ca75a9b70806dad64ce6055da)) - Johannes Guertler
#### Documentation
- complete MARS-name alignment and sync version-check docstring - ([f5a4015](https://github.com/Metirionic/mars-bluetooth-hci/commit/f5a4015eb3f52b60770b1cb53af3640faba7b2c6)) - Attila Römer, Claude
- add C/embedded integration guide (#14) - ([99a049e](https://github.com/Metirionic/mars-bluetooth-hci/commit/99a049e9aa82eaac1a2ba50a7932ea62bede27d3)) - Attila Römer, Claude
- scope file-reader-helper reference to origin in TryFrom doc - ([5bcedea](https://github.com/Metirionic/mars-bluetooth-hci/commit/5bcedeae8ab3f6fdbbed3997ed217ee76845f9c5)) - Attila Römer, Claude
- document ModeRoleSpecificInfoKind variants and add parse-from-bytes doctest (#7) - ([caf2398](https://github.com/Metirionic/mars-bluetooth-hci/commit/caf2398ee278187a702208d86861212660e01440)) - Attila Römer, Claude
#### Refactoring
- ![BREAKING](https://img.shields.io/badge/BREAKING-red) remove dead Unsupported placeholder struct - ([ecc4e57](https://github.com/Metirionic/mars-bluetooth-hci/commit/ecc4e57139ea8fb4a35058a4c251cef01684c494)) - Attila Römer, Claude

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