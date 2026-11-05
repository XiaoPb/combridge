import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useSerialStore, generateId } from '../stores/serialStore';
import { useBleStore, generateBleId } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import { useNotificationStore } from '../stores/notificationStore';
import { useGh3036Store } from '../stores/gh3036Store';
import { useOtaStore } from '../stores/otaStore';
import { decodePayload, type EventBusEvent } from '../utils/msgpack';
import type { OtaStatus } from '../types/ota';
import type {
  SerialDataPayload,
  BleDataPayload,
  SerialConnectedPayload,
  SerialDisconnectedPayload,
  BleConnectedPayload,
  BleDisconnectedPayload,
  Gh3036FramesPayload,
  Gh3036RefDataPayload,
  Gh3036StreamStatusPayload,
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
  const connection = store.connections.find((c) => c.address === deviceId);

  store.clearDisconnectedDevice(deviceId);

  const deviceName = connection?.name || deviceId;
  useLogStore.getState().addLog('info', 'BleManager', `设备 ${deviceName} 已断开`);
  useNotificationStore.getState().addNotification('warning', `设备 ${deviceName} 已断开`);
}

function handleGh3036Frames(payload: Gh3036FramesPayload) {
  useGh3036Store.getState().addFramesData(payload);
}

function handleGh3036RefData(payload: Gh3036RefDataPayload) {
  useGh3036Store.getState().updateRefData(payload);
}

function handleGh3036StreamStatus(payload: Gh3036StreamStatusPayload) {
  const store = useGh3036Store.getState();
  store.setStreamStalled(payload.stalled);
  if (payload.stalled) {
    useLogStore
      .getState()
      .addLog('warn', 'GH3036', `帧流已停止（超过 ${Math.round(payload.elapsed_ms / 1000)} 秒未收到数据）`);
  } else {
    useLogStore.getState().addLog('info', 'GH3036', '帧流已恢复');
  }
}

function dispatchEvent(event: EventBusEvent) {
  try {
    switch (event.topic) {
      case 'serial:data':
        handleSerialData(decodePayload<SerialDataPayload>(event));
        break;
      case 'serial:connected':
        handleSerialConnected(decodePayload<SerialConnectedPayload>(event));
        break;
      case 'serial:disconnected':
        handleSerialDisconnected(decodePayload<SerialDisconnectedPayload>(event));
        break;
      case 'ble:data':
        handleBleData(decodePayload<BleDataPayload>(event));
        break;
      case 'ble:connected':
        handleBleConnected(decodePayload<BleConnectedPayload>(event));
        break;
      case 'ble:disconnected':
        handleBleDisconnected(decodePayload<BleDisconnectedPayload>(event));
        break;
      case 'gh3036:frames':
        handleGh3036Frames(decodePayload<Gh3036FramesPayload>(event));
        break;
      case 'gh3036:ref_data':
        handleGh3036RefData(decodePayload<Gh3036RefDataPayload>(event));
        break;
      case 'gh3036:stream:stalled':
        handleGh3036StreamStatus(decodePayload<Gh3036StreamStatusPayload>(event));
        break;
      case 'gh3036:factory_test_progress':
        // 由 gh3036Store 的订阅处理
        break;
      case 'ota:progress': {
        const payload = decodePayload<OtaStatus>(event);
        useOtaStore.getState().handleProgress(payload);
        break;
      }
      default:
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
