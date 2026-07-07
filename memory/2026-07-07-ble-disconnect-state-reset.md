# BLE Disconnect State Reset Debug Report

Date: 2026-07-07

## Symptom

Active or passive BLE disconnects could leave backend or frontend state partially uncleared. After that, the BLE page could fail to scan/reconnect normally until the app was restarted.

## Root Cause

Passive native BLE disconnect did not go through `BleManager.disconnect()`. The monitor callback reset the `GattClient` and published `ble:disconnected`, but it left stale adapter clients and skipped manager-level cleanup such as subscriptions and AT tab state.

Frontend cleanup was also split between the command-driven disconnect path and the event listener path, so active and passive disconnects did not share one complete reset operation.

AT scanning had a secondary state-reset gap: if `scan()` errored after setting `is_scanning`, the flag could remain true.

## Fix

- Added shared backend disconnected-state cleanup in `BleManager`.
- Made native passive disconnect remove stale adapter clients and scanned-device cache before publishing `ble:disconnected`.
- Subscribed backend `BleManager` to `ble:disconnected` so passive disconnects clear manager-level state too.
- Added a single frontend `clearDisconnectedDevice(address)` store action and reused it from both event-driven and command-driven disconnect paths.
- Added an AT scan guard so scan state resets on success and error.

## Evidence

- `npx tsc --noEmit`: passed.
- `cd src-tauri && cargo fmt --check`: passed.
- `cd src-tauri && cargo check`: passed with existing warnings in sibling `rpc` crate.
- `cd src-tauri && cargo test`: 93 passed.
- `npm run build`: passed with existing Vite chunk-size/dynamic-import warnings.

## Regression Test

Added `device::ble::ble_manager::tests::clear_disconnected_state_removes_subscriptions_and_at_tabs`, which verifies disconnected-state cleanup removes BLE subscriptions and AT tabs for a disconnected address.

## Status

DONE_WITH_CONCERNS: Code-level cleanup and automated checks are complete. Physical active/passive BLE reconnect verification still requires real BLE hardware.
