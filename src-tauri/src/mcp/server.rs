//! MCP Server 定义与工具注册。
//!
//! P0：`combridge_about`。
//! P1：串口域 —— `combridge_ports` / `combridge_serial_open` / `combridge_serial_close` /
//! `combridge_serial_write` / `combridge_serial_read` / `combridge_serial_expect`。
//! P2：设备与 BLE —— `combridge_devices` / `combridge_ble_scan` / `combridge_ble_connect` /
//! `combridge_ble_gatt`。
//! P3：GH3036 —— `combridge_gh3036_rpc` / `combridge_gh3036_factory_test`。
//! P4：其余业务域 —— `combridge_waveform` / `combridge_protocol` / `combridge_ota`，
//! 以及 `combridge_about` 的 `action=logs`。
//!
//! 设计约定（docs/mcp-server-plan.md §5、§6）：
//! - 工具名 `combridge_` / `serial_` 前缀，域内不做 1:1 命令映射
//! - 独占资源操作必须持有效租约（`lease_id`），PORT_BUSY 时返回占用者信息
//! - 读类工具（ports/read/expect）不需要租约：数据经 hub 对所有人可见
//! - 注解：只读工具标 `read_only_hint = true`，写/关设备标 `destructive_hint = true`
//! - L3（OTA / 产测）在标准档一律拒绝，待 P5 的 elicitation 落地后改为双确认

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::ServerHandler;
use rmcp::{schemars, tool, tool_handler, tool_router};
use rmcp::{Peer, RoleServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::audit;
use super::ctx::McpContext;
use super::lease::{AcquireError, DEFAULT_TTL_SECS};
use super::ratelimit::Reject;
use crate::device::serial::serial_config::{
    BaudRate, DataBits, FlowControl, Parity, SerialPortConfig, StopBits,
};
use crate::gh3036::{ChannelConfig, ChannelType, FactoryTestStatus, FactoryTestStep};
use crate::ota::{
    OtaDeviceInfo, OtaPackageInfo, OtaProtocolDescriptor, OtaStage, OtaStartOptions, OtaState,
    OtaStatusDto,
};
use crate::protocol::{PluginInfo, PluginState};
use crate::waveform::{ParserConfig, ParserType, WaveformBufferConfig};

/// `combridge_about` 的入参。
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AboutRequest {
    /// 动作：`summary`（默认，服务概况）| `logs`（读日志尾部，用于自查失败原因）
    #[serde(default)]
    pub action: Option<String>,
    /// `logs`：返回行数，默认 100，上限 500
    #[serde(default)]
    pub lines: Option<usize>,
}

/// `combridge_about` 的返回结构。
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AboutResponse {
    /// 实际执行的 action
    pub action: String,
    /// ComBridge 应用版本
    pub app_version: String,
    /// 安全档位："dev"（研发档）| "standard"（标准档）
    pub tier: String,
    /// 服务启动时间（毫秒时间戳）
    pub started_at: u64,
    /// 当前已连接设备快照
    pub connected_devices: Vec<serde_json::Value>,
    /// 已实现的工具域
    pub tool_domains: Vec<String>,
    /// 尚未实现的工具域（调用无效，仅作能力预告）
    pub planned_tool_domains: Vec<String>,
    /// `logs`：读取的日志文件（绝对路径）
    pub log_file: Option<String>,
    /// `logs`：日志尾部内容（每项一行）
    pub log_lines: Vec<String>,
}

// ---------------------------------------------------------------------------
// combridge_ports
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PortEntry {
    /// 系统端口名（如 "COM3"）
    pub name: String,
    /// 端口类型（usb / native 等）
    pub port_type: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    /// 当前是否已打开（无论由 GUI 还是 MCP）
    pub open: bool,
    /// 当前租约（若被 MCP 客户端持有）
    pub lease: Option<LeaseView>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LeaseView {
    pub lease_id: String,
    pub owner: String,
    /// 过期时间（毫秒时间戳）
    pub expires_at: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PortsResponse {
    pub ports: Vec<PortEntry>,
}

// ---------------------------------------------------------------------------
// combridge_serial_open
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SerialOpenRequest {
    /// 要打开的端口名（来自 combridge_ports，如 "COM3"）
    pub port_name: String,
    /// 波特率，默认 115200
    #[serde(default = "d_baud")]
    pub baud_rate: u32,
    /// 数据位 5/6/7/8，默认 8
    #[serde(default = "d_data_bits")]
    pub data_bits: u32,
    /// 校验位："none" | "odd" | "even"，默认 none
    #[serde(default)]
    pub parity: String,
    /// 停止位 1/2，默认 1
    #[serde(default = "d_stop_bits")]
    pub stop_bits: u32,
    /// 流控："none" | "software" | "hardware"，默认 none
    #[serde(default)]
    pub flow_control: String,
    /// 客户端身份标识（如 "claude-code"），用于租约展示
    #[serde(default = "d_owner")]
    pub owner: String,
    /// 租约 TTL（秒），默认 3600，上限 86400
    #[serde(default = "d_ttl")]
    pub ttl_s: u64,
    /// 端口租约被其他 AI 客户端持有时是否强制抢占
    #[serde(default)]
    pub force: bool,
}

fn d_baud() -> u32 {
    115200
}
fn d_data_bits() -> u32 {
    8
}
fn d_stop_bits() -> u32 {
    1
}
fn d_owner() -> String {
    "ai-client".to_string()
}
fn d_ttl() -> u64 {
    DEFAULT_TTL_SECS
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SerialOpenResponse {
    /// 租约 ID，后续 write/close 必须携带
    pub lease_id: String,
    pub owner: String,
    /// 租约过期时间（毫秒时间戳）
    pub expires_at: u64,
    /// 当前数据游标（combridge_serial_read 的 since 起点）
    pub cursor: u64,
    /// 端口此前已被打开（GUI 或先前会话）；此模式下读可用，close 会被拒绝
    pub already_open: bool,
}

// ---------------------------------------------------------------------------
// combridge_serial_close
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SerialCloseRequest {
    pub port_name: String,
    /// combridge_serial_open 返回的租约 ID
    pub lease_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SerialCloseResponse {
    pub closed: bool,
    pub port_name: String,
}

// ---------------------------------------------------------------------------
// combridge_serial_write
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SerialWriteRequest {
    pub port_name: String,
    /// combridge_serial_open 返回的租约 ID
    pub lease_id: String,
    /// 要发送的内容
    pub data: String,
    /// "text"（原样 UTF-8）| "hex"（如 "AA BB 01"，空格/冒号/逗号可省略）
    #[serde(default = "d_text")]
    pub encoding: String,
    /// 末尾追加 "\r\n"（AT 命令常用）
    #[serde(default)]
    pub append_crlf: bool,
}

fn d_text() -> String {
    "text".to_string()
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SerialWriteResponse {
    pub bytes_written: usize,
    pub port_name: String,
}

// ---------------------------------------------------------------------------
// combridge_serial_read
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SerialReadRequest {
    pub port_name: String,
    /// 起始游标（上次返回的 next_cursor）；省略则从缓冲区最旧数据开始。
    /// 注意缓冲上限 1MB，久未读取的数据会被丢弃并置 truncated。
    #[serde(default)]
    pub since: Option<u64>,
    /// "text"（UTF-8 lossy）| "hex"
    #[serde(default = "d_text")]
    pub encoding: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SerialReadResponse {
    /// 接收数据（按 encoding 编码）
    pub data: String,
    pub encoding: String,
    /// 下一次读取应使用的游标
    pub next_cursor: u64,
    /// 请求区间内有数据因缓冲上限被丢弃
    pub truncated: bool,
    /// 内部事件消费滞后，区间内可能有缺口
    pub lagged: bool,
    /// 自端口打开以来累计接收字节数
    pub total_written: u64,
}

// ---------------------------------------------------------------------------
// combridge_serial_expect
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SerialExpectRequest {
    pub port_name: String,
    /// 等待的匹配内容：regex=true 时为正则，否则为子串
    pub pattern: String,
    /// 将 pattern 按正则解释
    #[serde(default)]
    pub regex: bool,
    /// 忽略大小写
    #[serde(default)]
    pub case_insensitive: bool,
    /// 最长等待毫秒数，默认 10000
    #[serde(default = "d_expect_timeout")]
    pub timeout_ms: u32,
    /// 起始游标；默认从"现在"开始（只看新到达的数据）。
    /// 若想匹配等待前已收到的数据，请传 combridge_serial_read 得到的 since。
    #[serde(default)]
    pub since: Option<u64>,
}

fn d_expect_timeout() -> u32 {
    10_000
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SerialExpectResponse {
    /// 是否在超时前匹配到
    pub matched: bool,
    /// 命中的片段
    pub matched_text: String,
    /// 等待期间累计的全部文本（含错误上下文）
    pub buffer: String,
    pub next_cursor: u64,
    pub elapsed_ms: u64,
    pub truncated: bool,
    pub lagged: bool,
    /// 等待期间端口断开
    pub disconnected: bool,
}

// ---------------------------------------------------------------------------
// combridge_devices
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LeaseEntry {
    pub resource: String,
    pub owner: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BleConnectionView {
    pub address: String,
    pub name: Option<String>,
    pub is_connected: bool,
    /// 已订阅的特征 UUID
    pub subscriptions: Vec<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DevicesResponse {
    /// BLE 后端模式："native"（系统蓝牙）| "at"（AT 指令）
    pub ble_mode: String,
    /// BLE 后端是否已配置；false 时 BLE 工具不可用
    pub ble_configured: bool,
    pub ble_connections: Vec<BleConnectionView>,
    /// App 状态中的已连接设备快照
    pub app_devices: Vec<serde_json::Value>,
    /// 当前全部有效租约（串口 / BLE）
    pub active_leases: Vec<LeaseEntry>,
}

// ---------------------------------------------------------------------------
// combridge_ble_scan
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BleScanRequest {
    /// 扫描时长（毫秒），默认 3000。扫描为阻塞调用，到时间返回结果。
    #[serde(default = "d_scan_ms")]
    pub duration_ms: u64,
    /// 置 true 则停止当前扫描并返回已发现列表（忽略 duration_ms）
    #[serde(default)]
    pub stop: bool,
}

fn d_scan_ms() -> u64 {
    3000
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BleDeviceView {
    pub address: String,
    pub name: Option<String>,
    pub rssi: Option<i16>,
    pub is_connectable: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BleScanResponse {
    pub devices: Vec<BleDeviceView>,
    /// 本次是否为"停止扫描"调用（stop=true）
    pub stopped: bool,
}

// ---------------------------------------------------------------------------
// combridge_ble_connect
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BleConnectRequest {
    /// "connect" | "disconnect" | "list"
    pub action: String,
    /// 设备地址（connect / disconnect 必填）
    #[serde(default)]
    pub address: String,
    /// disconnect 需携带 combridge_ble_connect 返回的 lease_id
    #[serde(default)]
    pub lease_id: String,
    #[serde(default = "d_owner")]
    pub owner: String,
    #[serde(default = "d_ttl")]
    pub ttl_s: u64,
    /// 连接租约被其他客户端持有时是否抢占
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BleConnectResponse {
    pub action: String,
    pub address: String,
    pub connected: bool,
    /// connect 返回的租约（后续 write / subscribe / disconnect 需携带）
    pub lease_id: Option<String>,
    pub subscriptions: Vec<String>,
    pub connections: Vec<BleConnectionView>,
}

// ---------------------------------------------------------------------------
// combridge_ble_gatt
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BleGattRequest {
    /// "discover_services" | "discover_characteristics" | "read" | "write" |
    /// "subscribe" | "unsubscribe" | "notifications" | "mtu" | "rssi"
    pub action: String,
    pub address: String,
    /// 特征 UUID（read/write/subscribe/unsubscribe/notifications 必填）
    #[serde(default)]
    pub char_uuid: String,
    /// 服务 UUID（discover_characteristics 必填）
    #[serde(default)]
    pub service_uuid: String,
    /// write 的数据
    #[serde(default)]
    pub data: String,
    #[serde(default = "d_text")]
    pub encoding: String,
    /// write 时使用无响应写（write without response）
    #[serde(default)]
    pub without_response: bool,
    /// write / subscribe / unsubscribe / mtu 需携带 ble_connect 返回的 lease_id
    #[serde(default)]
    pub lease_id: String,
    /// notifications 的起始游标（上次返回的 next_cursor）；省略读全部缓冲
    #[serde(default)]
    pub since: Option<u64>,
    /// 期望 MTU（action="mtu"，默认 247）
    #[serde(default)]
    pub mtu: u16,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CharView {
    pub uuid: String,
    pub service_uuid: String,
    /// 属性摘要，如 "read,write,notify"
    pub properties: String,
    pub subscribed: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ServiceView {
    pub uuid: String,
    pub primary: bool,
    pub characteristics: Vec<CharView>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BleGattResponse {
    pub action: String,
    /// read / notifications 的数据（按 encoding 编码）
    pub value: Option<String>,
    pub encoding: Option<String>,
    pub next_cursor: Option<u64>,
    pub truncated: Option<bool>,
    pub services: Option<Vec<ServiceView>>,
    pub characteristics: Option<Vec<CharView>>,
    pub mtu: Option<u16>,
    pub rssi: Option<i16>,
    pub subscriptions: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// 服务本体
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ComBridgeMcp {
    ctx: Arc<McpContext>,
}

impl ComBridgeMcp {
    pub fn new(ctx: Arc<McpContext>) -> Self {
        Self { ctx }
    }
}

/// 统一把内部错误转成 MCP 错误。
/// 工具返回值：成功为结构化 JSON，失败为 `isError: true` 的工具结果。
///
/// MCP 规范区分两种失败：协议错误（`Err(ErrorData)`，客户端渲染为不透明提示、
/// 模型读不到错误信息）与工具级错误（`isError: true`，错误内容直接进入模型的
/// 上下文，可据此自我纠正）。本服务一律使用后者。
type ToolR<T> = Result<Json<T>, CallToolResult>;

/// 构造工具级错误，字段与 docs/mcp-server-plan.md §6.6 一致。
///
/// `hint` 是模型的恢复依据（下一步该调用什么），`retryable` 表示原样重试是否有意义。
fn tool_err_full(
    code: &'static str,
    message: impl Into<String>,
    hint: Option<&str>,
    retryable: bool,
) -> CallToolResult {
    let payload = serde_json::json!({
        "code": code,
        "message": message.into(),
        "hint": hint,
        "retryable": retryable,
    });
    CallToolResult::error(vec![ContentBlock::text(payload.to_string())])
}

/// 无恢复提示的简写形式。
fn tool_err(code: &'static str, message: impl Into<String>) -> CallToolResult {
    tool_err_full(code, message, None, false)
}

fn mcp_err(msg: impl Into<String>) -> CallToolResult {
    tool_err("ERROR", msg)
}

fn map_baud(b: u32) -> Result<BaudRate, CallToolResult> {
    Ok(match b {
        1200 => BaudRate::B1200,
        2400 => BaudRate::B2400,
        4800 => BaudRate::B4800,
        9600 => BaudRate::B9600,
        19200 => BaudRate::B19200,
        38400 => BaudRate::B38400,
        57600 => BaudRate::B57600,
        115200 => BaudRate::B115200,
        230400 => BaudRate::B230400,
        460800 => BaudRate::B460800,
        921600 => BaudRate::B921600,
        _ => {
            return Err(mcp_err(format!(
                "不支持的波特率: {b}（支持 1200~921600 标准值）"
            )))
        }
    })
}

fn map_data_bits(n: u32) -> Result<DataBits, CallToolResult> {
    Ok(match n {
        5 => DataBits::Five,
        6 => DataBits::Six,
        7 => DataBits::Seven,
        8 => DataBits::Eight,
        _ => return Err(mcp_err(format!("不支持的数据位: {n}（5/6/7/8）"))),
    })
}

fn map_parity(s: &str) -> Result<Parity, CallToolResult> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "none" | "" => Parity::None,
        "odd" => Parity::Odd,
        "even" => Parity::Even,
        other => return Err(mcp_err(format!("不支持的校验位: {other}（none/odd/even）"))),
    })
}

fn map_stop_bits(n: u32) -> Result<StopBits, CallToolResult> {
    Ok(match n {
        1 => StopBits::One,
        2 => StopBits::Two,
        _ => return Err(mcp_err(format!("不支持的停止位: {n}（1/2）"))),
    })
}

fn map_flow(s: &str) -> Result<FlowControl, CallToolResult> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "none" | "" => FlowControl::None,
        "software" => FlowControl::Software,
        "hardware" => FlowControl::Hardware,
        other => {
            return Err(mcp_err(format!(
                "不支持的流控: {other}（none/software/hardware）"
            )))
        }
    })
}

fn encode_bytes(bytes: &[u8], encoding: &str) -> Result<String, CallToolResult> {
    match encoding {
        "text" => Ok(String::from_utf8_lossy(bytes).into_owned()),
        "hex" => Ok(bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")),
        other => Err(mcp_err(format!("未知编码: {other}（支持 text | hex）"))),
    }
}

fn decode_bytes(data: &str, encoding: &str) -> Result<Vec<u8>, CallToolResult> {
    match encoding {
        "text" => Ok(data.as_bytes().to_vec()),
        "hex" => {
            let cleaned: String = data
                .chars()
                .filter(|c| !c.is_whitespace() && *c != ':' && *c != ',')
                .collect();
            if !cleaned.len().is_multiple_of(2) {
                return Err(mcp_err("hex 数据长度必须为偶数"));
            }
            (0..cleaned.len() / 2)
                .map(|i| {
                    u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16)
                        .map_err(|e| mcp_err(format!("hex 解析失败（位置 {}）: {}", i * 2, e)))
                })
                .collect()
        }
        other => Err(mcp_err(format!("未知编码: {other}（支持 text | hex）"))),
    }
}

fn lease_to_view(lease: &super::lease::Lease) -> LeaseView {
    LeaseView {
        lease_id: lease.lease_id.clone(),
        owner: lease.owner.clone(),
        expires_at: lease.expires_at,
    }
}

fn to_char_view(c: &crate::device::ble::BleCharacteristic) -> CharView {
    CharView {
        uuid: c.uuid.clone(),
        service_uuid: c.service_uuid.clone(),
        properties: c.properties.to_string(),
        subscribed: c.subscribed,
    }
}

fn to_service_view(s: &crate::device::ble::BleService) -> ServiceView {
    ServiceView {
        uuid: s.uuid.clone(),
        primary: s.primary,
        characteristics: s.characteristics.iter().map(to_char_view).collect(),
    }
}

/// 等待匹配用的文本匹配器：输入累计文本，命中返回片段。
type Matcher = Box<dyn Fn(&str) -> Option<String> + Send>;

fn to_channel_view(c: &ChannelConfig) -> ChannelView {
    ChannelView {
        channel_type: match c.channel_type {
            ChannelType::Serial => "serial",
            ChannelType::Ble => "ble",
        }
        .to_string(),
        device_id: c.device_id.clone(),
        characteristic_uuid: c.characteristic_uuid.clone(),
    }
}

/// 连续十六进制（无分隔），用于 UUID 等定长字段。
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

/// 全部为可打印 ASCII 时返回文本，否则 None（二进制载荷不给模型塞乱码）。
fn printable_text(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let printable = bytes
        .iter()
        .all(|b| *b == b'\r' || *b == b'\n' || *b == b'\t' || (0x20..0x7F).contains(b));
    if printable {
        Some(String::from_utf8_lossy(bytes).trim_end().to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// combridge_gh3036_rpc / combridge_gh3036_factory_test
// ---------------------------------------------------------------------------

/// GH3036 是单芯片会话，MCP 侧用固定资源名做租约。
pub const GH3036_RESOURCE: &str = "gh3036";

/// 串口 / BLE 通道的只读视图。
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ChannelView {
    /// `serial` | `ble`
    pub channel_type: String,
    /// 串口名（`COM3`）或 BLE 地址
    pub device_id: String,
    /// BLE 写特征 UUID（串口通道为 None）
    pub characteristic_uuid: Option<String>,
}

/// RPC 命令目录项。
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct RpcCommandView {
    /// 命令键，调用 `action=rpc` 时填此值
    pub key: String,
    /// 命令名称
    pub name: String,
    /// 命令说明
    pub description: String,
    /// 参数说明（按顺序传入 `params`）
    pub params: Vec<String>,
}

/// 固件版本类型项。
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct VersionTypeView {
    /// 版本类型值，作为 `action=rpc`、`command_key=V` 的第一个参数
    pub type_value: u8,
    pub name: String,
    pub description: String,
}

/// `combridge_gh3036_rpc` 入参。
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Gh3036RpcRequest {
    /// 动作：`status` | `init` | `set_channel` | `catalog` | `rpc`
    pub action: String,
    /// 通道方向（`set_channel` 用）：`tx` | `rx`，省略表示同时配置
    #[serde(default)]
    pub direction: Option<String>,
    /// 通道类型：`serial` | `ble`
    #[serde(default)]
    pub channel_type: Option<String>,
    /// 串口名或 BLE 地址
    #[serde(default)]
    pub device_id: Option<String>,
    /// BLE 写特征 UUID（`channel_type=ble` 时通常需要）
    #[serde(default)]
    pub characteristic_uuid: Option<String>,
    /// RPC 命令键（`action=rpc` 必填），取值见 `action=catalog`
    #[serde(default)]
    pub command_key: Option<String>,
    /// RPC 参数，按命令定义顺序传入
    #[serde(default)]
    pub params: Vec<String>,
    /// 客户端身份（用于租约展示），默认 `mcp-client`
    #[serde(default)]
    pub owner: Option<String>,
    /// GH3036 租约 ID；省略时自动获取
    #[serde(default)]
    pub lease_id: Option<String>,
    /// 租约被其他客户端持有时是否抢占
    #[serde(default)]
    pub force: bool,
    /// 租约 TTL（秒），默认 3600
    #[serde(default)]
    pub ttl_s: Option<u64>,
}

/// `combridge_gh3036_rpc` 返回体。未涉及的字段为 None。
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Gh3036RpcResponse {
    /// 实际执行的 action
    pub action: String,
    /// GH3036 模块是否已初始化
    pub initialized: bool,
    /// 当前 GH3036 租约（后续调用可回传）
    pub lease_id: Option<String>,
    /// TX 通道（设备 → 芯片）
    pub tx_channel: Option<ChannelView>,
    /// RX 通道（芯片 → 设备）
    pub rx_channel: Option<ChannelView>,
    /// 产测当前状态（`status` action 返回）
    pub factory_status: Option<String>,
    /// 产测当前步骤
    pub factory_step: Option<String>,
    /// RPC 命令目录（`catalog` action 返回）
    pub commands: Vec<RpcCommandView>,
    /// 版本类型目录（`catalog` action 返回）
    pub version_types: Vec<VersionTypeView>,
    /// RPC 返回的原始字节数
    pub bytes: Option<usize>,
    /// RPC 返回数据（十六进制）
    pub hex: Option<String>,
    /// RPC 返回数据（全部为可打印 ASCII 时的文本形式）
    pub text: Option<String>,
}

/// 产测配置校验结果。
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ConfigValidationView {
    pub is_valid: bool,
    pub errors: Vec<String>,
    /// 已加载的配置文件（未找到的为 None）
    pub base_noise_config: Option<String>,
    pub ppg_noise_config: Option<String>,
    pub lpctr_config: Option<String>,
    pub lplctr_config: Option<String>,
}

/// `combridge_gh3036_factory_test` 入参。
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FactoryTestRequest {
    /// 动作：`run` | `status` | `continue` | `stop` | `validate_config` | `threshold_get`
    pub action: String,
    /// 卡控配置目录（`run` 可带上，省略则沿用已设置的目录）
    #[serde(default)]
    pub config_dir: Option<String>,
    /// `run` 的最长等待秒数，默认 180，上限 600；超时降级返回 job_id
    #[serde(default)]
    pub max_wait_s: Option<u64>,
    /// 客户端身份（用于租约展示），默认 `mcp-client`
    #[serde(default)]
    pub owner: Option<String>,
    /// GH3036 租约 ID；省略时自动获取
    #[serde(default)]
    pub lease_id: Option<String>,
    /// 租约被其他客户端持有时是否抢占
    #[serde(default)]
    pub force: bool,
    /// 租约 TTL（秒），默认 3600
    #[serde(default)]
    pub ttl_s: Option<u64>,
}

/// `validate_config` / `threshold_get` 的附加返回。
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FactoryTestExtraResponse {
    pub job_id: String,
    pub status: String,
    pub step: String,
    pub lease_id: Option<String>,
    /// `validate_config` 的校验结果
    pub config_validation: Option<ConfigValidationView>,
    /// `threshold_get` 返回的卡控配置 YAML
    pub threshold_yaml: Option<String>,
}

// ---------------------------------------------------------------------------
// combridge_waveform（P4）
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WaveformRequest {
    /// 动作：list | create | remove | configure_parser | parse | read | clear | status
    pub action: String,
    /// 缓冲 ID（除 list 外均必填）
    #[serde(default)]
    pub buffer_id: Option<String>,
    /// create：环形缓冲容量（保留最近 N 行），默认 1000
    #[serde(default)]
    pub capacity: Option<usize>,
    /// create：列名，默认 CH0..CH4
    #[serde(default)]
    pub column_names: Option<Vec<String>>,
    /// configure_parser：解析器配置
    #[serde(default)]
    pub parser: Option<ParserSpec>,
    /// parse：待解析文本，可含多行（每行一条样本，如 "1.2,3.4,5.6"）
    #[serde(default)]
    pub data: Option<String>,
    /// read：读取最近多少行，默认 200，上限 5000
    #[serde(default)]
    pub rows: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ParserSpec {
    /// "delimiter"（按分隔符切分）| "regex"（按正则捕获组）
    pub parser_type: String,
    /// delimiter 模式的分隔符，默认 ","
    #[serde(default)]
    pub delimiter: Option<String>,
    /// regex 模式的表达式（每行首个匹配的捕获组作为一行数据）
    #[serde(default)]
    pub pattern: Option<String>,
    /// 列名，省略则沿用缓冲已有列名
    #[serde(default)]
    pub column_names: Option<Vec<String>>,
    /// 去除首尾空白，默认 true
    #[serde(default)]
    pub trim_whitespace: Option<bool>,
}

#[derive(Debug, Default, Serialize, schemars::JsonSchema)]
pub struct WaveformResponse {
    pub action: String,
    /// list：全部缓冲 ID
    pub buffers: Vec<String>,
    pub buffer_id: Option<String>,
    /// 列名（与 rows 每行长度对齐）
    pub columns: Vec<String>,
    /// read：最近 N 行数据
    pub rows: Vec<Vec<f64>>,
    /// 缓冲内当前总行数
    pub row_count: usize,
    pub column_count: usize,
    /// 环形缓冲容量
    pub capacity: Option<usize>,
    /// 已配置的解析器类型：delimiter | regex
    pub parser_type: Option<String>,
    /// parse：成功写入的行数
    pub accepted: Option<usize>,
    /// parse：解析失败被丢弃的行数
    pub rejected: Option<usize>,
    /// parse：前若干条失败原因（最多 5 条）
    pub errors: Vec<String>,
}

impl WaveformResponse {
    fn new(action: &str) -> Self {
        Self {
            action: action.to_string(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// combridge_protocol（P4）
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProtocolRequest {
    /// 动作：list | get | load | unload | enable | disable | bind | unbind | bound
    pub action: String,
    /// 插件 ID（除 list 外均必填）
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// load：Lua 脚本绝对路径
    #[serde(default)]
    pub path: Option<String>,
    /// bind / unbind / bound：设备 ID
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PluginView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    /// Lua 脚本绝对路径
    pub path: String,
    /// 状态：unloaded | loaded | enabled | disabled | error
    pub state: String,
    /// 插件声明的 hook 列表
    pub hooks: Vec<String>,
    /// 已绑定设备 ID
    pub bound_devices: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Default, Serialize, schemars::JsonSchema)]
pub struct ProtocolResponse {
    pub action: String,
    /// list / bound：插件列表
    pub plugins: Vec<PluginView>,
    /// get / load：单个插件详情
    pub plugin: Option<PluginView>,
    /// 状态变更类动作的结果说明
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// combridge_ota（P4，L3 高危）
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OtaRequest {
    /// 动作：protocols | validate | preflight | start | status | cancel
    pub action: String,
    /// 协议 ID（省略时用默认协议），见 action=protocols
    #[serde(default)]
    pub protocol: Option<String>,
    /// 目标设备 BLE 地址（preflight / start 必填，如 "AA:BB:CC:DD:EE:FF"）
    #[serde(default)]
    pub address: Option<String>,
    /// 固件包路径（.zip 包或裸固件镜像；validate / start 必填）
    #[serde(default)]
    pub firmware_path: Option<String>,
    /// start：dry-run 返回的 confirm_token。**不带它调用 start 不会烧录**，只做前置检查
    #[serde(default)]
    pub confirm: Option<String>,
    /// start：dry-run 时是否预检设备（连设备读版本），默认 true
    #[serde(default)]
    pub check_device: Option<bool>,
    /// 跳过设备侧二次确认（少数协议支持），默认 false
    #[serde(default)]
    pub force_confirm: Option<bool>,
    /// 仅升级（不清除原有配置），默认 false
    #[serde(default)]
    pub upgrade_only: Option<bool>,
    /// Goodix 路由：auto | normal | fastService
    #[serde(default)]
    pub route: Option<String>,
    /// Goodix 传输模式：auto | normal | fast
    #[serde(default)]
    pub transfer_mode: Option<String>,
    /// Goodix Flash 模式：dual（默认双区）| single（原地升级）
    #[serde(default)]
    pub bank_mode: Option<String>,
    /// Goodix 显式写入地址（十六进制字符串）
    #[serde(default)]
    pub target_address: Option<String>,
    /// 完成后是否立即复位运行，默认 true
    #[serde(default)]
    pub run_immediately: Option<bool>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct OtaProtocolView {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct OtaKeyValueView {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct OtaPackageView {
    pub path: String,
    pub file_name: String,
    pub package_size: u64,
    pub protocol: String,
    pub image_name: String,
    pub image_size: u64,
    pub image_sha256: String,
    /// Goodix 双区写入地址（按加载地址 + 4K 对齐的镜像大小计算）
    pub dual_bank_address: Option<String>,
    /// 协议自定义展示字段（SMP 的应用名 / 目标板 / SoC / MCUboot 版本等）
    pub fields: Vec<OtaKeyValueView>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct OtaDeviceView {
    pub address: String,
    pub protocol: String,
    pub name: Option<String>,
    pub mode: Option<String>,
    pub mtu: Option<u16>,
    pub version: Option<String>,
    pub fields: Vec<OtaKeyValueView>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct OtaStatusView {
    /// idle | running | success | failed | cancelled
    pub state: String,
    /// idle | unpacking | validating | preparing | transferring | activating | rebooting | completed | failed
    pub stage: String,
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

/// dry-run 的执行计划。确认前模型可据此判断是否该问用户。
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct OtaPlanView {
    pub protocol: String,
    pub target: String,
    pub firmware_path: String,
    pub image_name: String,
    pub image_size: u64,
    pub image_sha256: String,
    /// 传输耗时粗估（秒）= 8s 固定开销 + 镜像大小 / 12KB/s
    pub est_seconds: u64,
    /// 目标设备信息（预检成功时填充）
    pub device: Option<OtaDeviceView>,
}

#[derive(Debug, Default, Serialize, schemars::JsonSchema)]
pub struct OtaResponse {
    pub action: String,
    /// 本次调用是否只做前置检查（true = 未烧录）
    pub dry_run: Option<bool>,
    /// action=protocols
    pub protocols: Vec<OtaProtocolView>,
    /// action=validate / start 的固件包信息
    pub package: Option<OtaPackageView>,
    /// action=preflight 的设备信息
    pub device: Option<OtaDeviceView>,
    /// action=start（dry-run）的执行计划
    pub plan: Option<OtaPlanView>,
    /// action=start（dry-run）签发的确认令牌；带回 confirm 才会真正烧录
    pub confirm_token: Option<String>,
    /// 令牌剩余有效秒数
    pub confirm_expires_in: Option<u64>,
    /// 前置检查发现的非阻断问题（如设备预检失败）
    pub warnings: Vec<String>,
    /// action=status / start（真实执行）的进度
    pub status: Option<OtaStatusView>,
}

impl OtaResponse {
    fn new(action: &str) -> Self {
        Self {
            action: action.to_string(),
            ..Default::default()
        }
    }
}

// 路由函数对外可见：设置页要报工具数量，不必凭空造一个 McpContext 只为了数一遍
#[tool_router(vis = "pub(crate)")]
impl ComBridgeMcp {
    #[tool(
        name = "combridge_about",
        description = "获取 ComBridge 服务概况：应用版本、安全档位、已连接设备快照与可用工具域。\
首次连接时调用一次即可；设备状态变化后可重新调用获取最新快照。\
action=logs 读取日志尾部（默认 100 行），工具返回错误且看不出原因时用它自查。",
        annotations(read_only_hint = true)
    )]
    async fn about(&self, Parameters(req): Parameters<AboutRequest>) -> ToolR<AboutResponse> {
        let action = req.action.as_deref().unwrap_or("summary").to_lowercase();

        if action == "logs" {
            let lines = req.lines.unwrap_or(100).clamp(1, 500);
            let path = super::logs::active_log_file().ok_or_else(|| {
                tool_err_full(
                    "LOG_NOT_FOUND",
                    format!(
                        "未找到日志文件（预期目录：{}）",
                        super::logs::log_dir()
                            .map(|d| d.display().to_string())
                            .unwrap_or_else(|| "<不可确定>".to_string())
                    ),
                    Some("确认应用以文件日志模式启动；或将 log.level 调到 info 后复现问题"),
                    false,
                )
            })?;
            let tail = super::logs::tail_lines(&path, lines)
                .map_err(|e| tool_err("LOG_READ_FAILED", e))?;
            return Ok(Json(AboutResponse {
                action: "logs".to_string(),
                app_version: self.ctx.app_version.clone(),
                tier: self.ctx.tier_name(),
                started_at: self.ctx.started_at,
                connected_devices: Vec::new(),
                tool_domains: Vec::new(),
                planned_tool_domains: Vec::new(),
                log_file: Some(path.display().to_string()),
                log_lines: tail,
            }));
        }

        if action != "summary" {
            return Err(tool_err_full(
                "INVALID_ARGUMENT",
                format!("未知 action: {action}"),
                Some("支持 summary（默认）| logs"),
                false,
            ));
        }

        let connected_devices = self.ctx.connected_devices().await;
        Ok(Json(AboutResponse {
            action: "summary".to_string(),
            app_version: self.ctx.app_version.clone(),
            tier: self.ctx.tier_name(),
            started_at: self.ctx.started_at,
            connected_devices,
            tool_domains: vec![
                "serial".to_string(),
                "ble".to_string(),
                "devices".to_string(),
                "gh3036".to_string(),
                "waveform".to_string(),
                "protocol".to_string(),
                "ota".to_string(),
            ],
            planned_tool_domains: Vec::new(),
            log_file: None,
            log_lines: Vec::new(),
        }))
    }

    #[tool(
        name = "combridge_ports",
        description = "列出系统串口：物理端口信息（制造商/产品/序列号）、是否已打开、当前租约持有者。\
操作串口前必须先调用本工具确认端口存在且可用。",
        annotations(read_only_hint = true)
    )]
    async fn ports(&self) -> ToolR<PortsResponse> {
        let scanned = self
            .ctx
            .serial
            .scan_ports()
            .map_err(|e| mcp_err(format!("扫描串口失败: {e}")))?;

        let open_ports: std::collections::HashSet<String> = self
            .ctx
            .serial
            .get_open_ports()
            .map_err(|e| mcp_err(format!("获取已打开端口失败: {e}")))?
            .into_iter()
            .collect();

        let mut entries = Vec::with_capacity(scanned.len());
        for p in scanned {
            let lease = self.ctx.leases.current(&p.name).map(|l| lease_to_view(&l));
            let open = open_ports.contains(&p.name);
            entries.push(PortEntry {
                name: p.name,
                port_type: p.port_type,
                manufacturer: p.manufacturer,
                product: p.product,
                serial_number: p.serial_number,
                open,
                lease,
            });
        }
        Ok(Json(PortsResponse { ports: entries }))
    }

    #[tool(
        name = "combridge_serial_open",
        description = "打开串口并获取租约。返回 lease_id（写/关操作必须携带）与数据游标 cursor。\
若端口已被打开（GUI 或其他会话），仍会返回租约与游标（already_open=true），此模式下可读但 close 会被拒绝；\
若租约被其他 AI 客户端持有且 force=false，返回 PORT_BUSY 及占用者信息。",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    async fn combridge_serial_open(
        &self,
        Parameters(req): Parameters<SerialOpenRequest>,
    ) -> ToolR<SerialOpenResponse> {
        // 参数映射（先验证再动手）
        let config = SerialPortConfig {
            port_name: req.port_name.clone(),
            baud_rate: map_baud(req.baud_rate)?,
            data_bits: map_data_bits(req.data_bits)?,
            parity: map_parity(&req.parity)?,
            stop_bits: map_stop_bits(req.stop_bits)?,
            flow_control: map_flow(&req.flow_control)?,
            ..Default::default()
        };

        // 1. 先拿租约（失败不产生副作用）
        let lease = match self
            .ctx
            .leases
            .acquire(&req.port_name, &req.owner, req.ttl_s, req.force)
        {
            Ok(l) => l,
            Err(AcquireError::Busy(info)) => {
                return Err(tool_err_full(
                    "PORT_BUSY",
                    format!(
                        "串口 {} 的租约被 {} 持有（lease={}，{} 过期）",
                        req.port_name, info.current_owner, info.lease_id, info.expires_at
                    ),
                    Some(
                        "等租约过期后重试；或用 force=true 重新调用 combridge_serial_open 抢占（会踢出对方）",
                    ),
                    true,
                ));
            }
        };

        // 2. 判断是否已打开
        let already_open = self
            .ctx
            .serial
            .is_port_open(&req.port_name)
            .map_err(|e| mcp_err(format!("查询端口状态失败: {e}")))?;

        // 3. 未打开则真正打开（数据经 EventBus 流入 hub，无需回调逻辑）
        if !already_open {
            if let Err(e) = self
                .ctx
                .serial
                .open_port(config, |_name, _data| { /* 数据由 SerialHub 订阅 */ })
            {
                self.ctx.leases.release(&req.port_name, &lease.lease_id);
                return Err(tool_err_full(
                    "SERIAL_ERROR",
                    format!("打开串口 {} 失败: {e}", req.port_name),
                    Some("确认端口未被其他程序占用，或改用 combridge_ports 检查端口是否仍存在"),
                    false,
                ));
            }
            self.ctx
                .opened_by_mcp
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .insert(req.port_name.clone());
            tracing::info!(
                "[MCP] 串口 {} 已由 MCP 打开 (owner={}, lease={})",
                req.port_name,
                req.owner,
                lease.lease_id
            );
        }

        let cursor = self.ctx.serial_hub.current_cursor(&req.port_name);
        // 复位断开标记：重开后的 expect 不应被上一次关闭误判
        self.ctx.serial_hub.mark_open(&req.port_name);
        Ok(Json(SerialOpenResponse {
            lease_id: lease.lease_id,
            owner: lease.owner,
            expires_at: lease.expires_at,
            cursor,
            already_open,
        }))
    }

    #[tool(
        name = "combridge_serial_close",
        description = "关闭串口并释放租约。仅限由 combridge_serial_open（MCP）打开的端口；GUI 打开的端口会被拒绝。",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn combridge_serial_close(
        &self,
        Parameters(req): Parameters<SerialCloseRequest>,
    ) -> ToolR<SerialCloseResponse> {
        self.ctx
            .leases
            .validate(&req.port_name, &req.lease_id)
            .map_err(|e| {
                tool_err_full(
                    "LEASE_INVALID",
                    e,
                    Some("先调用 combridge_serial_open 获取该端口的 lease_id"),
                    false,
                )
            })?;

        let opened_by_mcp = self
            .ctx
            .opened_by_mcp
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&req.port_name);

        if !opened_by_mcp {
            return Err(tool_err_full(
                "PORT_NOT_OWNED",
                format!(
                    "串口 {} 不是由 MCP 打开（可能由 GUI 打开），MCP 无权关闭",
                    req.port_name
                ),
                Some("在 ComBridge 界面关闭该端口，或由持有它的会话关闭"),
                false,
            ));
        }

        self.ctx
            .serial
            .close_port(&req.port_name)
            .map_err(|e| mcp_err(format!("关闭串口失败: {e}")))?;
        self.ctx
            .opened_by_mcp
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&req.port_name);
        self.ctx.leases.release(&req.port_name, &req.lease_id);

        tracing::info!("[MCP] 串口 {} 已由 MCP 关闭", req.port_name);
        Ok(Json(SerialCloseResponse {
            closed: true,
            port_name: req.port_name,
        }))
    }

    #[tool(
        name = "combridge_serial_write",
        description = "向串口写入数据（需持有该端口的租约）。\
encoding=text 原样发送；encoding=hex 发送二进制（如 \"AA BB 01\"）；append_crlf 用于 AT 命令。",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn combridge_serial_write(
        &self,
        Parameters(req): Parameters<SerialWriteRequest>,
        peer: Peer<RoleServer>,
    ) -> ToolR<SerialWriteResponse> {
        self.ctx
            .leases
            .validate(&req.port_name, &req.lease_id)
            .map_err(|e| {
                tool_err_full(
                    "LEASE_INVALID",
                    e,
                    Some("先调用 combridge_serial_open 获取该端口的 lease_id"),
                    false,
                )
            })?;

        let mut payload = decode_bytes(&req.data, &req.encoding)?;
        if req.append_crlf {
            payload.extend_from_slice(b"\r\n");
        }
        if payload.is_empty() {
            return Err(tool_err("INVALID_ARGUMENT", "待发送数据为空"));
        }

        // 标准档：写设备前必须由用户点确认（研发档跳过）
        self.confirm_if_needed(
            &peer,
            "combridge_serial_write",
            "串口写入",
            &req.port_name,
            format!(
                "即将向 {} 写入 {} 字节：{}",
                req.port_name,
                payload.len(),
                preview_payload(&payload, 32)
            ),
        )
        .await?;

        let written = self
            .ctx
            .serial
            .send_data(&req.port_name, &payload)
            .map_err(|e| mcp_err(format!("发送失败: {e}")))?;

        tracing::info!(
            "[MCP] combridge_serial_write: {} <- {} bytes (encoding={})",
            req.port_name,
            written,
            req.encoding
        );
        Ok(Json(SerialWriteResponse {
            bytes_written: written,
            port_name: req.port_name,
        }))
    }

    #[tool(
        name = "combridge_serial_read",
        description = "读取串口接收数据（游标语义，不需要租约）。\
首次调用省略 since 读缓冲区全部历史；之后传上次返回的 next_cursor 增量读取。\
返回 next_cursor / truncated / total_written。缓冲上限 1MB。",
        annotations(read_only_hint = true)
    )]
    async fn combridge_serial_read(
        &self,
        Parameters(req): Parameters<SerialReadRequest>,
    ) -> ToolR<SerialReadResponse> {
        let since = req.since.unwrap_or(0);
        let out = self.ctx.serial_hub.snapshot_since(&req.port_name, since);
        let data = encode_bytes(&out.data, &req.encoding)?;
        Ok(Json(SerialReadResponse {
            data,
            encoding: req.encoding,
            next_cursor: out.next_cursor,
            truncated: out.truncated,
            lagged: out.lagged,
            total_written: out.total_written,
        }))
    }

    #[tool(
        name = "combridge_serial_expect",
        description = "阻塞等待串口出现匹配内容（服务端等待，无需反复轮询 combridge_serial_read）。\
regex=true 时 pattern 为正则，否则为子串匹配；默认只匹配 since 之后的新数据。\
超时返回 matched=false 及已收到的全部文本。这是与设备交互的首选方式：write 后立即 expect。",
        annotations(read_only_hint = true)
    )]
    async fn combridge_serial_expect(
        &self,
        Parameters(req): Parameters<SerialExpectRequest>,
    ) -> ToolR<SerialExpectResponse> {
        let since = match req.since {
            Some(s) => s,
            None => self.ctx.serial_hub.current_cursor(&req.port_name),
        };
        let timeout = std::time::Duration::from_millis(req.timeout_ms.max(1) as u64);
        let pattern = req.pattern;

        let matcher: Matcher = if req.regex {
            let re = regex::RegexBuilder::new(&pattern)
                .case_insensitive(req.case_insensitive)
                .build()
                .map_err(|e| tool_err("INVALID_ARGUMENT", format!("正则表达式无效: {e}")))?;
            Box::new(move |text: &str| re.find(text).map(|m| m.as_str().to_string()))
        } else if req.case_insensitive {
            let pat = pattern.to_lowercase();
            Box::new(move |text: &str| {
                text.to_lowercase().find(&pat).map(|i| {
                    // 用原始文本按字节位置切片可能错位（大小写转换改变长度），
                    // 此处返回匹配到的原文片段时做一次安全回退
                    text.get(i..i + pat.len())
                        .map(str::to_string)
                        .unwrap_or_else(|| pat.clone())
                })
            })
        } else {
            let pat = pattern.clone();
            Box::new(move |text: &str| text.find(&pat).map(|i| text[i..i + pat.len()].to_string()))
        };

        let start = std::time::Instant::now();
        let out = self
            .ctx
            .serial_hub
            .wait_for_match(&req.port_name, since, timeout, matcher)
            .await;
        let elapsed = start.elapsed().as_millis() as u64;

        tracing::debug!(
            "[MCP] combridge_serial_expect: {} pattern={:?} matched={} elapsed={}ms",
            req.port_name,
            pattern,
            out.matched,
            elapsed
        );
        Ok(Json(SerialExpectResponse {
            matched: out.matched,
            matched_text: out.matched_text,
            buffer: out.buffer,
            next_cursor: out.next_cursor,
            elapsed_ms: elapsed,
            truncated: out.truncated,
            lagged: out.lagged,
            disconnected: out.disconnected,
        }))
    }

    #[tool(
        name = "combridge_devices",
        description = "汇总当前设备与占用状态：BLE 后端模式/是否可用、BLE 连接与订阅、App 状态中的设备快照、\
全部有效租约。开始任何多设备操作前先调用；也可用来判断某端口/地址是否被其他 AI 客户端占用。",
        annotations(read_only_hint = true)
    )]
    async fn devices(&self) -> ToolR<DevicesResponse> {
        let ble_mode = self.ctx.ble.mode().await.to_string();
        let ble_configured = self.ctx.ble.is_configured().await;

        let mut ble_connections = Vec::new();
        if ble_configured {
            match self.ctx.ble.get_connections().await {
                Ok(list) => {
                    for c in list {
                        let subscriptions = self.ctx.ble.get_subscriptions(&c.address).await;
                        ble_connections.push(BleConnectionView {
                            address: c.address,
                            name: c.name,
                            is_connected: c.is_connected,
                            subscriptions,
                        });
                    }
                }
                Err(e) => tracing::warn!("[MCP] 获取 BLE 连接失败: {}", e),
            }
        }

        let active_leases = self
            .ctx
            .leases
            .list()
            .into_iter()
            .map(|l| LeaseEntry {
                resource: l.resource,
                owner: l.owner,
                expires_at: l.expires_at,
            })
            .collect();

        Ok(Json(DevicesResponse {
            ble_mode,
            ble_configured,
            ble_connections,
            app_devices: self.ctx.connected_devices().await,
            active_leases,
        }))
    }

    #[tool(
        name = "combridge_ble_scan",
        description = "扫描 BLE 设备（阻塞 duration_ms 后返回结果），或 stop=true 停止正在进行的扫描。\
返回地址/名称/RSSI/可连接性；address 用于后续 combridge_ble_connect。",
        annotations(read_only_hint = true)
    )]
    async fn ble_scan(
        &self,
        Parameters(req): Parameters<BleScanRequest>,
    ) -> ToolR<BleScanResponse> {
        let found = if req.stop {
            self.ctx
                .ble
                .stop_scan()
                .await
                .map_err(|e| mcp_err(format!("停止扫描失败: {e}")))?
        } else {
            self.ctx
                .ble
                .scan(req.duration_ms.max(100))
                .await
                .map_err(|e| mcp_err(format!("扫描失败: {e}")))?
        };

        Ok(Json(BleScanResponse {
            devices: found
                .into_iter()
                .map(|d| BleDeviceView {
                    address: d.address,
                    name: d.name,
                    rssi: d.rssi,
                    is_connectable: d.is_connectable,
                })
                .collect(),
            stopped: req.stop,
        }))
    }

    #[tool(
        name = "combridge_ble_connect",
        description = "BLE 连接管理：action=connect 连接并获取租约（返回 lease_id）、disconnect 断开并释放租约、list 列出当前连接。\
租约被其他客户端持有时返回 BLE_BUSY，可 force=true 抢占。连接后需先 discover_services 才能读写特征。",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn ble_connect(
        &self,
        Parameters(req): Parameters<BleConnectRequest>,
    ) -> ToolR<BleConnectResponse> {
        let action = req.action.to_ascii_lowercase();
        if !matches!(action.as_str(), "connect" | "disconnect" | "list") {
            return Err(mcp_err(format!(
                "未知 action: {}（支持 connect | disconnect | list）",
                req.action
            )));
        }
        if action != "list" && req.address.trim().is_empty() {
            return Err(tool_err("INVALID_ARGUMENT", "address 不能为空"));
        }
        let resource = format!("ble:{}", req.address);

        let mut lease_id = None;
        let mut connected = false;
        let mut subscriptions = Vec::new();

        match action.as_str() {
            "connect" => {
                // 同一 owner 已持有租约时直接复用：否则同一客户端「连一次、再连一次」
                // （或断线后重连）会被自己的租约挡住，只能 force 抢占自己。
                let existing = if req.force {
                    None
                } else {
                    self.ctx
                        .leases
                        .current(&resource)
                        .filter(|l| l.owner == req.owner)
                };
                let lease = match existing {
                    Some(lease) => {
                        tracing::info!(
                            "[MCP] 复用 BLE 租约 {} -> {} (lease={})",
                            req.address,
                            req.owner,
                            lease.lease_id
                        );
                        lease
                    }
                    None => self
                        .ctx
                        .leases
                        .acquire(&resource, &req.owner, req.ttl_s, req.force)
                        .map_err(|e| match e {
                            AcquireError::Busy(info) => tool_err_full(
                                "BLE_BUSY",
                                format!(
                                    "{} 的租约被 {} 持有（lease={}，{} 过期）",
                                    req.address,
                                    info.current_owner,
                                    info.lease_id,
                                    info.expires_at
                                ),
                                Some(
                                    "等租约过期后重试；或用 force=true 重新调用 combridge_ble_connect 抢占",
                                ),
                                true,
                            ),
                        })?,
                };

                match self.ctx.ble.connect(&req.address).await {
                    Ok(conn) => {
                        connected = conn.is_connected;
                        subscriptions = self.ctx.ble.get_subscriptions(&req.address).await;
                        lease_id = Some(lease.lease_id.clone());
                        tracing::info!(
                            "[MCP] BLE 已连接 {} (owner={}, lease={})",
                            req.address,
                            req.owner,
                            lease.lease_id
                        );
                    }
                    Err(e) => {
                        self.ctx.leases.release(&resource, &lease.lease_id);
                        return Err(mcp_err(format!("连接 {} 失败: {e}", req.address)));
                    }
                }
            }
            "disconnect" => {
                self.ctx
                    .leases
                    .validate(&resource, &req.lease_id)
                    .map_err(|e| {
                        tool_err_full(
                            "LEASE_INVALID",
                            e,
                            Some("先调用 combridge_ble_connect 获取该地址的 lease_id"),
                            false,
                        )
                    })?;
                self.ctx
                    .ble
                    .disconnect(&req.address)
                    .await
                    .map_err(|e| mcp_err(format!("断开 {} 失败: {e}", req.address)))?;
                self.ctx.ble_hub.remove_address(&req.address);
                self.ctx.leases.release(&resource, &req.lease_id);
                tracing::info!("[MCP] BLE 已断开 {}", req.address);
            }
            _ => {}
        }

        // 统一返回当前连接列表，方便模型确认状态
        let mut connections = Vec::new();
        if self.ctx.ble.is_configured().await {
            if let Ok(list) = self.ctx.ble.get_connections().await {
                for c in list {
                    let subs = self.ctx.ble.get_subscriptions(&c.address).await;
                    connections.push(BleConnectionView {
                        address: c.address,
                        name: c.name,
                        is_connected: c.is_connected,
                        subscriptions: subs,
                    });
                }
            }
        }

        Ok(Json(BleConnectResponse {
            action,
            address: req.address,
            connected,
            lease_id,
            subscriptions,
            connections,
        }))
    }

    #[tool(
        name = "combridge_ble_gatt",
        description = "BLE GATT 操作。action 取值：\
discover_services / discover_characteristics（需 service_uuid）/ read / write（需 lease_id，支持 without_response）/ \
subscribe（需 lease_id）/ unsubscribe（需 lease_id）/ notifications（游标读取已订阅通知）/ mtu（需 lease_id）/ rssi。\
二进制数据用 encoding=hex 读写更可靠。",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn ble_gatt(
        &self,
        Parameters(req): Parameters<BleGattRequest>,
        peer: Peer<RoleServer>,
    ) -> ToolR<BleGattResponse> {
        let action = req.action.to_ascii_lowercase();
        let resource = format!("ble:{}", req.address);

        let need_char = matches!(
            action.as_str(),
            "read" | "write" | "subscribe" | "unsubscribe" | "notifications"
        );
        if need_char && req.char_uuid.trim().is_empty() {
            return Err(tool_err("INVALID_ARGUMENT", "char_uuid 不能为空"));
        }

        let require_lease = || -> Result<(), CallToolResult> {
            self.ctx
                .leases
                .validate(&resource, &req.lease_id)
                .map_err(|e| {
                    tool_err_full(
                        "LEASE_INVALID",
                        e,
                        Some("先调用 combridge_ble_connect 获取该地址的 lease_id"),
                        false,
                    )
                })
        };

        let mut resp = BleGattResponse {
            action: action.clone(),
            value: None,
            encoding: None,
            next_cursor: None,
            truncated: None,
            services: None,
            characteristics: None,
            mtu: None,
            rssi: None,
            subscriptions: None,
        };

        // 标准档：只有会改设备状态的动作才要确认（discover/read/notifications/rssi 免确认）
        if matches!(
            action.as_str(),
            "write" | "subscribe" | "unsubscribe" | "mtu"
        ) {
            let detail = match action.as_str() {
                "write" => format!(
                    "即将向 {} 的特征 {} 写入 {} 字符：{}",
                    req.address,
                    req.char_uuid,
                    req.data.chars().count(),
                    req.data.chars().take(64).collect::<String>()
                ),
                "mtu" => format!("即将把 {} 的 MTU 协商为 {}", req.address, req.mtu),
                _ => format!(
                    "即将对 {} 的特征 {} 执行 {action}",
                    req.address, req.char_uuid
                ),
            };
            self.confirm_if_needed(
                &peer,
                "combridge_ble_gatt",
                "BLE 设备写入",
                &req.address,
                detail,
            )
            .await?;
        }

        match action.as_str() {
            "discover_services" => {
                let services = self
                    .ctx
                    .ble
                    .discover_services(&req.address)
                    .await
                    .map_err(|e| mcp_err(format!("发现服务失败: {e}")))?;
                resp.services = Some(services.iter().map(to_service_view).collect());
            }
            "discover_characteristics" => {
                if req.service_uuid.trim().is_empty() {
                    return Err(tool_err(
                        "INVALID_ARGUMENT",
                        "discover_characteristics 需要 service_uuid",
                    ));
                }
                let chars = self
                    .ctx
                    .ble
                    .discover_characteristics(&req.address, &req.service_uuid)
                    .await
                    .map_err(|e| mcp_err(format!("发现特征失败: {e}")))?;
                resp.characteristics = Some(chars.iter().map(to_char_view).collect());
            }
            "read" => {
                let bytes = self
                    .ctx
                    .ble
                    .read_characteristic(&req.address, &req.char_uuid)
                    .await
                    .map_err(|e| mcp_err(format!("读取特征失败: {e}")))?;
                resp.value = Some(encode_bytes(&bytes, &req.encoding)?);
                resp.encoding = Some(req.encoding.clone());
            }
            "write" => {
                require_lease()?;
                let payload = decode_bytes(&req.data, &req.encoding)?;
                if payload.is_empty() {
                    return Err(tool_err("INVALID_ARGUMENT", "待写入数据为空"));
                }
                if req.without_response {
                    self.ctx
                        .ble
                        .write_without_response(&req.address, &req.char_uuid, &payload)
                        .await
                        .map_err(|e| mcp_err(format!("写入失败: {e}")))?;
                } else {
                    self.ctx
                        .ble
                        .write_characteristic(&req.address, &req.char_uuid, &payload)
                        .await
                        .map_err(|e| mcp_err(format!("写入失败: {e}")))?;
                }
                tracing::info!(
                    "[MCP] ble_gatt write: {} / {} <- {} bytes (without_response={})",
                    req.address,
                    req.char_uuid,
                    payload.len(),
                    req.without_response
                );
            }
            "subscribe" => {
                require_lease()?;
                // 重复订阅会在后端叠加回调：同一条通知被喂多份，
                // RPC 侧还会看到重复响应帧（stale response 告警）。
                if !self
                    .ctx
                    .ble
                    .get_subscriptions(&req.address)
                    .await
                    .iter()
                    .any(|c| c == &req.char_uuid)
                {
                    let cb: crate::device::ble::NotifyCallback = Arc::new(|_a, _c, _d| {});
                    self.ctx
                        .ble
                        .subscribe_notify(&req.address, &req.char_uuid, cb)
                        .await
                        .map_err(|e| mcp_err(format!("订阅失败: {e}")))?;
                }
                // 清空旧通知，游标从此刻起算
                let cursor = self.ctx.ble_hub.reset_to_now(&req.address, &req.char_uuid);
                resp.next_cursor = Some(cursor);
                resp.subscriptions = Some(self.ctx.ble.get_subscriptions(&req.address).await);
            }
            "unsubscribe" => {
                require_lease()?;
                self.ctx
                    .ble
                    .unsubscribe_notify(&req.address, &req.char_uuid)
                    .await
                    .map_err(|e| mcp_err(format!("取消订阅失败: {e}")))?;
                resp.subscriptions = Some(self.ctx.ble.get_subscriptions(&req.address).await);
            }
            "notifications" => {
                let since = req.since.unwrap_or(0);
                let out = self
                    .ctx
                    .ble_hub
                    .snapshot_since(&req.address, &req.char_uuid, since);
                resp.value = Some(encode_bytes(&out.data, &req.encoding)?);
                resp.encoding = Some(req.encoding.clone());
                resp.next_cursor = Some(out.next_cursor);
                resp.truncated = Some(out.truncated);
            }
            "mtu" => {
                require_lease()?;
                let mtu = if req.mtu == 0 { 247 } else { req.mtu };
                let applied = self
                    .ctx
                    .ble
                    .set_mtu(&req.address, mtu)
                    .await
                    .map_err(|e| mcp_err(format!("设置 MTU 失败: {e}")))?;
                resp.mtu = Some(applied);
            }
            "rssi" => {
                let rssi = self
                    .ctx
                    .ble
                    .get_rssi(&req.address)
                    .await
                    .map_err(|e| mcp_err(format!("读取 RSSI 失败: {e}")))?;
                resp.rssi = Some(rssi);
            }
            other => {
                return Err(mcp_err(format!(
                    "未知 action: {other}（支持 discover_services | discover_characteristics | read | write | \
subscribe | unsubscribe | notifications | mtu | rssi）"
                )));
            }
        }

        Ok(Json(resp))
    }

    // -----------------------------------------------------------------------
    // GH3036
    // -----------------------------------------------------------------------

    #[tool(
        name = "combridge_gh3036_rpc",
        description = "GH3036 芯片协议操作。action=status 看通道/初始化/产测状态；action=init 初始化协议栈（幂等）；\
action=set_channel 配置 TX/RX 通道（serial 传串口名，ble 传地址+特征 UUID）；action=catalog 列出可用 RPC 命令键；\
action=rpc 执行一条 RPC（command_key 取自 catalog，params 按命令定义顺序传，十六进制值可带 0x 前缀）。\
首次调用会自动获取 GH3036 租约并在返回中给出 lease_id；同一 owner 再次调用沿用该租约，\
其他 owner 占用时返回 GH3036_BUSY（force=true 可抢占）。",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false
        )
    )]
    async fn gh3036_rpc(
        &self,
        Parameters(req): Parameters<Gh3036RpcRequest>,
        peer: Peer<RoleServer>,
    ) -> ToolR<Gh3036RpcResponse> {
        let action = req.action.to_lowercase();
        let owner = req
            .owner
            .clone()
            .unwrap_or_else(|| "mcp-client".to_string());

        let mut resp = Gh3036RpcResponse {
            action: action.clone(),
            initialized: self.ctx.gh3036.is_initialized(),
            lease_id: None,
            tx_channel: None,
            rx_channel: None,
            factory_status: None,
            factory_step: None,
            commands: Vec::new(),
            version_types: Vec::new(),
            bytes: None,
            hex: None,
            text: None,
        };

        // 标准档：会改动芯片状态的动作先确认（status/catalog 只读，免确认）
        if matches!(action.as_str(), "rpc" | "init" | "set_channel") {
            let detail = match action.as_str() {
                "rpc" => format!(
                    "即将向 GH3036 发送 RPC 命令 {}，参数 [{}]",
                    req.command_key.clone().unwrap_or_default(),
                    req.params.join(", ")
                ),
                "init" => "即将初始化 GH3036 会话（会复位芯片通信）".to_string(),
                _ => format!(
                    "即将把 GH3036 通道切到 {}",
                    req.device_id.clone().unwrap_or_default()
                ),
            };
            // GH3036 是单芯片会话，按 owner 授权：同一轮联调不该每次 RPC 都点确认
            self.confirm_if_needed(&peer, "combridge_gh3036_rpc", "GH3036 写入", &owner, detail)
                .await?;
        }

        match action.as_str() {
            "status" => {
                resp.tx_channel = self
                    .ctx
                    .gh3036
                    .get_tx_channel()
                    .as_ref()
                    .map(to_channel_view);
                resp.rx_channel = self
                    .ctx
                    .gh3036
                    .get_rx_channel()
                    .as_ref()
                    .map(to_channel_view);
                let (status, step) = self.ctx.gh3036.factory_test_status();
                resp.factory_status = Some(super::gh3036_job::status_str(status));
                resp.factory_step = Some(super::gh3036_job::step_str(step));
                resp.lease_id = self.ctx.leases.current(GH3036_RESOURCE).map(|l| l.lease_id);
            }
            "init" => {
                let lease = self.acquire_gh3036_lease(
                    &owner,
                    req.lease_id.as_deref(),
                    req.force,
                    req.ttl_s,
                )?;
                resp.lease_id = Some(lease.lease_id);
                if !self.ctx.gh3036.is_initialized() {
                    self.ctx.gh3036.initialize().map_err(|e| {
                        tool_err_full(
                            "GH3036_INIT_FAILED",
                            e,
                            Some("检查设备连接与协议模块状态后重试"),
                            false,
                        )
                    })?;
                }
                resp.initialized = true;
                resp.tx_channel = self
                    .ctx
                    .gh3036
                    .get_tx_channel()
                    .as_ref()
                    .map(to_channel_view);
                resp.rx_channel = self
                    .ctx
                    .gh3036
                    .get_rx_channel()
                    .as_ref()
                    .map(to_channel_view);
            }
            "set_channel" => {
                let lease = self.acquire_gh3036_lease(
                    &owner,
                    req.lease_id.as_deref(),
                    req.force,
                    req.ttl_s,
                )?;
                resp.lease_id = Some(lease.lease_id);
                self.ensure_gh3036_initialized()?;

                let channel_type = req
                    .channel_type
                    .clone()
                    .unwrap_or_else(|| "serial".to_string());
                let ch_type = match channel_type.to_lowercase().as_str() {
                    "serial" => ChannelType::Serial,
                    "ble" => ChannelType::Ble,
                    other => {
                        return Err(tool_err(
                            "INVALID_ARGUMENT",
                            format!("不支持的 channel_type: {other}（serial | ble）"),
                        ))
                    }
                };
                let device_id = req.device_id.clone().ok_or_else(|| {
                    tool_err_full(
                        "INVALID_ARGUMENT",
                        "set_channel 需要 device_id（串口名或 BLE 地址）",
                        Some("先调用 combridge_ports 或 combridge_ble_scan 取得设备标识"),
                        false,
                    )
                })?;

                let config = ChannelConfig {
                    channel_type: ch_type,
                    device_id,
                    characteristic_uuid: req.characteristic_uuid.clone(),
                };
                let direction = req.direction.clone().unwrap_or_else(|| "both".to_string());
                match direction.to_lowercase().as_str() {
                    "tx" => self
                        .ctx
                        .gh3036
                        .configure_tx_channel(config)
                        .map_err(|e| tool_err("GH3036_CHANNEL_FAILED", e))?,
                    "rx" => self
                        .ctx
                        .gh3036
                        .configure_rx_channel(config)
                        .map_err(|e| tool_err("GH3036_CHANNEL_FAILED", e))?,
                    "both" => {
                        self.ctx
                            .gh3036
                            .configure_tx_channel(config.clone())
                            .map_err(|e| tool_err("GH3036_CHANNEL_FAILED", e))?;
                        self.ctx
                            .gh3036
                            .configure_rx_channel(config)
                            .map_err(|e| tool_err("GH3036_CHANNEL_FAILED", e))?;
                    }
                    other => {
                        return Err(tool_err(
                            "INVALID_ARGUMENT",
                            format!("不支持的 direction: {other}（tx | rx | both）"),
                        ))
                    }
                }

                resp.tx_channel = self
                    .ctx
                    .gh3036
                    .get_tx_channel()
                    .as_ref()
                    .map(to_channel_view);
                resp.rx_channel = self
                    .ctx
                    .gh3036
                    .get_rx_channel()
                    .as_ref()
                    .map(to_channel_view);
            }
            "catalog" => {
                resp.commands = crate::gh3036::get_rpc_commands()
                    .into_iter()
                    .map(|c| RpcCommandView {
                        key: c.key,
                        name: c.name,
                        description: c.description,
                        params: c
                            .params
                            .into_iter()
                            .map(|p| {
                                let default = p
                                    .default_value
                                    .map(|d| format!("，默认 {d}"))
                                    .unwrap_or_default();
                                format!(
                                    "{}（{}）：{}{}",
                                    p.name, p.param_type, p.description, default
                                )
                            })
                            .collect(),
                    })
                    .collect();
                resp.version_types = crate::gh3036::get_version_types()
                    .into_iter()
                    .map(|v| VersionTypeView {
                        type_value: v.type_value,
                        name: v.name,
                        description: v.description,
                    })
                    .collect();
                resp.tx_channel = self
                    .ctx
                    .gh3036
                    .get_tx_channel()
                    .as_ref()
                    .map(to_channel_view);
                resp.rx_channel = self
                    .ctx
                    .gh3036
                    .get_rx_channel()
                    .as_ref()
                    .map(to_channel_view);
            }
            "rpc" => {
                let lease = self.acquire_gh3036_lease(
                    &owner,
                    req.lease_id.as_deref(),
                    req.force,
                    req.ttl_s,
                )?;
                resp.lease_id = Some(lease.lease_id);
                self.ensure_gh3036_initialized()?;

                let key = req.command_key.clone().ok_or_else(|| {
                    tool_err_full(
                        "INVALID_ARGUMENT",
                        "action=rpc 需要 command_key",
                        Some("先调用 action=catalog 查看可用命令键"),
                        false,
                    )
                })?;

                let data = self
                    .ctx
                    .gh3036
                    .execute_rpc(&key, &req.params)
                    .await
                    .map_err(|e| {
                        tool_err_full(
                            "GH3036_RPC_FAILED",
                            format!("RPC {key} 执行失败: {e}"),
                            Some(
                                "检查通道是否已配置且设备在线；BLE 通道首次使用需先 discover_services \
并订阅通知特征；参数格式见 action=catalog",
                            ),
                            true,
                        )
                    })?;

                resp.bytes = Some(data.len());
                resp.hex = Some(hex_encode(&data));
                resp.text = printable_text(&data);
            }
            other => {
                return Err(tool_err_full(
                    "INVALID_ARGUMENT",
                    format!("未知 action: {other}"),
                    Some("支持 status | init | set_channel | catalog | rpc"),
                    false,
                ));
            }
        }

        Ok(Json(resp))
    }

    #[tool(
        name = "combridge_gh3036_factory_test",
        description = "GH3036 产测。action=run 是主路径：内部完成启动→轮询→等待→汇总，\
一次调用直接返回结论 {pass, failed_item, items[], csv_path}，无需关心中间步骤。\
max_wait_s 内未结束则返回 status=running 与 job_id，用 action=status 续等。\
流程中途停在待切换测试环境时会返回 status=waiting_environment_switch，\
需人工切换环境后调用 action=continue。action=stop 中止；action=validate_config 校验卡控与配置文件；\
action=threshold_get 取当前卡控配置 YAML。run/continue/stop 需要 GH3036 租约（首次调用自动获取）。",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false
        )
    )]
    async fn gh3036_factory_test(
        &self,
        Parameters(req): Parameters<FactoryTestRequest>,
        peer: Peer<RoleServer>,
    ) -> ToolR<super::gh3036_job::FactorySummary> {
        let action = req.action.to_lowercase();
        let owner = req
            .owner
            .clone()
            .unwrap_or_else(|| "mcp-client".to_string());

        // 标准档：产测整轮流程只确认一次 —— run 时用户可勾"本次会话内放行"，
        // 之后同一 owner 的 continue / stop 直接过；不勾则照旧逐次确认。
        if matches!(action.as_str(), "run" | "continue" | "stop") {
            self.confirm_if_needed(
                &peer,
                "combridge_gh3036_factory_test",
                "GH3036 产测",
                &owner,
                format!(
                    "即将执行产测 action={action}（最长等待 {}s）",
                    req.max_wait_s.unwrap_or(180)
                ),
            )
            .await?;
        }

        match action.as_str() {
            "run" => {
                let lease = self.acquire_gh3036_lease(
                    &owner,
                    req.lease_id.as_deref(),
                    req.force,
                    req.ttl_s,
                )?;
                self.ensure_gh3036_initialized()?;

                if let Some(dir) = req.config_dir.as_ref() {
                    self.ctx
                        .gh3036
                        .factory_test_set_config_dir(dir)
                        .map_err(|e| tool_err("CONFIG_INVALID", e))?;
                }

                // 前置校验：卡控配置 + 通道
                let validation = self.ctx.gh3036.factory_test_validate_config();
                if !validation.is_valid {
                    return Err(tool_err_full(
                        "CONFIG_INVALID",
                        format!("产测配置校验失败: {}", validation.errors.join("；")),
                        Some("用 action=validate_config 查看缺口，或先设置 config_dir"),
                        false,
                    ));
                }
                if self.ctx.gh3036.get_tx_channel().is_none()
                    || self.ctx.gh3036.get_rx_channel().is_none()
                {
                    return Err(tool_err_full(
                        "CHANNEL_NOT_CONFIGURED",
                        "产测需要先配置 TX/RX 通道",
                        Some("调用 combridge_gh3036_rpc 的 action=set_channel 配置通道后重试"),
                        false,
                    ));
                }

                let job = super::gh3036_job::FactoryJob::new();
                {
                    let mut slot = self
                        .ctx
                        .factory_job
                        .write()
                        .unwrap_or_else(|e| e.into_inner());
                    *slot = Some(job.clone());
                }

                self.ctx
                    .gh3036
                    .clone()
                    .factory_test_start()
                    .await
                    .map_err(|e| {
                        tool_err_full(
                            "FACTORY_TEST_FAILED",
                            format!("启动产测失败: {e}"),
                            Some("若提示已在运行，可用 action=status 查看进度或 action=stop 中止"),
                            false,
                        )
                    })?;

                let max_wait_s = req.max_wait_s.unwrap_or(180).clamp(5, 600);
                let deadline =
                    tokio::time::Instant::now() + std::time::Duration::from_secs(max_wait_s);

                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    let (status, step) = self.ctx.gh3036.factory_test_status();

                    match status {
                        FactoryTestStatus::Completed | FactoryTestStatus::Failed => {
                            let mut summary = self.build_factory_summary(&job, status, step);
                            summary.lease_id = Some(lease.lease_id.clone());
                            return Ok(Json(summary));
                        }
                        FactoryTestStatus::WaitingForEnvironmentSwitch => {
                            let mut summary = self.build_factory_summary(&job, status, step);
                            summary.lease_id = Some(lease.lease_id.clone());
                            summary.hint = Some(
                                "产测暂停等待切换测试环境：人工完成环境切换（如更换光源）后，\
调用 action=continue 继续"
                                    .to_string(),
                            );
                            return Ok(Json(summary));
                        }
                        FactoryTestStatus::Stopped => {
                            let mut summary = self.build_factory_summary(&job, status, step);
                            summary.lease_id = Some(lease.lease_id.clone());
                            summary.hint = Some("产测已被中止".to_string());
                            return Ok(Json(summary));
                        }
                        FactoryTestStatus::Idle if tokio::time::Instant::now() > deadline => {
                            let mut summary = self.build_factory_summary(&job, status, step);
                            summary.lease_id = Some(lease.lease_id.clone());
                            summary.hint =
                                Some("产测尚未开始，稍后用 action=status 复查".to_string());
                            return Ok(Json(summary));
                        }
                        _ => {
                            if tokio::time::Instant::now() > deadline {
                                let mut summary = self.build_factory_summary(&job, status, step);
                                summary.lease_id = Some(lease.lease_id.clone());
                                summary.hint = Some(format!(
                                    "已等待 {max_wait_s}s 仍未结束：用 action=status 续等，或用 action=stop 中止"
                                ));
                                return Ok(Json(summary));
                            }
                        }
                    }
                }
            }
            "status" => {
                let (status, step) = self.ctx.gh3036.factory_test_status();
                let job = self.current_or_transient_job();
                let mut summary = self.build_factory_summary(&job, status, step);
                summary.lease_id = self.ctx.leases.current(GH3036_RESOURCE).map(|l| l.lease_id);
                Ok(Json(summary))
            }
            "continue" => {
                let lease = self.acquire_gh3036_lease(
                    &owner,
                    req.lease_id.as_deref(),
                    req.force,
                    req.ttl_s,
                )?;
                self.ctx.gh3036.factory_test_continue().map_err(|e| {
                    tool_err_full(
                        "FACTORY_TEST_STATE",
                        e,
                        Some("仅当 action=status 返回 waiting_environment_switch 时可继续"),
                        false,
                    )
                })?;
                let (status, step) = self.ctx.gh3036.factory_test_status();
                let job = self.current_or_transient_job();
                let mut summary = self.build_factory_summary(&job, status, step);
                summary.lease_id = Some(lease.lease_id);
                summary.hint = Some(
                    "已继续，等待后续步骤；用 action=status 查询进度，或再次 action=run 前先确认已结束"
                        .to_string(),
                );
                Ok(Json(summary))
            }
            "stop" => {
                let lease = self.acquire_gh3036_lease(
                    &owner,
                    req.lease_id.as_deref(),
                    req.force,
                    req.ttl_s,
                )?;
                self.ctx.gh3036.factory_test_stop().await.map_err(|e| {
                    tool_err_full(
                        "FACTORY_TEST_STATE",
                        e,
                        Some("产测未在运行时无需 stop"),
                        false,
                    )
                })?;
                let (status, step) = self.ctx.gh3036.factory_test_status();
                let job = self.current_or_transient_job();
                let mut summary = self.build_factory_summary(&job, status, step);
                summary.lease_id = Some(lease.lease_id);
                summary.hint = Some("已中止".to_string());
                Ok(Json(summary))
            }
            "validate_config" => {
                let validation = self.ctx.gh3036.factory_test_validate_config();
                let (status, step) = self.ctx.gh3036.factory_test_status();
                let job = self.current_or_transient_job();
                let mut summary = self.build_factory_summary(&job, status, step);
                summary.lease_id = self.ctx.leases.current(GH3036_RESOURCE).map(|l| l.lease_id);
                // 校验结果通过 items 表达不便，这里用 hint 承载首条错误，结构化信息见 message
                if validation.is_valid {
                    summary.hint = Some("配置校验通过".to_string());
                } else {
                    return Err(tool_err_full(
                        "CONFIG_INVALID",
                        validation.errors.join("；"),
                        Some("补齐缺失的配置文件后重试 action=run"),
                        false,
                    ));
                }
                Ok(Json(summary))
            }
            "threshold_get" => {
                let config = self.ctx.gh3036.get_threshold_config().ok_or_else(|| {
                    tool_err_full(
                        "CONFIG_NOT_LOADED",
                        "尚未加载卡控配置",
                        Some("先调用 action=run 或设置 config_dir 触发加载，再用 action=validate_config 校验"),
                        false,
                    )
                })?;
                let yaml = serde_yaml::to_string(&config)
                    .map_err(|e| tool_err("CONFIG_INVALID", format!("序列化卡控配置失败: {e}")))?;
                let (status, step) = self.ctx.gh3036.factory_test_status();
                let job = self.current_or_transient_job();
                let mut summary = self.build_factory_summary(&job, status, step);
                summary.lease_id = self.ctx.leases.current(GH3036_RESOURCE).map(|l| l.lease_id);
                summary.hint = Some(yaml);
                Ok(Json(summary))
            }
            other => Err(tool_err_full(
                "INVALID_ARGUMENT",
                format!("未知 action: {other}"),
                Some("支持 run | status | continue | stop | validate_config | threshold_get"),
                false,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // P4：波形
    // -----------------------------------------------------------------------

    #[tool(
        name = "combridge_waveform",
        description = "波形数据缓冲（与 GUI 的波形面板共用同一份实例）。\
action=list 列出缓冲 ID；create 新建（buffer_id，可选 capacity / column_names）；\
configure_parser 设置解析器（parser.parser_type=delimiter 按分隔符，regex 按正则捕获组）；\
parse 把文本解析成数值行追加（data 可含多行，每行一条样本）；read 读取最近 N 行；\
clear 清空；status 查看行数与列名；remove 删除缓冲。\
典型用法：create → configure_parser → parse(串口收到的整段文本) → read 取值。",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    async fn waveform(
        &self,
        Parameters(req): Parameters<WaveformRequest>,
    ) -> ToolR<WaveformResponse> {
        let action = req.action.to_lowercase();

        match action.as_str() {
            "list" => {
                let mut resp = WaveformResponse::new("list");
                resp.buffers = self.ctx.waveform.list_buffers();
                Ok(Json(resp))
            }
            "create" => {
                let buffer_id = require_arg(req.buffer_id.as_deref(), "buffer_id", "create")?;
                let config = WaveformBufferConfig {
                    capacity: req.capacity.unwrap_or(1000).clamp(1, 1_000_000),
                    column_names: req.column_names.unwrap_or_else(default_columns),
                };
                self.ctx
                    .waveform
                    .create_buffer(&buffer_id, config.clone())
                    .map_err(|e| {
                        tool_err_full(
                            "BUFFER_EXISTS",
                            e.to_string(),
                            Some("换一个 buffer_id，或先 action=remove 删除同名缓冲"),
                            false,
                        )
                    })?;
                let status = self
                    .ctx
                    .waveform
                    .get_status(&buffer_id)
                    .map_err(|e| tool_err("INVALID_ARGUMENT", e.to_string()))?;
                Ok(Json(WaveformResponse {
                    action: "create".to_string(),
                    buffer_id: Some(buffer_id),
                    columns: status.column_names,
                    row_count: status.row_count,
                    column_count: status.column_count,
                    capacity: Some(status.capacity),
                    ..Default::default()
                }))
            }
            "remove" => {
                let buffer_id = require_arg(req.buffer_id.as_deref(), "buffer_id", "remove")?;
                self.ctx.waveform.remove_buffer(&buffer_id);
                Ok(Json(WaveformResponse {
                    action: "remove".to_string(),
                    buffer_id: Some(buffer_id),
                    ..Default::default()
                }))
            }
            "configure_parser" => {
                let buffer_id =
                    require_arg(req.buffer_id.as_deref(), "buffer_id", "configure_parser")?;
                let spec = req.parser.ok_or_else(|| {
                    tool_err_full(
                        "INVALID_ARGUMENT",
                        "configure_parser 需要 parser 参数",
                        Some("如 {\"parser_type\":\"delimiter\",\"delimiter\":\",\"}"),
                        false,
                    )
                })?;
                let config = build_parser_config(spec)?;
                self.ctx
                    .waveform
                    .configure_parser(&buffer_id, config)
                    .map_err(|e| tool_err("PARSER_INVALID", e.to_string()))?;
                let status = self
                    .ctx
                    .waveform
                    .get_status(&buffer_id)
                    .map_err(|e| tool_err("INVALID_ARGUMENT", e.to_string()))?;
                Ok(Json(WaveformResponse {
                    action: "configure_parser".to_string(),
                    buffer_id: Some(buffer_id),
                    columns: status.column_names,
                    row_count: status.row_count,
                    column_count: status.column_count,
                    capacity: Some(status.capacity),
                    parser_type: status.parser_type.map(parser_type_str),
                    ..Default::default()
                }))
            }
            "parse" => {
                let buffer_id = require_arg(req.buffer_id.as_deref(), "buffer_id", "parse")?;
                let data = req.data.ok_or_else(|| {
                    tool_err_full(
                        "INVALID_ARGUMENT",
                        "parse 需要 data 参数",
                        Some("把串口收到的整段文本传进来，多行会自动按行解析"),
                        false,
                    )
                })?;

                // 缓冲必须先存在：解析结果无处安放时早失败，避免"解析成功但没写进去"
                self.ctx
                    .waveform
                    .get_status(&buffer_id)
                    .map_err(|e| {
                        tool_err_full(
                            "BUFFER_NOT_FOUND",
                            e.to_string(),
                            Some("先用 action=create 创建该缓冲，或 action=list 查看现有 ID"),
                            false,
                        )
                    })?;

                let mut accepted = 0usize;
                let mut rejected = 0usize;
                let mut errors: Vec<String> = Vec::new();
                for (idx, line) in data.lines().enumerate() {
                    let line = line.trim_end_matches('\r');
                    if line.trim().is_empty() {
                        continue;
                    }
                    match self.ctx.waveform.parse_and_store(&buffer_id, line) {
                        Ok(()) => accepted += 1,
                        Err(e) => {
                            rejected += 1;
                            if errors.len() < 5 {
                                errors.push(format!("第 {} 行: {e}", idx + 1));
                            }
                        }
                    }
                }

                let after = self
                    .ctx
                    .waveform
                    .get_status(&buffer_id)
                    .map_err(|e| tool_err("INVALID_ARGUMENT", e.to_string()))?;
                Ok(Json(WaveformResponse {
                    action: "parse".to_string(),
                    buffer_id: Some(buffer_id),
                    columns: after.column_names,
                    row_count: after.row_count,
                    column_count: after.column_count,
                    capacity: Some(after.capacity),
                    parser_type: after.parser_type.map(parser_type_str),
                    accepted: Some(accepted),
                    rejected: Some(rejected),
                    errors,
                    ..Default::default()
                }))
            }
            "read" => {
                let buffer_id = require_arg(req.buffer_id.as_deref(), "buffer_id", "read")?;
                let rows = req.rows.unwrap_or(200).clamp(1, 5000);
                let data = self.ctx.waveform.read_data(&buffer_id, rows).map_err(|e| {
                    tool_err_full(
                        "BUFFER_NOT_FOUND",
                        e.to_string(),
                        Some("先用 action=list 确认 buffer_id"),
                        false,
                    )
                })?;
                let status = self
                    .ctx
                    .waveform
                    .get_status(&buffer_id)
                    .map_err(|e| tool_err("INVALID_ARGUMENT", e.to_string()))?;
                Ok(Json(WaveformResponse {
                    action: "read".to_string(),
                    buffer_id: Some(buffer_id),
                    column_count: data.columns.len(),
                    columns: data.columns,
                    rows: data.rows,
                    row_count: status.row_count,
                    capacity: Some(status.capacity),
                    parser_type: status.parser_type.map(parser_type_str),
                    ..Default::default()
                }))
            }
            "clear" => {
                let buffer_id = require_arg(req.buffer_id.as_deref(), "buffer_id", "clear")?;
                self.ctx.waveform.clear_buffer(&buffer_id).map_err(|e| {
                    tool_err_full(
                        "BUFFER_NOT_FOUND",
                        e.to_string(),
                        Some("先用 action=list 确认 buffer_id"),
                        false,
                    )
                })?;
                let after = self
                    .ctx
                    .waveform
                    .get_status(&buffer_id)
                    .map_err(|e| tool_err("INVALID_ARGUMENT", e.to_string()))?;
                Ok(Json(WaveformResponse {
                    action: "clear".to_string(),
                    buffer_id: Some(buffer_id),
                    columns: after.column_names,
                    row_count: after.row_count,
                    column_count: after.column_count,
                    capacity: Some(after.capacity),
                    parser_type: after.parser_type.map(parser_type_str),
                    ..Default::default()
                }))
            }
            "status" => {
                let buffer_id = require_arg(req.buffer_id.as_deref(), "buffer_id", "status")?;
                let status = self.ctx.waveform.get_status(&buffer_id).map_err(|e| {
                    tool_err_full(
                        "BUFFER_NOT_FOUND",
                        e.to_string(),
                        Some("先用 action=list 确认 buffer_id"),
                        false,
                    )
                })?;
                Ok(Json(WaveformResponse {
                    action: "status".to_string(),
                    buffer_id: Some(buffer_id),
                    columns: status.column_names,
                    row_count: status.row_count,
                    column_count: status.column_count,
                    capacity: Some(status.capacity),
                    parser_type: status.parser_type.map(parser_type_str),
                    ..Default::default()
                }))
            }
            other => Err(tool_err_full(
                "INVALID_ARGUMENT",
                format!("未知 action: {other}"),
                Some(
                    "支持 list | create | remove | configure_parser | parse | read | clear | status",
                ),
                false,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // P4：协议插件（Lua）
    // -----------------------------------------------------------------------

    #[tool(
        name = "combridge_protocol",
        description = "Lua 协议插件管理。action=list 列出已装插件与状态；get 看单个插件；\
load 从脚本路径加载（path 为 Lua 文件绝对路径）；unload 卸载；enable/disable 启停；\
bind/unbind 把插件绑定到设备（device_id）；bound 查某设备已绑定的插件。\
禁用（disable）只是停止响应事件，插件仍占用脚本与 hook 注册；彻底移除用 unload。",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    async fn protocol(
        &self,
        Parameters(req): Parameters<ProtocolRequest>,
    ) -> ToolR<ProtocolResponse> {
        let action = req.action.to_lowercase();

        match action.as_str() {
            "list" => {
                let plugins = self
                    .ctx
                    .protocols
                    .list_protocols()
                    .map_err(|e| tool_err("PROTOCOL_ERROR", format!("列出插件失败: {e}")))?;
                Ok(Json(ProtocolResponse {
                    action: "list".to_string(),
                    plugins: plugins.iter().map(plugin_view).collect(),
                    plugin: None,
                    message: None,
                }))
            }
            "get" => {
                let plugin_id = require_arg(req.plugin_id.as_deref(), "plugin_id", "get")?;
                let info = self.ctx.protocols.get_plugin(&plugin_id).map_err(|e| {
                    tool_err_full(
                        "PROTOCOL_NOT_FOUND",
                        e.to_string(),
                        Some("先用 action=list 查看已加载的插件 ID"),
                        false,
                    )
                })?;
                Ok(Json(ProtocolResponse {
                    action: "get".to_string(),
                    plugins: Vec::new(),
                    plugin: Some(plugin_view(&info)),
                    message: None,
                }))
            }
            "load" => {
                let plugin_id = require_arg(req.plugin_id.as_deref(), "plugin_id", "load")?;
                let path = require_arg(req.path.as_deref(), "path", "load")?;
                let path_buf = std::path::PathBuf::from(&path);
                if !path_buf.exists() {
                    return Err(tool_err_full(
                        "INVALID_ARGUMENT",
                        format!("脚本文件不存在: {path}"),
                        Some("传入 Lua 脚本的绝对路径（Windows 路径注意反斜杠转义）"),
                        false,
                    ));
                }
                let info = self
                    .ctx
                    .protocols
                    .load_plugin(&plugin_id, path_buf)
                    .map_err(|e| {
                        tool_err_full(
                            "PROTOCOL_ERROR",
                            format!("加载插件失败: {e}"),
                            Some("检查 Lua 语法、require 依赖与 manifest 中的 name/version"),
                            false,
                        )
                    })?;
                Ok(Json(ProtocolResponse {
                    action: "load".to_string(),
                    plugins: Vec::new(),
                    plugin: Some(plugin_view(&info)),
                    message: Some(format!("插件 {plugin_id} 已加载")),
                }))
            }
            "unload" => {
                let plugin_id = require_arg(req.plugin_id.as_deref(), "plugin_id", "unload")?;
                self.ctx.protocols.unload_plugin(&plugin_id).map_err(|e| {
                    tool_err_full(
                        "PROTOCOL_ERROR",
                        format!("卸载插件失败: {e}"),
                        Some("插件可能正在处理事件，稍后重试"),
                        true,
                    )
                })?;
                Ok(Json(ProtocolResponse {
                    action: "unload".to_string(),
                    message: Some(format!("插件 {plugin_id} 已卸载")),
                    ..Default::default()
                }))
            }
            "enable" | "disable" => {
                let plugin_id =
                    require_arg(req.plugin_id.as_deref(), "plugin_id", action.as_str())?;
                if action == "enable" {
                    self.ctx
                        .protocols
                        .enable_plugin(&plugin_id)
                        .map_err(|e| tool_err("PROTOCOL_ERROR", format!("启用插件失败: {e}")))?;
                } else {
                    self.ctx
                        .protocols
                        .disable_plugin(&plugin_id)
                        .map_err(|e| tool_err("PROTOCOL_ERROR", format!("禁用插件失败: {e}")))?;
                }
                let info = self
                    .ctx
                    .protocols
                    .get_plugin(&plugin_id)
                    .map_err(|e| tool_err("PROTOCOL_ERROR", e.to_string()))?;
                Ok(Json(ProtocolResponse {
                    action: action.clone(),
                    plugin: Some(plugin_view(&info)),
                    message: Some(format!(
                        "插件 {plugin_id} 已{}",
                        if action == "enable" {
                            "启用"
                        } else {
                            "禁用"
                        }
                    )),
                    ..Default::default()
                }))
            }
            "bind" => {
                let plugin_id = require_arg(req.plugin_id.as_deref(), "plugin_id", "bind")?;
                let device_id = require_arg(req.device_id.as_deref(), "device_id", "bind")?;
                self.ctx
                    .protocols
                    .bind_protocol(&plugin_id, &device_id)
                    .map_err(|e| {
                        tool_err_full(
                            "PROTOCOL_ERROR",
                            format!("绑定失败: {e}"),
                            Some("设备 ID 来自 combridge_devices；插件需先 load 且 enable"),
                            false,
                        )
                    })?;
                Ok(Json(ProtocolResponse {
                    action: "bind".to_string(),
                    message: Some(format!("插件 {plugin_id} 已绑定到 {device_id}")),
                    ..Default::default()
                }))
            }
            "unbind" => {
                let plugin_id = require_arg(req.plugin_id.as_deref(), "plugin_id", "unbind")?;
                let device_id = require_arg(req.device_id.as_deref(), "device_id", "unbind")?;
                self.ctx
                    .protocols
                    .unbind_protocol(&plugin_id, &device_id)
                    .map_err(|e| tool_err("PROTOCOL_ERROR", format!("解绑失败: {e}")))?;
                Ok(Json(ProtocolResponse {
                    action: "unbind".to_string(),
                    message: Some(format!("插件 {plugin_id} 已与 {device_id} 解绑")),
                    ..Default::default()
                }))
            }
            "bound" => {
                let device_id = require_arg(req.device_id.as_deref(), "device_id", "bound")?;
                let plugins = self
                    .ctx
                    .protocols
                    .get_bound_plugins(&device_id)
                    .map_err(|e| tool_err("PROTOCOL_ERROR", format!("查询绑定关系失败: {e}")))?;
                Ok(Json(ProtocolResponse {
                    action: "bound".to_string(),
                    plugins: plugins.iter().map(plugin_view).collect(),
                    ..Default::default()
                }))
            }
            other => Err(tool_err_full(
                "INVALID_ARGUMENT",
                format!("未知 action: {other}"),
                Some("支持 list | get | load | unload | enable | disable | bind | unbind | bound"),
                false,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // P4：OTA（L3，默认 dry-run + confirm 硬门）
    // -----------------------------------------------------------------------

    #[tool(
        name = "combridge_ota",
        description = "BLE OTA 固件升级（L3 高危，默认只做前置检查）。\
action=protocols 列出可用升级协议；validate 只校验固件包（不连设备）；preflight 只预检设备；\
action=start **不带 confirm 时永远只做 dry-run**：校验包 + 预检设备，返回 plan、warnings 与 confirm_token，\
不会烧录。把该 token 原样放到 confirm 再调用一次 start 才真正执行（令牌一次性、5 分钟过期、\
绑定目标设备+固件路径+镜像 sha256）。action=status 查进度（state/stage/percent）；action=cancel 中止。\
标准档（standard）下 start 会被 TIER_BLOCKED 拒绝，请在 GUI 中操作或改用研发档。",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false
        )
    )]
    async fn ota(
        &self,
        Parameters(req): Parameters<OtaRequest>,
        peer: Peer<RoleServer>,
    ) -> ToolR<OtaResponse> {
        let action = req.action.to_lowercase();

        match action.as_str() {
            "protocols" => {
                let mut resp = OtaResponse::new("protocols");
                resp.protocols = self
                    .ctx
                    .ota
                    .list_protocols()
                    .into_iter()
                    .map(ota_protocol_view)
                    .collect();
                Ok(Json(resp))
            }
            "validate" => {
                let path = require_arg(req.firmware_path.as_deref(), "firmware_path", "validate")?;
                let info = self
                    .ctx
                    .ota
                    .validate_package(req.protocol.as_deref(), &path)
                    .await
                    .map_err(|e| {
                        tool_err_full(
                            "PACKAGE_INVALID",
                            format!("固件包校验失败: {e}"),
                            Some("确认路径存在、协议匹配（见 action=protocols）；.zip 包需含协议要求的镜像"),
                            false,
                        )
                    })?;
                let mut resp = OtaResponse::new("validate");
                resp.package = Some(ota_package_view(&info));
                Ok(Json(resp))
            }
            "preflight" => {
                let address = require_arg(req.address.as_deref(), "address", "preflight")?;
                let info = self
                    .ctx
                    .ota
                    .preflight(req.protocol.as_deref(), &address)
                    .await
                    .map_err(|e| map_ota_error(e, "预检设备失败"))?;
                let mut resp = OtaResponse::new("preflight");
                resp.device = Some(ota_device_view(&info));
                Ok(Json(resp))
            }
            "start" => self.ota_start(req, &peer).await,
            "status" => {
                let status = self.ctx.ota.status();
                let mut resp = OtaResponse::new("status");
                resp.status = Some(ota_status_view(&status));
                Ok(Json(resp))
            }
            "cancel" => {
                self.ctx.ota.cancel().map_err(|e| {
                    tool_err_full(
                        "OTA_NOT_RUNNING",
                        e.to_string(),
                        Some("action=status 确认当前是否有升级在跑"),
                        false,
                    )
                })?;
                let status = self.ctx.ota.status();
                let mut resp = OtaResponse::new("cancel");
                resp.status = Some(ota_status_view(&status));
                Ok(Json(resp))
            }
            other => Err(tool_err_full(
                "INVALID_ARGUMENT",
                format!("未知 action: {other}"),
                Some("支持 protocols | validate | preflight | start | status | cancel"),
                false,
            )),
        }
    }
}

/// GH3036 相关的非工具辅助方法（`#[tool_router]` 之外，避免被宏当作工具处理）。
impl ComBridgeMcp {
    /// 获取或校验 GH3036 租约。
    ///
    /// 带 `lease_id` 时只校验不续期；不带时自动获取 —— GH3036 没有「打开设备」这一步，
    /// 强制先 attach 会让 `factory_test(action=run)` 变成两次往返。
    fn acquire_gh3036_lease(
        &self,
        owner: &str,
        lease_id: Option<&str>,
        force: bool,
        ttl_s: Option<u64>,
    ) -> Result<super::lease::Lease, CallToolResult> {
        if let Some(id) = lease_id {
            self.ctx.leases.validate(GH3036_RESOURCE, id).map_err(|e| {
                tool_err_full(
                    "LEASE_INVALID",
                    e,
                    Some("省略 lease_id 重新获取，或确认占用方已释放"),
                    false,
                )
            })?;
            return self.ctx.leases.current(GH3036_RESOURCE).ok_or_else(|| {
                tool_err_full(
                    "LEASE_INVALID",
                    "GH3036 租约不存在",
                    Some("省略 lease_id 重新获取"),
                    false,
                )
            });
        }

        // 无 lease_id：同一 owner 已在持有则直接复用。
        // GH3036 没有「打开设备」这一步，若不复用，同一客户端的第二次调用会被自己挡住。
        if !force {
            if let Some(existing) = self.ctx.leases.current(GH3036_RESOURCE) {
                if existing.owner == owner {
                    return Ok(existing);
                }
            }
        }

        match self.ctx.leases.acquire(
            GH3036_RESOURCE,
            owner,
            ttl_s.unwrap_or(DEFAULT_TTL_SECS),
            force,
        ) {
            Ok(l) => Ok(l),
            Err(AcquireError::Busy(info)) => Err(tool_err_full(
                "GH3036_BUSY",
                format!(
                    "GH3036 已被 {} 占用（lease={}，{} 过期）",
                    info.current_owner, info.lease_id, info.expires_at
                ),
                Some("等租约过期后重试，或用 force=true 抢占"),
                true,
            )),
        }
    }

    /// 首次使用 GH3036 时惰性初始化（`initialize` 幂等但会重复装回调，故先查状态）。
    fn ensure_gh3036_initialized(&self) -> Result<(), CallToolResult> {
        if self.ctx.gh3036.is_initialized() {
            return Ok(());
        }
        self.ctx.gh3036.initialize().map_err(|e| {
            tool_err_full(
                "GH3036_INIT_FAILED",
                e,
                Some("检查设备连接与协议模块状态后重试 action=init"),
                false,
            )
        })
    }

    /// 取当前 job；没有则造一个临时 job，让 `status` 在首次调用时也能返回合法结构。
    fn current_or_transient_job(&self) -> super::gh3036_job::FactoryJob {
        self.ctx
            .factory_job
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or_else(|| super::gh3036_job::FactoryJob {
                job_id: "ft_unknown".to_string(),
                started_at: super::gh3036_job::now_millis(),
            })
    }

    /// 从 Manager 当前状态构建产测结论。
    fn build_factory_summary(
        &self,
        job: &super::gh3036_job::FactoryJob,
        status: FactoryTestStatus,
        step: FactoryTestStep,
    ) -> super::gh3036_job::FactorySummary {
        let result = self.ctx.gh3036.factory_test_get_result();
        let evaluation = self.ctx.gh3036.get_evaluation_result();
        let csv_path = self.ctx.gh3036.factory_test_csv_path();
        let mut summary = super::gh3036_job::build_summary(
            job,
            status,
            step,
            result.as_ref(),
            evaluation.as_ref(),
            csv_path,
            None,
        );
        // 失败时补上终态消息：UUID/准备步骤这类不在卡控表里的失败
        // 在 items 里没有对应项，只靠 status=failed 模型无法解释原因。
        if matches!(
            status,
            FactoryTestStatus::Failed | FactoryTestStatus::Stopped
        ) {
            if summary.hint.is_none() {
                summary.hint = self.ctx.gh3036.factory_test_last_message();
            }
            if summary.failed_item.is_none() {
                summary.failed_item = Some(super::gh3036_job::step_str(step));
            }
        }
        summary
    }

    /// 危险操作前的用户确认（标准档），研发档跳过。
    ///
    /// 返回 `Ok(())` 才允许改设备；任何"问不到"的情况都按拒绝处理 ——
    /// 确认机制失效时默认关闭，而不是默认放行。
    ///
    /// 客户端声明了 elicitation 就在客户端弹窗，没声明（桌面宿主普遍如此）
    /// 就由 ComBridge 自己的窗口弹，见 [`super::elicit::ask`]。
    ///
    /// `scope` 是授权范围（端口名 / 蓝牙地址 / 产测 owner）：用户在弹窗里勾了
    /// "本次会话内放行"后，同工具同 scope 的后续调用直接放行。
    async fn confirm_if_needed(
        &self,
        peer: &rmcp::Peer<rmcp::RoleServer>,
        tool: &str,
        risk: &str,
        scope: &str,
        detail: impl Into<String>,
    ) -> Result<(), CallToolResult> {
        self.confirm_action(peer, tool, risk, scope, scope, true, detail.into())
            .await
    }

    /// 同上，但可指定 scope 的人读文案与是否允许"记住"。
    #[allow(clippy::too_many_arguments)]
    async fn confirm_action(
        &self,
        peer: &rmcp::Peer<rmcp::RoleServer>,
        tool: &str,
        risk: &str,
        scope: &str,
        scope_label: &str,
        grantable: bool,
        detail: String,
    ) -> Result<(), CallToolResult> {
        if !needs_user_confirmation(&self.ctx.tier_name()) {
            return Ok(());
        }
        let req = super::confirm::Request {
            tool,
            risk,
            scope,
            scope_label,
            detail: &detail,
            grantable,
        };
        match super::elicit::ask(peer, &self.ctx.confirm, req).await {
            d if d.is_accepted() => {
                tracing::info!("[MCP] {tool} 已确认（通道 {}）", d.channel());
                Ok(())
            }
            d => Err(tool_err_full(
                "CONFIRM_REQUIRED",
                d.reason(),
                Some("这是标准档的强制确认：确认框会出现在 ComBridge 窗口中（客户端若支持 elicitation 则在客户端弹窗），请在 120s 内点确认后重试；也可临时把档位切到 dev"),
                false,
            )),
        }
    }

    /// 写审计。审计目录不可用时静默跳过（启动时已告警），绝不阻断业务。
    fn record_audit(
        &self,
        tool: &str,
        args: &serde_json::Map<String, serde_json::Value>,
        result: &CallToolResult,
        started: std::time::Instant,
    ) {
        let text = result_text(result);
        let code = audit::result_code(result.is_error, text);
        self.record_audit_with_code(tool, args, &code, audit::is_dry_run(text), started);
    }

    fn record_audit_with_code(
        &self,
        tool: &str,
        args: &serde_json::Map<String, serde_json::Value>,
        code: &str,
        dry_run: bool,
        started: std::time::Instant,
    ) {
        let Some(logger) = self.ctx.audit.as_ref() else {
            return;
        };
        logger.record(&audit::AuditEntry {
            ts: audit::now_millis(),
            tier: self.ctx.tier_name(),
            tool: tool.to_string(),
            args: audit::digest_args(args),
            result_code: code.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            dry_run,
        });
    }

    /// `combridge_ota(action=start)`：dry-run 与真实执行的分流点。
    ///
    /// 判定顺序刻意固定：**先校验参数与固件包，再判令牌** ——
    /// 这样即使令牌有效、包却已经损坏（换了文件），也不会走到烧录。
    async fn ota_start(&self, req: OtaRequest, peer: &Peer<RoleServer>) -> ToolR<OtaResponse> {
        let address = require_arg(req.address.as_deref(), "address", "start")?;
        let firmware_path = require_arg(req.firmware_path.as_deref(), "firmware_path", "start")?;

        // 1) 固件包校验（本地，只读）
        let package = self
            .ctx
            .ota
            .validate_package(req.protocol.as_deref(), &firmware_path)
            .await
            .map_err(|e| {
                tool_err_full(
                    "PACKAGE_INVALID",
                    format!("固件包校验失败: {e}"),
                    Some("先用 action=validate 单独排查；确认包路径与协议匹配"),
                    false,
                )
            })?;

        // 2) 设备预检（连设备，非破坏性）。失败不阻断 dry-run，但要如实告警：
        //    模型需要的正是这条信息去提示用户"设备还没连上"。
        let mut warnings: Vec<String> = Vec::new();
        let device = if req.check_device.unwrap_or(true) {
            match self
                .ctx
                .ota
                .preflight(req.protocol.as_deref(), &address)
                .await
            {
                Ok(info) => Some(info),
                Err(e) => {
                    warnings.push(format!("设备预检未通过：{e}"));
                    None
                }
            }
        } else {
            warnings.push("已跳过设备预检（check_device=false），目标设备状态未知".to_string());
            None
        };

        let protocol_id = package.protocol.clone();
        let plan = OtaPlanView {
            protocol: protocol_id.clone(),
            target: address.clone(),
            firmware_path: firmware_path.clone(),
            image_name: package.image_name.clone(),
            image_size: package.image_size,
            image_sha256: package.image_sha256.clone(),
            est_seconds: 8 + package.image_size.div_ceil(12 * 1024),
            device: device.as_ref().map(ota_device_view),
        };

        // 3) 令牌校验：不带 confirm = 正常 dry-run；带了但无效 = 明确报错并给新令牌
        let Some(token) = req.confirm.as_deref() else {
            return Ok(Json(self.issue_ota_dry_run(
                &address,
                &firmware_path,
                package,
                plan,
                warnings,
            )));
        };

        {
            let normalized_protocol = Some(protocol_id.as_str());
            if let Err(e) = self.ctx.ota_confirm.consume(
                token,
                normalized_protocol,
                &address,
                &firmware_path,
                &package.image_sha256,
            ) {
                let fresh = self.ctx.ota_confirm.issue(
                    Some(protocol_id.clone()),
                    &address,
                    &firmware_path,
                    &package.image_sha256,
                );
                let mut payload = serde_json::json!({
                    "code": e.code(),
                    "message": e.describe(),
                    "hint": "确认没有烧录。核对 plan 与目标设备后，用返回的新 confirm_token 重新调用 start",
                    "retryable": true,
                    "dry_run": true,
                    "confirm_token": fresh.token,
                    "confirm_expires_in": fresh.remaining_secs(),
                });
                if let Some(p) = payload.as_object_mut() {
                    p.insert(
                        "plan".to_string(),
                        serde_json::to_value(&plan).unwrap_or_default(),
                    );
                }
                return Err(CallToolResult::error(vec![ContentBlock::text(
                    payload.to_string(),
                )]));
            }
        }

        // 4) 令牌有效 → 标准档再要一次用户确认（L3 双确认）→ 真正烧录
        //    烧录不接受"记住"：每次都得有人点。
        self.confirm_action(
            peer,
            "combridge_ota",
            "OTA 烧录",
            plan.target.as_str(),
            plan.target.as_str(),
            false,
            format!(
                "即将把 {}（{} 字节, 协议 {}）烧录到 {}\nsha256: {}",
                plan.image_name,
                plan.image_size,
                plan.protocol,
                plan.target,
                &plan.image_sha256[..plan.image_sha256.len().min(16)]
            ),
        )
        .await?;

        let options = OtaStartOptions {
            force_confirm: req.force_confirm.unwrap_or(false),
            upgrade_only: req.upgrade_only.unwrap_or(false),
            route: req.route.clone(),
            transfer_mode: req.transfer_mode.clone(),
            bank_mode: req.bank_mode.clone(),
            target_address: req.target_address.clone(),
            run_immediately: req.run_immediately.or(Some(true)),
        };

        let status = self
            .ctx
            .ota
            .start(Some(protocol_id.clone()), address, firmware_path, options)
            .await
            .map_err(|e| map_ota_error(e, "启动升级失败"))?;

        let mut resp = OtaResponse::new("start");
        resp.dry_run = Some(false);
        resp.status = Some(ota_status_view(&status));
        resp.warnings = warnings;
        Ok(Json(resp))
    }

    /// 签发 dry-run 响应（含新令牌）。
    ///
    /// 令牌里固定写**已解析的协议 ID**（`package.protocol`），而不是调用方原样传入的字符串 ——
    /// 否则 `protocol="GOODIX"` 这类大小写差异会让正确令牌在确认时被判为不一致。
    fn issue_ota_dry_run(
        &self,
        address: &str,
        firmware_path: &str,
        package: OtaPackageInfo,
        plan: OtaPlanView,
        warnings: Vec<String>,
    ) -> OtaResponse {
        let pending = self.ctx.ota_confirm.issue(
            Some(package.protocol.clone()),
            address,
            firmware_path,
            &package.image_sha256,
        );
        OtaResponse {
            action: "start".to_string(),
            dry_run: Some(true),
            protocols: Vec::new(),
            package: Some(ota_package_view(&package)),
            device: plan.device.clone(),
            plan: Some(plan),
            confirm_token: Some(pending.token.clone()),
            confirm_expires_in: Some(pending.remaining_secs()),
            warnings,
            status: None,
        }
    }
}

// ---------------------------------------------------------------------------
// P4 视图转换与参数辅助
// ---------------------------------------------------------------------------

/// 必填参数校验，失败信息里带上 action，模型才知道是哪一步缺参数。
fn require_arg(value: Option<&str>, name: &str, action: &str) -> Result<String, CallToolResult> {
    match value {
        Some(v) if !v.trim().is_empty() => Ok(v.to_string()),
        _ => Err(tool_err_full(
            "INVALID_ARGUMENT",
            format!("action={action} 缺少必填参数 {name}"),
            None,
            false,
        )),
    }
}

fn default_columns() -> Vec<String> {
    (0..5).map(|i| format!("CH{i}")).collect()
}

/// 标准档是否需要为写设备 / 产测 / 烧录弹确认框。
///
/// 抽成自由函数是为了能单测档位语义 —— `confirm_if_needed` 需要整个 `McpContext` 与 peer，
/// 而"哪种档位要确认"只依赖字符串。
fn needs_user_confirmation(tier: &str) -> bool {
    tier != "dev"
}

/// 取工具结果里的第一段文本（结构化错误/成功体都放在这里）。
fn result_text(result: &CallToolResult) -> Option<&str> {
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
}

/// 确认弹窗里的载荷预览：可打印内容原样展示，二进制转 hex，最多 `max` 字节。
///
/// 用户在弹窗里看到 "41 54 2B 52 53 54" 才知道自己要放行的是什么字节；
/// 只写"128 字节"等于没让人确认。
fn preview_payload(bytes: &[u8], max: usize) -> String {
    let head = &bytes[..bytes.len().min(max)];
    let printable = head
        .iter()
        .all(|b| b.is_ascii_graphic() || matches!(*b, b' ' | b'\r' | b'\n' | b'\t'));
    let body = if printable {
        String::from_utf8_lossy(head).escape_debug().to_string()
    } else {
        head.iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    if bytes.len() > head.len() {
        format!("{body} …（共 {} 字节）", bytes.len())
    } else {
        body
    }
}

/// 限流拒绝 → 工具级错误（模型能读懂、能据此放慢节奏）。
fn reject_result(tool: &str, reject: Reject) -> CallToolResult {
    match reject {
        Reject::Qps { retry_after_ms } => tool_err_full(
            "RATE_LIMITED",
            format!("调用过快：{tool} 需等待约 {retry_after_ms}ms"),
            Some("放慢调用频率；多行数据建议合并成一次调用（如 waveform parse 一次传多行）"),
            true,
        ),
        Reject::Busy { limit } => tool_err_full(
            "RATE_LIMITED",
            format!("{tool} 并发已达上限 {limit}"),
            Some("等当前等待结束后再发起；串口数据可用同一次 expect 的返回值，不必并发等待"),
            true,
        ),
    }
}

/// 把工具参数里的解析器描述转成 `ParserConfig`，并做模式相关的必填校验。
fn build_parser_config(spec: ParserSpec) -> Result<ParserConfig, CallToolResult> {
    let parser_type = match spec.parser_type.to_ascii_lowercase().as_str() {
        "delimiter" => ParserType::Delimiter,
        "regex" => ParserType::Regex,
        other => {
            return Err(tool_err_full(
                "INVALID_ARGUMENT",
                format!("未知 parser_type: {other}"),
                Some("支持 delimiter | regex"),
                false,
            ))
        }
    };

    if parser_type == ParserType::Delimiter && spec.delimiter.is_none() {
        // 默认逗号，与 GUI 一致
    }
    if parser_type == ParserType::Regex && spec.pattern.is_none() {
        return Err(tool_err_full(
            "INVALID_ARGUMENT",
            "regex 解析器需要 pattern",
            Some("如 {\"parser_type\":\"regex\",\"pattern\":\"CH0=(\\\\d+)\"}"),
            false,
        ));
    }

    Ok(ParserConfig {
        parser_type,
        delimiter: spec.delimiter.or_else(|| Some(",".to_string())),
        pattern: spec.pattern,
        column_names: spec.column_names.unwrap_or_else(default_columns),
        trim_whitespace: spec.trim_whitespace.unwrap_or(true),
    })
}

fn parser_type_str(t: ParserType) -> String {
    match t {
        ParserType::Delimiter => "delimiter".to_string(),
        ParserType::Regex => "regex".to_string(),
    }
}

fn plugin_state_str(state: PluginState) -> String {
    match state {
        PluginState::Unloaded => "unloaded",
        PluginState::Loaded => "loaded",
        PluginState::Enabled => "enabled",
        PluginState::Disabled => "disabled",
        PluginState::Error => "error",
    }
    .to_string()
}

fn plugin_view(info: &PluginInfo) -> PluginView {
    PluginView {
        id: info.id.clone(),
        name: info.name.clone(),
        version: info.version.clone(),
        description: info.description.clone(),
        author: info.author.clone(),
        path: info.path.display().to_string(),
        state: plugin_state_str(info.state),
        hooks: info.hooks.clone(),
        bound_devices: info.bound_devices.clone(),
        error: info.error_message.clone(),
    }
}

fn ota_protocol_view(p: OtaProtocolDescriptor) -> OtaProtocolView {
    OtaProtocolView {
        id: p.id,
        name: p.name,
        description: p.description,
    }
}

fn ota_kv_view(items: &[crate::ota::OtaKeyValue]) -> Vec<OtaKeyValueView> {
    items
        .iter()
        .map(|kv| OtaKeyValueView {
            key: kv.key.clone(),
            value: kv.value.clone(),
        })
        .collect()
}

fn ota_package_view(p: &OtaPackageInfo) -> OtaPackageView {
    OtaPackageView {
        path: p.path.clone(),
        file_name: p.file_name.clone(),
        package_size: p.package_size,
        protocol: p.protocol.clone(),
        image_name: p.image_name.clone(),
        image_size: p.image_size,
        image_sha256: p.image_sha256.clone(),
        dual_bank_address: p.dual_bank_address.clone(),
        fields: ota_kv_view(&p.fields),
    }
}

fn ota_device_view(d: &OtaDeviceInfo) -> OtaDeviceView {
    OtaDeviceView {
        address: d.address.clone(),
        protocol: d.protocol.clone(),
        name: d.name.clone(),
        mode: d.mode.clone(),
        mtu: d.mtu,
        version: d.version.clone(),
        fields: ota_kv_view(&d.fields),
    }
}

fn ota_state_str(state: OtaState) -> String {
    match state {
        OtaState::Idle => "idle",
        OtaState::Running => "running",
        OtaState::Success => "success",
        OtaState::Failed => "failed",
        OtaState::Cancelled => "cancelled",
    }
    .to_string()
}

fn ota_stage_str(stage: OtaStage) -> String {
    match stage {
        OtaStage::Idle => "idle",
        OtaStage::Unpacking => "unpacking",
        OtaStage::Validating => "validating",
        OtaStage::Preparing => "preparing",
        OtaStage::Transferring => "transferring",
        OtaStage::Activating => "activating",
        OtaStage::Rebooting => "rebooting",
        OtaStage::Completed => "completed",
        OtaStage::Failed => "failed",
    }
    .to_string()
}

fn ota_status_view(s: &OtaStatusDto) -> OtaStatusView {
    OtaStatusView {
        state: ota_state_str(s.state),
        stage: ota_stage_str(s.stage),
        protocol: s.protocol.clone(),
        address: s.address.clone(),
        firmware_path: s.firmware_path.clone(),
        current: s.current,
        total: s.total,
        percent: s.percent,
        message: s.message.clone(),
        error: s.error.clone(),
        updated_at: s.updated_at,
    }
}

/// OTA 域错误统一出口：把协议层的原始错误包成模型能照着做的提示。
fn map_ota_error(e: impl std::fmt::Display, what: &str) -> CallToolResult {
    tool_err_full(
        "OTA_ERROR",
        format!("{what}: {e}"),
        Some(
            "确认目标设备已通过 combridge_ble_connect 连接（地址见 combridge_ble_scan）；\
若提示通道正忙，说明另有预检/升级在进行，可 action=status 查看",
        ),
        false,
    )
}

#[tool_handler]
impl ServerHandler for ComBridgeMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "ComBridge 串口/BLE/GH3036/OTA 调试工具的 MCP 服务。\
【串口】combridge_ports 发现端口 → combridge_serial_open 打开并获取租约 → combridge_serial_write 发送 → \
combridge_serial_expect 等待响应（或 combridge_serial_read 游标读取）→ combridge_serial_close 释放。\
写/关操作需携带 combridge_serial_open 返回的 lease_id；端口被其他客户端占用时返回 PORT_BUSY。\
【BLE】combridge_ble_scan 扫描 → combridge_ble_connect 连接（lease 资源 ble:<address>）→ \
combridge_ble_gatt 发现服务/读写/订阅；combridge_ble_gatt(action=notifications) 按游标取通知数据。\
【GH3036】combridge_gh3036_rpc(action=set_channel) 先绑通道 → action=rpc 执行命令（命令键见 action=catalog）；\
combridge_gh3036_factory_test(action=run) 一次调用直接产出产测结论（pass/failed_item/csv_path），\
只有超时才返回 status=running 需用 action=status 续等；停在待切换环境时用 action=continue。\
【数据】combridge_waveform 把串口文本解析为数值行（create → configure_parser → parse → read）；\
combridge_protocol 管理 Lua 协议插件；combridge_about(action=logs) 读日志尾部用于自查。\
【OTA】combridge_ota(action=start) 默认只做 dry-run 并返回 confirm_token，\
把令牌放进 confirm 再调一次才真正烧录（一次性、5 分钟、绑定设备+包+镜像 sha256）。\
标准档下写设备 / 产测 / 烧录会先向客户端请求用户确认，拒绝或无法确认时返回 CONFIRM_REQUIRED。\
工具失败时返回 isError=true 的结构化错误 {code, message, hint, retryable}：先读 hint 再决定是否重试。",
            )
            .with_server_info(
                Implementation::new("combridge", env!("CARGO_PKG_VERSION"))
                    .with_title("ComBridge")
                    .with_description("ComBridge cross-platform serial/BLE debugging tool"),
            )
    }

    /// 所有工具调用的统一入口：限流 → 熔断 → 执行 → 审计。
    ///
    /// `#[tool_handler]` 见 `call_tool` 已存在就不再生成，因此这里是唯一收口点 ——
    /// 审计与限流不可能被某个工具"忘记"。
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResponse, rmcp::ErrorData> {
        let tool = request.name.to_string();
        let args = request.arguments.clone().unwrap_or_default();
        let started = std::time::Instant::now();

        // 1) 限流 + 并发上限
        let guard = match self.ctx.limiter.admit(&tool) {
            Ok(guard) => guard,
            Err(reject) => {
                let result = reject_result(&tool, reject);
                self.record_audit(&tool, &args, &result, started);
                return Ok(result.into());
            }
        };

        // 2) 端口熔断：坏线缆 + 重试循环不该反复打硬件
        let port = args
            .get("port_name")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        if let Some(port) = port.as_deref() {
            if let Some(secs) = self.ctx.breaker.suspended_for(port) {
                let result = tool_err_full(
                    "PORT_SUSPENDED",
                    format!("{port} 已连续 5 次硬件错误，暂停使用 {secs}s"),
                    Some("检查线缆 / 波特率 / 设备供电；等待冷却后重试，或换一个端口"),
                    true,
                );
                self.record_audit(&tool, &args, &result, started);
                return Ok(result.into());
            }
        }

        // 3) 执行
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        let outcome = Self::tool_router().call(tcc).await;

        // 4) 审计 + 熔断计数
        match &outcome {
            Ok(rmcp::model::CallToolResponse::Complete(result)) => {
                self.record_audit(&tool, &args, result, started);
                if let Some(port) = port.as_deref() {
                    let text = result_text(result);
                    let code = audit::result_code(result.is_error, text);
                    if code == "SERIAL_ERROR" {
                        if self.ctx.breaker.record_failure(port) {
                            tracing::warn!("[MCP] 端口 {port} 连续硬件错误，熔断 30s");
                        }
                    } else if result.is_error != Some(true) {
                        self.ctx.breaker.record_success(port);
                    }
                }
            }
            Ok(_) => {
                // InputRequired / Task 分支：当前没有工具走这两条路径，留个兜底记录
                self.record_audit_with_code(&tool, &args, "OK", false, started);
            }
            Err(e) => {
                self.record_audit_with_code(&tool, &args, "PROTOCOL_ERROR", false, started);
                tracing::warn!("[MCP] 工具 {tool} 返回协议错误: {e}");
            }
        }

        drop(guard);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 工具错误必须是 `isError: true` 的结果而非协议错误，
    /// 否则客户端会渲染成 "internal error"，模型读不到错误内容。
    #[test]
    fn test_tool_err_is_error_result() {
        let e = tool_err("PORT_BUSY", "COM3 被 claude-code 持有");
        assert_eq!(e.is_error, Some(true), "工具错误必须标记 isError");
        let raw = serde_json::to_string(&e.content).unwrap();
        assert!(raw.contains("PORT_BUSY"), "错误码应出现在内容中: {raw}");
        assert!(raw.contains("claude-code"), "应带占用者信息: {raw}");
    }

    /// 错误结构须与文档 §6.6 一致：code / message / hint / retryable 四字段。
    #[test]
    fn test_tool_err_full_shape() {
        let e = tool_err_full("PORT_BUSY", "占用中", Some("用 force=true 重试"), true);
        assert_eq!(e.is_error, Some(true));
        let raw = serde_json::to_string(&e.content).unwrap();
        for k in ["code", "message", "hint", "retryable"] {
            assert!(raw.contains(k), "缺字段 {k}: {raw}");
        }
        assert!(raw.contains("force=true"), "hint 应进入内容: {raw}");
    }

    /// 工具名与 schema 由宏生成，此项可防止改名/漏注册。
    #[test]
    fn test_tool_names_registered() {
        let names = [
            (
                ComBridgeMcp::about_tool_attr().name.to_string(),
                "combridge_about",
            ),
            (
                ComBridgeMcp::ports_tool_attr().name.to_string(),
                "combridge_ports",
            ),
            (
                ComBridgeMcp::combridge_serial_open_tool_attr()
                    .name
                    .to_string(),
                "combridge_serial_open",
            ),
            (
                ComBridgeMcp::combridge_serial_close_tool_attr()
                    .name
                    .to_string(),
                "combridge_serial_close",
            ),
            (
                ComBridgeMcp::combridge_serial_write_tool_attr()
                    .name
                    .to_string(),
                "combridge_serial_write",
            ),
            (
                ComBridgeMcp::combridge_serial_read_tool_attr()
                    .name
                    .to_string(),
                "combridge_serial_read",
            ),
            (
                ComBridgeMcp::combridge_serial_expect_tool_attr()
                    .name
                    .to_string(),
                "combridge_serial_expect",
            ),
            (
                ComBridgeMcp::devices_tool_attr().name.to_string(),
                "combridge_devices",
            ),
            (
                ComBridgeMcp::ble_scan_tool_attr().name.to_string(),
                "combridge_ble_scan",
            ),
            (
                ComBridgeMcp::ble_connect_tool_attr().name.to_string(),
                "combridge_ble_connect",
            ),
            (
                ComBridgeMcp::ble_gatt_tool_attr().name.to_string(),
                "combridge_ble_gatt",
            ),
            (
                ComBridgeMcp::gh3036_rpc_tool_attr().name.to_string(),
                "combridge_gh3036_rpc",
            ),
            (
                ComBridgeMcp::gh3036_factory_test_tool_attr()
                    .name
                    .to_string(),
                "combridge_gh3036_factory_test",
            ),
            (
                ComBridgeMcp::waveform_tool_attr().name.to_string(),
                "combridge_waveform",
            ),
            (
                ComBridgeMcp::protocol_tool_attr().name.to_string(),
                "combridge_protocol",
            ),
            (
                ComBridgeMcp::ota_tool_attr().name.to_string(),
                "combridge_ota",
            ),
        ];
        for (actual, expected) in names {
            assert_eq!(actual, expected);
        }
    }

    /// 每个带参工具的 input schema 应包含必填字段。
    #[test]
    fn test_tool_schemas() {
        let open = ComBridgeMcp::combridge_serial_open_tool_attr();
        let schema = serde_json::to_string(&*open.input_schema).unwrap();
        assert!(schema.contains("port_name"));
        assert!(schema.contains("baud_rate"));

        let read = ComBridgeMcp::combridge_serial_read_tool_attr();
        assert!(serde_json::to_string(&*read.input_schema)
            .unwrap()
            .contains("since"));

        let expect = ComBridgeMcp::combridge_serial_expect_tool_attr();
        let schema = serde_json::to_string(&*expect.input_schema).unwrap();
        assert!(schema.contains("pattern"));
        assert!(schema.contains("timeout_ms"));

        // BLE 域
        let gatt = ComBridgeMcp::ble_gatt_tool_attr();
        let schema = serde_json::to_string(&*gatt.input_schema).unwrap();
        assert!(schema.contains("action"));
        assert!(schema.contains("char_uuid"));
        assert!(schema.contains("lease_id"));

        let connect = ComBridgeMcp::ble_connect_tool_attr();
        assert!(serde_json::to_string(&*connect.input_schema)
            .unwrap()
            .contains("force"));

        let scan = ComBridgeMcp::ble_scan_tool_attr();
        assert!(serde_json::to_string(&*scan.input_schema)
            .unwrap()
            .contains("duration_ms"));

        // GH3036 域
        let rpc = ComBridgeMcp::gh3036_rpc_tool_attr();
        let schema = serde_json::to_string(&*rpc.input_schema).unwrap();
        assert!(schema.contains("command_key"));
        assert!(schema.contains("channel_type"));
        assert!(schema.contains("lease_id"));

        let ft = ComBridgeMcp::gh3036_factory_test_tool_attr();
        let schema = serde_json::to_string(&*ft.input_schema).unwrap();
        assert!(schema.contains("max_wait_s"));
        assert!(schema.contains("config_dir"));

        // P4 域
        let wf = ComBridgeMcp::waveform_tool_attr();
        let schema = serde_json::to_string(&*wf.input_schema).unwrap();
        assert!(schema.contains("buffer_id"));
        assert!(schema.contains("parser"));
        assert!(schema.contains("rows"));

        let proto = ComBridgeMcp::protocol_tool_attr();
        let schema = serde_json::to_string(&*proto.input_schema).unwrap();
        assert!(schema.contains("plugin_id"));
        assert!(schema.contains("device_id"));

        let ota = ComBridgeMcp::ota_tool_attr();
        let schema = serde_json::to_string(&*ota.input_schema).unwrap();
        for field in ["confirm", "firmware_path", "address", "bank_mode"] {
            assert!(schema.contains(field), "OTA schema 缺少 {field}");
        }

        // about 的 logs 子命令（P4）
        let about = ComBridgeMcp::about_tool_attr();
        let schema = serde_json::to_string(&*about.input_schema).unwrap();
        assert!(schema.contains("action"));
        assert!(schema.contains("lines"));
    }

    #[test]
    fn test_parser_config_built_from_spec() {
        let cfg = build_parser_config(ParserSpec {
            parser_type: "delimiter".to_string(),
            delimiter: Some(";".to_string()),
            pattern: None,
            column_names: Some(vec!["A".to_string(), "B".to_string()]),
            trim_whitespace: None,
        })
        .unwrap();
        assert_eq!(cfg.parser_type, ParserType::Delimiter);
        assert_eq!(cfg.delimiter.as_deref(), Some(";"));
        assert!(cfg.trim_whitespace);
        assert_eq!(cfg.column_names.len(), 2);

        // regex 缺 pattern 直接拒绝
        assert!(build_parser_config(ParserSpec {
            parser_type: "regex".to_string(),
            delimiter: None,
            pattern: None,
            column_names: None,
            trim_whitespace: None,
        })
        .is_err());

        // 未知类型
        assert!(build_parser_config(ParserSpec {
            parser_type: "csv".to_string(),
            delimiter: None,
            pattern: None,
            column_names: None,
            trim_whitespace: None,
        })
        .is_err());
    }

    #[test]
    fn test_require_arg() {
        assert_eq!(require_arg(Some("COM3"), "port", "open").unwrap(), "COM3");
        assert!(require_arg(None, "port", "open").is_err());
        assert!(require_arg(Some("   "), "port", "open").is_err());
    }

    /// 标准档必须为准入型操作弹确认；研发档跳过（内部联调不应每次点弹窗）。
    #[test]
    fn test_confirmation_policy_by_tier() {
        assert!(!needs_user_confirmation("dev"));
        assert!(needs_user_confirmation("standard"));
        assert!(needs_user_confirmation(""));
    }

    /// 确认弹窗里给用户看的必须是人能判断的内容，而不是"128 字节"。
    #[test]
    fn test_preview_payload() {
        assert_eq!(preview_payload(b"AT+RST\r\n", 32), "AT+RST\\r\\n");
        assert_eq!(preview_payload(&[0xAA, 0xBB], 32), "AA BB");
        let long = vec![b'x'; 40];
        let shown = preview_payload(&long, 8);
        assert!(shown.starts_with("xxxxxxxx"), "{shown}");
        assert!(shown.contains("共 40 字节"), "{shown}");
    }

    /// 限流错误也必须是 isError 结果，带上可执行的等待建议。
    #[test]
    fn test_reject_result_is_tool_error() {
        let r = reject_result("combridge_ports", Reject::Qps { retry_after_ms: 50 });
        assert_eq!(r.is_error, Some(true));
        let raw = serde_json::to_string(&r.content).unwrap();
        assert!(raw.contains("RATE_LIMITED"), "{raw}");
        assert!(raw.contains("50"), "{raw}");

        let busy = reject_result("combridge_serial_expect", Reject::Busy { limit: 8 });
        let raw = serde_json::to_string(&busy.content).unwrap();
        assert!(raw.contains("并发已达上限 8"), "{raw}");
    }

    #[test]
    fn test_ota_view_maps_state() {
        let view = ota_status_view(&OtaStatusDto::idle());
        assert_eq!(view.state, "idle");
        assert_eq!(view.stage, "idle");
        assert_eq!(view.percent, 0.0);
    }

    #[test]
    fn test_printable_text_and_hex() {
        assert_eq!(printable_text(b"OK\r\n").as_deref(), Some("OK"));
        assert_eq!(printable_text(&[0x00, 0x01]), None);
        assert_eq!(printable_text(b""), None);
        assert_eq!(hex_encode(&[0xde, 0xad]), "DEAD");
    }

    #[test]
    fn test_hex_roundtrip() {
        let bytes = decode_bytes("AA BB 01", "hex").unwrap();
        assert_eq!(bytes, vec![0xAA, 0xBB, 0x01]);
        assert_eq!(encode_bytes(&bytes, "hex").unwrap(), "AA BB 01");

        // 冒号 / 逗号 / 无分隔符均可
        assert_eq!(decode_bytes("aa:bb,01", "hex").unwrap(), bytes);
        assert_eq!(decode_bytes("AABB01", "hex").unwrap(), bytes);
    }

    #[test]
    fn test_hex_invalid() {
        assert!(decode_bytes("AAB", "hex").is_err());
        assert!(decode_bytes("ZZ", "hex").is_err());
        assert!(decode_bytes("00", "base64").is_err());
    }

    #[test]
    fn test_text_encoding_lossy() {
        // 非法 UTF-8 在 text 模式下 lossy 输出，不 panic
        let out = encode_bytes(&[0xFF, 0xFE], "text").unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn test_param_mapping() {
        assert!(map_baud(115200).is_ok());
        assert!(map_baud(12345).is_err());
        assert!(map_data_bits(8).is_ok());
        assert!(map_data_bits(9).is_err());
        assert!(map_parity("EVEN").is_ok());
        assert!(map_parity("mark").is_err());
        assert!(map_stop_bits(2).is_ok());
        assert!(map_flow("hardware").is_ok());
    }
}
