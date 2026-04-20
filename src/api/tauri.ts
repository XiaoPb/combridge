import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { SerialConfig, SerialPortInfo } from '../types';
import type { BleDeviceInfo, BleScanOptions, BleConnection, BleService, BleCharacteristic } from '../types';
import type { PluginInfo, CacheData } from './types';
import type { SerialPreferences } from '../stores/serialStore';
import type { BlePreferences } from '../stores/bleStore';

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return tauriInvoke<T>(cmd, args);
}

export const serialApi = {
  async listPorts(): Promise<SerialPortInfo[]> {
    return invoke<SerialPortInfo[]>('scan_serial_ports');
  },

  async openPort(portName: string, config: SerialConfig): Promise<void> {
    await invoke<void>('open_serial_port', {
      config: {
        portName: portName,
        baudRate: String(config.baudRate),
        dataBits: config.dataBits,
        parity: config.parity,
        stopBits: config.stopBits,
        flowControl: config.flowControl,
        packTimeoutMs: 50,
      },
    });
  },

  async closePort(portName: string): Promise<void> {
    await invoke<void>('close_serial_port', { portName });
  },

  async sendData(portName: string, data: number[]): Promise<void> {
    await invoke<void>('send_serial_data', { portName, data });
  },

  async getOpenPorts(): Promise<string[]> {
    return invoke<string[]>('get_open_ports');
  },

  async isConnected(portName: string): Promise<boolean> {
    return invoke<boolean>('is_port_open', { portName });
  },

  async exportData(portName: string, allData: Array<{timestamp: number; data: number[]; direction: string}>, rxData: number[]): Promise<{logPath: string; datPath: string}> {
    return invoke<{logPath: string; datPath: string}>('export_serial_data', {
      portName,
      allData,
      rxData,
    });
  },

  async getCache(portName: string): Promise<CacheData> {
    return invoke<CacheData>('get_serial_cache', { portName });
  },
};

export const bleApi = {
  async configureBle(mode: 'native' | 'at', serialPort?: string): Promise<void> {
    await invoke<void>('configure_ble', {
      config: {
        mode: mode,
        portName: serialPort,
        baudRate: undefined,
        timeoutMs: undefined,
      }
    });
  },

  async scanBleDevices(options?: BleScanOptions): Promise<BleDeviceInfo[]> {
    const durationMs = options?.timeout ?? 5000;
    return invoke<BleDeviceInfo[]>('scan_ble_devices', { durationMs });
  },

  async stopBleScan(): Promise<BleDeviceInfo[]> {
    return invoke<BleDeviceInfo[]>('stop_ble_scan');
  },

  async connectBle(address: string, _timeout?: number): Promise<BleConnection> {
    return invoke<BleConnection>('connect_ble', { deviceId: address });
  },

  async disconnectBle(deviceId: string): Promise<void> {
    await invoke<void>('disconnect_ble', { deviceId });
  },

  async getConnections(): Promise<BleConnection[]> {
    return invoke<BleConnection[]>('get_ble_connections');
  },

  async discoverBleServices(deviceId: string): Promise<BleService[]> {
    return invoke<BleService[]>('discover_ble_services', { deviceId });
  },

  async discoverBleCharacteristics(deviceId: string, serviceUuid: string): Promise<BleCharacteristic[]> {
    return invoke<BleCharacteristic[]>('discover_ble_characteristics', {
      deviceId,
      serviceUuid,
    });
  },

  async readBleCharacteristic(deviceId: string, characteristicUuid: string): Promise<number[]> {
    return invoke<number[]>('read_ble_characteristic', {
      deviceId,
      characteristicUuid,
    });
  },

  async writeBleCharacteristic(deviceId: string, characteristicUuid: string, data: number[], _withoutResponse?: boolean): Promise<void> {
    await invoke<void>('write_ble_characteristic', {
      deviceId,
      characteristicUuid,
      data,
    });
  },

  async writeBleWithoutResponse(deviceId: string, characteristicUuid: string, data: number[]): Promise<void> {
    await invoke<void>('write_ble_without_response', {
      deviceId,
      characteristicUuid,
      data,
    });
  },

  async subscribeBleNotify(deviceId: string, characteristicUuid: string): Promise<void> {
    await invoke<void>('subscribe_ble_notify', {
      deviceId,
      characteristicUuid,
    });
  },

  async unsubscribeBleNotify(deviceId: string, characteristicUuid: string): Promise<void> {
    await invoke<void>('unsubscribe_ble_notify', {
      deviceId,
      characteristicUuid,
    });
  },

  async getRssi(deviceId: string): Promise<number> {
    return invoke<number>('get_ble_rssi', { deviceId });
  },

  async setBleMtu(deviceId: string, mtu: number): Promise<number> {
    return invoke<number>('set_ble_mtu', { deviceId, mtu });
  },

  async getMode(): Promise<string> {
    return invoke<string>('get_ble_mode');
  },

  async isConfigured(): Promise<boolean> {
    return invoke<boolean>('is_ble_configured');
  },

  async getCache(characteristicUuid: string): Promise<CacheData> {
    return invoke<CacheData>('get_ble_cache', { characteristicUuid });
  },

  async getSubscriptions(deviceId: string): Promise<string[]> {
    return invoke<string[]>('get_ble_subscriptions', { deviceId });
  },
};

export const systemApi = {
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

  async getSystemStatus() {
    return invoke<{
      cpu_usage: number;
      memory_usage: number;
      disk_usage: number;
      uptime: number;
    }>('get_system_status');
  },

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

  async configureLog(level: string, filePath?: string): Promise<void> {
    await invoke<void>('configure_log', { level, filePath });
  },

  async getLogConfig(): Promise<{ level: string; filePath: string }> {
    return invoke<{ level: string; filePath: string }>('get_log_config');
  },
};

export const protocolApi = {
  async loadProtocol(pluginId: string, path: string): Promise<PluginInfo> {
    return invoke<PluginInfo>('load_protocol', { pluginId, path });
  },

  async unloadProtocol(pluginId: string): Promise<void> {
    await invoke<void>('unload_protocol', { pluginId });
  },

  async enableProtocol(pluginId: string): Promise<void> {
    await invoke<void>('enable_protocol', { pluginId });
  },

  async disableProtocol(pluginId: string): Promise<void> {
    await invoke<void>('disable_protocol', { pluginId });
  },

  async bindProtocol(pluginId: string, deviceId: string): Promise<void> {
    await invoke<void>('bind_protocol', { pluginId, deviceId });
  },

  async unbindProtocol(pluginId: string, deviceId: string): Promise<void> {
    await invoke<void>('unbind_protocol', { pluginId, deviceId });
  },

  async listProtocols(): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('list_protocols');
  },

  async getProtocol(pluginId: string): Promise<PluginInfo> {
    return invoke<PluginInfo>('get_protocol', { pluginId });
  },

  async getBoundProtocols(deviceId: string): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('get_bound_protocols', { deviceId });
  },
};

export interface WaveformPreferences {
  display_rows: number;
  refresh_interval: number;
  sidebar_collapsed: boolean;
}

export interface Gh3036ChannelPreferences {
  connection_type: string;
  serial_port: string;
  ble_device: string;
  tx_char: string;
  rx_char: string;
}

export interface Preferences {
  serial: SerialPreferences;
  ble: BlePreferences;
  waveform?: WaveformPreferences;
  gh3036_channel?: Gh3036ChannelPreferences;
}

export const preferencesApi = {
  async get(): Promise<Preferences> {
    return invoke<Preferences>('get_preferences');
  },

  async save(prefs: Preferences): Promise<void> {
    await invoke<void>('save_preferences', { prefs });
  },

  async updateSerial(prefs: SerialPreferences): Promise<void> {
    await invoke<void>('update_serial_preferences', {
      displayFormat: prefs.displayFormat,
      displayMode: prefs.displayMode,
      sendFormat: prefs.sendFormat,
      appendNewline: prefs.appendNewline,
      newlineType: prefs.newlineType,
      autoScroll: prefs.autoScroll,
    });
  },

  async updateBle(prefs: BlePreferences): Promise<void> {
    await invoke<void>('update_ble_preferences', {
      displayFormat: prefs.displayFormat,
      autoScroll: prefs.autoScroll,
      inputFormat: prefs.inputFormat,
      withoutResponse: prefs.withoutResponse,
      configCollapsed: prefs.configCollapsed,
      gattCollapsed: prefs.gattCollapsed,
      panelCollapsed: prefs.panelCollapsed,
    });
  },

  async updateWaveform(prefs: WaveformPreferences): Promise<void> {
    await invoke<void>('update_waveform_preferences', {
      displayRows: prefs.display_rows,
      refreshInterval: prefs.refresh_interval,
      sidebarCollapsed: prefs.sidebar_collapsed,
    });
  },

  async updateGh3036Channel(prefs: Gh3036ChannelPreferences): Promise<void> {
    await invoke<void>('update_gh3036_channel_preferences', {
      connectionType: prefs.connection_type,
      serialPort: prefs.serial_port,
      bleDevice: prefs.ble_device,
      txChar: prefs.tx_char,
      rxChar: prefs.rx_char,
    });
  },
};
