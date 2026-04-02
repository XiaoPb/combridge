use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tracing::{debug, error, info};

use crate::device::ble::{
    AtConfig, BleCharacteristic, BleConnection, BleDevice, BleManagerRef, BleMode, BleService,
};
use crate::error::{ComBridgeError, Result};

/// BLE配置DTO，用于前端传递配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleConfigDto {
    /// BLE模式：native（原生）或 at（AT指令）
    pub mode: String,
    /// AT模式下的串口名称
    pub port_name: Option<String>,
    /// AT模式下的波特率，默认115200
    pub baud_rate: Option<u32>,
    /// AT指令超时时间（毫秒），默认1000
    pub timeout_ms: Option<u64>,
}

/// BLE通知事件，用于向前端推送接收到的通知数据
#[derive(Debug, Clone, Serialize)]
pub struct BleNotifyEvent {
    /// 设备地址
    pub address: String,
    /// 特征UUID
    pub char_uuid: String,
    /// 接收到的数据
    pub data: Vec<u8>,
}

/// 配置BLE模式
/// 
/// 配置BLE工作模式（原生或AT指令模式）
/// - native: 使用系统原生蓝牙API
/// - at: 通过串口AT指令控制BLE模块
#[tauri::command]
pub async fn configure_ble(
    manager: State<'_, BleManagerRef>,
    config: BleConfigDto,
) -> Result<()> {
    info!("开始配置BLE，模式: {}", config.mode);
    
    let manager = manager.inner();

    let mode = match config.mode.to_lowercase().as_str() {
        "native" => {
            info!("使用原生BLE模式");
            BleMode::Native
        }
        "at" => {
            info!("使用AT指令BLE模式");
            BleMode::At
        }
        _ => {
            error!("无效的BLE模式: {}", config.mode);
            return Err(ComBridgeError::ble(format!(
                "无效的BLE模式: {}，支持的模式：native, at",
                config.mode
            )));
        }
    };

    let result = match mode {
        BleMode::Native => {
            debug!("配置原生BLE模式");
            manager.configure_native().await
        }
        BleMode::At => {
            let port_name = match config.port_name {
                Some(ref port) => port.clone(),
                None => {
                    error!("AT模式需要指定port_name");
                    return Err(ComBridgeError::ble("AT模式需要指定串口名称(port_name)"));
                }
            };
            
            let at_config = AtConfig {
                port_name: port_name.clone(),
                baud_rate: config.baud_rate.unwrap_or(115200),
                timeout_ms: config.timeout_ms.unwrap_or(1000),
            };
            
            debug!(
                "配置AT模式，串口: {}, 波特率: {}, 超时: {}ms",
                at_config.port_name, at_config.baud_rate, at_config.timeout_ms
            );
            manager.configure_at(at_config).await
        }
    };

    match result {
        Ok(()) => {
            info!("BLE配置成功，模式: {}", mode);
            Ok(())
        }
        Err(e) => {
            error!("BLE配置失败: {}", e);
            Err(e)
        }
    }
}

/// 扫描BLE设备
/// 
/// 扫描周围的BLE设备，返回发现的设备列表
/// duration_ms: 扫描持续时间（毫秒）
#[tauri::command]
pub async fn scan_ble_devices(
    manager: State<'_, BleManagerRef>,
    duration_ms: u64,
) -> Result<Vec<BleDevice>> {
    info!("开始扫描BLE设备，持续时间: {}ms", duration_ms);
    
    let manager = manager.inner();
    match manager.scan(duration_ms).await {
        Ok(devices) => {
            info!("BLE扫描完成，发现 {} 个设备", devices.len());
            for device in &devices {
                debug!(
                    "发现BLE设备: {} ({})",
                    device.name.as_deref().unwrap_or("未知"),
                    device.address
                );
            }
            Ok(devices)
        }
        Err(e) => {
            error!("BLE扫描失败: {}", e);
            Err(e)
        }
    }
}

/// 连接BLE设备
/// 
/// 连接到指定地址的BLE设备
#[tauri::command]
pub async fn connect_ble(
    manager: State<'_, BleManagerRef>,
    device_id: String,
) -> Result<BleConnection> {
    info!("尝试连接BLE设备: {}", device_id);

    let manager = manager.inner();
    match manager.connect(&device_id).await {
        Ok(conn) => {
            info!("BLE设备连接成功: {}", device_id);
            Ok(conn)
        }
        Err(e) => {
            error!("BLE设备连接失败 {}: {}", device_id, e);
            Err(e)
        }
    }
}

/// 断开BLE连接
///
/// 断开与指定BLE设备的连接
#[tauri::command]
pub async fn disconnect_ble(
    manager: State<'_, BleManagerRef>,
    device_id: String,
) -> Result<()> {
    info!("尝试断开BLE设备: {}", device_id);

    let manager = manager.inner();
    match manager.disconnect(&device_id).await {
        Ok(()) => {
            info!("BLE设备已断开: {}", device_id);
            Ok(())
        }
        Err(e) => {
            error!("BLE设备断开失败 {}: {}", device_id, e);
            Err(e)
        }
    }
}

/// 获取BLE连接列表
/// 
/// 获取当前所有已连接的BLE设备列表
#[tauri::command]
pub async fn get_ble_connections(
    manager: State<'_, BleManagerRef>,
) -> Result<Vec<BleConnection>> {
    debug!("获取BLE连接列表");
    
    let manager = manager.inner();
    match manager.get_connections().await {
        Ok(connections) => {
            debug!("当前有 {} 个BLE连接", connections.len());
            Ok(connections)
        }
        Err(e) => {
            error!("获取BLE连接列表失败: {}", e);
            Err(e)
        }
    }
}

/// 发现GATT服务
/// 
/// 发现指定BLE设备的所有GATT服务
#[tauri::command]
pub async fn discover_ble_services(
    manager: State<'_, BleManagerRef>,
    device_id: String,
) -> Result<Vec<BleService>> {
    info!("发现BLE设备GATT服务: {}", device_id);
    
    let manager = manager.inner();
    match manager.discover_services(&device_id).await {
        Ok(services) => {
            info!("发现 {} 个GATT服务", services.len());
            for service in &services {
                debug!("GATT服务: {} ({})", service.uuid, if service.primary { "主要" } else { "次要" });
            }
            Ok(services)
        }
        Err(e) => {
            error!("发现GATT服务失败 {}: {}", device_id, e);
            Err(e)
        }
    }
}

/// 发现GATT特征
/// 
/// 发现指定服务的所有GATT特征
#[tauri::command]
pub async fn discover_ble_characteristics(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    service_uuid: String,
) -> Result<Vec<BleCharacteristic>> {
    info!("发现GATT特征，设备: {}, 服务: {}", device_id, service_uuid);
    
    let manager = manager.inner();
    match manager.discover_characteristics(&device_id, &service_uuid).await {
        Ok(characteristics) => {
            info!("发现 {} 个GATT特征", characteristics.len());
            for char in &characteristics {
                debug!(
                    "GATT特征: {} (读:{}, 写:{}, 通知:{})",
                    char.uuid,
                    char.properties.read,
                    char.properties.write,
                    char.properties.notify
                );
            }
            Ok(characteristics)
        }
        Err(e) => {
            error!("发现GATT特征失败，设备: {}, 服务: {}: {}", device_id, service_uuid, e);
            Err(e)
        }
    }
}

/// 读取特征值
/// 
/// 读取指定特征的值
#[tauri::command]
pub async fn read_ble_characteristic(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    characteristic_uuid: String,
) -> Result<Vec<u8>> {
    debug!("读取特征值，设备: {}, 特征: {}", device_id, characteristic_uuid);
    
    let manager = manager.inner();
    match manager.read_characteristic(&device_id, &characteristic_uuid).await {
        Ok(data) => {
            debug!("读取特征值成功，长度: {} 字节", data.len());
            Ok(data)
        }
        Err(e) => {
            error!("读取特征值失败，设备: {}, 特征: {}: {}", device_id, characteristic_uuid, e);
            Err(e)
        }
    }
}

/// 写入特征值
/// 
/// 向指定特征写入数据
#[tauri::command]
pub async fn write_ble_characteristic(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    characteristic_uuid: String,
    data: Vec<u8>,
) -> Result<()> {
    debug!("写入特征值，设备: {}, 特征: {}, 数据长度: {} 字节", device_id, characteristic_uuid, data.len());
    
    let manager = manager.inner();
    match manager.write_characteristic(&device_id, &characteristic_uuid, &data).await {
        Ok(()) => {
            debug!("写入特征值成功");
            Ok(())
        }
        Err(e) => {
            error!("写入特征值失败，设备: {}, 特征: {}: {}", device_id, characteristic_uuid, e);
            Err(e)
        }
    }
}

/// 订阅特征通知
/// 
/// 订阅指定特征的通知，当特征值变化时会通过事件推送到前端
#[tauri::command]
pub async fn subscribe_ble_notify(
    manager: State<'_, BleManagerRef>,
    app: AppHandle,
    device_id: String,
    characteristic_uuid: String,
) -> Result<()> {
    info!("订阅特征通知，设备: {}, 特征: {}", device_id, characteristic_uuid);
    
    let manager = manager.inner();

    let app_clone = app.clone();
    let device_id_clone = device_id.clone();
    let char_clone = characteristic_uuid.clone();
    let callback = std::sync::Arc::new(move |_addr: &str, _char: &str, data: &[u8]| {
        debug!("收到BLE通知，设备: {}, 特征: {}, 数据长度: {}", _addr, _char, data.len());
        let event = BleNotifyEvent {
            address: device_id_clone.clone(),
            char_uuid: char_clone.clone(),
            data: data.to_vec(),
        };
        let _ = app_clone.emit("ble-notify", &event);
    });

    match manager.subscribe_notify(&device_id, &characteristic_uuid, callback).await {
        Ok(()) => {
            info!("订阅特征通知成功");
            Ok(())
        }
        Err(e) => {
            error!("订阅特征通知失败，设备: {}, 特征: {}: {}", device_id, characteristic_uuid, e);
            Err(e)
        }
    }
}

/// 取消订阅特征通知
/// 
/// 取消订阅指定特征的通知
#[tauri::command]
pub async fn unsubscribe_ble_notify(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    characteristic_uuid: String,
) -> Result<()> {
    info!("取消订阅特征通知，设备: {}, 特征: {}", device_id, characteristic_uuid);
    
    let manager = manager.inner();
    match manager.unsubscribe_notify(&device_id, &characteristic_uuid).await {
        Ok(()) => {
            info!("取消订阅成功");
            Ok(())
        }
        Err(e) => {
            error!("取消订阅失败，设备: {}, 特征: {}: {}", device_id, characteristic_uuid, e);
            Err(e)
        }
    }
}

/// 获取BLE信号强度
/// 
/// 获取指定设备的RSSI信号强度值
#[tauri::command]
pub async fn get_ble_rssi(
    manager: State<'_, BleManagerRef>,
    address: String,
) -> Result<i16> {
    debug!("获取BLE信号强度，设备: {}", address);
    
    let manager = manager.inner();
    match manager.get_rssi(&address).await {
        Ok(rssi) => {
            debug!("BLE信号强度: {} dBm", rssi);
            Ok(rssi)
        }
        Err(e) => {
            error!("获取信号强度失败，设备: {}: {}", address, e);
            Err(e)
        }
    }
}

/// 获取BLE模式
/// 
/// 获取当前BLE的工作模式
#[tauri::command]
pub async fn get_ble_mode(
    manager: State<'_, BleManagerRef>,
) -> Result<String> {
    debug!("获取BLE模式");
    
    let manager = manager.inner();
    let mode = manager.mode().await.to_string();
    debug!("当前BLE模式: {}", mode);
    Ok(mode)
}

/// 检查BLE是否已配置
/// 
/// 检查BLE是否已完成初始化配置
#[tauri::command]
pub async fn is_ble_configured(
    manager: State<'_, BleManagerRef>,
) -> Result<bool> {
    debug!("检查BLE是否已配置");
    
    let manager = manager.inner();
    let configured = manager.is_configured().await;
    debug!("BLE配置状态: {}", if configured { "已配置" } else { "未配置" });
    Ok(configured)
}
