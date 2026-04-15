import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useSerialStore, generateId } from '../stores/serialStore';
import { useBleStore, generateBleId } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import { useNotificationStore } from '../stores/notificationStore';
import { decodePayload, type EventBusEvent } from '../utils/msgpack';
import type {
  SerialDataPayload,
  BleDataPayload,
  SerialConnectedPayload,
  SerialDisconnectedPayload,
  BleConnectedPayload,
  BleDisconnectedPayload,
} from '../api/events';

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

function dispatchEvent(event: EventBusEvent) {
  console.log('[EventListeners] Received event-bus event:', event.topic, event);
  
  try {
    switch (event.topic) {
      case 'serial:data':
        console.log('[EventListeners] Handling serial:data');
        handleSerialData(decodePayload<SerialDataPayload>(event));
        break;
      case 'serial:connected':
        console.log('[EventListeners] Handling serial:connected');
        handleSerialConnected(decodePayload<SerialConnectedPayload>(event));
        break;
      case 'serial:disconnected':
        console.log('[EventListeners] Handling serial:disconnected');
        handleSerialDisconnected(decodePayload<SerialDisconnectedPayload>(event));
        break;
      case 'ble:data':
        console.log('[EventListeners] Handling ble:data');
        handleBleData(decodePayload<BleDataPayload>(event));
        break;
      case 'ble:connected':
        console.log('[EventListeners] Handling ble:connected');
        handleBleConnected(decodePayload<BleConnectedPayload>(event));
        break;
      case 'ble:disconnected':
        console.log('[EventListeners] Handling ble:disconnected');
        handleBleDisconnected(decodePayload<BleDisconnectedPayload>(event));
        break;
      default:
        console.log('[EventListeners] Unknown topic:', event.topic);
        break;
    }
  } catch (err) {
    console.error(`[EventListeners] Failed to decode payload for topic "${event.topic}":`, err);
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
    console.log('[EventListeners] Initializing event-bus listener...');
    eventBusListener = await listen<EventBusEvent>('event-bus', (event) => {
      dispatchEvent(event.payload);
    });

    initialized = true;
    initPromise = null;
    console.log('[EventListeners] Event-bus listener initialized successfully');
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
