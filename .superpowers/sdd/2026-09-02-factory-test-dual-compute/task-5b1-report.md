# Task 5B1 Report

## Changes
- Locked `CHIP_INIT` to select `mcu` or `app` once, then removed the old unreachable branch body.
- Added UUID dual-path handling for MCU and App modes, including 32-byte normalization, 2-channel step data, and app-only eFuse reads for GH3036/GH3038.
- Split UUID error codes into `0x2001` and `0x2002` while keeping the legacy `0x2001` compatibility path.
- Added focused tests for helper behavior, MCU UUID flow, App UUID flow, unsupported chips, and existing CHIP_INIT fallback cases.

## Tests
- `cargo test --manifest-path src-tauri/Cargo.toml factory_test::tests`
- `cargo test --manifest-path src-tauri/Cargo.toml factory_test::collector_lifecycle_tests`
- `rustfmt --check --edition 2021 src-tauri\\src\\gh3036\\factory_test.rs src-tauri\\src\\gh3036\\threshold_config.rs`

## Risks
- App UUID coverage depends on the live eFuse reader behavior on real hardware.
- Legacy UUID error handling is preserved, but broader Task 5B2 routing is intentionally untouched here.
