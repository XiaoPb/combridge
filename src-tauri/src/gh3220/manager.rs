//! GH3220 会话管理器。
//!
//! 负责：连接生命周期、命令串行化、数据解码、事件 ACK、批量上报、CSV 落盘与统计。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use gh_uprotocol::cmd::{self, Request};
use gh_uprotocol::{chip_ctrl, command, event, ver_type, DataDecoder, Frame, FrameDecoder};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::device::ble::BleManagerRef;
use crate::service::event_bus::{topics, BleConnectionEvent, BleDataEvent, EventBus};

use super::csv_writer::CsvWriter;
use super::link::Gh3220Link;
use super::session::Session;
use super::types::{
    status_text, Gh3220CaptureConfig, Gh3220CaptureStatus, Gh3220CsvConfig, Gh3220DataEvent,
    Gh3220DebugStatus, Gh3220DeviceInfo, Gh3220Error, Gh3220EventPayload, Gh3220LinkConfig,
    Gh3220LinkKind, Gh3220Record, Gh3220State, Gh3220Stats, Gh3220Status, Gh3220StatusEvent,
    DATA_BATCH_INTERVAL_MS, DATA_BATCH_MAX_RECORDS, MAX_CSV_IDLE_CLOSE_SECS, NOTIFY_UUID,
};

const STATS_PUBLISH_INTERVAL: Duration = Duration::from_secs(1);
const REG_LIST_BATCH: usize = 50;

struct Runtime {
    state: Gh3220State,
    kind: Option<Gh3220LinkKind>,
    address: Option<String>,
    name: Option<String>,
    error: Option<String>,
    chunk_size: usize,
    notify_uuid: String,
    link: Option<Arc<Gh3220Link>>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            state: Gh3220State::Disconnected,
            kind: None,
            address: None,
            name: None,
            error: None,
            chunk_size: 0,
            notify_uuid: NOTIFY_UUID.to_string(),
            link: None,
        }
    }
}

#[derive(Default)]
struct CaptureRuntime {
    active: bool,
    started_at_ms: u64,
    records: u64,
    csv_enabled: bool,
    channels_override: Option<usize>,
    func_mask: u32,
    auto_stop: bool,
    files: Vec<String>,
    writers: HashMap<u8, CsvWriter>,
}

pub struct Gh3220Manager {
    ble_manager: BleManagerRef,
    event_bus: Arc<EventBus>,
    runtime: Mutex<Runtime>,
    session: Mutex<Option<Arc<Session>>>,
    ingress: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    decoder_task: Mutex<Option<JoinHandle<()>>>,
    capture: Mutex<CaptureRuntime>,
    stats: Mutex<Gh3220Stats>,
    csv_config: Mutex<Gh3220CsvConfig>,
    debug_tcp_enabled: AtomicBool,
    subscribed: AtomicBool,
    connect_lock: tokio::sync::Mutex<()>,
    batch_seq: AtomicU64,
    reset_stats: AtomicBool,
    self_weak: Mutex<Option<Weak<Self>>>,
}

impl Gh3220Manager {
    pub fn new(ble_manager: BleManagerRef, event_bus: Arc<EventBus>) -> Self {
        Self {
            ble_manager,
            event_bus,
            runtime: Mutex::new(Runtime::default()),
            session: Mutex::new(None),
            ingress: Mutex::new(None),
            decoder_task: Mutex::new(None),
            capture: Mutex::new(CaptureRuntime::default()),
            stats: Mutex::new(Gh3220Stats::default()),
            csv_config: Mutex::new(Gh3220CsvConfig::default()),
            debug_tcp_enabled: AtomicBool::new(false),
            subscribed: AtomicBool::new(false),
            connect_lock: tokio::sync::Mutex::new(()),
            batch_seq: AtomicU64::new(0),
            reset_stats: AtomicBool::new(false),
            self_weak: Mutex::new(None),
        }
    }

    /// 只在启动期调用一次；重复调用会被忽略。
    pub fn start_event_bridge(self: &Arc<Self>) {
        *self.lock(&self.self_weak) = Some(Arc::downgrade(self));
        if self.subscribed.swap(true, Ordering::SeqCst) {
            return;
        }

        let weak = Arc::downgrade(self);
        self.event_bus.subscribe_msgpack::<BleDataEvent, _>(
            topics::BLE_DATA,
            move |_topic, event| {
                if let Some(mgr) = weak.upgrade() {
                    mgr.on_ble_data(event);
                }
            },
        );

        let weak = Arc::downgrade(self);
        self.event_bus.subscribe_json::<BleConnectionEvent, _>(
            topics::BLE_DISCONNECTED,
            move |_t, e| {
                if let Some(mgr) = weak.upgrade() {
                    mgr.on_ble_disconnected(&e.address);
                }
            },
        );

        let weak = Arc::downgrade(self);
        self.event_bus.subscribe_json::<BleConnectionEvent, _>(
            topics::BLE_CONNECTED,
            move |_t, e| {
                if let Some(mgr) = weak.upgrade() {
                    mgr.on_ble_connected(&e.address, e.name.clone());
                }
            },
        );
    }

    pub fn set_csv_config(&self, config: Gh3220CsvConfig) {
        let idle = normalize_idle_close(config.idle_close_secs);
        let mut current = self.lock(&self.csv_config);
        *current = Gh3220CsvConfig {
            enabled: config.enabled,
            output_dir: normalize_output_dir(&config.output_dir),
            idle_close_secs: idle,
        };
    }

    pub fn csv_config(&self) -> Gh3220CsvConfig {
        self.lock(&self.csv_config).clone()
    }

    pub fn set_debug_tcp_enabled(&self, enabled: bool) {
        self.debug_tcp_enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn status(&self) -> Gh3220Status {
        let runtime = self.lock(&self.runtime);
        let capture = self.lock(&self.capture);
        Gh3220Status {
            state: runtime.state,
            link: runtime.kind,
            address: runtime.address.clone(),
            name: runtime.name.clone(),
            error: runtime.error.clone(),
            capturing: capture.active,
            chunk_size: runtime.chunk_size,
        }
    }

    pub fn stats(&self) -> Gh3220Stats {
        self.lock(&self.stats).clone()
    }

    pub fn clear_stats(&self) {
        *self.lock(&self.stats) = Gh3220Stats::default();
        self.reset_stats.store(true, Ordering::SeqCst);
    }

    pub fn capture_status(&self) -> Gh3220CaptureStatus {
        let capture = self.lock(&self.capture);
        Gh3220CaptureStatus {
            active: capture.active,
            started_at_ms: capture.started_at_ms,
            records: capture.records,
            csv_enabled: capture.csv_enabled,
            files: capture.files.clone(),
            channels_override: capture.channels_override,
        }
    }

    // ------------------------------------------------------------------
    // 连接生命周期
    // ------------------------------------------------------------------

    pub async fn connect(
        self: &Arc<Self>,
        config: Gh3220LinkConfig,
    ) -> Result<Gh3220Status, Gh3220Error> {
        let _guard = self.connect_lock.lock().await;

        {
            let runtime = self.lock(&self.runtime);
            if runtime.state != Gh3220State::Disconnected {
                return Err(Gh3220Error::Invalid(
                    "已有活动连接，请先断开再连接".to_string(),
                ));
            }
        }

        {
            let mut runtime = self.lock(&self.runtime);
            runtime.state = Gh3220State::Connecting;
            runtime.error = None;
        }
        self.publish_status();

        match self.do_connect(config).await {
            Ok(status) => {
                info!(
                    "[GH3220] 连接成功: {:?} chunk={}",
                    status.address, status.chunk_size
                );
                self.publish_status();
                Ok(status)
            }
            Err(e) => {
                warn!("[GH3220] 连接失败: {}", e);
                {
                    let mut runtime = self.lock(&self.runtime);
                    *runtime = Runtime::default();
                    runtime.error = Some(e.to_string());
                }
                self.publish_status();
                Err(e)
            }
        }
    }

    async fn do_connect(
        self: &Arc<Self>,
        config: Gh3220LinkConfig,
    ) -> Result<Gh3220Status, Gh3220Error> {
        let tcp_enabled = self.debug_tcp_enabled.load(Ordering::SeqCst);
        let conn = Gh3220Link::connect(&config, self.ble_manager.clone(), tcp_enabled).await?;
        let link = Arc::new(conn.link);
        let session = Arc::new(Session::new(link.clone()));
        let notify_uuid = config
            .notify_uuid
            .clone()
            .unwrap_or_else(|| NOTIFY_UUID.to_string());

        let (tx, rx) = mpsc::unbounded_channel();
        if let Some(mut link_rx) = conn.rx {
            let forward = tx.clone();
            tokio::spawn(async move {
                while let Some(bytes) = link_rx.recv().await {
                    if forward.send(bytes).is_err() {
                        break;
                    }
                }
            });
        }

        *self.lock(&self.session) = Some(session.clone());
        *self.lock(&self.ingress) = Some(tx);
        {
            let mut runtime = self.lock(&self.runtime);
            runtime.state = Gh3220State::Ready;
            runtime.kind = Some(link.kind());
            runtime.address = Some(config.address.trim().to_string());
            runtime.name = conn.name;
            runtime.error = None;
            runtime.chunk_size = link.chunk_size();
            runtime.notify_uuid = notify_uuid;
            runtime.link = Some(link);
        }
        *self.lock(&self.stats) = Gh3220Stats::default();
        self.reset_stats.store(false, Ordering::SeqCst);

        let mgr = Arc::clone(self);
        let handle = tokio::spawn(async move {
            mgr.decode_loop(rx, session).await;
        });
        *self.lock(&self.decoder_task) = Some(handle);

        Ok(self.status())
    }

    pub async fn disconnect(&self) -> Result<(), Gh3220Error> {
        let _guard = self.connect_lock.lock().await;
        self.teardown(true).await;
        self.publish_status();
        Ok(())
    }

    async fn teardown(&self, actively: bool) {
        if let Some(session) = self.lock(&self.session).take() {
            session.close();
        }
        if let Some(tx) = self.lock(&self.ingress).take() {
            drop(tx);
        }
        if let Some(handle) = self.lock(&self.decoder_task).take() {
            handle.abort();
        }

        self.finish_capture(false).await;

        let link = {
            let mut runtime = self.lock(&self.runtime);
            let link = runtime.link.take();
            *runtime = Runtime::default();
            link
        };
        if actively {
            if let Some(link) = link {
                link.close().await;
            }
        }
    }

    fn on_ble_data(&self, event: BleDataEvent) {
        let (kind, address, notify_uuid) = {
            let runtime = self.lock(&self.runtime);
            (
                runtime.kind,
                runtime.address.clone(),
                runtime.notify_uuid.clone(),
            )
        };

        if kind != Some(Gh3220LinkKind::Ble) {
            return;
        }
        let Some(address) = address else {
            return;
        };
        if !event.address.eq_ignore_ascii_case(&address)
            || !event.characteristic_uuid.eq_ignore_ascii_case(&notify_uuid)
        {
            return;
        }

        let sender = self.lock(&self.ingress).clone();
        if let Some(tx) = sender {
            let _ = tx.send(event.data);
        }
    }

    fn on_ble_disconnected(&self, address: &str) {
        let matched = {
            let runtime = self.lock(&self.runtime);
            runtime
                .address
                .as_ref()
                .is_some_and(|a| a.eq_ignore_ascii_case(address))
                && runtime.kind == Some(Gh3220LinkKind::Ble)
        };
        if !matched {
            return;
        }

        info!("[GH3220] 链路断开: {}", address);
        let weak = self.lock(&self.self_weak).clone();
        tokio::spawn(async move {
            if let Some(weak) = weak {
                if let Some(mgr) = weak.upgrade() {
                    let _guard = mgr.connect_lock.lock().await;
                    mgr.teardown(false).await;
                    mgr.publish_status();
                }
            }
        });
    }

    fn on_ble_connected(&self, address: &str, name: Option<String>) {
        let mut runtime = self.lock(&self.runtime);
        if runtime
            .address
            .as_ref()
            .is_some_and(|a| a.eq_ignore_ascii_case(address))
        {
            runtime.name = name;
        }
    }

    // ------------------------------------------------------------------
    // 命令
    // ------------------------------------------------------------------

    fn session(&self) -> Result<Arc<Session>, Gh3220Error> {
        self.lock(&self.session)
            .clone()
            .ok_or(Gh3220Error::NotConnected)
    }

    async fn request_code(&self, req: Request) -> Result<u8, Gh3220Error> {
        let session = self.session()?;
        let resp = session.request(req.cmd, req.payload).await?;
        Ok(cmd::parse_status(&resp.payload)?)
    }

    pub async fn set_work_mode(&self, mode: u8, func_mask: u32) -> Result<u8, Gh3220Error> {
        self.request_code(Request::work_mode(mode, func_mask)).await
    }

    pub async fn start(&self, func_mask: u32, slot_en: u8) -> Result<u8, Gh3220Error> {
        self.request_code(Request::start_ctrl(true, 0, slot_en, func_mask))
            .await
    }

    pub async fn stop(&self, func_mask: u32) -> Result<u8, Gh3220Error> {
        self.request_code(Request::start_ctrl(false, 0, 0, func_mask))
            .await
    }

    pub async fn switch_slot_func(
        &self,
        slot: u8,
        extra: u8,
        func_mask: u32,
        on: bool,
    ) -> Result<u8, Gh3220Error> {
        self.request_code(Request::switch_slot_func(slot, extra, func_mask, on))
            .await
    }

    pub async fn reg_read(&self, addr: u16, count: u8) -> Result<Vec<u16>, Gh3220Error> {
        let session = self.session()?;
        let resp = session
            .request(command::REG_RW, Request::reg_read(addr, count).payload)
            .await?;
        Ok(cmd::parse_reg_values(&resp.payload)?)
    }

    pub async fn reg_write(&self, addr: u16, values: &[u16]) -> Result<(), Gh3220Error> {
        if values.is_empty() {
            return Err(Gh3220Error::Invalid("寄存器值不能为空".to_string()));
        }
        let session = self.session()?;
        session
            .request(command::REG_RW, Request::reg_write(addr, values).payload)
            .await?;
        Ok(())
    }

    pub async fn load_reg_list(&self, file_path: &str) -> Result<u8, Gh3220Error> {
        let regs = parse_reg_list_file(Path::new(file_path))?;
        if regs.is_empty() {
            return Err(Gh3220Error::Invalid("寄存器表为空".to_string()));
        }
        let session = self.session()?;
        let mut last = 0u8;
        for batch in regs.chunks(REG_LIST_BATCH) {
            let req = Request::load_reg_list(batch);
            let resp = session
                .request_with_timeout(req.cmd, req.payload, Duration::from_millis(1500))
                .await?;
            last = cmd::parse_status(&resp.payload)?;
            if last != 0 {
                break;
            }
        }
        Ok(last)
    }

    pub async fn chip_ctrl(&self, action: &str) -> Result<u8, Gh3220Error> {
        let ty = match action.to_ascii_lowercase().as_str() {
            "wakeup" => chip_ctrl::WAKEUP,
            "sleep" => chip_ctrl::SLEEP,
            "soft-reset" | "soft_reset" => chip_ctrl::SOFT_RESET,
            "hard-reset" | "hard_reset" => chip_ctrl::HARD_RESET,
            "rx-reset" | "rx_reset" => chip_ctrl::RX_RESET,
            other => return Err(Gh3220Error::Invalid(format!("未知芯片控制动作: {other}"))),
        };
        self.request_code(Request::chip_ctrl(ty)).await
    }

    pub async fn set_fifo_thr(&self, value: u16) -> Result<u8, Gh3220Error> {
        self.request_code(Request::fifo_thr(value)).await
    }

    pub async fn set_report_event(&self, mask: u16) -> Result<u8, Gh3220Error> {
        self.request_code(Request::report_event_set(mask)).await
    }

    pub async fn debug_status(&self, kind: &str) -> Result<Gh3220DebugStatus, Gh3220Error> {
        let ty = match kind.to_ascii_lowercase().as_str() {
            "int" => 0x00,
            "rst" => 0x01,
            "spcs" => 0x02,
            "spdo" => 0x03,
            "conn" => 0x04,
            "status" => 0x10,
            other => parse_u16(other).map(|v| v as u8)?,
        };
        let session = self.session()?;
        let resp = session.request(command::DEBUG_STATUS, vec![ty]).await?;
        let (kind, bytes, data) = cmd::parse_debug(&resp.payload)?;
        Ok(Gh3220DebugStatus { kind, bytes, data })
    }

    pub async fn package_test(&self, hex: &str) -> Result<Vec<u8>, Gh3220Error> {
        let data = super::types::parse_hex(hex)?;
        let session = self.session()?;
        let req = Request::package_test(&data);
        let resp = session.request(req.cmd, req.payload).await?;
        Ok(resp.payload)
    }

    pub async fn send_raw(&self, hex: &str) -> Result<(), Gh3220Error> {
        let bytes = super::types::parse_hex(hex)?;
        let frame = Frame::decode(&bytes)?;
        self.session()?.send(&frame).await
    }

    pub async fn device_info(&self) -> Result<Gh3220DeviceInfo, Gh3220Error> {
        let session = self.session()?;

        let mut versions = [String::new(), String::new(), String::new(), String::new()];
        for (idx, ty) in [
            ver_type::FW,
            ver_type::PROTOCOL,
            ver_type::CHIP,
            ver_type::DRV,
        ]
        .into_iter()
        .enumerate()
        {
            let resp = session.request(command::GET_VER, vec![ty]).await?;
            let (_, text) = cmd::parse_version(&resp.payload)?;
            versions[idx] = text;
        }

        let resp = session
            .request(command::GET_MAX_LEN, Request::get_max_len().payload)
            .await?;
        let max_len = cmd::parse_max_len(&resp.payload)?;

        let resp = session
            .request(command::GET_OPENED_FUNC, Request::get_opened_func().payload)
            .await?;
        let (_, opened_func) = cmd::parse_opened_func(&resp.payload)?;

        let resp = session
            .request(command::GET_CONN_STATUS, Request::conn_status().payload)
            .await?;
        let link_code = cmd::parse_status(&resp.payload)?;

        let [fw_version, protocol_version, chip_version, driver_version] = versions;

        Ok(Gh3220DeviceInfo {
            fw_version,
            protocol_version,
            chip_version,
            driver_version,
            max_len,
            opened_func,
            link_ok: link_code == 0,
        })
    }

    // ------------------------------------------------------------------
    // 采集与 CSV
    // ------------------------------------------------------------------

    pub async fn capture_start(&self, config: Gh3220CaptureConfig) -> Result<(), Gh3220Error> {
        {
            let capture = self.lock(&self.capture);
            if capture.active {
                return Err(Gh3220Error::Invalid("采集已在进行中".to_string()));
            }
        }
        let _session = self.session()?;
        if config.auto_start {
            if config.func_mask == 0 {
                return Err(Gh3220Error::Invalid(
                    "自动启动需要提供功能位 func_mask".to_string(),
                ));
            }
            let code = self
                .request_code(Request::work_mode(config.mode, config.func_mask))
                .await?;
            if code != 0 {
                return Err(Gh3220Error::Status { code });
            }
            let code = self
                .request_code(Request::start_ctrl(
                    true,
                    0,
                    config.slot_en,
                    config.func_mask,
                ))
                .await?;
            if code != 0 {
                return Err(Gh3220Error::Status { code });
            }
        }

        let csv_enabled = config
            .csv_enabled
            .unwrap_or_else(|| self.csv_config().enabled);

        {
            let mut capture = self.lock(&self.capture);
            *capture = CaptureRuntime {
                active: true,
                started_at_ms: now_ms(),
                records: 0,
                csv_enabled,
                channels_override: config.channels_override,
                func_mask: config.func_mask,
                auto_stop: config.auto_stop,
                files: Vec::new(),
                writers: HashMap::new(),
            };
        }
        {
            let mut runtime = self.lock(&self.runtime);
            runtime.state = Gh3220State::Capturing;
        }
        self.publish_status();
        Ok(())
    }

    pub async fn capture_stop(&self) -> Result<(), Gh3220Error> {
        let (auto_stop, func_mask) = {
            let capture = self.lock(&self.capture);
            (capture.auto_stop, capture.func_mask)
        };

        self.finish_capture(auto_stop).await;

        if auto_stop && func_mask != 0 {
            let _ = self
                .request_code(Request::start_ctrl(false, 0, 0, func_mask))
                .await;
        }

        {
            let mut runtime = self.lock(&self.runtime);
            if runtime.state == Gh3220State::Capturing {
                runtime.state = Gh3220State::Ready;
            }
        }
        self.publish_status();
        Ok(())
    }

    async fn finish_capture(&self, _send_stop: bool) {
        let mut capture = self.lock(&self.capture);
        for writer in capture.writers.values_mut() {
            let _ = writer.flush();
            writer.close();
        }
        capture.writers.clear();
        capture.active = false;
    }

    // ------------------------------------------------------------------
    // 解码任务
    // ------------------------------------------------------------------

    async fn decode_loop(
        self: Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<Vec<u8>>,
        session: Arc<Session>,
    ) {
        let mut frame_decoder = FrameDecoder::new();
        let mut data_decoder = DataDecoder::new();
        let mut batch: Vec<Gh3220Record> = Vec::new();
        let mut ticker = tokio::time::interval(Duration::from_millis(DATA_BATCH_INTERVAL_MS));
        let mut last_stats = Instant::now();

        loop {
            tokio::select! {
                maybe = rx.recv() => {
                    let Some(bytes) = maybe else {
                        break;
                    };
                    let frames = frame_decoder.push(&bytes);
                    self.sync_frame_stats(&frame_decoder);

                    for frame in frames {
                        if frame.cmd != command::NEW_STRUCT_RAWDATA {
                            debug!("[GH3220] rx cmd=0x{:02X} len={}", frame.cmd, frame.payload.len());
                        }
                        match frame.cmd {
                            command::NEW_STRUCT_RAWDATA => {
                                data_decoder.chnl_override = self.lock(&self.capture).channels_override;
                                match data_decoder.feed(&frame.payload) {
                                    Ok(records) => {
                                        let ts = now_ms();
                                        for rec in records {
                                            batch.push(Gh3220Record::from_record(&rec, ts));
                                        }
                                        self.sync_data_stats(&data_decoder);
                                        if batch.len() >= DATA_BATCH_MAX_RECORDS {
                                            self.flush_batch(&mut batch);
                                        }
                                    }
                                    Err(e) => {
                                        debug!("[GH3220] 数据包解码失败: {}", e);
                                        self.sync_data_stats(&data_decoder);
                                    }
                                }
                            }
                            command::CHIP_EVENT_REPORT => {
                                if let Some((event_id, report_id)) = event::parse(&frame.payload) {
                                    let acked = session.send(&event::ack(report_id)).await.is_ok();
                                    let payload = Gh3220EventPayload {
                                        event_id,
                                        report_id,
                                        acked,
                                        ts_ms: now_ms(),
                                    };
                                    self.event_bus.publish_msgpack(topics::GH3220_EVENT, &payload);
                                }
                            }
                            _ => {
                                let cmd = frame.cmd;
                                if !session.dispatch(frame) {
                                    debug!("[GH3220] 丢弃未知响应 cmd=0x{cmd:02X}");
                                }
                            }
                        }
                    }
                }
                _ = ticker.tick() => {
                    if self.reset_stats.swap(false, Ordering::SeqCst) {
                        frame_decoder.stats = gh_uprotocol::frame::DecoderStats::default();
                        data_decoder.stats = gh_uprotocol::data::DataStats::default();
                        *self.lock(&self.stats) = Gh3220Stats::default();
                    }
                    if !batch.is_empty() {
                        self.flush_batch(&mut batch);
                    }
                    if last_stats.elapsed() >= STATS_PUBLISH_INTERVAL {
                        last_stats = Instant::now();
                        if self.lock(&self.runtime).state != Gh3220State::Disconnected {
                            let stats = self.stats();
                            self.event_bus.publish_msgpack(topics::GH3220_STATS, &stats);
                        }
                    }
                }
            }
        }

        if !batch.is_empty() {
            self.flush_batch(&mut batch);
        }
        {
            let mut capture = self.lock(&self.capture);
            for writer in capture.writers.values_mut() {
                let _ = writer.flush();
                writer.close();
            }
            capture.writers.clear();
            capture.active = false;
        }
    }

    fn flush_batch(&self, batch: &mut Vec<Gh3220Record>) {
        if batch.is_empty() {
            return;
        }
        let records = std::mem::take(batch);
        for rec in &records {
            self.write_csv(rec);
        }
        self.flush_csv();

        let seq = self.batch_seq.fetch_add(1, Ordering::SeqCst);
        let event = Gh3220DataEvent {
            seq,
            records,
            dropped: 0,
        };
        self.event_bus.publish_msgpack(topics::GH3220_DATA, &event);
    }

    fn write_csv(&self, rec: &Gh3220Record) {
        let mut capture = self.lock(&self.capture);
        if !capture.active {
            return;
        }
        capture.records += 1;
        if !capture.csv_enabled {
            return;
        }

        if !capture.writers.contains_key(&rec.function_id) {
            let config = self.csv_config();
            let dir = PathBuf::from(&config.output_dir);
            let writer = CsvWriter::new(&dir, rec.function_id, config.idle_close_secs);
            capture
                .files
                .push(writer.path().to_string_lossy().to_string());
            capture.writers.insert(rec.function_id, writer);
        }

        if let Some(writer) = capture.writers.get_mut(&rec.function_id) {
            if let Err(e) = writer.write(rec) {
                warn!("[GH3220] CSV 写入失败: {}", e);
            }
        }
    }

    fn flush_csv(&self) {
        let mut capture = self.lock(&self.capture);
        for writer in capture.writers.values_mut() {
            if let Err(e) = writer.flush() {
                warn!("[GH3220] CSV flush 失败: {}", e);
            }
        }
    }

    fn sync_frame_stats(&self, decoder: &FrameDecoder) {
        let mut stats = self.lock(&self.stats);
        stats.frames = decoder.stats.frames;
        stats.crc_errors = decoder.stats.crc_errors;
        stats.resyncs = decoder.stats.resyncs;
        stats.bad_len = decoder.stats.bad_len;
    }

    fn sync_data_stats(&self, decoder: &DataDecoder) {
        let mut stats = self.lock(&self.stats);
        stats.packets = decoder.stats.packets;
        stats.records = decoder.stats.records;
        stats.bad_packets = decoder.stats.bad_packets;
        stats.splice_fragments = decoder.stats.splice_fragments;
        stats.splice_aborts = decoder.stats.splice_aborts;
        stats.lost_frames = decoder.stats.lost_frames;
    }

    fn publish_status(&self) {
        let event = Gh3220StatusEvent {
            status: self.status(),
        };
        self.event_bus.publish_typed(topics::GH3220_STATUS, &event);
    }

    fn lock<'a, T>(&self, mutex: &'a Mutex<T>) -> std::sync::MutexGuard<'a, T> {
        mutex.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn normalize_idle_close(secs: u64) -> u64 {
    if secs == 0 {
        0
    } else {
        secs.min(MAX_CSV_IDLE_CLOSE_SECS)
    }
}

fn normalize_output_dir(dir: &str) -> String {
    if Path::new(dir).is_absolute() {
        dir.to_string()
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .map(|d| d.join(dir).to_string_lossy().to_string())
            .unwrap_or_else(|| dir.to_string())
    }
}

fn parse_u16(input: &str) -> Result<u16, Gh3220Error> {
    let s = input.trim();
    let value = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        s.parse::<u32>()
    }
    .map_err(|e| Gh3220Error::Invalid(format!("数值解析失败 {s}: {e}")))?;
    Ok(value as u16)
}

fn parse_reg_list_file(path: &Path) -> Result<Vec<(u16, u16)>, Gh3220Error> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Gh3220Error::Io(format!("读取 {} 失败: {e}", path.display())))?;
    let mut regs = Vec::new();
    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split([',', ' ', '\t']).filter(|s| !s.is_empty());
        let addr = parts
            .next()
            .ok_or_else(|| Gh3220Error::Invalid(format!("第 {} 行缺少地址", line_no + 1)))?;
        let value = parts
            .next()
            .ok_or_else(|| Gh3220Error::Invalid(format!("第 {} 行缺少数值", line_no + 1)))?;
        regs.push((parse_u16(addr)?, parse_u16(value)?));
    }
    Ok(regs)
}

/// 设备状态码的可读文案（命令层直接返回 u8，前端负责展示）
pub fn status_label(code: u8) -> &'static str {
    status_text(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_close_is_normalized() {
        assert_eq!(normalize_idle_close(0), 0);
        assert_eq!(normalize_idle_close(5), 5);
        assert_eq!(normalize_idle_close(99_999), MAX_CSV_IDLE_CLOSE_SECS);
    }

    #[test]
    fn parses_reg_list_with_comments_and_hex() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("regs.csv");
        std::fs::write(&path, "# comment\n0x0010, 0x1234\n0x0012 7\n").unwrap();
        let regs = parse_reg_list_file(&path).unwrap();
        assert_eq!(regs, vec![(0x0010, 0x1234), (0x0012, 7)]);
    }

    #[test]
    fn parse_u16_accepts_decimal_and_hex() {
        assert_eq!(parse_u16("0x10").unwrap(), 16);
        assert_eq!(parse_u16("16").unwrap(), 16);
        assert!(parse_u16("zz").is_err());
    }
}
