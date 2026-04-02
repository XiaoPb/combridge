use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::device::{
    BaudRate, DataBits, FlowControl, Parity, PortInfo, SerialManagerRef,
    SerialPortConfig, StopBits,
};
use crate::error::{ComBridgeError, Result};

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
                _ => Err(ComBridgeError::serial(format!("无效的数据位: {}", b))),
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
                _ => Err(ComBridgeError::serial(format!("无效的停止位: {}", b))),
            })
            .transpose()?
            .unwrap_or_default();

        let flow_control = dto
            .flow_control
            .map(|s| parse_flow_control(&s))
            .transpose()?
            .unwrap_or_default();

        Ok(SerialPortConfig {
            port_name: dto.port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            flow_control,
            timeout_ms: dto.timeout_ms.unwrap_or(1000),
        })
    }
}

fn parse_baud_rate(s: &str) -> Result<BaudRate> {
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
        _ => Err(ComBridgeError::serial(format!("无效的波特率: {}", s))),
    }
}

fn parse_parity(s: &str) -> Result<Parity> {
    match s.to_lowercase().as_str() {
        "none" => Ok(Parity::None),
        "odd" => Ok(Parity::Odd),
        "even" => Ok(Parity::Even),
        _ => Err(ComBridgeError::serial(format!("无效的校验位: {}", s))),
    }
}

fn parse_flow_control(s: &str) -> Result<FlowControl> {
    match s.to_lowercase().as_str() {
        "none" => Ok(FlowControl::None),
        "software" => Ok(FlowControl::Software),
        "hardware" => Ok(FlowControl::Hardware),
        _ => Err(ComBridgeError::serial(format!("无效的流控制: {}", s))),
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
    let manager = manager.inner();
    manager.scan_ports()
}

#[tauri::command]
pub async fn open_serial_port(
    manager: State<'_, SerialManagerRef>,
    app: AppHandle,
    config: SerialPortConfigDto,
) -> Result<()> {
    let manager = manager.inner();
    let config: SerialPortConfig = config.try_into()?;

    let app_clone = app.clone();
    manager.register_callback(move |name, data| {
        let event = SerialDataEvent {
            port_name: name.to_string(),
            data: data.to_vec(),
        };
        let _ = app_clone.emit("serial-data", &event);
    });

    manager.open_port(config)
}

#[tauri::command]
pub async fn close_serial_port(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
) -> Result<()> {
    let manager = manager.inner();
    manager.close_port(&port_name)
}

#[tauri::command]
pub async fn send_serial_data(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
    data: Vec<u8>,
) -> Result<usize> {
    let manager = manager.inner();
    manager.send_data(&port_name, &data)
}

#[tauri::command]
pub async fn get_open_ports(
    manager: State<'_, SerialManagerRef>,
) -> Result<Vec<String>> {
    let manager = manager.inner();
    Ok(manager.get_open_ports())
}

#[tauri::command]
pub async fn is_port_open(
    manager: State<'_, SerialManagerRef>,
    port_name: String,
) -> Result<bool> {
    let manager = manager.inner();
    Ok(manager.is_port_open(&port_name))
}
