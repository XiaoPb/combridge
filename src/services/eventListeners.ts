import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useSerialStore, generateId } from '../stores/serialStore';
import { useBleStore, generateBleId } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import { useNotificationStore } from '../stores/notificationStore';

interface EventBusEvent {
  topic: string;
  payload: string;
  timestamp: number;
}

interface SerialDataPayload {
  device_id: string;
  data: number[];
  timestamp: number;
}

interface SerialConnectedPayload {
  port_name: string;
  timestamp: number;
}

interface SerialDisconnectedPayload {
  port_name: string;
  timestamp: number;
}

interface BleDataPayload {
  device_id: string;
  address: string;
  characteristic_uuid: string;
  data: number[];
  timestamp: number;
}

interface BleConnectedPayload {
  address: string;
  name?: string;
  timestamp: number;
}

interface BleDisconnectedPayload {
  address: string;
  name?: string;
  timestamp: number;
}

let eventBusListener: UnlistenFn | undefined;
let initialized = false;
let initPromise: Promise<void> | null = null;

function handleSerialData(payload: SerialDataPayload) {
  const store = useSerialStore.getState();
  store.addReceivedData(payload.device_id, {
    id: generateId(),
    timestamp: payload.timestamp ?? Date.now(),
    data: payload.data,
    direction: 'receive',
    format: 'hex',
  });
}

function handleSerialConnected(payload: SerialConnectedPayload) {
  const store = useSerialStore.getState();
  store.addPortTab(payload.port_name);
  useLogStore.getState().addLog('info', 'SerialManager', `串口 ${payload.port_name} 已连接`);
  useNotificationStore.getState().addNotification('success', `串口 ${payload.port_name} 已连接`);
}

function handleSerialDisconnected(payload: SerialDisconnectedPayload) {
  const store = useSerialStore.getState();
  const tab = store.tabs.find((t) => t.portName === payload.port_name && t.tabType === 'port');
  if (tab) {
    store.updateTab(tab.key, { isConnected: false });
  }
  useLogStore.getState().addLog('info', 'SerialManager', `串口 ${payload.port_name} 已断开`);
  useNotificationStore.getState().addNotification('info', `串口 ${payload.port_name} 已断开`);
}

function handleBleData(payload: BleDataPayload) {
  const store = useBleStore.getState();
  store.addNotification({
    id: generateBleId(),
    deviceId: payload.device_id,
    characteristicUuid: payload.characteristic_uuid,
    data: payload.data,
    timestamp: payload.timestamp,
  });
}

function handleBleConnected(payload: BleConnectedPayload) {
  const store = useBleStore.getState();
  const deviceId = payload.address;
  store.addConnection({
    deviceId,
    address: payload.address,
    name: payload.name,
    isConnected: true,
    services: [],
    connectedAt: Date.now(),
  });
  store.setIsConnecting(false);
  useLogStore.getState().addLog('info', 'BleManager', `设备 ${payload.name || payload.address} 已连接`);
  useNotificationStore.getState().addNotification('success', `设备 ${payload.name || payload.address} 已连接`);
}

function handleBleDisconnected(payload: BleDisconnectedPayload) {
  const store = useBleStore.getState();
  const deviceId = payload.address;
  store.removeConnection(deviceId);
  if (store.currentDevice === deviceId) {
    store.setCurrentDevice(null);
    store.clearServices();
    store.clearCharacteristics();
  }
  useLogStore.getState().addLog('info', 'BleManager', `设备 ${payload.address} 已断开`);
  useNotificationStore.getState().addNotification('info', `设备 ${payload.address} 已断开`);
}

function dispatchEvent(topic: string, payloadStr: string) {
  try {
    const payload = JSON.parse(payloadStr);

    switch (topic) {
      case 'serial:data':
        handleSerialData(payload as SerialDataPayload);
        break;
      case 'serial:connected':
        handleSerialConnected(payload as SerialConnectedPayload);
        break;
      case 'serial:disconnected':
        handleSerialDisconnected(payload as SerialDisconnectedPayload);
        break;
      case 'ble:data':
        handleBleData(payload as BleDataPayload);
        break;
      case 'ble:connected':
        handleBleConnected(payload as BleConnectedPayload);
        break;
      case 'ble:disconnected':
        handleBleDisconnected(payload as BleDisconnectedPayload);
        break;
      default:
        break;
    }
  } catch (err) {
    console.error(`[EventListeners] Failed to parse payload for topic "${topic}":`, err);
  }
}

export async function initAllEventListeners(): Promise<void> {
  if (initialized) {
    return;
  }

  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    eventBusListener = await listen<EventBusEvent>('event-bus', (event) => {
      const { topic, payload } = event.payload;
      dispatchEvent(topic, payload);
    });

    initialized = true;
    initPromise = null;
  })();

  return initPromise;
}

export async function cleanupAllEventListeners(): Promise<void> {
  if (eventBusListener) {
    eventBusListener();
    eventBusListener = undefined;
  }
  initialized = false;
}

export function isListenersInitialized(): boolean {
  return initialized;
}

export {
  initAllEventListeners as initSerialEventListeners,
  initAllEventListeners as initBleEventListeners,
  cleanupAllEventListeners as cleanupSerialEventListeners,
  cleanupAllEventListeners as cleanupBleEventListeners,
  isListenersInitialized as isSerialListenersInitialized,
  isListenersInitialized as isBleListenersInitialized,
};
