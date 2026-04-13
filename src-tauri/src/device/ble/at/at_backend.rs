use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{ComBridgeError, LockResultExt, Result};
use super::super::ble_traits::{
    BleBackend, BleDevice, BleConnection, BleService, BleCharacteristic,
    BleCharacteristicProperties, NotifyCallback,
};
use super::at_commands::{AtCommand, AtResponse, AtConnectionConfig, ScanDevice};
use super::at_parser::AtParser;
use super::at_transport::{AtTransport, TransportMode, DataCallback};

const DEFAULT_SERVICE_UUID: &str = "0000FFE0-0000-1000-8000-00805F9B34FB";
const DEFAULT_TX_UUID: &str = "0000FFE1-0000-1000-8000-00805F9B34FB";
const DEFAULT_RX_UUID: &str = "0000FFE2-0000-1000-8000-00805F9B34FB";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtConnectionInfo {
    pub address: String,
    pub name: Option<String>,
    pub rssi: i16,
    pub tx_uuid: String,
    pub rx_uuid: String,
    pub srv_uuid: String,
    pub connected_at: Option<u64>,
}

impl Default for AtConnectionInfo {
    fn default() -> Self {
        Self {
            address: String::new(),
            name: None,
            rssi: -100,
            tx_uuid: DEFAULT_TX_UUID.to_string(),
            rx_uuid: DEFAULT_RX_UUID.to_string(),
            srv_uuid: DEFAULT_SERVICE_UUID.to_string(),
            connected_at: None,
        }
    }
}

pub struct AtBleBackend {
    transport: Arc<Mutex<Option<AtTransport>>>,
    connections: Arc<Mutex<HashMap<String, AtConnectionInfo>>>,
    config: AtConnectionConfig,
    notify_callbacks: Arc<Mutex<HashMap<String, NotifyCallback>>>,
    is_scanning: Arc<AtomicBool>,
}

impl AtBleBackend {
    pub fn new() -> Self {
        Self {
            transport: Arc::new(Mutex::new(None)),
            connections: Arc::new(Mutex::new(HashMap::new())),
            config: AtConnectionConfig::new(),
            notify_callbacks: Arc::new(Mutex::new(HashMap::new())),
            is_scanning: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_transport(transport: AtTransport) -> Self {
        Self {
            transport: Arc::new(Mutex::new(Some(transport))),
            connections: Arc::new(Mutex::new(HashMap::new())),
            config: AtConnectionConfig::new(),
            notify_callbacks: Arc::new(Mutex::new(HashMap::new())),
            is_scanning: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_config(transport: AtTransport, config: AtConnectionConfig) -> Self {
        Self {
            transport: Arc::new(Mutex::new(Some(transport))),
            connections: Arc::new(Mutex::new(HashMap::new())),
            config,
            notify_callbacks: Arc::new(Mutex::new(HashMap::new())),
            is_scanning: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_config(&mut self, config: AtConnectionConfig) {
        self.config = config;
    }

    fn send_command_and_wait(&self, command: &AtCommand, timeout_ms: u64) -> Result<Vec<String>> {
        let mut transport_guard = self.transport.lock().lock_err("AT传输层")?;
        let transport = transport_guard.as_mut().ok_or_else(|| {
            ComBridgeError::ble("AT传输层未初始化")
        })?;
        
        transport.send_command(command)?;
        transport.read_response(Some(timeout_ms))
    }

    fn parse_ok_response(responses: &[String]) -> Result<()> {
        if responses.is_empty() {
            return Err(ComBridgeError::ble("未收到响应"));
        }

        let last = responses.last().ok_or_else(|| ComBridgeError::ble("响应为空"))?;
        if last == "OK" {
            Ok(())
        } else if last.starts_with("ERROR") {
            Err(ComBridgeError::ble(format!("AT指令错误: {}", last)))
        } else {
            Err(ComBridgeError::ble(format!("未知响应: {}", last)))
        }
    }

    fn make_callback_key(address: &str) -> String {
        address.to_string()
    }

    fn configure_uuids(&self) -> Result<()> {
        if let Some(ref tx_uuid) = self.config.tx_uuid {
            self.send_command_and_wait(&AtCommand::SetTxUuid(tx_uuid.clone()), 1000)?;
            debug!("已设置TX UUID: {}", tx_uuid);
        }
        if let Some(ref rx_uuid) = self.config.rx_uuid {
            self.send_command_and_wait(&AtCommand::SetRxUuid(rx_uuid.clone()), 1000)?;
            debug!("已设置RX UUID: {}", rx_uuid);
        }
        if let Some(ref srv_uuid) = self.config.srv_uuid {
            self.send_command_and_wait(&AtCommand::SetSrvUuid(srv_uuid.clone()), 1000)?;
            debug!("已设置服务UUID: {}", srv_uuid);
        }
        Ok(())
    }

    fn build_virtual_service(&self, address: &str) -> BleService {
        let connections = self.connections.lock().unwrap_or_else(|e| e.into_inner());
        let conn_info = connections.get(address);
        
        let tx_uuid = conn_info
            .map(|c| c.tx_uuid.as_str())
            .unwrap_or(DEFAULT_TX_UUID);
        let rx_uuid = conn_info
            .map(|c| c.rx_uuid.as_str())
            .unwrap_or(DEFAULT_RX_UUID);
        let srv_uuid = conn_info
            .map(|c| c.srv_uuid.as_str())
            .unwrap_or(DEFAULT_SERVICE_UUID);

        BleService {
            uuid: srv_uuid.to_string(),
            primary: true,
            characteristics: vec![
                BleCharacteristic {
                    uuid: tx_uuid.to_string(),
                    service_uuid: srv_uuid.to_string(),
                    properties: BleCharacteristicProperties {
                        read: false,
                        write: false,
                        write_without_response: false,
                        notify: true,
                        indicate: false,
                    },
                    subscribed: false,
                },
                BleCharacteristic {
                    uuid: rx_uuid.to_string(),
                    service_uuid: srv_uuid.to_string(),
                    properties: BleCharacteristicProperties {
                        read: false,
                        write: true,
                        write_without_response: true,
                        notify: false,
                        indicate: false,
                    },
                    subscribed: false,
                },
            ],
        }
    }

    fn setup_transparent_callback(&self, address: String) -> Result<()> {
        let callbacks = self.notify_callbacks.clone();
        let address_clone = address.clone();
        
        let callback: DataCallback = Arc::new(move |data: &[u8]| {
            debug!("透传接收数据: {} 字节", data.len());
            if let Some(cb) = callbacks.lock().unwrap_or_else(|e| e.into_inner()).get(&address_clone) {
                cb(&address_clone, "", data);
            }
        });

        let mut transport_guard = self.transport.lock().lock_err("AT传输层")?;
        if let Some(ref mut transport) = *transport_guard {
            transport.set_data_callback(callback);
        }
        Ok(())
    }

    fn scan_device_to_ble_device(device: &ScanDevice) -> BleDevice {
        BleDevice {
            address: device.address.clone(),
            name: device.name.clone(),
            rssi: Some(device.rssi),
            is_connectable: true,
        }
    }
}

impl Default for AtBleBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AtBleBackend {
    fn drop(&mut self) {
        let mut transport_guard = self.transport.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut transport) = *transport_guard {
            let _ = transport.close();
        }
    }
}

#[async_trait]
impl BleBackend for AtBleBackend {
    async fn configure(&mut self) -> Result<()> {
        let mut transport_guard = self.transport.lock().lock_err("AT传输层")?;
        let transport = transport_guard.as_mut().ok_or_else(|| {
            ComBridgeError::ble("AT传输层未初始化")
        })?;
        
        transport.send_command(&AtCommand::Test)?;
        let responses = transport.read_response(Some(1000))?;
        Self::parse_ok_response(&responses)?;

        transport.send_command(&AtCommand::SetRole(1))?;
        let responses = transport.read_response(Some(1000))?;
        Self::parse_ok_response(&responses)?;
        
        drop(transport_guard);
        
        self.configure_uuids()?;
        info!("AT BLE后端配置成功（主机模式）");
        Ok(())
    }

    async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>> {
        self.is_scanning.store(true, Ordering::SeqCst);
        
        let responses = self.send_command_and_wait(&AtCommand::ScanStart, duration_ms + 2000)?;
        
        let mut devices = Vec::new();
        let parser = AtParser::new();

        for line in &responses {
            if line.starts_with("+SCAN:") {
                if let Ok(response) = parser.parse_response(line) {
                    if let AtResponse::ScanResult { devices: scanned } = response {
                        for dev in scanned {
                            devices.push(Self::scan_device_to_ble_device(&dev));
                        }
                    }
                }
            }
        }

        self.is_scanning.store(false, Ordering::SeqCst);
        info!("扫描完成，发现 {} 个设备", devices.len());
        Ok(devices)
    }

    async fn stop_scan(&self) -> Result<Vec<BleDevice>> {
        let responses = self.send_command_and_wait(&AtCommand::ScanStop, 3000)?;
        Self::parse_ok_response(&responses)?;
        
        self.is_scanning.store(false, Ordering::SeqCst);
        info!("停止扫描");
        Ok(Vec::new())
    }

    async fn connect(&self, address: &str) -> Result<BleConnection> {
        let responses = self.send_command_and_wait(
            &AtCommand::Connect(address.to_string()),
            15000
        )?;

        let mut connected_address = address.to_string();
        for line in &responses {
            if line.starts_with("+CONN:") {
                if let Ok(response) = AtParser::new().parse_response(line) {
                    if let AtResponse::Connected { address: addr } = response {
                        connected_address = addr;
                    }
                }
            }
        }

        Self::parse_ok_response(&responses)?;

        let mut conn_info = AtConnectionInfo::default();
        conn_info.address = connected_address.clone();
        conn_info.tx_uuid = self.config.tx_uuid.clone().unwrap_or_else(|| DEFAULT_TX_UUID.to_string());
        conn_info.rx_uuid = self.config.rx_uuid.clone().unwrap_or_else(|| DEFAULT_RX_UUID.to_string());
        conn_info.srv_uuid = self.config.srv_uuid.clone().unwrap_or_else(|| DEFAULT_SERVICE_UUID.to_string());
        conn_info.connected_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64);

        self.connections.lock().lock_err("AT连接")?.insert(connected_address.clone(), conn_info);

        let mut transport_guard = self.transport.lock().lock_err("AT传输层")?;
        if let Some(ref mut transport) = *transport_guard {
            transport.enter_transparent_mode()?;
        }

        self.setup_transparent_callback(connected_address.clone())?;

        info!("已连接到设备: {}", connected_address);
        
        let service = self.build_virtual_service(&connected_address);
        Ok(BleConnection {
            address: connected_address,
            name: None,
            is_connected: true,
            services: vec![service],
        })
    }

    async fn disconnect(&self, address: &str) -> Result<()> {
        let mut transport_guard = self.transport.lock().lock_err("AT传输层")?;
        if let Some(ref mut transport) = *transport_guard {
            let _ = transport.exit_transparent_mode();
        }
        drop(transport_guard);

        let responses = self.send_command_and_wait(
            &AtCommand::Disconnect(address.to_string()),
            5000
        )?;

        self.connections.lock().lock_err("AT连接")?.remove(address);
        self.notify_callbacks.lock().lock_err("AT回调")?.remove(address);

        Self::parse_ok_response(&responses)?;
        info!("已断开设备: {}", address);
        Ok(())
    }

    async fn get_connections(&self) -> Result<Vec<BleConnection>> {
        let connections = self.connections.lock().lock_err("AT连接")?;
        let result: Vec<BleConnection> = connections
            .iter()
            .map(|(addr, info)| {
                let service = self.build_virtual_service(addr);
                BleConnection {
                    address: addr.clone(),
                    name: info.name.clone(),
                    is_connected: true,
                    services: vec![service],
                }
            })
            .collect();
        Ok(result)
    }

    async fn discover_services(&self, address: &str) -> Result<Vec<BleService>> {
        let service = self.build_virtual_service(address);
        info!("返回虚拟服务（AT模块不支持服务发现）");
        Ok(vec![service])
    }

    async fn discover_characteristics(&self, address: &str, _service_uuid: &str) -> Result<Vec<BleCharacteristic>> {
        let service = self.build_virtual_service(address);
        info!("返回虚拟特征（AT模块不支持特征发现）");
        Ok(service.characteristics)
    }

    async fn read_characteristic(&self, _address: &str, _char_uuid: &str) -> Result<Vec<u8>> {
        Err(ComBridgeError::ble("AT模块不支持读取特征值，请使用透传模式接收数据"))
    }

    async fn write_characteristic(&self, _address: &str, _char_uuid: &str, data: &[u8]) -> Result<()> {
        let mut transport_guard = self.transport.lock().lock_err("AT传输层")?;
        let transport = transport_guard.as_mut().ok_or_else(|| {
            ComBridgeError::ble("AT传输层未初始化")
        })?;

        if transport.mode() == TransportMode::Transparent {
            transport.send_transparent_data(data)?;
            debug!("透传发送: {} 字节", data.len());
        } else {
            transport.send_command(&AtCommand::SendData(data.to_vec()))?;
            let responses = transport.read_response(Some(5000))?;
            Self::parse_ok_response(&responses)?;
        }

        Ok(())
    }

    async fn write_without_response(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        self.write_characteristic(address, char_uuid, data).await
    }

    async fn subscribe_notify(&self, address: &str, _char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let key = Self::make_callback_key(address);
        self.notify_callbacks.lock().lock_err("AT回调")?.insert(key, callback);
        
        info!("已设置透传数据回调: {}", address);
        Ok(())
    }

    async fn unsubscribe_notify(&self, address: &str, _char_uuid: &str) -> Result<()> {
        let key = Self::make_callback_key(address);
        self.notify_callbacks.lock().lock_err("AT回调")?.remove(&key);
        
        info!("已移除透传数据回调: {}", address);
        Ok(())
    }

    async fn get_rssi(&self, _address: &str) -> Result<i16> {
        let responses = self.send_command_and_wait(&AtCommand::GetRssi(0), 3000)?;

        for line in &responses {
            if line.starts_with("+RSSI:") {
                if let Ok(response) = AtParser::new().parse_response(line) {
                    if let AtResponse::Rssi { rssi } = response {
                        return Ok(rssi);
                    }
                }
            }
        }

        Err(ComBridgeError::ble("获取RSSI失败"))
    }

    async fn set_mtu(&self, _address: &str, mtu: u16) -> Result<u16> {
        let responses = self.send_command_and_wait(&AtCommand::SetMtu(mtu), 3000)?;
        Self::parse_ok_response(&responses)?;
        info!("MTU设置成功: {}", mtu);
        Ok(mtu)
    }
}

pub type AtBleBackendRef = Arc<Mutex<AtBleBackend>>;
