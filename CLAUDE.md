# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ComBridge is a cross-platform serial/BLE communication debugging tool built with Tauri 2.0 (Rust backend + React frontend). It targets embedded device development workflows, providing serial port communication, dual-mode BLE (native + AT command), Lua protocol plugins, waveform visualization, and WebSocket connectivity.

## Build & Development Commands

```bash
# Install frontend dependencies
npm install

# Run in development mode (starts both Vite dev server and Tauri)
npm run tauri dev

# Production build
npm run tauri build

# Frontend only (Vite dev server on port 1420)
npm run dev

# Rust lint and format
cd src-tauri && cargo clippy
cd src-tauri && cargo fmt

# TypeScript type check
npx tsc --noEmit
```

Requirements: Node.js >= 18, Rust >= 1.70, Windows MSVC build tools + WebView2.

## Architecture

### Backend (src-tauri/src/)

The Rust backend uses an **event-driven architecture** centered on `EventBus` (tokio broadcast channels):

- **device/** — Hardware abstraction. `DeviceManager` is the unified facade over `SerialManager` and `BleManager`. BLE has two backends: native (via `bluest`) and AT-command (over serial).
- **service/** — Core infrastructure. `EventBus` is the pub/sub backbone. `EventBridge` forwards Rust events to the Tauri frontend. `MsgPackHandler` handles binary serialization.
- **state/** — Redux-like state management. `ActionDispatcher` processes `Action` objects to mutate `AppState` (behind `RwLock`). `StatePersistence` saves state to disk.
- **protocol/** — Lua scripting engine (`mlua`, Lua 5.4). `PluginManager` manages script lifecycle; `HookExecutor` dispatches data through user-defined hooks.
- **gh3036/** — Domain-specific module for GH3036 sensor chip. Handles factory testing, heart-rate/SpO2 reference monitoring, threshold evaluation, and CSV data recording.
- **waveform/** — Ring-buffer based time-series storage with configurable parsers for raw-byte-to-float conversion.
- **websocket/** — WebSocket client with connection pooling and automatic reconnection.
- **commands/** — Tauri IPC command handlers, one sub-module per domain. These are the frontend-facing API surface.

**Data flow:** Device → EventBus → GH3036/Protocol/Waveform subscribers → EventBridge → Frontend

### Frontend (src/)

React 19 + TypeScript + Vite. Key libraries: Ant Design (UI), Zustand (state), ECharts/Recharts (charts), react-router-dom v7, i18next (zh-CN/en-US).

### External Dependencies

- `../libs/protocol_rust/` — Local crate dependencies (`gh-rpc`, `rpc`) for RPC protocol definitions. These must exist at the sibling path.

## Key Patterns

- All cross-module communication goes through `EventBus`. Device managers, GH3036, and protocol hooks publish/subscribe to typed events.
- Tauri managed state uses `Arc<Mutex<T>>` or `Arc<RwLock<T>>` patterns (look for `*Ref` type aliases like `DeviceManagerRef`, `Gh3036ManagerRef`).
- Protocol plugins are Lua scripts loaded from `src-tauri/parser_scripts/` (bundled as resources in release builds).
- The window is frameless (`decorations: false, transparent: true`) — the frontend implements its own title bar.
- Commit messages are in Chinese, following conventional-commit style with scope: `fix(gh3036):`, `feat(gh3036):`.
