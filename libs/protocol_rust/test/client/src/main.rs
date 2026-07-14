use gh_rpc::{
    KEY_F_GET_MODE, KEY_F_SET_MODE, KEY_GH3X_CHIP_CTRL, KEY_GH3X_GET_VERSION,
    KEY_GH3X_REG_BIT_FIELD_WRITE_CMD, KEY_GH3X_REGS_BIT_FIELD_WRITE_CMD,
    KEY_GH3X_REGS_LIST_WRITE_CMD, KEY_GH3X_REGS_READ_CMD, KEY_GH3X_REGS_WRITE_CMD,
    KEY_GH3X_SW_FUNCTION_CMD, KEY_GH_SET_WORK_MODE_CMD, KEY_GET_CHIP_LINK_STATUS,
    KEY_GH_LOW_POWER_CMD, KEY_GH_TIME_SET, KEY_GH_TIMESTAMP_SET,
    FMT_GH3X_GET_VERSION, FMT_GH3X_REGS_READ_CMD, FMT_GH3X_CHIP_CTRL,
    FMT_GH3X_REGS_WRITE_CMD, FMT_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REG_BIT_FIELD_WRITE_CMD,
    FMT_GH3X_REGS_BIT_FIELD_WRITE_CMD, FMT_GH3X_SW_FUNCTION_CMD, FMT_GH_SET_WORK_MODE_CMD,
    FMT_GET_CHIP_LINK_STATUS, FMT_GH_LOW_POWER_CMD, FMT_GH_TIME_SET, FMT_GH_TIMESTAMP_SET,
    FMT_F_GET_MODE, FMT_F_SET_MODE,
    RET_GH3X_GET_VERSION, RET_GH3X_REGS_READ_CMD, RET_GET_CHIP_LINK_STATUS, RET_F_GET_MODE,
    GhFuncFrame, CommandExecutor, FrameCallback,
    unpack, UnpackValue,
};
use rpc::{RpcConfig, SendFunction, LogCallback, LogLevel};
use std::sync::Arc;
use std::time::Duration;
use std::io::Write as IoWrite;
use tokio::sync::RwLock;

const SERIAL_PORT: &str = "COM10";
const BAUD_RATE: u32 = 115200;

type LogSender = std::sync::mpsc::Sender<String>;

struct Logger {
    tx: LogSender,
}

impl Logger {
    fn new() -> std::io::Result<(Self, std::thread::JoinHandle<()>)> {
        std::fs::create_dir_all("log")?;
        let now = chrono::Local::now();
        let filename = format!("log/client_{}.log", now.format("%Y%m%d_%H%M%S"));
        let mut file = std::fs::File::create(&filename)?;
        println!("日志文件: {}", filename);
        
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        
        let handle = std::thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                println!("{}", msg);
                let _ = file.write_all(msg.as_bytes());
                let _ = file.write_all(b"\n");
            }
        });
        
        Ok((Self { tx }, handle))
    }

    fn log(&self, msg: &str) {
        let _ = self.tx.send(msg.to_string());
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}

impl LogCallback for Logger {
    fn log(&self, level: LogLevel, context: &str, message: &str) {
        let level_str = match level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        let _ = self.tx.send(format!("[{}] [{}] {}", level_str, context, message));
    }
}

struct TestStats {
    total: std::sync::atomic::AtomicUsize,
    passed: std::sync::atomic::AtomicUsize,
    failed: std::sync::atomic::AtomicUsize,
}

impl TestStats {
    fn new() -> Self {
        Self {
            total: std::sync::atomic::AtomicUsize::new(0),
            passed: std::sync::atomic::AtomicUsize::new(0),
            failed: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn record_pass(&self, _test_name: &str) {
        self.total.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.passed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn record_fail(&self, _test_name: &str, _error: &str) {
        self.total.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.failed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn print_summary(&self, logger: &Logger) {
        logger.log("\n========== Test Summary ==========");
        logger.log(&format!("Total:  {}", self.total.load(std::sync::atomic::Ordering::SeqCst)));
        logger.log(&format!("Passed: {}", self.passed.load(std::sync::atomic::Ordering::SeqCst)));
        logger.log(&format!("Failed: {}", self.failed.load(std::sync::atomic::Ordering::SeqCst)));
        logger.log("==================================");
        
        if self.failed.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            logger.log("\nAll tests passed!");
        }
    }
}

async fn test_get_version(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_get_version";
    logger.log(&format!("\n>>> Running {}...", test_name));

    match core.call(KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION, &[1]).await {
        Ok(response) => {
            match unpack(&response, RET_GH3X_GET_VERSION) {
                Ok(UnpackValue::U8Array(arr)) => {
                    let version = String::from_utf8_lossy(&arr).trim_matches(char::from(0)).to_string();
                    logger.log(&format!("    Version: {}", version));
                    stats.record_pass(test_name);
                }
                Ok(other) => {
                    logger.log(&format!("    Unexpected type: {:?}", other));
                    stats.record_fail(test_name, "Unexpected type");
                }
                Err(e) => {
                    logger.log(&format!("    Unpack error: {:?}", e));
                    stats.record_fail(test_name, &format!("Unpack error: {:?}", e));
                }
            }
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_regs_write(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_regs_write";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let regs: Vec<u16> = vec![0x1000, 0x1234, 0x1002, 0x5678];
    let mut data = Vec::new();
    data.extend_from_slice(&(regs.len() as u16).to_le_bytes());
    for reg in &regs {
        data.extend_from_slice(&reg.to_le_bytes());
    }

    match core.send(KEY_GH3X_REGS_WRITE_CMD, FMT_GH3X_REGS_WRITE_CMD, &data).await {
        Ok(()) => {
            logger.log("    Registers written successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_regs_read(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_regs_read";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let mut data = Vec::new();
    data.extend_from_slice(&0x1000u16.to_le_bytes());
    data.extend_from_slice(&10i32.to_le_bytes());

    match core.call(KEY_GH3X_REGS_READ_CMD, FMT_GH3X_REGS_READ_CMD, &data).await {
        Ok(response) => {
            logger.log(&format!("response: {:02X?}", response));
            match unpack(&response, RET_GH3X_REGS_READ_CMD) {
                Ok(UnpackValue::U16Array(values)) => {
                    logger.log(&format!("    Read {} registers: {:04X?}", values.len(), values));
                    stats.record_pass(test_name);
                }
                Ok(other) => {
                    logger.log(&format!("    Unexpected type: {:?}", other));
                    stats.record_fail(test_name, "Unexpected type");
                }
                Err(e) => {
                    logger.log(&format!("    Unpack error: {:?}", e));
                    stats.record_fail(test_name, &format!("Unpack error: {:?}", e));
                }
            }
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_chip_ctrl(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_chip_ctrl";
    logger.log(&format!("\n>>> Running {}...", test_name));

    match core.send(KEY_GH3X_CHIP_CTRL, FMT_GH3X_CHIP_CTRL, &[0x01]).await {
        Ok(()) => {
            logger.log("    Chip control executed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_regs_list_write_single_frame(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_regs_list_write_single_frame";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let pairs: Vec<(u16, u16)> = (0..10).map(|i| (0x1000 + i, 0x1000 + i)).collect();
    let mut data = Vec::new();
    data.extend_from_slice(&(pairs.len() as u16).to_le_bytes());
    for (addr, val) in &pairs {
        data.extend_from_slice(&addr.to_le_bytes());
        data.extend_from_slice(&val.to_le_bytes());
    }

    logger.log(&format!("    Writing {} register pairs ({} bytes)", pairs.len(), data.len()));

    match core.send(KEY_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REGS_LIST_WRITE_CMD, &data).await {
        Ok(()) => {
            logger.log("    Single frame write completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_regs_list_write_multi_frame(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_regs_list_write_multi_frame";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let pairs: Vec<(u16, u16)> = (0..150).map(|i| (0x1000 + (i % 100), 0x2000 + i)).collect();
    let mut data = Vec::new();
    data.extend_from_slice(&(pairs.len() as u16).to_le_bytes());
    for (addr, val) in &pairs {
        data.extend_from_slice(&addr.to_le_bytes());
        data.extend_from_slice(&val.to_le_bytes());
    }

    logger.log(&format!("    Writing {} register pairs ({} bytes)", pairs.len(), data.len()));

    match core.send(KEY_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REGS_LIST_WRITE_CMD, &data).await {
        Ok(()) => {
            logger.log("    Multi frame write completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_reg_bit_field_write(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_reg_bit_field_write";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let mut data = Vec::new();
    data.extend_from_slice(&0x1000u16.to_le_bytes());
    data.push(0);
    data.push(7);
    data.extend_from_slice(&0xFFu16.to_le_bytes());

    match core.send(KEY_GH3X_REG_BIT_FIELD_WRITE_CMD, FMT_GH3X_REG_BIT_FIELD_WRITE_CMD, &data).await {
        Ok(()) => {
            logger.log("    Register bit field write completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_regs_bit_field_write(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_regs_bit_field_write";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let bits: Vec<u16> = vec![0x1000, 0x00FF, 0x1001, 0x00F0];
    let mut data = Vec::new();
    data.extend_from_slice(&(bits.len() as u16).to_le_bytes());
    for bit in &bits {
        data.extend_from_slice(&bit.to_le_bytes());
    }

    match core.send(KEY_GH3X_REGS_BIT_FIELD_WRITE_CMD, FMT_GH3X_REGS_BIT_FIELD_WRITE_CMD, &data).await {
        Ok(()) => {
            logger.log("    Registers bit field write completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_sw_function(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_sw_function";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let mut data = Vec::new();
    data.extend_from_slice(&0x00000001u32.to_le_bytes());
    data.push(0x01);

    match core.send(KEY_GH3X_SW_FUNCTION_CMD, FMT_GH3X_SW_FUNCTION_CMD, &data).await {
        Ok(()) => {
            logger.log("    Software function completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_set_work_mode(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_set_work_mode";
    logger.log(&format!("\n>>> Running {}...", test_name));

    match core.send(KEY_GH_SET_WORK_MODE_CMD, FMT_GH_SET_WORK_MODE_CMD, &[0x01]).await {
        Ok(()) => {
            logger.log("    Work mode set successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_get_chip_link_status(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_get_chip_link_status";
    logger.log(&format!("\n>>> Running {}...", test_name));

    match core.call(KEY_GET_CHIP_LINK_STATUS, FMT_GET_CHIP_LINK_STATUS, &[0x01]).await {
        Ok(response) => {
            match unpack(&response, RET_GET_CHIP_LINK_STATUS) {
                Ok(UnpackValue::I8Array(status)) => {
                    logger.log(&format!("    Chip link status: {:?}", status));
                    stats.record_pass(test_name);
                }
                Ok(UnpackValue::U8Array(status)) => {
                    logger.log(&format!("    Chip link status: {:?}", status));
                    stats.record_pass(test_name);
                }
                Ok(other) => {
                    logger.log(&format!("    Unexpected type: {:?}", other));
                    stats.record_fail(test_name, "Unexpected type");
                }
                Err(e) => {
                    logger.log(&format!("    Unpack error: {:?}", e));
                    stats.record_fail(test_name, &format!("Unpack error: {:?}", e));
                }
            }
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_f_get_mode(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_f_get_mode";
    logger.log(&format!("\n>>> Running {}...", test_name));

    match core.call(KEY_F_GET_MODE, FMT_F_GET_MODE, &[0x01]).await {
        Ok(response) => {
            match unpack(&response, RET_F_GET_MODE) {
                Ok(UnpackValue::U16Array(modes)) => {
                    logger.log(&format!("    Modes: {:?}", modes));
                    stats.record_pass(test_name);
                }
                Ok(other) => {
                    logger.log(&format!("    Unexpected type: {:?}", other));
                    stats.record_fail(test_name, "Unexpected type");
                }
                Err(e) => {
                    logger.log(&format!("    Unpack error: {:?}", e));
                    stats.record_fail(test_name, &format!("Unpack error: {:?}", e));
                }
            }
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_f_set_mode(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_f_set_mode";
    logger.log(&format!("\n>>> Running {}...", test_name));

    match core.send(KEY_F_SET_MODE, FMT_F_SET_MODE, &[0x01]).await {
        Ok(()) => {
            logger.log("    Mode set successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_low_power_cmd(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_low_power_cmd";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let mut data = Vec::new();
    data.extend_from_slice(&0x00000001u32.to_le_bytes());
    data.push(0x01);

    match core.send(KEY_GH_LOW_POWER_CMD, FMT_GH_LOW_POWER_CMD, &data).await {
        Ok(()) => {
            logger.log("    Low power command completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_time_set(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_time_set";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let mut data = Vec::new();
    data.extend_from_slice(&12345678u32.to_le_bytes());
    data.push(8);

    match core.send(KEY_GH_TIME_SET, FMT_GH_TIME_SET, &data).await {
        Ok(()) => {
            logger.log("    Time set completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

async fn test_timestamp_set(core: &CommandExecutor, stats: &TestStats, logger: &Logger) {
    let test_name = "test_timestamp_set";
    logger.log(&format!("\n>>> Running {}...", test_name));

    let data = 12345678u32.to_le_bytes().to_vec();

    match core.send(KEY_GH_TIMESTAMP_SET, FMT_GH_TIMESTAMP_SET, &data).await {
        Ok(()) => {
            logger.log("    Timestamp set completed successfully (ack received)");
            stats.record_pass(test_name);
        }
        Err(e) => {
            stats.record_fail(test_name, &e.to_string());
        }
    }
}

#[tokio::main]
async fn main() {
    let (logger, _log_handle) = Logger::new().expect("无法创建日志文件");
    
    logger.log("========================================");
    logger.log("  GH Protocol Client Test Program");
    logger.log(&format!("  Serial Port: {} @ {}", SERIAL_PORT, BAUD_RATE));
    logger.log("========================================\n");

    logger.log("扫描可用串口...");
    let ports = serialport::available_ports().unwrap_or_default();
    logger.log(&format!("找到 {} 个串口:", ports.len()));
    for port in &ports {
        logger.log(&format!("  - {} [{}]", port.port_name, match &port.port_type {
            serialport::SerialPortType::UsbPort(info) => format!("USB (VID:{:04x}, PID:{:04x})", info.vid, info.pid),
            _ => "Unknown".to_string(),
        }));
    }

    logger.log(&format!("\n正在打开串口 {}...", SERIAL_PORT));
    let port = match serialport::new(SERIAL_PORT, BAUD_RATE)
        .timeout(Duration::from_millis(100))
        .open()
    {
        Ok(p) => {
            logger.log(&format!("串口 {} 打开成功\n", SERIAL_PORT));
            p
        }
        Err(e) => {
            logger.log(&format!("无法打开串口 {}: {}", SERIAL_PORT, e));
            return;
        }
    };

    let (tx_send, rx_send) = std::sync::mpsc::channel::<Vec<u8>>();
    let send_fn: SendFunction = Arc::new(move |data: Vec<u8>| {
        let sender = tx_send.clone();
        Box::pin(async move {
            sender.send(data).map_err(|_| rpc::RpcError::SendFail)?;
            Ok(())
        })
    });

    let executor = Arc::new(RwLock::new(
        CommandExecutor::new(RpcConfig {
            timeout_ms: 1000,
            ..RpcConfig::default()
        }).with_logger(Arc::new(logger.clone()))
    ));
    
    {
        let mut exec = executor.write().await;
        exec.set_send_function(send_fn.clone()).await;

        let logger_clone = logger.clone();
        let g_callback: FrameCallback = Arc::new(move |frame: &GhFuncFrame| {
            logger_clone.log("\n========== G回调 ==========");
            logger_clone.log(&format!("帧计数: {}, 时间戳: {}, ID: {:?}", 
                frame.frame_cnt, frame.timestamp, frame.id));
            logger_clone.log(&format!("通道数: {}, 最大通道: {}", frame.ch_num, frame.ch_max));
            logger_clone.log(&format!("G传感器: acc=[{}, {}, {}]", 
                frame.gsensor_data.acc[0], frame.gsensor_data.acc[1], frame.gsensor_data.acc[2]));
            for (i, data) in frame.data.iter().enumerate() {
                logger_clone.log(&format!("  通道[{}]: ipd_pa={}, rawdata={}, flag={{led_adj:{}, sa:{}, param_change:{}, dre_update:{}, skip_ok:{}}}",
                    i, data.ipd_pa, data.rawdata,
                    data.flag.led_adj_flag, data.flag.sa_flag,
                    data.flag.param_change_flag, data.flag.dre_update,
                    data.flag.skip_ok_flag));
                logger_clone.log(&format!("  AGC: gain_code={}, bg_cancel_range={}, dc_cancel_range={}, dc_cancel_code={}, led_drv0={}, led_drv1={}, bg_cancel_code={}, tia_gain={}",
                    data.agc_info.gain_code, data.agc_info.bg_cancel_range, data.agc_info.dc_cancel_range,
                    data.agc_info.dc_cancel_code, data.agc_info.led_drv0, data.agc_info.led_drv1,
                    data.agc_info.bg_cancel_code, data.agc_info.tia_gain));
            }
            logger_clone.log(&format!("帧级别: led_drv_fs=[{}, {}]", frame.led_drv_fs[0], frame.led_drv_fs[1]));
            logger_clone.log("===========================\n");
        });
        exec.register_frame_callback(g_callback);
        exec.register_g_handler().await.expect("注册G回调失败");
    }
    logger.log("已注册 G 协议回调\n");

    let executor_clone = executor.clone();
    let rt = tokio::runtime::Handle::current();
    let logger_for_thread = logger.clone();
    std::thread::spawn(move || {
        let mut port = port;
        let mut buffer = [0u8; 4096];
        
        loop {
            if let Ok(data) = rx_send.try_recv() {
                logger_for_thread.log(&format!("[TX] 发送 {} 字节", data.len()));
                if let Err(e) = port.write_all(&data) {
                    logger_for_thread.log(&format!("[TX] 发送失败: {}", e));
                }
            }

            match port.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    logger_for_thread.log(&format!("[RX] 收到 {} 字节", n));
                    let data = buffer[..n].to_vec();
                    let executor = executor_clone.clone();
                    let logger_for_spawn = logger_for_thread.clone();
                    rt.spawn(async move {
                        let exec = executor.read().await;
                        let results = exec.process(&data).await;
                        for result in results {
                            match result {
                                Ok(parse_result) => {
                                    logger_for_spawn.log(&format!("[RX] 解析成功: key={}, len={}", 
                                        parse_result.key, parse_result.param.len()));
                                }
                                Err(e) => {
                                    logger_for_spawn.log(&format!("[RX] 解析失败: {:?}", e));
                                }
                            }
                        }
                    });
                }
                _ => {}
            }
            
            std::thread::sleep(Duration::from_millis(1));
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    logger.log("启动接收任务...\n");

    let stats = Arc::new(TestStats::new());

    logger.log("========================================");
    logger.log("  开始执行测试用例");
    logger.log("========================================");

    {
        let exec = executor.read().await;
        test_get_version(&exec, &stats, &logger).await;
        test_regs_write(&exec, &stats, &logger).await;
        test_regs_read(&exec, &stats, &logger).await;
        test_chip_ctrl(&exec, &stats, &logger).await;
        test_regs_list_write_single_frame(&exec, &stats, &logger).await;
        test_regs_list_write_multi_frame(&exec, &stats, &logger).await;
        test_reg_bit_field_write(&exec, &stats, &logger).await;
        test_regs_bit_field_write(&exec, &stats, &logger).await;
        test_sw_function(&exec, &stats, &logger).await;
        test_set_work_mode(&exec, &stats, &logger).await;
        test_get_chip_link_status(&exec, &stats, &logger).await;
        test_f_get_mode(&exec, &stats, &logger).await;
        test_f_set_mode(&exec, &stats, &logger).await;
        test_low_power_cmd(&exec, &stats, &logger).await;
        test_time_set(&exec, &stats, &logger).await;
        test_timestamp_set(&exec, &stats, &logger).await;
    }

    tokio::time::sleep(Duration::from_secs(120)).await;

    stats.print_summary(&logger);
}
