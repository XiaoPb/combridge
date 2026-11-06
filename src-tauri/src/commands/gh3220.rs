//! GH3220 UProtocol 的 Tauri 命令。

use tauri::State;

use crate::error::ErrorResponse;
use crate::gh3220::{
    Gh3220CaptureConfig, Gh3220CaptureStatus, Gh3220DebugStatus, Gh3220DeviceInfo, Gh3220Error,
    Gh3220LinkConfig, Gh3220ManagerRef, Gh3220Stats, Gh3220Status,
};
use crate::state::{Gh3220Preferences, StatePersistenceRef};

fn map_err(error: Gh3220Error) -> ErrorResponse {
    error.into_combridge().to_error_response()
}

#[tauri::command]
pub async fn gh3220_connect(
    config: Gh3220LinkConfig,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Gh3220Status, ErrorResponse> {
    manager.inner().connect(config).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_disconnect(manager: State<'_, Gh3220ManagerRef>) -> Result<(), ErrorResponse> {
    manager.inner().disconnect().await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_get_status(
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Gh3220Status, ErrorResponse> {
    Ok(manager.inner().status())
}

#[tauri::command]
pub async fn gh3220_get_device_info(
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Gh3220DeviceInfo, ErrorResponse> {
    manager.inner().device_info().await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_set_work_mode(
    mode: u8,
    func_mask: u32,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager
        .inner()
        .set_work_mode(mode, func_mask)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_start(
    func_mask: u32,
    slot_en: u8,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager
        .inner()
        .start(func_mask, slot_en)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_stop(
    func_mask: u32,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager.inner().stop(func_mask).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_switch_slot_func(
    slot: u8,
    extra: u8,
    func_mask: u32,
    on: bool,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager
        .inner()
        .switch_slot_func(slot, extra, func_mask, on)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_reg_read(
    addr: u16,
    count: u8,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Vec<u16>, ErrorResponse> {
    manager.inner().reg_read(addr, count).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_reg_write(
    addr: u16,
    values: Vec<u16>,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager
        .inner()
        .reg_write(addr, &values)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_load_reg_list(
    file_path: String,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager
        .inner()
        .load_reg_list(&file_path)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_chip_ctrl(
    action: String,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager.inner().chip_ctrl(&action).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_set_fifo_thr(
    value: u16,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager.inner().set_fifo_thr(value).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_set_report_event(
    mask: u16,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<u8, ErrorResponse> {
    manager
        .inner()
        .set_report_event(mask)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_debug_status(
    kind: String,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Gh3220DebugStatus, ErrorResponse> {
    manager.inner().debug_status(&kind).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_package_test(
    hex: String,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Vec<u8>, ErrorResponse> {
    manager.inner().package_test(&hex).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_send_raw(
    hex: String,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager.inner().send_raw(&hex).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_capture_start(
    config: Gh3220CaptureConfig,
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager.inner().capture_start(config).await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_capture_stop(
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<(), ErrorResponse> {
    manager.inner().capture_stop().await.map_err(map_err)
}

#[tauri::command]
pub async fn gh3220_get_capture_status(
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Gh3220CaptureStatus, ErrorResponse> {
    Ok(manager.inner().capture_status())
}

#[tauri::command]
pub async fn gh3220_get_stats(
    manager: State<'_, Gh3220ManagerRef>,
) -> Result<Gh3220Stats, ErrorResponse> {
    Ok(manager.inner().stats())
}

#[tauri::command]
pub async fn gh3220_clear_stats(manager: State<'_, Gh3220ManagerRef>) -> Result<(), ErrorResponse> {
    manager.inner().clear_stats();
    Ok(())
}

#[tauri::command]
pub async fn gh3220_get_preferences(
    persistence: State<'_, StatePersistenceRef>,
) -> Result<Gh3220Preferences, ErrorResponse> {
    let persistence = persistence.inner().read().await;
    let prefs = persistence
        .load_preferences()
        .await
        .map_err(|e| e.to_error_response())?;
    Ok(prefs.gh3220)
}

#[tauri::command]
pub async fn gh3220_update_preferences(
    prefs: Gh3220Preferences,
    manager: State<'_, Gh3220ManagerRef>,
    persistence: State<'_, StatePersistenceRef>,
) -> Result<(), ErrorResponse> {
    {
        let persistence = persistence.inner().write().await;
        let mut all = persistence
            .load_preferences()
            .await
            .map_err(|e| e.to_error_response())?;
        all.gh3220 = prefs.clone();
        persistence
            .save_preferences(&all)
            .await
            .map_err(|e| e.to_error_response())?;
    }

    manager.inner().set_csv_config(prefs.to_csv_config());
    manager
        .inner()
        .set_debug_tcp_enabled(prefs.debug_tcp.enabled);
    Ok(())
}

// ---------------------------------------------------------------------------
// §6.6 方案 B：把协议命令号 / 版本类型以目录形式暴露给前端与 MCP。
// 只读、无副作用，前端拉取失败时回退到常量层目录（见 gh3220BackendCatalog.ts）。
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220NamedValueDto {
    pub key: &'static str,
    pub value: u8,
    pub name_zh: &'static str,
    pub name_en: &'static str,
}

#[tauri::command]
pub async fn gh3220_get_version_types() -> Vec<Gh3220NamedValueDto> {
    use gh_uprotocol::ver_type as v;
    vec![
        Gh3220NamedValueDto { key: "fw", value: v::FW, name_zh: "固件版本", name_en: "Firmware" },
        Gh3220NamedValueDto { key: "demo", value: v::DEMO, name_zh: "Demo 版本", name_en: "Demo" },
        Gh3220NamedValueDto { key: "virtual_reg", value: v::VIRTUAL_REG, name_zh: "虚拟寄存器版本", name_en: "Virtual register" },
        Gh3220NamedValueDto { key: "bootloader", value: v::BOOTLOADER, name_zh: "Bootloader 版本", name_en: "Bootloader" },
        Gh3220NamedValueDto { key: "protocol", value: v::PROTOCOL, name_zh: "协议版本", name_en: "Protocol" },
        Gh3220NamedValueDto { key: "func_support", value: v::FUNC_SUPPORT, name_zh: "功能支持版本", name_en: "Function support" },
        Gh3220NamedValueDto { key: "drv", value: v::DRV, name_zh: "驱动版本", name_en: "Driver" },
        Gh3220NamedValueDto { key: "chip", value: v::CHIP, name_zh: "芯片版本", name_en: "Chip" },
        Gh3220NamedValueDto { key: "algo", value: v::ALGO, name_zh: "算法版本", name_en: "Algorithm" },
    ]
}

#[derive(serde::Serialize)]
pub struct Gh3220CommandCodeDto {
    pub key: &'static str,
    pub cmd: u8,
}

#[tauri::command]
pub async fn gh3220_get_command_codes() -> Vec<Gh3220CommandCodeDto> {
    use gh_uprotocol::command as c;
    vec![
        Gh3220CommandCodeDto { key: "regRw", cmd: c::REG_RW },
        Gh3220CommandCodeDto { key: "packageTest", cmd: c::PACKAGE_TEST },
        Gh3220CommandCodeDto { key: "newStructRawdata", cmd: c::NEW_STRUCT_RAWDATA },
        Gh3220CommandCodeDto { key: "startCtrl", cmd: c::START_CTRL },
        Gh3220CommandCodeDto { key: "workMode", cmd: c::WORK_MODE },
        Gh3220CommandCodeDto { key: "fifoThrConfig", cmd: c::FIFO_THR_CONFIG },
        Gh3220CommandCodeDto { key: "reportEventSet", cmd: c::REPORT_EVENT_SET },
        Gh3220CommandCodeDto { key: "ecgClassification", cmd: c::ECG_CLASSIFICATION },
        Gh3220CommandCodeDto { key: "chipEventReport", cmd: c::CHIP_EVENT_REPORT },
        Gh3220CommandCodeDto { key: "chipCtrl", cmd: c::CHIP_CTRL },
        Gh3220CommandCodeDto { key: "getVer", cmd: c::GET_VER },
        Gh3220CommandCodeDto { key: "getConnStatus", cmd: c::GET_CONN_STATUS },
        Gh3220CommandCodeDto { key: "switchSlotFunc", cmd: c::SWITCH_SLOT_FUNC },
        Gh3220CommandCodeDto { key: "getOpenedFunc", cmd: c::GET_OPENED_FUNC },
        Gh3220CommandCodeDto { key: "setEcgCascade", cmd: c::SET_ECG_CASCADE },
        Gh3220CommandCodeDto { key: "bpDataTrans", cmd: c::BP_DATA_TRANS },
        Gh3220CommandCodeDto { key: "functionInfoUpdate", cmd: c::FUNCTION_INFO_UPDATE },
        Gh3220CommandCodeDto { key: "getMaxLen", cmd: c::GET_MAX_LEN },
        Gh3220CommandCodeDto { key: "loadRegList", cmd: c::LOAD_REG_LIST },
        Gh3220CommandCodeDto { key: "debugStatus", cmd: c::DEBUG_STATUS },
    ]
}
