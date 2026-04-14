use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, error, info};

use crate::device::ble::{
    AtConfig, AtConnectionTab, BleCharacteristic, BleConnection, BleDevice, BleManagerRef, BleMode, BleService,
};
use crate::error::{ComBridgeError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleConfigDto {
    pub mode: String,
    pub port_name: Option<String>,
    pub baud_rate: Option<u32>,
    pub timeout_ms: Option<u64>,
    pub tx_uuid: Option<String>,
    pub rx_uuid: Option<String>,
    pub srv_uuid: Option<String>,
}

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
                tx_uuid: config.tx_uuid,
                rx_uuid: config.rx_uuid,
                srv_uuid: config.srv_uuid,
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

#[tauri::command]
pub async fn stop_ble_scan(
    manager: State<'_, BleManagerRef>,
) -> Result<Vec<BleDevice>> {
    info!("停止BLE扫描");
    
    let manager = manager.inner();
    match manager.stop_scan().await {
        Ok(devices) => {
            info!("BLE扫描已停止，返回 {} 个设备", devices.len());
            Ok(devices)
        }
        Err(e) => {
            error!("停止BLE扫描失败: {}", e);
            Err(e)
        }
    }
}

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
            
            let manager_clone = manager.clone();
            let device_id_clone = device_id.clone();
            let callback = std::sync::Arc::new(move |_addr: &str, _char: &str, data: &[u8]| {
                debug!("收到BLE通知，设备: {}, 数据长度: {}", _addr, data.len());
                
                let manager = manager_clone.clone();
                let device_id = device_id_clone.clone();
                let data_vec = data.to_vec();
                tokio::spawn(async move {
                    manager.add_at_received_data(&device_id, data_vec).await;
                });
            });
            
            let _ = manager.subscribe_notify(&device_id, "", callback).await;
            
            Ok(conn)
        }
        Err(e) => {
            error!("BLE设备连接失败 {}: {}", device_id, e);
            Err(e)
        }
    }
}

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
            manager.add_at_sent_data(&device_id, data).await;
            Ok(())
        }
        Err(e) => {
            error!("写入特征值失败，设备: {}, 特征: {}: {}", device_id, characteristic_uuid, e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn write_ble_without_response(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    characteristic_uuid: String,
    data: Vec<u8>,
) -> Result<()> {
    debug!("无响应写入特征值，设备: {}, 特征: {}, 数据长度: {} 字节", device_id, characteristic_uuid, data.len());
    
    let manager = manager.inner();
    match manager.write_without_response(&device_id, &characteristic_uuid, &data).await {
        Ok(()) => {
            debug!("无响应写入成功");
            manager.add_at_sent_data(&device_id, data).await;
            Ok(())
        }
        Err(e) => {
            error!("无响应写入失败，设备: {}, 特征: {}: {}", device_id, characteristic_uuid, e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn subscribe_ble_notify(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    characteristic_uuid: String,
) -> Result<()> {
    info!("订阅特征通知，设备: {}, 特征: {}", device_id, characteristic_uuid);
    
    let manager = manager.inner();

    let existing = manager.get_subscriptions(&device_id).await;
    if existing.contains(&characteristic_uuid) {
        info!("特征已订阅，跳过重复订阅: 设备: {}, 特征: {}", device_id, characteristic_uuid);
        return Ok(());
    }

    let callback = std::sync::Arc::new(move |_addr: &str, _char: &str, data: &[u8]| {
        debug!("收到BLE通知，设备: {}, 特征: {}, 数据长度: {}", _addr, _char, data.len());
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

#[tauri::command]
pub async fn set_ble_mtu(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    mtu: u16,
) -> Result<u16> {
    info!("设置BLE MTU，设备: {}, 请求MTU: {}", device_id, mtu);
    
    let manager = manager.inner();
    match manager.set_mtu(&device_id, mtu).await {
        Ok(actual_mtu) => {
            info!("MTU协商成功，实际MTU: {}", actual_mtu);
            Ok(actual_mtu)
        }
        Err(e) => {
            error!("MTU协商失败，设备: {}: {}", device_id, e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_ble_subscriptions(
    manager: State<'_, BleManagerRef>,
    device_id: String,
) -> Result<Vec<String>> {
    debug!("获取已订阅特征列表，设备: {}", device_id);
    
    let manager = manager.inner();
    let subscriptions = manager.get_subscriptions(&device_id).await;
    debug!("设备 {} 已订阅 {} 个特征", device_id, subscriptions.len());
    Ok(subscriptions)
}

#[tauri::command]
pub async fn get_at_config(
    manager: State<'_, BleManagerRef>,
) -> Result<AtConfig> {
    debug!("获取AT配置");
    
    let manager = manager.inner();
    let config = manager.get_at_config().await;
    Ok(config)
}

#[tauri::command]
pub async fn update_at_uuid_config(
    manager: State<'_, BleManagerRef>,
    tx_uuid: Option<String>,
    rx_uuid: Option<String>,
    srv_uuid: Option<String>,
) -> Result<()> {
    info!("更新AT UUID配置");
    
    let manager = manager.inner();
    manager.update_at_uuid_config(tx_uuid, rx_uuid, srv_uuid).await;
    Ok(())
}

#[tauri::command]
pub async fn get_at_tabs(
    manager: State<'_, BleManagerRef>,
) -> Result<Vec<AtConnectionTab>> {
    debug!("获取AT连接TAB列表");
    
    let manager = manager.inner();
    let tabs = manager.get_at_tabs().await;
    debug!("当前有 {} 个AT连接TAB", tabs.len());
    Ok(tabs)
}

#[tauri::command]
pub async fn get_at_tab(
    manager: State<'_, BleManagerRef>,
    tab_id: String,
) -> Result<Option<AtConnectionTab>> {
    debug!("获取AT连接TAB: {}", tab_id);
    
    let manager = manager.inner();
    let tab = manager.get_at_tab(&tab_id).await;
    Ok(tab)
}

#[tauri::command]
pub async fn clear_at_tab_data(
    manager: State<'_, BleManagerRef>,
    tab_id: String,
) -> Result<()> {
    info!("清空AT连接TAB数据: {}", tab_id);
    
    let manager = manager.inner();
    manager.clear_at_tab_data(&tab_id).await;
    Ok(())
}

#[tauri::command]
pub async fn remove_at_tab(
    manager: State<'_, BleManagerRef>,
    tab_id: String,
) -> Result<()> {
    info!("移除AT连接TAB: {}", tab_id);
    
    let manager = manager.inner();
    manager.remove_at_tab(&tab_id).await;
    Ok(())
}

#[tauri::command]
pub async fn send_at_data(
    manager: State<'_, BleManagerRef>,
    device_id: String,
    data: Vec<u8>,
) -> Result<()> {
    debug!("发送AT透传数据，设备: {}, 数据长度: {} 字节", device_id, data.len());
    
    let manager = manager.inner();
    match manager.write_characteristic(&device_id, "", &data).await {
        Ok(()) => {
            debug!("AT透传数据发送成功");
            manager.add_at_sent_data(&device_id, data).await;
            Ok(())
        }
        Err(e) => {
            error!("AT透传数据发送失败，设备: {}: {}", device_id, e);
            Err(e)
        }
    }
}
