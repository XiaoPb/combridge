use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::device::ble::{
    AtConfig, BleCharacteristic, BleConnection, BleDevice, BleManagerRef, BleMode, BleService,
};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleConfigDto {
    pub mode: String,
    pub port_name: Option<String>,
    pub baud_rate: Option<u32>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BleNotifyEvent {
    pub address: String,
    pub char_uuid: String,
    pub data: Vec<u8>,
}

#[tauri::command]
pub async fn configure_ble(
    manager: State<'_, BleManagerRef>,
    config: BleConfigDto,
) -> Result<()> {
    let manager = manager.inner();

    let mode = match config.mode.to_lowercase().as_str() {
        "native" => BleMode::Native,
        "at" => BleMode::At,
        _ => {
            return Err(crate::error::ComBridgeError::ble(format!(
                "无效的BLE模式: {}",
                config.mode
            )))
        }
    };

    match mode {
        BleMode::Native => manager.configure_native().await?,
        BleMode::At => {
            let port_name = config.port_name.ok_or_else(|| {
                crate::error::ComBridgeError::ble("AT模式需要指定port_name")
            })?;
            let at_config = AtConfig {
                port_name,
                baud_rate: config.baud_rate.unwrap_or(115200),
                timeout_ms: config.timeout_ms.unwrap_or(1000),
            };
            manager.configure_at(at_config).await?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn scan_ble_devices(
    manager: State<'_, BleManagerRef>,
    duration_ms: u64,
) -> Result<Vec<BleDevice>> {
    let manager = manager.inner();
    manager.scan(duration_ms).await
}

#[tauri::command]
pub async fn connect_ble(
    manager: State<'_, BleManagerRef>,
    address: String,
) -> Result<BleConnection> {
    let manager = manager.inner();
    manager.connect(&address).await
}

#[tauri::command]
pub async fn disconnect_ble(
    manager: State<'_, BleManagerRef>,
    address: String,
) -> Result<()> {
    let manager = manager.inner();
    manager.disconnect(&address).await
}

#[tauri::command]
pub async fn get_ble_connections(
    manager: State<'_, BleManagerRef>,
) -> Result<Vec<BleConnection>> {
    let manager = manager.inner();
    manager.get_connections().await
}

#[tauri::command]
pub async fn discover_ble_services(
    manager: State<'_, BleManagerRef>,
    address: String,
) -> Result<Vec<BleService>> {
    let manager = manager.inner();
    manager.discover_services(&address).await
}

#[tauri::command]
pub async fn discover_ble_characteristics(
    manager: State<'_, BleManagerRef>,
    address: String,
    service_uuid: String,
) -> Result<Vec<BleCharacteristic>> {
    let manager = manager.inner();
    manager.discover_characteristics(&address, &service_uuid).await
}

#[tauri::command]
pub async fn read_ble_characteristic(
    manager: State<'_, BleManagerRef>,
    address: String,
    char_uuid: String,
) -> Result<Vec<u8>> {
    let manager = manager.inner();
    manager.read_characteristic(&address, &char_uuid).await
}

#[tauri::command]
pub async fn write_ble_characteristic(
    manager: State<'_, BleManagerRef>,
    address: String,
    char_uuid: String,
    data: Vec<u8>,
) -> Result<()> {
    let manager = manager.inner();
    manager.write_characteristic(&address, &char_uuid, &data).await
}

#[tauri::command]
pub async fn subscribe_ble_notify(
    manager: State<'_, BleManagerRef>,
    app: AppHandle,
    address: String,
    char_uuid: String,
) -> Result<()> {
    let manager = manager.inner();

    let app_clone = app.clone();
    let addr_clone = address.clone();
    let char_clone = char_uuid.clone();
    let callback = std::sync::Arc::new(move |_addr: &str, _char: &str, data: &[u8]| {
        let event = BleNotifyEvent {
            address: addr_clone.clone(),
            char_uuid: char_clone.clone(),
            data: data.to_vec(),
        };
        let _ = app_clone.emit("ble-notify", &event);
    });

    manager.subscribe_notify(&address, &char_uuid, callback).await
}

#[tauri::command]
pub async fn unsubscribe_ble_notify(
    manager: State<'_, BleManagerRef>,
    address: String,
    char_uuid: String,
) -> Result<()> {
    let manager = manager.inner();
    manager.unsubscribe_notify(&address, &char_uuid).await
}

#[tauri::command]
pub async fn get_ble_rssi(
    manager: State<'_, BleManagerRef>,
    address: String,
) -> Result<i16> {
    let manager = manager.inner();
    manager.get_rssi(&address).await
}

#[tauri::command]
pub async fn get_ble_mode(
    manager: State<'_, BleManagerRef>,
) -> Result<String> {
    let manager = manager.inner();
    Ok(manager.mode().await.to_string())
}

#[tauri::command]
pub async fn is_ble_configured(
    manager: State<'_, BleManagerRef>,
) -> Result<bool> {
    let manager = manager.inner();
    Ok(manager.is_configured().await)
}
