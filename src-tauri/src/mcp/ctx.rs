//! MCP 运行上下文：持有各业务 Manager 的句柄。
//!
//! 工具实现通过 ctx 访问硬件与状态，**直接调 Manager，不走 `#[tauri::command]` 层**
//! （command 层的 `State<'_, T>` 无法在 MCP handler 内构造，且 DTO 命名面向前端）。

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use serde_json::Value;
use tracing::warn;

use crate::commands::waveform::WaveformManager;
use crate::device::{BleManagerRef, SerialManagerRef};
use crate::gh3036::Gh3036ManagerRef;
use crate::ota::OtaManager;
use crate::protocol::PluginManager;
use crate::service::EventBus;
use crate::state::AppStateRef;

use super::audit::AuditLogger;
use super::ble_hub::BleHub;
use super::gh3036_job::FactoryJob;
use super::lease::LeaseManager;
use super::ota_confirm::OtaConfirmStore;
use super::ratelimit::{PortBreaker, RateLimiter};
use super::serial_hub::SerialHub;

pub struct McpContext {
    /// ComBridge 应用版本
    pub app_version: String,
    /// 安全档位："dev" | "standard"
    pub tier: String,
    /// 服务启动时间（毫秒时间戳）
    pub started_at: u64,
    pub serial: SerialManagerRef,
    pub ble: BleManagerRef,
    pub app_state: AppStateRef,
    pub event_bus: Arc<EventBus>,
    /// 设备租约（P1：串口端口）
    pub leases: Arc<LeaseManager>,
    /// 串口数据订阅 hub（游标 + 阻塞等待）
    pub serial_hub: Arc<SerialHub>,
    /// BLE 通知数据 hub（按 地址+特征 分桶的游标读取）
    pub ble_hub: Arc<BleHub>,
    /// 由 MCP 打开的端口（GUI 打开的端口 MCP 不允许 close）
    pub opened_by_mcp: RwLock<HashSet<String>>,
    /// GH3036 协议管理器（单芯片会话，资源名固定为 `gh3036`）
    pub gh3036: Gh3036ManagerRef,
    /// 最近一次 MCP 发起的产测 job（用于超时降级后的续等）
    pub factory_job: RwLock<Option<FactoryJob>>,
    /// 波形缓冲（与 GUI 共用同一份实例，MCP 写入的数据 GUI 立即可见）
    pub waveform: Arc<WaveformManager>,
    /// Lua 协议插件管理器
    pub protocols: Arc<PluginManager>,
    /// OTA 升级管理器
    pub ota: Arc<OtaManager>,
    /// OTA dry-run 签发的确认令牌（L3 硬门）
    pub ota_confirm: OtaConfirmStore,
    /// 审计写入器；目录不可确定时为 None（启动时已记告警）
    pub audit: Option<Arc<AuditLogger>>,
    /// QPS + 并发限流（按档位）
    pub limiter: RateLimiter,
    /// 端口级熔断（连续硬件错误）
    pub breaker: PortBreaker,
}

impl McpContext {
    /// 从 Tauri AppHandle 提取各 Manager。必须在 `setup` 之后调用。
    pub fn from_app(app: &tauri::AppHandle, tier: String) -> Self {
        use tauri::Manager;

        let serial = app.state::<SerialManagerRef>().inner().clone();
        let ble = app.state::<BleManagerRef>().inner().clone();
        let app_state = app.state::<AppStateRef>().inner().clone();
        let event_bus = app.state::<Arc<EventBus>>().inner().clone();
        let gh3036 = app.state::<Gh3036ManagerRef>().inner().clone();
        let waveform = app.state::<Arc<WaveformManager>>().inner().clone();
        let protocols = app.state::<Arc<PluginManager>>().inner().clone();
        let ota = app.state::<Arc<OtaManager>>().inner().clone();

        let audit = match AuditLogger::try_new() {
            Some(logger) => {
                tracing::info!("[MCP] 审计目录: {}", logger.dir().display());
                Some(Arc::new(logger))
            }
            None => {
                tracing::warn!("[MCP] 无法确定审计目录，审计降级为仅 tracing 输出");
                None
            }
        };

        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            tier: tier.clone(),
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            serial,
            ble,
            app_state,
            event_bus,
            leases: Arc::new(LeaseManager::new()),
            serial_hub: Arc::new(SerialHub::new()),
            ble_hub: Arc::new(BleHub::new()),
            opened_by_mcp: RwLock::new(HashSet::new()),
            gh3036,
            factory_job: RwLock::new(None),
            waveform,
            protocols,
            ota,
            ota_confirm: OtaConfirmStore::new(),
            audit,
            limiter: RateLimiter::for_tier(&tier),
            breaker: PortBreaker::new(),
        }
    }

    /// 当前已连接设备快照（序列化为 JSON，避免依赖内部 Device 类型字段）。
    pub async fn connected_devices(&self) -> Vec<Value> {
        let state = self.app_state.read().await;
        state
            .get_connected_devices()
            .into_iter()
            .filter_map(|d| match serde_json::to_value(d) {
                Ok(v) => Some(v),
                Err(e) => {
                    warn!("[MCP] 设备序列化失败: {}", e);
                    None
                }
            })
            .collect()
    }
}
