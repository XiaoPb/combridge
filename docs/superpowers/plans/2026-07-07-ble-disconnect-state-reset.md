# BLE Disconnect State Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make active and passive BLE disconnects fully clear backend and frontend connection state so users can scan, reconnect, and continue the normal BLE workflow without restarting ComBridge.

**Architecture:** Centralize backend disconnect cleanup in `BleManager` and make the native adapter remove stale `GattClient` entries during passive disconnects. Centralize frontend cleanup in the BLE store so command-driven disconnects and event-driven disconnects perform the same reset without duplicate, drifting logic.

**Tech Stack:** Rust/Tauri 2 backend, `tokio`, `bluest`, `EventBus`; React 19 frontend, TypeScript, Zustand.

---

## Root Cause Analysis

Observed symptom: after active or passive BLE disconnect, some state remains in memory. The frontend can get stuck with stale BLE state and scanning/reconnecting only recovers after restarting the app.

Primary root cause hypothesis: passive native BLE disconnect bypasses `BleManager.disconnect()`. The passive path in `src-tauri/src/device/ble/native/adapter.rs` only calls `GattClient.reset_state()` and publishes `ble:disconnected`; it does not remove the client from `BleAdapter.clients`, does not clear `BleManager.subscriptions`, and does not clear AT tabs or any future manager-level per-device state.

Evidence in current code:
- Active disconnect cleanup is centralized in `BleManager.disconnect()` at `src-tauri/src/device/ble/ble_manager.rs:296`: it calls backend disconnect, removes `subscriptions`, removes `at_tabs`, and publishes `ble:disconnected`.
- Passive native disconnect is handled in `BleAdapter.connect_device()` at `src-tauri/src/device/ble/native/adapter.rs:199`: callback only runs `client_clone.reset_state()` and publishes `ble:disconnected`; `clients` keeps a stale entry and manager-level state is skipped.
- `GattClient.reset_state()` at `src-tauri/src/device/ble/native/gatt_client.rs:85` clears the client internals, but it cannot remove itself from `BleAdapter.clients`.
- Frontend `handleBleDisconnected()` at `src/services/eventListeners.ts:88` removes connection and device tab, but cleanup logic is duplicated with `useBle.disconnectDevice()` at `src/hooks/useBle.ts:165`; neither path currently has one shared "clear this device after disconnect" API.
- AT scan sets `is_scanning` true at `src-tauri/src/device/ble/at/at_backend.rs:253`, but command errors before line 273 can leave it true. That is a secondary scan-state cleanup gap.

Secondary risk: active disconnect currently publishes a backend event and also immediately mutates frontend state in `useBle.disconnectDevice()`. The duplicated cleanup is mostly harmless today, but it makes regressions likely because active and passive disconnects are not forced through the same frontend reset path.

## File Structure

Modify these files:
- `src-tauri/src/device/ble/ble_manager.rs`  
  Add a manager-level cleanup helper and a passive-disconnect callback path for native backend. Active and passive disconnects both use the same cleanup helper.
- `src-tauri/src/device/ble/native/native_backend.rs`  
  Accept a passive disconnect callback from `BleManager` and pass it into `BleAdapter`.
- `src-tauri/src/device/ble/native/adapter.rs`  
  Store a manager callback, remove stale clients/scanned cache on passive disconnect, and publish one consistent disconnect event through the manager callback.
- `src-tauri/src/device/ble/at/at_backend.rs`  
  Ensure scan state is reset when `scan()` exits with an error.
- `src/stores/bleStore.ts`  
  Add `clearDisconnectedDevice(address)` as the single frontend reset operation.
- `src/services/eventListeners.ts`  
  Replace manual BLE disconnect cleanup with `clearDisconnectedDevice()`.
- `src/hooks/useBle.ts`  
  Use `clearDisconnectedDevice()` after command success or rely on the event path, but do not keep a second partial cleanup implementation.

No new runtime dependency is needed.

## Task 1: Backend Shared Disconnect Cleanup

**Files:**
- Modify: `src-tauri/src/device/ble/ble_manager.rs`

- [ ] **Step 1: Add a private cleanup helper**

Add this helper inside `impl BleManager`, near `disconnect()`:

```rust
    async fn clear_device_state(&self, address: &str) {
        let mut subscriptions = self.subscriptions.write().await;
        if subscriptions.remove(address).is_some() {
            info!("清理设备 {} 的订阅记录", address);
        }
        drop(subscriptions);

        let mut tabs = self.at_tabs.write().await;
        let before = tabs.len();
        tabs.retain(|_, tab| tab.address != address);
        if tabs.len() != before {
            info!("清理设备 {} 的AT连接TAB", address);
        }
    }
```

- [ ] **Step 2: Make active disconnect use the helper**

Replace the duplicated cleanup block in `disconnect()`:

```rust
        let mut subscriptions = self.subscriptions.write().await;
        if subscriptions.remove(address).is_some() {
            info!("清理设备 {} 的订阅记录", address);
        }

        let mut tabs = self.at_tabs.write().await;
        tabs.retain(|_, tab| tab.address != address);
```

with:

```rust
        self.clear_device_state(address).await;
```

- [ ] **Step 3: Add a passive-disconnect entry point**

Add this public method inside `impl BleManager`:

```rust
    pub async fn handle_passive_disconnect(&self, address: &str) {
        self.clear_device_state(address).await;

        let event = BleConnectionEvent::new(address, None);
        self.event_bus
            .publish_typed(topics::BLE_DISCONNECTED, &event);
        info!("BLE设备被动断开，状态已清理: {}", address);
    }
```

- [ ] **Step 4: Run backend format check**

Run:

```bash
cd src-tauri && cargo fmt --check
```

Expected: format check either passes, or reports only formatting in touched Rust files. If it reports formatting, run `cd src-tauri && cargo fmt` before continuing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/device/ble/ble_manager.rs
git commit -m "fix(ble): 统一断链状态清理入口"
```

## Task 2: Native Passive Disconnect Removes Stale Client

**Files:**
- Modify: `src-tauri/src/device/ble/native/adapter.rs`
- Modify: `src-tauri/src/device/ble/ble_manager.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Make adapter clients shareable with callback handles**

In `src-tauri/src/device/ble/native/adapter.rs`, change the `clients` field from:

```rust
    clients: RwLock<HashMap<String, Arc<GattClient>>>,
```

to:

```rust
    clients: Arc<RwLock<HashMap<String, Arc<GattClient>>>>,
```

Initialize it in the constructor with:

```rust
            clients: Arc::new(RwLock::new(HashMap::new())),
```

- [ ] **Step 2: Add callback handle struct**

Add this struct in `adapter.rs` below `BleAdapter`:

```rust
#[derive(Clone)]
struct BleAdapterHandles {
    scanned_devices: Arc<RwLock<HashMap<DeviceId, (Arc<Device>, Option<i16>)>>>,
    clients: Arc<RwLock<HashMap<String, Arc<GattClient>>>>,
    event_bus: Option<Arc<EventBus>>,
}
```

Add this method to `impl BleAdapter`:

```rust
    fn clone_handles(&self) -> BleAdapterHandles {
        BleAdapterHandles {
            scanned_devices: self.scanned_devices.clone(),
            clients: self.clients.clone(),
            event_bus: self.event_bus.clone(),
        }
    }
```

Add the cleanup implementation:

```rust
impl BleAdapterHandles {
    fn cleanup_passive_disconnect(&self, address: &str) {
        {
            let mut clients = self.clients.write().unwrap_or_else(|e| e.into_inner());
            clients.remove(address);
        }
        {
            let mut devices = self.scanned_devices.write().unwrap_or_else(|e| e.into_inner());
            devices.retain(|id, _| id.to_string() != address);
        }
        if let Some(event_bus) = &self.event_bus {
            let event = BleConnectionEvent::new(address, None);
            event_bus.publish_typed(topics::BLE_DISCONNECTED, &event);
        }
        info!("BLE设备被动断开并清理适配器状态: {}", address);
    }
}
```

- [ ] **Step 3: Replace passive callback body**

In `connect_device()`, replace the current callback block:

```rust
            let event_bus = event_bus.clone();
            let client_clone = client.clone();
            let callback: DisconnectCallback = Arc::new(move |addr: &str| {
                client_clone.reset_state();
                let event = BleConnectionEvent::new(addr, None);
                event_bus.publish_typed(topics::BLE_DISCONNECTED, &event);
                info!("BLE设备被动断开: {}", addr);
            });
            client.set_disconnect_callback(callback);
```

with a callback that also removes the stale client:

```rust
        let adapter_for_callback = self.clone_handles();
        let client_clone = client.clone();
        let callback: DisconnectCallback = Arc::new(move |addr: &str| {
            client_clone.reset_state();
            adapter_for_callback.cleanup_passive_disconnect(addr);
        });
        client.set_disconnect_callback(callback);
```

- [ ] **Step 4: Expose manager cleanup for event subscriber**

In `src-tauri/src/device/ble/ble_manager.rs`, add this public wrapper:

```rust
    pub async fn clear_disconnected_state(&self, address: &str) {
        self.clear_device_state(address).await;
    }
```

- [ ] **Step 5: Register backend cleanup subscriber**

In `src-tauri/src/lib.rs`, after `let device_manager = Arc::new(DeviceManager::new(event_bus.clone()));`, register:

```rust
    let ble_manager_for_disconnect = device_manager.ble_manager.clone();
    event_bus.subscribe_json::<crate::service::event_bus::BleConnectionEvent, _>(
        topics::BLE_DISCONNECTED,
        move |_topic, event| {
            let ble_manager = ble_manager_for_disconnect.clone();
            tokio::spawn(async move {
                ble_manager.clear_disconnected_state(&event.address).await;
            });
        },
    );
```

This keeps passive cleanup centralized without creating self-referential `Arc` plumbing.

- [ ] **Step 6: Run Rust checks**

Run:

```bash
cd src-tauri && cargo fmt
cd src-tauri && cargo check
```

Expected: `cargo check` passes. If local `../libs/protocol_rust/` crates are missing, record the exact missing path and continue with TypeScript checks.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/device/ble/native/adapter.rs src-tauri/src/device/ble/ble_manager.rs src-tauri/src/lib.rs
git commit -m "fix(ble): 清理被动断链残留连接"
```

## Task 3: Frontend Single Disconnect Cleanup API

**Files:**
- Modify: `src/stores/bleStore.ts`
- Modify: `src/services/eventListeners.ts`
- Modify: `src/hooks/useBle.ts`

- [ ] **Step 1: Add store action type**

In `BleState`, add:

```ts
  clearDisconnectedDevice: (address: string) => void;
```

- [ ] **Step 2: Implement cleanup action**

Add this action near `removeConnection`:

```ts
  clearDisconnectedDevice: (address: string) =>
    set((state) => {
      const { [address]: _removedTab, ...remainingTabs } = state.deviceTabs;
      const remainingConnections = state.connections.filter((c) => c.address !== address);
      const wasCurrentDevice = state.currentDevice === address;
      return {
        connections: remainingConnections,
        currentDevice: wasCurrentDevice ? null : state.currentDevice,
        services: wasCurrentDevice ? [] : state.services,
        characteristics: wasCurrentDevice ? [] : state.characteristics,
        notifications: wasCurrentDevice ? [] : state.notifications,
        deviceTabs: remainingTabs,
        atTabs: state.atTabs.filter((tab) => tab.address !== address),
        activeAtTabId: state.atTabs.some((tab) => tab.id === state.activeAtTabId && tab.address === address)
          ? null
          : state.activeAtTabId,
        isConnecting: false,
        error: null,
      };
    }),
```

- [ ] **Step 3: Use it in event listener**

Replace manual cleanup in `handleBleDisconnected()`:

```ts
  const wasCurrentDevice = store.currentDevice === deviceId;
  
  store.removeConnection(deviceId);
  store.removeDeviceTab(deviceId);
  
  if (wasCurrentDevice) {
    store.setCurrentDevice(null);
    store.clearServices();
    store.clearCharacteristics();
    store.clearNotifications();
  }
```

with:

```ts
  const connection = store.connections.find((c) => c.address === deviceId);
  store.clearDisconnectedDevice(deviceId);
```

Then replace:

```ts
  const deviceName = store.connections.find(c => c.address === deviceId)?.name || deviceId;
```

with:

```ts
  const deviceName = connection?.name || deviceId;
```

- [ ] **Step 4: Use it in command-driven disconnect**

In `useBle.ts`, destructure:

```ts
    clearDisconnectedDevice,
```

Replace the local cleanup in `disconnectDevice()`:

```ts
      removeConnection(deviceId);
      if (currentDevice === deviceId) {
        setCurrentDevice(null);
        clearServices();
        clearCharacteristics();
      }
```

with:

```ts
      clearDisconnectedDevice(deviceId);
```

Update the hook dependency array to include `clearDisconnectedDevice` and remove `removeConnection`, `currentDevice`, `clearServices`, and `clearCharacteristics` if no longer used by `disconnectDevice()`.

- [ ] **Step 5: Type-check**

Run:

```bash
npx tsc --noEmit
```

Expected: no TypeScript errors.

- [ ] **Step 6: Commit**

```bash
git add src/stores/bleStore.ts src/services/eventListeners.ts src/hooks/useBle.ts
git commit -m "fix(ble): 统一前端断链状态清理"
```

## Task 4: AT Scan State Always Resets

**Files:**
- Modify: `src-tauri/src/device/ble/at/at_backend.rs`

- [ ] **Step 1: Wrap AT scan with a guard**

Add this helper struct near `AtBleBackend`:

```rust
struct ScanStateGuard<'a> {
    is_scanning: &'a AtomicBool,
}

impl<'a> ScanStateGuard<'a> {
    fn new(is_scanning: &'a AtomicBool) -> Self {
        is_scanning.store(true, Ordering::SeqCst);
        Self { is_scanning }
    }
}

impl Drop for ScanStateGuard<'_> {
    fn drop(&mut self) {
        self.is_scanning.store(false, Ordering::SeqCst);
    }
}
```

- [ ] **Step 2: Use guard in `scan()`**

Replace:

```rust
        self.is_scanning.store(true, Ordering::SeqCst);
```

with:

```rust
        let _scan_guard = ScanStateGuard::new(&self.is_scanning);
```

Remove:

```rust
        self.is_scanning.store(false, Ordering::SeqCst);
```

at the end of `scan()` because the guard now handles it on success and error.

- [ ] **Step 3: Run backend checks**

Run:

```bash
cd src-tauri && cargo fmt
cd src-tauri && cargo check
```

Expected: `cargo check` passes or only fails because the sibling local crates under `../libs/protocol_rust/` are absent.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/device/ble/at/at_backend.rs
git commit -m "fix(ble): 确保AT扫描状态复位"
```

## Task 5: Manual Verification

**Files:**
- No source changes expected.

- [ ] **Step 1: Start app**

Run:

```bash
npm run tauri dev
```

Expected: app opens and BLE page loads.

- [ ] **Step 2: Verify active disconnect**

Manual flow:

1. Open BLE page.
2. Scan for a BLE device.
3. Connect to a device.
4. Discover services.
5. Click disconnect.
6. Scan again.
7. Connect to the same device again.

Expected:
- Connection list no longer contains the disconnected device.
- Current device is cleared.
- GATT services/characteristics panel is cleared.
- Device can appear in scan results again.
- Reconnect works without restarting the app.

- [ ] **Step 3: Verify passive disconnect**

Manual flow:

1. Scan and connect to a BLE device.
2. Power off the BLE peripheral or move it out of range.
3. Wait at least 6 seconds because the monitor checks every 2 seconds and confirms two failures after an initial delay.
4. Scan again.
5. Power device back on.
6. Reconnect.

Expected:
- `ble:disconnected` notification appears.
- Connection list and current device clear.
- Backend `get_ble_connections` returns no stale connection for that address.
- Re-scan and reconnect work without restarting.

- [ ] **Step 4: Verify AT mode disconnect**

Manual flow:

1. Configure AT BLE mode with serial port and UUIDs.
2. Scan and connect.
3. Disconnect using UI.
4. Scan again.

Expected:
- AT tab for the address is removed.
- AT connection list is empty.
- Scan button is usable after command errors or normal completion.

- [ ] **Step 5: Final checks**

Run:

```bash
npx tsc --noEmit
cd src-tauri && cargo fmt --check
cd src-tauri && cargo check
```

Expected: all pass, except `cargo check` may be blocked if required sibling crates are missing locally.

- [ ] **Step 6: Commit verification notes**

If manual verification reveals no additional code changes, do not create an empty commit. Add a short note to the PR or final report with:

```text
Verified active BLE disconnect, passive BLE disconnect, and AT scan-state reset on Windows.
```

## Self-Review

Spec coverage:
- Active disconnect cleanup: covered by Task 1 and Task 3.
- Passive native disconnect cleanup: covered by Task 2 and Task 3.
- Frontend stale state cleanup: covered by Task 3.
- Scan state getting stuck: covered by Task 4.
- Verification without restart: covered by Task 5.

Placeholder scan:
- No `TBD`, `TODO`, or deferred test instructions are present.
- All changed files and commands are named explicitly.

Implementation note:
- The callback wiring in Task 2 should be implemented carefully. If the direct `BleManager` callback becomes awkward because `BleManager` methods currently use `&self`, prefer the `EventBus` subscription cleanup path in `lib.rs`; it keeps the diff smaller and avoids self-referential ownership.
