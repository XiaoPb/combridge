import type { SerialConfig, SerialPortInfo } from '../types';
import type { BleDeviceInfo, BleScanOptions, BleConnection } from '../types';
import type { 
  SerialListPortsResult, 
  SerialOpenParams, 
  SerialWriteParams,
  BleScanResult,
  BleWriteParams
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

  async open(params: SerialOpenParams): Promise<void> {
    await invoke<void>('serial_open', { params });
  },

  async close(portName: string): Promise<void> {
    await invoke<void>('serial_close', { portName });
  },

  async write(params: SerialWriteParams): Promise<void> {
    await invoke<void>('serial_write', { params });
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
};

export const bleApi = {
  async scan(options?: BleScanOptions): Promise<BleDeviceInfo[]> {
    const result = await invoke<BleScanResult>('ble_scan', { options });
    return result.devices;
  },

  async stopScan(): Promise<void> {
    await invoke<void>('ble_stop_scan');
  },

  async connect(address: string): Promise<BleConnection> {
    return invoke<BleConnection>('ble_connect', { address });
  },

  async disconnect(deviceId: string): Promise<void> {
    await invoke<void>('ble_disconnect', { deviceId });
  },

  async write(params: BleWriteParams): Promise<void> {
    await invoke<void>('ble_write', { params });
  },

  async read(deviceId: string, characteristicUuid: string): Promise<number[]> {
    return invoke<number[]>('ble_read', { deviceId, characteristicUuid });
  },

  async subscribe(deviceId: string, characteristicUuid: string): Promise<void> {
    await invoke<void>('ble_subscribe', { deviceId, characteristicUuid });
  },

  async unsubscribe(deviceId: string, characteristicUuid: string): Promise<void> {
    await invoke<void>('ble_unsubscribe', { deviceId, characteristicUuid });
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
