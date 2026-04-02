import type { SerialConfig, SerialPortInfo } from '../types';
import type { BleDeviceInfo, BleScanOptions, BleConnection, BleService, BleCharacteristic } from '../types';
import type { 
  SerialListPortsResult, 
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

export const serialApi = {
  async listPorts(): Promise<SerialPortInfo[]> {
    const result = await invoke<SerialListPortsResult>('serial_list_ports');
    return result.ports;
  },

  scanPorts(): Promise<SerialPortInfo[]> {
    return this.listPorts();
  },

  async open(params: SerialOpenParams): Promise<void> {
    await invoke<void>('serial_open', { params });
  },

  openPort(portName: string, config: SerialConfig): Promise<void> {
    return this.open({ portName, config });
  },

  async close(portName: string): Promise<void> {
    await invoke<void>('serial_close', { portName });
  },

  closePort(portName: string): Promise<void> {
    return this.close(portName);
  },

  async write(params: SerialWriteParams): Promise<void> {
    await invoke<void>('serial_write', { params });
  },

  sendData(portName: string, data: number[]): Promise<void> {
    return this.write({ portName, data });
  },

  async getConfig(portName: string): Promise<SerialConfig> {
    return invoke<SerialConfig>('serial_get_config', { portName });
  },

  async setConfig(portName: string, config: SerialConfig): Promise<void> {
    await invoke<void>('serial_set_config', { portName, config });
  },

  async isConnected(portName: string): Promise<boolean> {
    return invoke<boolean>('serial_is_connected', { portName });
  },

  async getOpenPorts(): Promise<string[]> {
    return invoke<string[]>('serial_get_open_ports');
  },
};

export const bleApi = {
  async configure(params: BleConfigureParams): Promise<void> {
    await invoke<void>('ble_configure', { params });
  },

  configureBle(mode: 'native' | 'at', serialPort?: string): Promise<void> {
    return this.configure({ mode, serialPort });
  },

  async scan(options?: BleScanOptions): Promise<BleDeviceInfo[]> {
    const result = await invoke<BleScanResult>('ble_scan', { options });
    return result.devices;
  },

  scanBleDevices(options?: BleScanOptions): Promise<BleDeviceInfo[]> {
    return this.scan(options);
  },

  async stopScan(): Promise<void> {
    await invoke<void>('ble_stop_scan');
  },

  async connect(params: BleConnectParams): Promise<BleConnection> {
    return invoke<BleConnection>('ble_connect', { params });
  },

  connectBle(address: string, timeout?: number): Promise<BleConnection> {
    return this.connect({ address, timeout });
  },

  async disconnect(deviceId: string): Promise<void> {
    await invoke<void>('ble_disconnect', { deviceId });
  },

  disconnectBle(deviceId: string): Promise<void> {
    return this.disconnect(deviceId);
  },

  async discoverServices(params: BleDiscoverServicesParams): Promise<BleService[]> {
    return invoke<BleService[]>('ble_discover_services', { params });
  },

  discoverBleServices(deviceId: string): Promise<BleService[]> {
    return this.discoverServices({ deviceId });
  },

  async discoverCharacteristics(params: BleDiscoverCharacteristicsParams): Promise<BleCharacteristic[]> {
    return invoke<BleCharacteristic[]>('ble_discover_characteristics', { params });
  },

  discoverBleCharacteristics(deviceId: string, serviceUuid: string): Promise<BleCharacteristic[]> {
    return this.discoverCharacteristics({ deviceId, serviceUuid });
  },

  async read(params: BleReadParams): Promise<number[]> {
    return invoke<number[]>('ble_read', { params });
  },

  readBleCharacteristic(deviceId: string, characteristicUuid: string): Promise<number[]> {
    return this.read({ deviceId, characteristicUuid });
  },

  async write(params: BleWriteParams): Promise<void> {
    await invoke<void>('ble_write', { params });
  },

  writeBleCharacteristic(deviceId: string, characteristicUuid: string, data: number[], withoutResponse?: boolean): Promise<void> {
    return this.write({ deviceId, characteristicUuid, data, withoutResponse });
  },

  async subscribe(params: BleSubscribeParams): Promise<void> {
    await invoke<void>('ble_subscribe', { params });
  },

  subscribeBleNotify(deviceId: string, characteristicUuid: string): Promise<void> {
    return this.subscribe({ deviceId, characteristicUuid });
  },

  async unsubscribe(deviceId: string, characteristicUuid: string): Promise<void> {
    await invoke<void>('ble_unsubscribe', { deviceId, characteristicUuid });
  },

  unsubscribeBleNotify(deviceId: string, characteristicUuid: string): Promise<void> {
    return this.unsubscribe(deviceId, characteristicUuid);
  },

  async isConnected(deviceId: string): Promise<boolean> {
    return invoke<boolean>('ble_is_connected', { deviceId });
  },

  async getMtu(deviceId: string): Promise<number> {
    return invoke<number>('ble_get_mtu', { deviceId });
  },

  async requestMtu(deviceId: string, mtu: number): Promise<number> {
    return invoke<number>('ble_request_mtu', { deviceId, mtu });
  },
};

export const systemApi = {
  async getAppVersion(): Promise<string> {
    return invoke<string>('get_app_version');
  },

  async getPlatform(): Promise<string> {
    return invoke<string>('get_platform');
  },

  async openUrl(url: string): Promise<void> {
    await invoke<void>('open_url', { url });
  },

  async showInFolder(path: string): Promise<void> {
    await invoke<void>('show_in_folder', { path });
  },
};

export const protocolApi = {
  async load(params: ProtocolLoadParams): Promise<PluginInfo> {
    return invoke<PluginInfo>('load_protocol', { ...params });
  },

  async loadProtocol(pluginId: string, path: string): Promise<PluginInfo> {
    return this.load({ plugin_id: pluginId, path });
  },

  async unload(pluginId: string): Promise<void> {
    await invoke<void>('unload_protocol', { plugin_id: pluginId });
  },

  async unloadProtocol(pluginId: string): Promise<void> {
    return this.unload(pluginId);
  },

  async enable(pluginId: string): Promise<void> {
    await invoke<void>('enable_protocol', { plugin_id: pluginId });
  },

  async enableProtocol(pluginId: string): Promise<void> {
    return this.enable(pluginId);
  },

  async disable(pluginId: string): Promise<void> {
    await invoke<void>('disable_protocol', { plugin_id: pluginId });
  },

  async disableProtocol(pluginId: string): Promise<void> {
    return this.disable(pluginId);
  },

  async bind(params: ProtocolBindParams): Promise<void> {
    await invoke<void>('bind_protocol', { ...params });
  },

  async bindProtocol(pluginId: string, deviceId: string): Promise<void> {
    return this.bind({ plugin_id: pluginId, device_id: deviceId });
  },

  async unbind(params: ProtocolBindParams): Promise<void> {
    await invoke<void>('unbind_protocol', { ...params });
  },

  async unbindProtocol(pluginId: string, deviceId: string): Promise<void> {
    return this.unbind({ plugin_id: pluginId, device_id: deviceId });
  },

  async list(): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('list_protocols');
  },

  async listProtocols(): Promise<PluginInfo[]> {
    return this.list();
  },

  async get(pluginId: string): Promise<PluginInfo> {
    return invoke<PluginInfo>('get_protocol', { plugin_id: pluginId });
  },

  async getProtocol(pluginId: string): Promise<PluginInfo> {
    return this.get(pluginId);
  },

  async getBound(deviceId: string): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('get_bound_protocols', { device_id: deviceId });
  },

  async getBoundProtocols(deviceId: string): Promise<PluginInfo[]> {
    return this.getBound(deviceId);
  },
};
