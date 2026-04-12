use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tracing::{debug, error, info};

use crate::device::{
    BaudRate, DataBits, FlowControl, Parity, PortInfo, SerialManagerRef,
    SerialPortConfig, StopBits,
};
use crate::error::{ComBridgeError, Result};

fn format_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortConfigDto {
    pub port_name: String,
    pub baud_rate: Option<String>,
    pub data_bits: Option<u8>,
    pub parity: Option<String>,
    pub stop_bits: Option<u8>,
    pub flow_control: Option<String>,
    pub timeout_ms: Option<u64>,
    pub pack_timeout_ms: Option<u64>,
}

impl TryFrom<SerialPortConfigDto> for SerialPortConfig {
    type Error = ComBridgeError;

    fn try_from(dto: SerialPortConfigDto) -> Result<Self> {
        debug!("转换串口配置DTO: {:?}", dto);

        let baud_rate = dto
            .baud_rate
            .map(|s| parse_baud_rate(&s))
            .transpose()?
            .unwrap_or_default();

        let data_bits = dto
            .data_bits
            .map(|b| match b {
                5 => Ok(DataBits::Five),
                6 => Ok(DataBits::Six),
                7 => Ok(DataBits::Seven),
                8 => Ok(DataBits::Eight),
                _ => {
                    error!("无效的数据位配置: {}", b);
                    Err(ComBridgeError::serial(format!("无效的数据位: {}", b)))
                }
            })
            .transpose()?
            .unwrap_or_default();

        let parity = dto
            .parity
            .map(|s| parse_parity(&s))
            .transpose()?
            .unwrap_or_default();

        let stop_bits = dto
            .stop_bits
            .map(|b| match b {
                1 => Ok(StopBits::One),
                2 => Ok(StopBits::Two),
                _ => {
                    error!("无效的停止位配置: {}", b);
                    Err(ComBridgeError::serial(format!("无效的停止位: {}", b)))
                }
            })
            .transpose()?
            .unwrap_or_default();

        let flow_control = dto
            .flow_control
            .map(|s| parse_flow_control(&s))
            .transpose()?
            .unwrap_or_default();

        let config = SerialPortConfig {
            port_name: dto.port_name.clone(),
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            flow_control,
            timeout_ms: dto.timeout_ms.unwrap_or(1000),
            pack_timeout_ms: dto.pack_timeout_ms.unwrap_or(50),
        };

        debug!("串口配置转换成功: {:?}", config);
        Ok(config)
    }
}

fn parse_baud_rate(s: &str) -> Result<BaudRate> {
    debug!("解析波特率: {}", s);
    match s {
        "1200" => Ok(BaudRate::B1200),
        "2400" => Ok(BaudRate::B2400),
        "4800" => Ok(BaudRate::B4800),
        "9600" => Ok(BaudRate::B9600),
        "19200" => Ok(BaudRate::B19200),
        "38400" => Ok(BaudRate::B38400),
        "57600" => Ok(BaudRate::B57600),
        "115200" => Ok(BaudRate::B115200),
        "230400" => Ok(BaudRate::B230400),
        "460800" => Ok(BaudRate::B460800),
        "921600" => Ok(BaudRate::B921600),
        _ => {
            error!("无效的波特率: {}", s);
            Err(ComBridgeError::serial(format!("无效的波特率: {}", s)))
        }
    }
}

fn parse_parity(s: &str) -> Result<Parity> {
    debug!("解析校验位: {}", s);
    match s.to_lowercase().as_str() {
        "none" => Ok(Parity::None),
        "odd" => Ok(Parity::Odd),
        "even" => Ok(Parity::Even),
        _ => {
            error!("无效的校验位: {}", s);
            Err(ComBridgeError::serial(format!("无效的校验位: {}", s)))
        }
    }
}

fn parse_flow_control(s: &str) -> Result<FlowControl> {
    debug!("解析流控制: {}", s);
    match s.to_lowercase().as_str() {
        "none" => Ok(FlowControl::None),
        "software" => Ok(FlowControl::Software),
        "hardware" => Ok(FlowControl::Hardware),
        _ => {
            error!("无效的流控制: {}", s);
            Err(ComBridgeError::serial(format!("无效的流控制: {}", s)))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerialDataEvent {
    pub port_name: String,
    pub data: Vec<u8>,
}

#[tauri::command]
pub async fn scan_serial_ports(
    manager: State<'_, SerialManagerRef>,
) -> Result<Vec<PortInfo>> {
    info!("开始扫描串口设备");
    
    let manager = manager.inner();
    match manager.scan_ports() {
        Ok(ports) => {
            info!("串口扫描完成，发现 {} 个设备", ports.len());
            for port in &ports {
                debug!("发现串口: {} ({})", port.name, port.port_type);
            }
            Ok(ports)
        }
        Err(e) => {
            error!("串口扫描失败: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn open_serial_port(
    manager: State<'_, SerialManagerRef>,
    app: AppHandle,
    config: SerialPortConfigDto,
) -> Result<()> {
    info!("尝试打开串口: {}", config.port_name);
    
    let manager = manager.inner();
    let config: SerialPortConfig = match config.try_into() {
        Ok(c) => c,
        Err(e) => {
            error!("串口配置转换失败: {}", e);
            return Err(e);
        }
    };

    let port_name = config.port_name.clone();
    let app_clone = app.clone();
    
    match manager.open_port(config, move |name, data| {
        let event = SerialDataEvent {
            port_name: name.to_string(),
            data: data.to_vec(),
        };
        debug!("串口 {} 接收到 {} 字节数据", name, data.len());
        let _ = app_clone.emit("serial-data", &event);
    }) {
        Ok(()) => {
            info!("串口 {} 打开成功", port_name);
            Ok(())
        }
        Err(e) => {
            error!("串口 {} 打开失败: {}", port_name, e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn close_serial_port(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
) -> Result<()> {
    info!("尝试关闭串口: {}", port_name);

    let manager = manager.inner();
    match manager.close_port(&port_name) {
        Ok(()) => {
            info!("串口 {} 关闭成功", port_name);
            Ok(())
        }
        Err(e) => {
            error!("串口 {} 关闭失败: {}", port_name, e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn send_serial_data(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
    data: Vec<u8>,
) -> Result<usize> {
    let data_hex = format_hex(&data);
    info!("[SEND] 串口 {} 发送 {} 字节: {}", port_name, data.len(), data_hex);

    let manager = manager.inner();
    match manager.send_data(&port_name, &data) {
        Ok(bytes_written) => {
            info!("[SEND] 串口 {} 发送成功，{} 字节", port_name, bytes_written);
            Ok(bytes_written)
        }
        Err(e) => {
            error!("[SEND] 串口 {} 发送失败: {}", port_name, e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_open_ports(
    manager: State<'_, SerialManagerRef>,
) -> Result<Vec<String>> {
    debug!("获取已打开的端口列表");
    
    let manager = manager.inner();
    let ports = manager.get_open_ports()?;
    debug!("当前已打开 {} 个端口", ports.len());
    Ok(ports)
}

#[tauri::command]
pub async fn is_port_open(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
) -> Result<bool> {
    debug!("检查端口 {} 是否已打开", port_name);
    
    let manager = manager.inner();
    let is_open = manager.is_port_open(&port_name)?;
    debug!("端口 {} 状态: {}", port_name, if is_open { "已打开" } else { "已关闭" });
    Ok(is_open)
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub log_path: String,
    pub dat_path: String,
}

#[tauri::command]
pub async fn export_serial_data(
    _app: AppHandle,
    port_name: String,
    all_data: Vec<ExportDataEntry>,
    rx_data: Vec<u8>,
) -> Result<ExportResult> {
    use std::fs;
    use std::io::Write;

    info!("导出串口 {} 的数据", port_name);

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let log_dir = std::env::current_dir()
        .map(|p| p.join("logs"))
        .unwrap_or_else(|_| std::path::PathBuf::from("logs"));

    if !log_dir.exists() {
        fs::create_dir_all(&log_dir).map_err(|e| {
            error!("创建日志目录失败: {}", e);
            ComBridgeError::serial(format!("创建日志目录失败: {}", e))
        })?;
    }

    let safe_port_name = port_name.replace("/", "_").replace("\\", "_");
    let log_filename = format!("{}_all_data_{}.log", safe_port_name, timestamp);
    let dat_filename = format!("{}_rx_data_{}.dat", safe_port_name, timestamp);

    let log_path = log_dir.join(&log_filename);
    let dat_path = log_dir.join(&dat_filename);

    let log_content = all_data
        .iter()
        .filter(|entry| entry.direction == "receive")
        .map(|entry| {
            let timestamp_str = format_timestamp(entry.timestamp);
            let data_ascii: String = entry.data.iter()
                .map(|b| {
                    if *b >= 32 && *b <= 126 {
                        (*b as char).to_string()
                    } else {
                        format!("\\x{:02X}", b)
                    }
                })
                .collect();
            format!("[{}][RX][{} byte] {}", timestamp_str, entry.data.len(), data_ascii)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut log_file = fs::File::create(&log_path).map_err(|e| {
        error!("创建日志文件失败: {}", e);
        ComBridgeError::serial(format!("创建日志文件失败: {}", e))
    })?;
    log_file.write_all(log_content.as_bytes()).map_err(|e| {
        error!("写入日志文件失败: {}", e);
        ComBridgeError::serial(format!("写入日志文件失败: {}", e))
    })?;

    let mut dat_file = fs::File::create(&dat_path).map_err(|e| {
        error!("创建数据文件失败: {}", e);
        ComBridgeError::serial(format!("创建数据文件失败: {}", e))
    })?;
    dat_file.write_all(&rx_data).map_err(|e| {
        error!("写入数据文件失败: {}", e);
        ComBridgeError::serial(format!("写入数据文件失败: {}", e))
    })?;

    info!("数据导出成功: log={}, dat={}", log_path.display(), dat_path.display());

    Ok(ExportResult {
        log_path: log_path.to_string_lossy().to_string(),
        dat_path: dat_path.to_string_lossy().to_string(),
    })
}

fn format_timestamp(timestamp: u64) -> String {
    let total_ms = timestamp % 1000;
    let total_seconds = timestamp / 1000;
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, total_ms)
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportDataEntry {
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub direction: String,
}
