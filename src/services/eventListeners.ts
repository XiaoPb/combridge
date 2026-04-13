import {
  onSerialData,
  onSerialError,
  onSerialConnected,
  onSerialDisconnected,
  onBleData,
  onBleConnected,
  onBleDisconnected,
  onBleError,
  onBleScanResult,
  onBleModeChanged,
} from '../api/events';
import { useSerialStore, generateId } from '../stores/serialStore';
import { useBleStore, generateBleId } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import { useNotificationStore } from '../stores/notificationStore';
import type { UnlistenFn } from '@tauri-apps/api/event';

let serialListeners: {
  data?: UnlistenFn;
  error?: UnlistenFn;
  connected?: UnlistenFn;
  disconnected?: UnlistenFn;
} = {};

let bleListeners: {
  data?: UnlistenFn;
  connected?: UnlistenFn;
  disconnected?: UnlistenFn;
  error?: UnlistenFn;
  scanResult?: UnlistenFn;
  modeChanged?: UnlistenFn;
} = {};

let serialInitialized = false;
let bleInitialized = false;
let serialInitPromise: Promise<void> | null = null;
let bleInitPromise: Promise<void> | null = null;

export async function initSerialEventListeners(): Promise<void> {
  if (serialInitialized) {
    return;
  }

  if (serialInitPromise) {
    return serialInitPromise;
  }

  serialInitPromise = (async () => {
    serialListeners.data = await onSerialData((event) => {
      const store = useSerialStore.getState();
      store.addReceivedData(event.port_name, {
        id: generateId(),
        timestamp: event.timestamp ?? Date.now(),
        data: event.data,
        direction: 'receive',
        format: 'hex',
      });
    });

    serialListeners.error = await onSerialError((event) => {
      const store = useSerialStore.getState();
      store.setError(event.error);
      useLogStore.getState().addLog('error', 'SerialManager', `串口错误: ${event.error}`);
      useNotificationStore.getState().addNotification('error', `串口错误: ${event.error}`);
    });

    serialListeners.connected = await onSerialConnected((portName) => {
      useLogStore.getState().addLog('info', 'SerialManager', `串口 ${portName} 已连接`);
      useNotificationStore.getState().addNotification('success', `串口 ${portName} 已连接`);
    });

    serialListeners.disconnected = await onSerialDisconnected((portName) => {
      const store = useSerialStore.getState();
      const tab = store.tabs.find((t) => t.portName === portName && t.tabType === 'port');
      if (tab) {
        store.updateTab(tab.key, { isConnected: false });
      }
      useLogStore.getState().addLog('info', 'SerialManager', `串口 ${portName} 已断开`);
      useNotificationStore.getState().addNotification('info', `串口 ${portName} 已断开`);
    });

    serialInitialized = true;
    serialInitPromise = null;
  })();

  return serialInitPromise;
}

export async function cleanupSerialEventListeners(): Promise<void> {
  if (serialListeners.data) {
    serialListeners.data();
    serialListeners.data = undefined;
  }
  if (serialListeners.error) {
    serialListeners.error();
    serialListeners.error = undefined;
  }
  if (serialListeners.connected) {
    serialListeners.connected();
    serialListeners.connected = undefined;
  }
  if (serialListeners.disconnected) {
    serialListeners.disconnected();
    serialListeners.disconnected = undefined;
  }
  serialInitialized = false;
}

export function isSerialListenersInitialized(): boolean {
  return serialInitialized;
}

export async function initBleEventListeners(): Promise<void> {
  if (bleInitialized) {
    return;
  }

  if (bleInitPromise) {
    return bleInitPromise;
  }

  bleInitPromise = (async () => {
    bleListeners.data = await onBleData((event) => {
      const store = useBleStore.getState();
      store.addNotification({
        id: generateBleId(),
        deviceId: event.deviceId,
        characteristicUuid: event.characteristicUuid,
        data: event.data,
        timestamp: event.timestamp,
      });
    });

    bleListeners.connected = await onBleConnected((event) => {
      const store = useBleStore.getState();
      store.addConnection({
        deviceId: event.deviceId,
        address: event.address,
        name: event.name,
        isConnected: true,
        services: [],
        connectedAt: Date.now(),
      });
      store.setIsConnecting(false);
      useLogStore.getState().addLog('info', 'BleManager', `设备 ${event.name || event.address} 已连接`);
      useNotificationStore.getState().addNotification('success', `设备 ${event.name || event.address} 已连接`);
    });

    bleListeners.disconnected = await onBleDisconnected((event) => {
      const store = useBleStore.getState();
      store.removeConnection(event.deviceId);
      if (store.currentDevice === event.deviceId) {
        store.setCurrentDevice(null);
        store.clearServices();
        store.clearCharacteristics();
      }
      useLogStore.getState().addLog('info', 'BleManager', `设备 ${event.address} 已断开`);
      useNotificationStore.getState().addNotification('info', `设备 ${event.address} 已断开`);
    });

    bleListeners.error = await onBleError((event) => {
      const errorMsg = event.error;
      const store = useBleStore.getState();
      store.setError(errorMsg);
      store.setIsConnecting(false);
      store.setIsScanning(false);
      useLogStore.getState().addLog('error', 'BleManager', `BLE错误: ${errorMsg}`);
      useNotificationStore.getState().addNotification('error', `BLE错误: ${errorMsg}`);
    });

    bleListeners.scanResult = await onBleScanResult((device: unknown) => {
      const deviceInfo = device as {
        address: string;
        name?: string;
        rssi?: number;
        isConnectable: boolean;
        services?: string[];
      };
      const store = useBleStore.getState();
      store.addDevice({
        address: deviceInfo.address,
        name: deviceInfo.name,
        rssi: deviceInfo.rssi,
        isConnectable: deviceInfo.isConnectable,
        services: deviceInfo.services,
        discoveredAt: Date.now(),
      });
    });

    bleListeners.modeChanged = await onBleModeChanged((event) => {
      const store = useBleStore.getState();
      store.setMode(event.mode);
      store.setSerialPort(event.serialPort || null);
      useLogStore.getState().addLog('info', 'BleManager', `BLE模式已切换为 ${event.mode}`);
      useNotificationStore.getState().addNotification('info', `BLE模式已切换为 ${event.mode}`);
    });

    bleInitialized = true;
    bleInitPromise = null;
  })();

  return bleInitPromise;
}

export async function cleanupBleEventListeners(): Promise<void> {
  if (bleListeners.data) {
    bleListeners.data();
    bleListeners.data = undefined;
  }
  if (bleListeners.connected) {
    bleListeners.connected();
    bleListeners.connected = undefined;
  }
  if (bleListeners.disconnected) {
    bleListeners.disconnected();
    bleListeners.disconnected = undefined;
  }
  if (bleListeners.error) {
    bleListeners.error();
    bleListeners.error = undefined;
  }
  if (bleListeners.scanResult) {
    bleListeners.scanResult();
    bleListeners.scanResult = undefined;
  }
  if (bleListeners.modeChanged) {
    bleListeners.modeChanged();
    bleListeners.modeChanged = undefined;
  }
  bleInitialized = false;
}

export function isBleListenersInitialized(): boolean {
  return bleInitialized;
}

export async function initAllEventListeners(): Promise<void> {
  await Promise.all([initSerialEventListeners(), initBleEventListeners()]);
}

export async function cleanupAllEventListeners(): Promise<void> {
  await Promise.all([cleanupSerialEventListeners(), cleanupBleEventListeners()]);
}
