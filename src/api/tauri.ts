import type { SerialConfig, SerialPortInfo } from '../types';
import type { BleDeviceInfo, BleScanOptions, BleConnection, BleService, BleCharacteristic } from '../types';
import type { 
  SerialOpenParams, 
  SerialWriteParams, 
  BleScanResult,
  BleWriteParams,
  BleConfigureParams,
  BleConnectParams,
  BleDiscoverServicesParams,
  BleDiscoverCharacteristicsParams,
  BleReadParams,
  BleSubscribeParams,
  PluginInfo,
  ProtocolLoadParams,
  ProtocolBindParams
} from './types';

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
  return tauriInvoke<T>(cmd, args);
}

/**
 * 串口相关API
 * 所有命令名称与后端 commands/serial.rs 中注册的命令名称保持一致
 */
export const serialApi = {
  /**
   * 扫描可用串口列表
   * 对应后端命令: scan_serial_ports
   */
  async listPorts(): Promise<SerialPortInfo[]> {
    return invoke<SerialPortInfo[]>('scan_serial_ports');
  },

  scanPorts(): Promise<SerialPortInfo[]> {
    return this.listPorts();
  },

  /**
   * 打开串口
   * 对应后端命令: open_serial_port
   */
  async open(params: SerialOpenParams): Promise<void> {
    await invoke<void>('open_serial_port', { params });
  },

  openPort(portName: string, config: SerialConfig): Promise<void> {
    return this.open({ portName, config });
  },

  /**
   * 关闭串口
   * 对应后端命令: close_serial_port
   */
  async close(portName: string): Promise<void> {
    await invoke<void>('close_serial_port', { portName });
  },

  closePort(portName: string): Promise<void> {
    return this.close(portName);
  },

  /**
   * 发送数据
   * 对应后端命令: send_serial_data
   */
  async write(params: SerialWriteParams): Promise<void> {
    await invoke<void>('send_serial_data', { params });
  },

  sendData(portName: string, data: number[]): Promise<void> {
    return this.write({ portName, data });
  },

  /**
   * 获取已打开的端口列表
   * 对应后端命令: get_open_ports
   */
  async getOpenPorts(): Promise<string[]> {
    return invoke<string[]>('get_open_ports');
  },

  /**
   * 检查端口是否已打开
   * 对应后端命令: is_port_open
   */
  async isConnected(portName: string): Promise<boolean> {
    return invoke<boolean>('is_port_open', { portName });
  },
};

/**
 * BLE相关API
 * 所有命令名称与后端 commands/ble.rs 中注册的命令名称保持一致
 */
export const bleApi = {
  /**
   * 配置BLE模式（原生/AT）
   * 对应后端命令: configure_ble
   */
  async configure(params: BleConfigureParams): Promise<void> {
    await invoke<void>('configure_ble', { params });
  },

  configureBle(mode: 'native' | 'at', serialPort?: string): Promise<void> {
    return this.configure({ mode, serialPort });
  },

  /**
   * 扫描BLE设备
   * 对应后端命令: scan_ble_devices
   */
  async scan(options?: BleScanOptions): Promise<BleDeviceInfo[]> {
    const result = await invoke<BleScanResult>('scan_ble_devices', { options });
    return result.devices;
  },

  scanBleDevices(options?: BleScanOptions): Promise<BleDeviceInfo[]> {
    return this.scan(options);
  },

  /**
   * 连接BLE设备
   * 对应后端命令: connect_ble
   */
  async connect(params: BleConnectParams): Promise<BleConnection> {
    return invoke<BleConnection>('connect_ble', { params });
  },

  connectBle(address: string, timeout?: number): Promise<BleConnection> {
    return this.connect({ address, timeout });
  },

  /**
   * 断开BLE连接
   * 对应后端命令: disconnect_ble
   */
  async disconnect(deviceId: string): Promise<void> {
    await invoke<void>('disconnect_ble', { deviceId });
  },

  disconnectBle(deviceId: string): Promise<void> {
    return this.disconnect(deviceId);
  },

  /**
   * 获取BLE连接列表
   * 对应后端命令: get_ble_connections
   */
  async getConnections(): Promise<BleConnection[]> {
    return invoke<BleConnection[]>('get_ble_connections');
  },

  /**
   * 发现GATT服务
   * 对应后端命令: discover_ble_services
   */
  async discoverServices(params: BleDiscoverServicesParams): Promise<BleService[]> {
    return invoke<BleService[]>('discover_ble_services', { params });
  },

  discoverBleServices(deviceId: string): Promise<BleService[]> {
    return this.discoverServices({ deviceId });
  },

  /**
   * 发现GATT特征
   * 对应后端命令: discover_ble_characteristics
   */
  async discoverCharacteristics(params: BleDiscoverCharacteristicsParams): Promise<BleCharacteristic[]> {
    return invoke<BleCharacteristic[]>('discover_ble_characteristics', { params });
  },

  discoverBleCharacteristics(deviceId: string, serviceUuid: string): Promise<BleCharacteristic[]> {
    return this.discoverCharacteristics({ deviceId, serviceUuid });
  },

  /**
   * 读取特征值
   * 对应后端命令: read_ble_characteristic
   */
  async read(params: BleReadParams): Promise<number[]> {
    return invoke<number[]>('read_ble_characteristic', { params });
  },

  readBleCharacteristic(deviceId: string, characteristicUuid: string): Promise<number[]> {
    return this.read({ deviceId, characteristicUuid });
  },

  /**
   * 写入特征值
   * 对应后端命令: write_ble_characteristic
   */
  async write(params: BleWriteParams): Promise<void> {
    await invoke<void>('write_ble_characteristic', { params });
  },

  writeBleCharacteristic(deviceId: string, characteristicUuid: string, data: number[], withoutResponse?: boolean): Promise<void> {
    return this.write({ deviceId, characteristicUuid, data, withoutResponse });
  },

  /**
   * 订阅通知
   * 对应后端命令: subscribe_ble_notify
   */
  async subscribe(params: BleSubscribeParams): Promise<void> {
    await invoke<void>('subscribe_ble_notify', { params });
  },

  subscribeBleNotify(deviceId: string, characteristicUuid: string): Promise<void> {
    return this.subscribe({ deviceId, characteristicUuid });
  },

  /**
   * 取消订阅通知
   * 对应后端命令: unsubscribe_ble_notify
   */
  async unsubscribe(deviceId: string, characteristicUuid: string): Promise<void> {
    await invoke<void>('unsubscribe_ble_notify', { deviceId, characteristicUuid });
  },

  unsubscribeBleNotify(deviceId: string, characteristicUuid: string): Promise<void> {
    return this.unsubscribe(deviceId, characteristicUuid);
  },

  /**
   * 获取信号强度
   * 对应后端命令: get_ble_rssi
   */
  async getRssi(deviceId: string): Promise<number> {
    return invoke<number>('get_ble_rssi', { deviceId });
  },

  /**
   * 获取BLE模式
   * 对应后端命令: get_ble_mode
   */
  async getMode(): Promise<string> {
    return invoke<string>('get_ble_mode');
  },

  /**
   * 检查BLE是否已配置
   * 对应后端命令: is_ble_configured
   */
  async isConfigured(): Promise<boolean> {
    return invoke<boolean>('is_ble_configured');
  },
};

/**
 * 系统相关API
 * 所有命令名称与后端 commands/system.rs 中注册的命令名称保持一致
 */
export const systemApi = {
  /**
   * 获取系统信息
   * 对应后端命令: get_system_info
   */
  async getSystemInfo() {
    return invoke<{
      os: string;
      arch: string;
      version: string;
      hostname: string;
      cpu_count: number;
      total_memory: number;
      used_memory: number;
    }>('get_system_info');
  },

  /**
   * 获取系统状态
   * 对应后端命令: get_system_status
   */
  async getSystemStatus() {
    return invoke<{
      cpu_usage: number;
      memory_usage: number;
      disk_usage: number;
      uptime: number;
    }>('get_system_status');
  },

  /**
   * 获取应用版本
   * 对应后端命令: get_app_version
   */
  async getAppVersion(): Promise<string> {
    return invoke<string>('get_app_version');
  },

  /**
   * 获取平台信息
   * 对应后端命令: get_platform
   */
  async getPlatform(): Promise<string> {
    return invoke<string>('get_platform');
  },

  /**
   * 打开URL
   * 对应后端命令: open_url
   */
  async openUrl(url: string): Promise<void> {
    await invoke<void>('open_url', { url });
  },

  /**
   * 在文件管理器中显示
   * 对应后端命令: show_in_folder
   */
  async showInFolder(path: string): Promise<void> {
    await invoke<void>('show_in_folder', { path });
  },

  /**
   * 配置日志
   * 对应后端命令: configure_log
   */
  async configureLog(level: string, filePath?: string): Promise<void> {
    await invoke<void>('configure_log', { level, filePath });
  },

  /**
   * 获取日志配置
   * 对应后端命令: get_log_config
   */
  async getLogConfig(): Promise<{ level: string; filePath: string }> {
    return invoke<{ level: string; filePath: string }>('get_log_config');
  },
};

/**
 * 协议相关API
 * 所有命令名称与后端 commands/protocol.rs 中注册的命令名称保持一致
 */
export const protocolApi = {
  /**
   * 加载协议
   * 对应后端命令: load_protocol
   */
  async load(params: ProtocolLoadParams): Promise<PluginInfo> {
    return invoke<PluginInfo>('load_protocol', { ...params });
  },

  async loadProtocol(pluginId: string, path: string): Promise<PluginInfo> {
    return this.load({ plugin_id: pluginId, path });
  },

  /**
   * 卸载协议
   * 对应后端命令: unload_protocol
   */
  async unload(pluginId: string): Promise<void> {
    await invoke<void>('unload_protocol', { plugin_id: pluginId });
  },

  async unloadProtocol(pluginId: string): Promise<void> {
    return this.unload(pluginId);
  },

  /**
   * 启用协议
   * 对应后端命令: enable_protocol
   */
  async enable(pluginId: string): Promise<void> {
    await invoke<void>('enable_protocol', { plugin_id: pluginId });
  },

  async enableProtocol(pluginId: string): Promise<void> {
    return this.enable(pluginId);
  },

  /**
   * 禁用协议
   * 对应后端命令: disable_protocol
   */
  async disable(pluginId: string): Promise<void> {
    await invoke<void>('disable_protocol', { plugin_id: pluginId });
  },

  async disableProtocol(pluginId: string): Promise<void> {
    return this.disable(pluginId);
  },

  /**
   * 绑定协议到设备
   * 对应后端命令: bind_protocol
   */
  async bind(params: ProtocolBindParams): Promise<void> {
    await invoke<void>('bind_protocol', { ...params });
  },

  async bindProtocol(pluginId: string, deviceId: string): Promise<void> {
    return this.bind({ plugin_id: pluginId, device_id: deviceId });
  },

  /**
   * 解绑协议
   * 对应后端命令: unbind_protocol
   */
  async unbind(params: ProtocolBindParams): Promise<void> {
    await invoke<void>('unbind_protocol', { ...params });
  },

  async unbindProtocol(pluginId: string, deviceId: string): Promise<void> {
    return this.unbind({ plugin_id: pluginId, device_id: deviceId });
  },

  /**
   * 获取协议列表
   * 对应后端命令: list_protocols
   */
  async list(): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('list_protocols');
  },

  async listProtocols(): Promise<PluginInfo[]> {
    return this.list();
  },

  /**
   * 获取单个协议信息
   * 对应后端命令: get_protocol
   */
  async get(pluginId: string): Promise<PluginInfo> {
    return invoke<PluginInfo>('get_protocol', { plugin_id: pluginId });
  },

  async getProtocol(pluginId: string): Promise<PluginInfo> {
    return this.get(pluginId);
  },

  /**
   * 获取设备绑定的协议
   * 对应后端命令: get_bound_protocols
   */
  async getBound(deviceId: string): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('get_bound_protocols', { device_id: deviceId });
  },

  async getBoundProtocols(deviceId: string): Promise<PluginInfo[]> {
    return this.getBound(deviceId);
  },
};

/**
 * WebSocket相关API
 * 所有命令名称与后端 commands/websocket.rs 中注册的命令名称保持一致
 */
export const websocketApi = {
  /**
   * 连接WebSocket
   * 对应后端命令: connect_websocket
   */
  async connect(id: string, url: string, reconnect?: boolean): Promise<void> {
    await invoke<void>('connect_websocket', { id, url, reconnect });
  },

  /**
   * 发送消息
   * 对应后端命令: send_websocket_message
   */
  async send(id: string, message: string): Promise<void> {
    await invoke<void>('send_websocket_message', { id, message });
  },

  /**
   * 断开连接
   * 对应后端命令: disconnect_websocket
   */
  async disconnect(id: string): Promise<void> {
    await invoke<void>('disconnect_websocket', { id });
  },

  /**
   * 获取连接状态
   * 对应后端命令: get_websocket_status
   */
  async getStatus(id: string): Promise<string> {
    return invoke<string>('get_websocket_status', { id });
  },

  /**
   * 获取所有连接ID
   * 对应后端命令: get_all_websocket_connections
   */
  async getAllConnections(): Promise<string[]> {
    return invoke<string[]>('get_all_websocket_connections');
  },

  /**
   * 获取所有连接状态
   * 对应后端命令: get_all_websocket_status
   */
  async getAllStatus(): Promise<Record<string, string>> {
    return invoke<Record<string, string>>('get_all_websocket_status');
  },
};
