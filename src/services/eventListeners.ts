import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useSerialStore, generateId } from '../stores/serialStore';
import { useBleStore, generateBleId } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import { useNotificationStore } from '../stores/notificationStore';
import { useGh3036Store } from '../stores/gh3036Store';
import { decodePayload, type EventBusEvent } from '../utils/msgpack';
import type {
  SerialDataPayload,
  BleDataPayload,
  SerialConnectedPayload,
  SerialDisconnectedPayload,
  BleConnectedPayload,
  BleDisconnectedPayload,
  Gh3036FramesPayload,
} from '../api/events';

let eventBusListener: UnlistenFn | undefined;
let initialized = false;
let initPromise: Promise<void> | null = null;

function handleSerialData(payload: SerialDataPayload) {
  console.log('[EventListeners] handleSerialData called with payload:', payload);
  console.log('[EventListeners] device_id:', payload.device_id, 'data type:', typeof payload.data, 'data length:', payload.data?.length);
  
  const store = useSerialStore.getState();
  const matchingTab = store.tabs.find(t => t.portName === payload.device_id && t.tabType === 'port');
  console.log('[EventListeners] Matching tab found:', !!matchingTab, 'Available tabs:', store.tabs.map(t => ({ portName: t.portName, tabType: t.tabType })));
  
  store.addReceivedData(payload.device_id, {
    id: generateId(),
    timestamp: payload.timestamp ?? Date.now(),
    data: payload.data,
    direction: 'receive',
    format: 'hex',
  });
  console.log('[EventListeners] addReceivedData called for device_id:', payload.device_id);
}

function handleSerialConnected(payload: SerialConnectedPayload) {
  console.log('[EventListeners] handleSerialConnected called with payload:', payload);
  const store = useSerialStore.getState();
  store.addPortTab(payload.port_name);
  console.log('[EventListeners] Added port tab for:', payload.port_name);
  console.log('[EventListeners] Current tabs after addPortTab:', store.tabs.map(t => ({ portName: t.portName, tabType: t.tabType })));
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
  console.log('[EventListeners] handleBleDisconnected called with payload:', payload);
  const store = useBleStore.getState();
  const deviceId = payload.address;
  
  const wasCurrentDevice = store.currentDevice === deviceId;
  
  store.removeConnection(deviceId);
  
  if (wasCurrentDevice) {
    store.setCurrentDevice(null);
    store.clearServices();
    store.clearCharacteristics();
    store.clearNotifications();
  }
  
  const deviceName = store.connections.find(c => c.address === deviceId)?.name || deviceId;
  useLogStore.getState().addLog('info', 'BleManager', `设备 ${deviceName} 已断开`);
  useNotificationStore.getState().addNotification('warning', `设备 ${deviceName} 已断开`);
}

function handleGh3036Frames(payload: Gh3036FramesPayload) {
  console.log('[EventListeners] handleGh3036Frames:', payload);
  const store = useGh3036Store.getState();
  store.addFramesData(payload);
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
      case 'gh3036:frames':
        handleGh3036Frames(decodePayload<Gh3036FramesPayload>(event));
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
