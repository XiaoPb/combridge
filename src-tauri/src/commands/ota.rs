use std::sync::Arc;

use tauri::State;

use crate::error::ComBridgeError;
use crate::ota::{
    OtaDeviceInfo, OtaManager, OtaPackageInfo, OtaProtocolDescriptor, OtaStartOptions, OtaStatusDto,
};

#[tauri::command]
pub async fn ota_list_protocols(
    manager: State<'_, Arc<OtaManager>>,
) -> Result<Vec<OtaProtocolDescriptor>, ComBridgeError> {
    Ok(manager.list_protocols())
}

#[tauri::command]
pub async fn ota_validate_package(
    manager: State<'_, Arc<OtaManager>>,
    firmware_path: String,
    protocol: Option<String>,
) -> Result<OtaPackageInfo, ComBridgeError> {
    manager
        .validate_package(protocol.as_deref(), &firmware_path)
        .await
}

#[tauri::command]
pub async fn ota_preflight(
    manager: State<'_, Arc<OtaManager>>,
    address: String,
    protocol: Option<String>,
) -> Result<OtaDeviceInfo, ComBridgeError> {
    manager.preflight(protocol.as_deref(), &address).await
}

#[tauri::command]
pub async fn ota_start(
    manager: State<'_, Arc<OtaManager>>,
    address: String,
    firmware_path: String,
    protocol: Option<String>,
    options: OtaStartOptions,
) -> Result<OtaStatusDto, ComBridgeError> {
    manager
        .start(protocol, address, firmware_path, options)
        .await
}

#[tauri::command]
pub async fn ota_get_status(
    manager: State<'_, Arc<OtaManager>>,
) -> Result<OtaStatusDto, ComBridgeError> {
    Ok(manager.status())
}

#[tauri::command]
pub async fn ota_cancel(manager: State<'_, Arc<OtaManager>>) -> Result<(), ComBridgeError> {
    manager.cancel()
}
