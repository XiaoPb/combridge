use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tracing::{debug, error, info};

use crate::device::{
    BaudRate, DataBits, FlowControl, Parity, PortInfo, SerialManagerRef,
    SerialPortConfig, StopBits,
};
use crate::error::{ComBridgeError, Result};

/// 串口配置DTO，用于前端传递配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialPortConfigDto {
    pub port_name: String,
    pub baud_rate: Option<String>,
    pub data_bits: Option<u8>,
    pub parity: Option<String>,
    pub stop_bits: Option<u8>,
    pub flow_control: Option<String>,
    pub timeout_ms: Option<u64>,
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

/// 串口数据事件，用于向前端推送接收到的数据
#[derive(Debug, Clone, Serialize)]
pub struct SerialDataEvent {
    pub port_name: String,
    pub data: Vec<u8>,
}

/// 扫描可用串口列表
/// 
/// 返回系统中所有可用的串口设备信息，包括端口名称、厂商、产品等信息
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

/// 打开串口
/// 
/// 根据配置打开指定的串口设备，并注册数据接收回调
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
    
    manager.register_callback(move |name, data| {
        let event = SerialDataEvent {
            port_name: name.to_string(),
            data: data.to_vec(),
        };
        debug!("串口 {} 接收到 {} 字节数据", name, data.len());
        let _ = app_clone.emit("serial-data", &event);
    });

    match manager.open_port(config) {
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

/// 关闭串口
/// 
/// 关闭指定的串口设备并释放资源
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

/// 发送串口数据
/// 
/// 向指定串口发送数据
#[tauri::command]
pub async fn send_serial_data(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
    data: Vec<u8>,
) -> Result<usize> {
    debug!("向串口 {} 发送 {} 字节数据", port_name, data.len());
    
    let manager = manager.inner();
    match manager.send_data(&port_name, &data) {
        Ok(bytes_written) => {
            debug!("串口 {} 成功发送 {} 字节", port_name, bytes_written);
            Ok(bytes_written)
        }
        Err(e) => {
            error!("串口 {} 发送数据失败: {}", port_name, e);
            Err(e)
        }
    }
}

/// 获取已打开的端口列表
/// 
/// 返回当前已打开的所有串口名称列表
#[tauri::command]
pub async fn get_open_ports(
    manager: State<'_, SerialManagerRef>,
) -> Result<Vec<String>> {
    debug!("获取已打开的端口列表");
    
    let manager = manager.inner();
    let ports = manager.get_open_ports();
    debug!("当前已打开 {} 个端口", ports.len());
    Ok(ports)
}

/// 检查端口是否已打开
/// 
/// 检查指定的串口是否处于打开状态
#[tauri::command]
pub async fn is_port_open(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
) -> Result<bool> {
    debug!("检查端口 {} 是否已打开", port_name);
    
    let manager = manager.inner();
    let is_open = manager.is_port_open(&port_name);
    debug!("端口 {} 状态: {}", port_name, if is_open { "已打开" } else { "已关闭" });
    Ok(is_open)
}
