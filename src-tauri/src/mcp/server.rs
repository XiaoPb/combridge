//! MCP Server 定义与工具注册。
//!
//! P0：`combridge_about`。
//! P1：串口域 —— `combridge_ports` / `combridge_serial_open` / `combridge_serial_close` /
//! `combridge_serial_write` / `combridge_serial_read` / `combridge_serial_expect`。
//!
//! 设计约定（docs/mcp-server-plan.md §5、§6）：
//! - 工具名 `combridge_` / `serial_` 前缀，域内不做 1:1 命令映射
//! - 独占资源操作必须持有效租约（`lease_id`），PORT_BUSY 时返回占用者信息
//! - 读类工具（ports/read/expect）不需要租约：数据经 hub 对所有人可见
//! - 注解：只读工具标 `read_only_hint = true`，写/关设备标 `destructive_hint = true`

use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::{schemars, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::ctx::McpContext;
use super::lease::{AcquireError, DEFAULT_TTL_SECS};
use crate::device::serial::serial_config::{
    BaudRate, DataBits, FlowControl, Parity, SerialPortConfig, StopBits,
};

/// `combridge_about` 的返回结构。
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AboutResponse {
    /// ComBridge 应用版本
    pub app_version: String,
    /// 安全档位："dev"（研发档）| "standard"（标准档）
    pub tier: String,
    /// 服务启动时间（毫秒时间戳）
    pub started_at: u64,
    /// 当前已连接设备快照
    pub connected_devices: Vec<serde_json::Value>,
    /// 可用工具域
    pub planned_tool_domains: Vec<String>,
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
fn mcp_err(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

fn map_baud(b: u32) -> Result<BaudRate, McpError> {
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
        _ => return Err(mcp_err(format!("不支持的波特率: {b}（支持 1200~921600 标准值）"))),
    })
}

fn map_data_bits(n: u32) -> Result<DataBits, McpError> {
    Ok(match n {
        5 => DataBits::Five,
        6 => DataBits::Six,
        7 => DataBits::Seven,
        8 => DataBits::Eight,
        _ => return Err(mcp_err(format!("不支持的数据位: {n}（5/6/7/8）"))),
    })
}

fn map_parity(s: &str) -> Result<Parity, McpError> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "none" | "" => Parity::None,
        "odd" => Parity::Odd,
        "even" => Parity::Even,
        other => return Err(mcp_err(format!("不支持的校验位: {other}（none/odd/even）"))),
    })
}

fn map_stop_bits(n: u32) -> Result<StopBits, McpError> {
    Ok(match n {
        1 => StopBits::One,
        2 => StopBits::Two,
        _ => return Err(mcp_err(format!("不支持的停止位: {n}（1/2）"))),
    })
}

fn map_flow(s: &str) -> Result<FlowControl, McpError> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "none" | "" => FlowControl::None,
        "software" => FlowControl::Software,
        "hardware" => FlowControl::Hardware,
        other => return Err(mcp_err(format!("不支持的流控: {other}（none/software/hardware）"))),
    })
}

fn encode_bytes(bytes: &[u8], encoding: &str) -> Result<String, McpError> {
    match encoding {
        "text" => Ok(String::from_utf8_lossy(bytes).into_owned()),
        "hex" => Ok(bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")),
        other => Err(mcp_err(format!(
            "未知编码: {other}（支持 text | hex）"
        ))),
    }
}

fn decode_bytes(data: &str, encoding: &str) -> Result<Vec<u8>, McpError> {
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
        other => Err(mcp_err(format!(
            "未知编码: {other}（支持 text | hex）"
        ))),
    }
}

fn lease_to_view(lease: &super::lease::Lease) -> LeaseView {
    LeaseView {
        lease_id: lease.lease_id.clone(),
        owner: lease.owner.clone(),
        expires_at: lease.expires_at,
    }
}

/// 等待匹配用的文本匹配器：输入累计文本，命中返回片段。
type Matcher = Box<dyn Fn(&str) -> Option<String> + Send>;

#[tool_router]
impl ComBridgeMcp {
    #[tool(
        name = "combridge_about",
        description = "获取 ComBridge 服务概况：应用版本、安全档位、已连接设备快照与可用工具域。\
首次连接时调用一次即可；设备状态变化后可重新调用获取最新快照。",
        annotations(read_only_hint = true)
    )]
    async fn about(&self) -> Result<Json<AboutResponse>, McpError> {
        let connected_devices = self.ctx.connected_devices().await;
        Ok(Json(AboutResponse {
            app_version: self.ctx.app_version.clone(),
            tier: self.ctx.tier.clone(),
            started_at: self.ctx.started_at,
            connected_devices,
            planned_tool_domains: vec![
                "serial".to_string(),
                "ble".to_string(),
                "devices".to_string(),
                "gh3036".to_string(),
                "waveform".to_string(),
                "protocol".to_string(),
                "ota".to_string(),
            ],
        }))
    }

    #[tool(
        name = "combridge_ports",
        description = "列出系统串口：物理端口信息（制造商/产品/序列号）、是否已打开、当前租约持有者。\
操作串口前必须先调用本工具确认端口存在且可用。",
        annotations(read_only_hint = true)
    )]
    async fn ports(&self) -> Result<Json<PortsResponse>, McpError> {
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
        annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = false)
    )]
    async fn combridge_serial_open(
        &self,
        Parameters(req): Parameters<SerialOpenRequest>,
    ) -> Result<Json<SerialOpenResponse>, McpError> {
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
                return Err(mcp_err(format!(
                    "PORT_BUSY: 串口 {} 的租约被 {} 持有（lease={}，{} 过期）。\
可等待其过期，或用 force=true 抢占（会踢出对方）。",
                    req.port_name, info.current_owner, info.lease_id, info.expires_at
                )));
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
                return Err(mcp_err(format!("打开串口 {} 失败: {e}", req.port_name)));
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
    ) -> Result<Json<SerialCloseResponse>, McpError> {
        self.ctx
            .leases
            .validate(&req.port_name, &req.lease_id)
            .map_err(mcp_err)?;

        let opened_by_mcp = self
            .ctx
            .opened_by_mcp
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&req.port_name);

        if !opened_by_mcp {
            return Err(mcp_err(format!(
                "串口 {} 不是由 MCP 打开的（可能由 GUI 打开），请通过界面关闭",
                req.port_name
            )));
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
    ) -> Result<Json<SerialWriteResponse>, McpError> {
        self.ctx
            .leases
            .validate(&req.port_name, &req.lease_id)
            .map_err(mcp_err)?;

        let mut payload = decode_bytes(&req.data, &req.encoding)?;
        if req.append_crlf {
            payload.extend_from_slice(b"\r\n");
        }
        if payload.is_empty() {
            return Err(mcp_err("待发送数据为空"));
        }

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
    ) -> Result<Json<SerialReadResponse>, McpError> {
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
    ) -> Result<Json<SerialExpectResponse>, McpError> {
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
                .map_err(|e| mcp_err(format!("正则表达式无效: {e}")))?;
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
            Box::new(move |text: &str| {
                text.find(&pat)
                    .map(|i| text[i..i + pat.len()].to_string())
            })
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
}

#[tool_handler]
impl ServerHandler for ComBridgeMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "ComBridge 串口/BLE/GH3036 调试工具的 MCP 服务。\
串口域已可用：combridge_ports 发现端口 → combridge_serial_open 打开并获取租约 → combridge_serial_write 发送 → \
combridge_serial_expect 等待响应（或 combridge_serial_read 游标读取）→ combridge_serial_close 释放。\
写/关操作需携带 combridge_serial_open 返回的 lease_id；端口被其他客户端占用时返回 PORT_BUSY。",
            )
            .with_server_info(
                Implementation::new("combridge", env!("CARGO_PKG_VERSION"))
                    .with_title("ComBridge")
                    .with_description("ComBridge cross-platform serial/BLE debugging tool"),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 工具名与 schema 由宏生成，此项可防止改名/漏注册。
    #[test]
    fn test_tool_names_registered() {
        let names = [
            (ComBridgeMcp::about_tool_attr().name.to_string(), "combridge_about"),
            (ComBridgeMcp::ports_tool_attr().name.to_string(), "combridge_ports"),
            (ComBridgeMcp::combridge_serial_open_tool_attr().name.to_string(), "combridge_serial_open"),
            (ComBridgeMcp::combridge_serial_close_tool_attr().name.to_string(), "combridge_serial_close"),
            (ComBridgeMcp::combridge_serial_write_tool_attr().name.to_string(), "combridge_serial_write"),
            (ComBridgeMcp::combridge_serial_read_tool_attr().name.to_string(), "combridge_serial_read"),
            (ComBridgeMcp::combridge_serial_expect_tool_attr().name.to_string(), "combridge_serial_expect"),
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
        assert!(serde_json::to_string(&*read.input_schema).unwrap().contains("since"));

        let expect = ComBridgeMcp::combridge_serial_expect_tool_attr();
        let schema = serde_json::to_string(&*expect.input_schema).unwrap();
        assert!(schema.contains("pattern"));
        assert!(schema.contains("timeout_ms"));
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
