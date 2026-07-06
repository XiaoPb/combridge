use tauri::State;

use crate::error::{ComBridgeError, ErrorResponse};
use crate::gh3036::{
    ChannelConfig, ChannelType, ConfigValidationResult, CsvConfig, FactoryEvaluationResult,
    FactoryTestResult, FactoryTestStatus, FactoryTestStep, FactoryThresholdConfig,
    Gh3036ConfigPreview, Gh3036ManagerRef, RefDataStatus, RpcCommand, ThresholdConfigValidation,
    VersionTypeConfig,
};
use crate::state::StatePersistenceRef;

#[tauri::command]
pub async fn gh3036_init(
    manager: State<'_, Gh3036ManagerRef>,
    persistence: State<'_, StatePersistenceRef>,
) -> Result<(), ErrorResponse> {
    let persistence = persistence.inner().read().await;
    let prefs = persistence.load_preferences().await.map_err(|e| {
        ComBridgeError::config(format!("加载偏好设置失败: {}", e)).to_error_response()
    })?;

    let csv_config = CsvConfig {
        enabled: prefs.gh3036_csv.enabled,
        output_dir: prefs.gh3036_csv.output_dir,
    };
    manager.set_csv_config(csv_config).map_err(|e| {
        ComBridgeError::protocol(format!("设置CSV配置失败: {}", e)).to_error_response()
    })?;

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
            return Err(
                ComBridgeError::protocol(format!("不支持的通道类型: {}", channel_type))
                    .to_error_response(),
            )
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
            return Err(
                ComBridgeError::protocol(format!("不支持的通道类型: {}", channel_type))
                    .to_error_response(),
            )
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
    let config = CsvConfig {
        enabled,
        output_dir,
    };
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
) -> Result<Gh3036ConfigPreview, ErrorResponse> {
    manager
        .load_config_file(&file_path)
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_download_config_file(
    file_path: String,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .download_config_file(&file_path)
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

#[tauri::command]
pub async fn gh3036_validate_threshold_config(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<ThresholdConfigValidation, ErrorResponse> {
    Ok(manager.validate_threshold_config())
}

#[tauri::command]
pub async fn gh3036_get_threshold_config(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<Option<FactoryThresholdConfig>, ErrorResponse> {
    Ok(manager.get_threshold_config())
}

#[tauri::command]
pub async fn gh3036_get_evaluation_result(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<Option<FactoryEvaluationResult>, ErrorResponse> {
    Ok(manager.get_evaluation_result())
}

#[tauri::command]
pub async fn gh3036_set_hr_ref(
    values: Vec<i32>,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .set_hr_ref(&values)
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_set_hrv_ref(
    values: Vec<i32>,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .set_hrv_ref(&values)
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_set_spo2_ref(
    values: Vec<i32>,
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .set_spo2_ref(&values)
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_clear_hr_ref(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager.clear_hr_ref();
    Ok(())
}

#[tauri::command]
pub async fn gh3036_clear_hrv_ref(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager.clear_hrv_ref();
    Ok(())
}

#[tauri::command]
pub async fn gh3036_clear_spo2_ref(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager.clear_spo2_ref();
    Ok(())
}

#[tauri::command]
pub async fn gh3036_clear_all_ref(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager.clear_all_ref();
    Ok(())
}

#[tauri::command]
pub async fn gh3036_get_ref_data_status(
    manager: State<'_, Gh3036ManagerRef>,
) -> Result<RefDataStatus, ErrorResponse> {
    Ok(manager.get_ref_data_status())
}

#[tauri::command]
pub async fn gh3036_start_hr_ref_monitor(device_address: String) -> Result<(), ErrorResponse> {
    crate::gh3036::start_hr_ref_monitor(&device_address)
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub async fn gh3036_stop_hr_ref_monitor() -> Result<(), ErrorResponse> {
    crate::gh3036::stop_hr_ref_monitor()
        .await
        .map_err(|e| ComBridgeError::protocol(e).to_error_response())
}

#[tauri::command]
pub fn gh3036_get_hr_ref_monitor_status() -> Result<(bool, i32, i32), ErrorResponse> {
    let is_running = crate::gh3036::is_hr_ref_monitor_running();
    let current_hr = crate::gh3036::get_hr_ref_monitor_current_hr();
    let collected_count = crate::gh3036::get_hr_ref_monitor_collected_count();
    Ok((is_running, current_hr, collected_count))
}
