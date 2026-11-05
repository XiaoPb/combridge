//! GH3220 UProtocol 模块的对外类型、配置与错误定义。

use serde::{Deserialize, Serialize};

use crate::error::ComBridgeError;

/// GH3220 的 GATT 服务（真机广播）
pub const SERVICE_UUID: &str = "0000190e-0000-1000-8000-00805f9b34fb";
/// 主机 -> 设备（写）
pub const WRITE_UUID: &str = "00000004-0000-1000-8000-00805f9b34fb";
/// 设备 -> 主机（通知）
pub const NOTIFY_UUID: &str = "00000003-0000-1000-8000-00805f9b34fb";

/// 单条命令默认超时（毫秒）
///
/// 真机在持续上报数据时，BLE 通知会挤占命令响应；800ms 在 Rust/BlueZ 链路上
/// 偶发超时，2s 兼顾响应速度与稳定性。
pub const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 2000;
/// 单帧最大字节数（payload 238 + 5）
pub const MAX_FRAME_BYTES: usize = 244;
/// 默认分片（BLE 未协商 MTU 时的安全值）
pub const DEFAULT_CHUNK_SIZE: usize = 20;
/// 数据批量上报间隔（毫秒）
pub const DATA_BATCH_INTERVAL_MS: u64 = 60;
/// 单批最大记录数
pub const DATA_BATCH_MAX_RECORDS: usize = 100;

/// CSV 空闲关闭默认时长（秒）
pub const DEFAULT_CSV_IDLE_CLOSE_SECS: u64 = 10;
/// CSV 空闲关闭时长上限（秒）
pub const MAX_CSV_IDLE_CLOSE_SECS: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Gh3220LinkKind {
    #[default]
    Ble,
    Tcp,
}

impl std::fmt::Display for Gh3220LinkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gh3220LinkKind::Ble => write!(f, "ble"),
            Gh3220LinkKind::Tcp => write!(f, "tcp"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Gh3220State {
    #[default]
    Disconnected,
    Connecting,
    Ready,
    Capturing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220LinkConfig {
    #[serde(default)]
    pub kind: Gh3220LinkKind,
    /// BLE 地址，或 TCP 调试通道的 `host:port`
    pub address: String,
    #[serde(default)]
    pub write_uuid: Option<String>,
    #[serde(default)]
    pub notify_uuid: Option<String>,
    #[serde(default)]
    pub mtu: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220Status {
    pub state: Gh3220State,
    pub link: Option<Gh3220LinkKind>,
    pub address: Option<String>,
    pub name: Option<String>,
    pub error: Option<String>,
    pub capturing: bool,
    pub chunk_size: usize,
}

impl Default for Gh3220Status {
    fn default() -> Self {
        Self {
            state: Gh3220State::Disconnected,
            link: None,
            address: None,
            name: None,
            error: None,
            capturing: false,
            chunk_size: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220DeviceInfo {
    pub fw_version: String,
    pub protocol_version: String,
    pub chip_version: String,
    pub driver_version: String,
    pub max_len: u8,
    pub opened_func: u32,
    pub link_ok: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220Record {
    pub function_id: u8,
    pub frame_id: u8,
    pub raw: Vec<u32>,
    pub agc: Vec<u32>,
    pub gs: Option<[i16; 3]>,
    pub gyro: Option<[i16; 3]>,
    pub cap: Option<[u32; 4]>,
    pub temp: Option<[u32; 4]>,
    pub flags: [u32; 5],
    pub algo: Vec<(u8, i32)>,
    pub ts_ms: u64,
}

impl Gh3220Record {
    pub fn from_record(rec: &gh_uprotocol::Record, ts_ms: u64) -> Self {
        Self {
            function_id: rec.function_id,
            frame_id: rec.frame_id,
            raw: rec.raw.clone(),
            agc: rec.agc.clone(),
            gs: rec.gs,
            gyro: rec.gyro,
            cap: rec.cap,
            temp: rec.temp,
            flags: rec.flags,
            algo: rec.algo.clone(),
            ts_ms,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220Stats {
    pub frames: u64,
    pub crc_errors: u64,
    pub resyncs: u64,
    pub bad_len: u64,
    pub packets: u64,
    pub records: u64,
    pub bad_packets: u64,
    pub splice_fragments: u64,
    pub splice_aborts: u64,
    pub lost_frames: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220DataEvent {
    pub seq: u64,
    pub records: Vec<Gh3220Record>,
    pub dropped: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220EventPayload {
    pub event_id: u16,
    pub report_id: u8,
    pub acked: bool,
    pub ts_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220ErrorEvent {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220StatusEvent {
    pub status: Gh3220Status,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220DebugStatus {
    pub kind: u8,
    pub bytes: u8,
    pub data: [u8; 4],
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220CaptureConfig {
    /// 启动采集前自动下发 WORK_MODE + START
    #[serde(default)]
    pub auto_start: bool,
    /// 停止采集时自动下发 STOP
    #[serde(default)]
    pub auto_stop: bool,
    #[serde(default)]
    pub mode: u8,
    #[serde(default)]
    pub slot_en: u8,
    #[serde(default)]
    pub func_mask: u32,
    /// 覆盖通道数（设备包头不含通道总数）
    #[serde(default)]
    pub channels_override: Option<usize>,
    /// 是否写 CSV；缺省跟随偏好设置
    #[serde(default)]
    pub csv_enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3220CaptureStatus {
    pub active: bool,
    pub started_at_ms: u64,
    pub records: u64,
    pub csv_enabled: bool,
    pub files: Vec<String>,
    pub channels_override: Option<usize>,
}

/// CSV 写入配置（由偏好设置转换）
#[derive(Debug, Clone)]
pub struct Gh3220CsvConfig {
    pub enabled: bool,
    pub output_dir: String,
    pub idle_close_secs: u64,
}

impl Default for Gh3220CsvConfig {
    fn default() -> Self {
        let output_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .map(|d| d.join("data"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("data"));

        Self {
            enabled: true,
            output_dir,
            idle_close_secs: DEFAULT_CSV_IDLE_CLOSE_SECS,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Gh3220Error {
    #[error("未连接 GH3220 设备")]
    NotConnected,
    #[error("命令 0x{cmd:02X} 超时")]
    Timeout { cmd: u8 },
    #[error("设备忙：已有未完成命令")]
    Busy,
    #[error("链路已断开")]
    Disconnected,
    #[error("调试 TCP 通道未启用")]
    DebugTcpDisabled,
    #[error("参数错误: {0}")]
    Invalid(String),
    #[error("协议错误: {0}")]
    Protocol(String),
    #[error("BLE 错误: {0}")]
    Ble(String),
    #[error("设备返回状态 0x{code:02X}")]
    Status { code: u8 },
    #[error("IO 错误: {0}")]
    Io(String),
}

impl Gh3220Error {
    pub fn into_combridge(self) -> ComBridgeError {
        match self {
            Gh3220Error::NotConnected
            | Gh3220Error::Disconnected
            | Gh3220Error::Busy
            | Gh3220Error::Timeout { .. }
            | Gh3220Error::Status { .. } => ComBridgeError::protocol(self.to_string()),
            Gh3220Error::DebugTcpDisabled | Gh3220Error::Invalid(_) => {
                ComBridgeError::config(self.to_string())
            }
            Gh3220Error::Ble(_) => ComBridgeError::ble(self.to_string()),
            Gh3220Error::Protocol(_) => ComBridgeError::protocol(self.to_string()),
            Gh3220Error::Io(_) => ComBridgeError::io(self.to_string()),
        }
    }
}

impl From<ComBridgeError> for Gh3220Error {
    fn from(value: ComBridgeError) -> Self {
        match value {
            ComBridgeError::BleError(msg) => Gh3220Error::Ble(msg),
            other => Gh3220Error::Protocol(other.to_string()),
        }
    }
}

impl From<gh_uprotocol::FrameError> for Gh3220Error {
    fn from(value: gh_uprotocol::FrameError) -> Self {
        Gh3220Error::Protocol(value.to_string())
    }
}

impl From<gh_uprotocol::DataError> for Gh3220Error {
    fn from(value: gh_uprotocol::DataError) -> Self {
        Gh3220Error::Protocol(value.to_string())
    }
}

/// 将设备状态码格式化为可读文案（0 成功 / 1 失败 / 0xFF 不支持）
pub fn status_text(code: u8) -> &'static str {
    match code {
        0 => "ok",
        1 => "fail",
        0xFF => "unsupported",
        _ => "unknown",
    }
}

/// 统一解析十六进制入参：允许空格、逗号、`-` 与 `0x` 前缀
pub fn parse_hex(input: &str) -> Result<Vec<u8>, Gh3220Error> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '-')
        .collect();
    let cleaned = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
        .unwrap_or(&cleaned);
    if !cleaned.len().is_multiple_of(2) {
        return Err(Gh3220Error::Invalid("十六进制长度必须为偶数".to_string()));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| Gh3220Error::Invalid(format!("十六进制解析失败: {e}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_accepts_common_formats() {
        assert_eq!(parse_hex("AA11A001").unwrap(), vec![0xAA, 0x11, 0xA0, 0x01]);
        assert_eq!(
            parse_hex("0xAA 11, A0-01").unwrap(),
            vec![0xAA, 0x11, 0xA0, 0x01]
        );
        assert!(parse_hex("AA1").is_err());
        assert!(parse_hex("GG").is_err());
    }

    #[test]
    fn status_text_matches_protocol() {
        assert_eq!(status_text(0), "ok");
        assert_eq!(status_text(1), "fail");
        assert_eq!(status_text(0xFF), "unsupported");
    }
}
