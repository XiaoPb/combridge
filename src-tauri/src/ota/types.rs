// src-tauri/src/ota/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OtaState {
    Idle,
    Running,
    Success,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OtaStage {
    Idle,
    Unpacking,
    Validating,
    Preparing,
    Transferring,
    Activating,
    Rebooting,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaKeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaStatusDto {
    pub state: OtaState,
    pub stage: OtaStage,
    pub protocol: Option<String>,
    pub address: Option<String>,
    pub firmware_path: Option<String>,
    pub current: u64,
    pub total: u64,
    pub percent: f64,
    pub message: String,
    pub error: Option<String>,
    pub updated_at: u64,
}

impl OtaStatusDto {
    pub fn idle() -> Self {
        Self {
            state: OtaState::Idle,
            stage: OtaStage::Idle,
            protocol: None,
            address: None,
            firmware_path: None,
            current: 0,
            total: 0,
            percent: 0.0,
            message: String::new(),
            error: None,
            updated_at: now_millis(),
        }
    }
}

/// 协议描述，供前端填充协议下拉框。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaProtocolDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaDeviceInfo {
    pub address: String,
    pub protocol: String,
    pub name: Option<String>,
    pub mode: Option<String>,
    pub mtu: Option<u16>,
    pub version: Option<String>,
    /// 协议自定义信息（如 SMP 的 bootloader、镜像槽位），前端按 key 动态展示。
    pub fields: Vec<OtaKeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaPackageInfo {
    pub path: String,
    pub file_name: String,
    pub package_size: u64,
    pub protocol: String,
    pub image_name: String,
    pub image_size: u64,
    pub image_sha256: String,
    /// Goodix 双区写入地址（按加载地址 + 4K 对齐的镜像大小计算）；其他协议为 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dual_bank_address: Option<String>,
    /// 协议自定义展示字段（SMP 为应用名、目标板、SoC、MCUboot 版本等）。
    pub fields: Vec<OtaKeyValue>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaStartOptions {
    pub force_confirm: bool,
    pub upgrade_only: bool,
    /// Goodix 路由：`auto` / `normal` / `fastService`；SMP 忽略该字段。
    #[serde(default)]
    pub route: Option<String>,
    /// Goodix 传输模式：`auto` / `normal` / `fast`；SMP 忽略该字段。
    #[serde(default)]
    pub transfer_mode: Option<String>,
    /// Goodix Flash 模式：`dual`（默认，后台拷贝）/ `single`（原地升级）；SMP 忽略该字段。
    #[serde(default)]
    pub bank_mode: Option<String>,
    /// Goodix 显式写入地址（十六进制字符串）；为空时使用镜像加载地址。
    #[serde(default)]
    pub target_address: Option<String>,
    /// Goodix 完成后是否立即复位运行；默认 `true`。
    #[serde(default)]
    pub run_immediately: Option<bool>,
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
