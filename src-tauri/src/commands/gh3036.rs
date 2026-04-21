use tauri::{AppHandle, State};

use crate::error::{ComBridgeError, ErrorResponse};
use crate::gh3036::{
    ChannelConfig, ChannelType, CsvConfig, FactoryTestResult,
    FactoryTestStep, FactoryTestStatus, Gh3036ManagerRef, RpcCommand, VersionTypeConfig,
    ConfigValidationResult,
};

#[tauri::command]
pub async fn gh3036_init(
    manager: State<'_, Gh3036ManagerRef>,
    app_handle: AppHandle,
) -> Result<(), ErrorResponse> {
    manager.set_app_handle(app_handle);
    manager
        .initialize()
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_is_initialized(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<bool, ErrorResponse> {
    Ok(manager.is_initialized())
}

#[tauri::command]
pub async fn gh3036_configure_tx_channel(
    channel_type: String,
    device_id: String,
    characteristic_uuid: Option<String>,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    let ch_type = match channel_type.to_lowercase().as_str() {
        "serial" => ChannelType::Serial,
        "ble" => ChannelType::Ble,
        _ => {
            return Err(ComBridgeError::protocol(format!(
                "不支持的通道类型: {}",
                channel_type
            ))
            .to_error_response())
        }
    };

    let config = ChannelConfig {
        channel_type: ch_type,
        device_id,
        characteristic_uuid,
    };

    manager
        .configure_tx_channel(config)
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_configure_rx_channel(
    channel_type: String,
    device_id: String,
    characteristic_uuid: Option<String>,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    let ch_type = match channel_type.to_lowercase().as_str() {
        "serial" => ChannelType::Serial,
        "ble" => ChannelType::Ble,
        _ => {
            return Err(ComBridgeError::protocol(format!(
                "不支持的通道类型: {}",
                channel_type
            ))
            .to_error_response())
        }
    };

    let config = ChannelConfig {
        channel_type: ch_type,
        device_id,
        characteristic_uuid,
    };

    manager
        .configure_rx_channel(config)
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_get_channels(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(Option<ChannelConfig>, Option<ChannelConfig>), ErrorResponse> {
    let tx = manager.get_tx_channel();
    let rx = manager.get_rx_channel();
    Ok((tx, rx))
}

#[tauri::command]
pub async fn gh3036_send_data(
    data: Vec<u8>,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .send_data(&data)
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_set_csv_config(
    enabled: bool,
    output_dir: String,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    let config = CsvConfig { enabled, output_dir };
    manager
        .set_csv_config(config)
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_get_csv_config(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<CsvConfig, ErrorResponse> {
    Ok(manager.get_csv_config())
}

#[tauri::command]
pub fn gh3036_get_rpc_commands() -> Result<Vec<RpcCommand>, ErrorResponse> {
    Ok(crate::gh3036::get_rpc_commands())
}

#[tauri::command]
pub fn gh3036_get_version_types() -> Result<Vec<VersionTypeConfig>, ErrorResponse> {
    Ok(crate::gh3036::get_version_types())
}

#[tauri::command]
pub async fn gh3036_execute_rpc(
    command_key: String,
    params: Vec<String>,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<Vec<u8>, ErrorResponse> {
    manager
        .execute_rpc(&command_key, &params)
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_subscribe_events(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<bool, ErrorResponse> {
    Ok(manager.subscribe_events())
}

#[tauri::command]
pub async fn gh3036_get_library_status(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(bool, bool), ErrorResponse> {
    Ok(manager.get_library_status())
}

#[tauri::command]
pub async fn gh3036_load_config_file(
    file_path: String,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<Vec<String>, ErrorResponse> {
    manager
        .load_config_file(&file_path)
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_factory_test_start(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    let manager_clone = manager.inner().clone();
    manager_clone
        .factory_test_start()
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_factory_test_stop(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .factory_test_stop()
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_factory_test_status(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(FactoryTestStatus, FactoryTestStep), ErrorResponse> {
    Ok(manager.factory_test_status())
}

#[tauri::command]
pub async fn gh3036_factory_test_continue(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .factory_test_continue()
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_factory_test_set_config_dir(
    config_dir: String,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .factory_test_set_config_dir(&config_dir)
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_factory_test_validate_config(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<ConfigValidationResult, ErrorResponse> {
    Ok(manager.factory_test_validate_config())
}

#[tauri::command]
pub async fn gh3036_factory_test_get_result(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<Option<FactoryTestResult>, ErrorResponse> {
    Ok(manager.factory_test_get_result())
}
