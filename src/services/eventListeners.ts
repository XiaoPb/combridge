import { onSerialData, onSerialError, onSerialConnected, onSerialDisconnected } from '../api/events';
import { useSerialStore, generateId } from '../stores/serialStore';
import { useLogStore } from '../stores/logStore';
import { message } from 'antd';
import type { UnlistenFn } from '@tauri-apps/api/event';

let listeners: {
  data?: UnlistenFn;
  error?: UnlistenFn;
  connected?: UnlistenFn;
  disconnected?: UnlistenFn;
} = {};

let isInitialized = false;
let initPromise: Promise<void> | null = null;

export async function initSerialEventListeners(): Promise<void> {
  if (isInitialized) {
    return;
  }

  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    listeners.data = await onSerialData((event) => {
      const store = useSerialStore.getState();
      store.addReceivedData(event.port_name, {
        id: generateId(),
        timestamp: event.timestamp ?? Date.now(),
        data: event.data,
        direction: 'receive',
        format: 'hex',
      });
    });

    listeners.error = await onSerialError((event) => {
      const store = useSerialStore.getState();
      store.setError(event.error);
      useLogStore.getState().addLog('error', 'SerialManager', `串口错误: ${event.error}`);
      message.error(`串口错误: ${event.error}`);
    });

    listeners.connected = await onSerialConnected((portName) => {
      useLogStore.getState().addLog('info', 'SerialManager', `串口 ${portName} 已连接`);
      message.success(`串口 ${portName} 已连接`);
    });

    listeners.disconnected = await onSerialDisconnected((portName) => {
      const store = useSerialStore.getState();
      const tab = store.tabs.find((t) => t.portName === portName && t.tabType === 'port');
      if (tab) {
        store.updateTab(tab.key, { isConnected: false });
      }
      useLogStore.getState().addLog('info', 'SerialManager', `串口 ${portName} 已断开`);
      message.info(`串口 ${portName} 已断开`);
    });

    isInitialized = true;
    initPromise = null;
  })();

  return initPromise;
}

export async function cleanupSerialEventListeners(): Promise<void> {
  if (listeners.data) {
    listeners.data();
    listeners.data = undefined;
  }
  if (listeners.error) {
    listeners.error();
    listeners.error = undefined;
  }
  if (listeners.connected) {
    listeners.connected();
    listeners.connected = undefined;
  }
  if (listeners.disconnected) {
    listeners.disconnected();
    listeners.disconnected = undefined;
  }
  isInitialized = false;
}

export function isSerialListenersInitialized(): boolean {
  return isInitialized;
}
