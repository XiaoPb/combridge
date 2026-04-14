import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface EventBusEvent {
  topic: string;
  payload: string;
  timestamp: number;
}

export type SerialDataEvent = {
  device_id: string;
  data: number[];
  timestamp: number;
};

export type SerialConnectedEvent = {
  port_name: string;
  timestamp: number;
};

export type SerialDisconnectedEvent = {
  port_name: string;
  timestamp: number;
};

export type BleDataEvent = {
  device_id: string;
  address: string;
  characteristic_uuid: string;
  data: number[];
  timestamp: number;
};

export type BleConnectedEvent = {
  address: string;
  name?: string;
  timestamp: number;
};

export type BleDisconnectedEvent = {
  address: string;
  name?: string;
  timestamp: number;
};

export type Gh3036FrameEvent = {
  function_id: number;
  function_name: string;
  frame_id: number;
  timestamp: number;
  channel_count: number;
  channels: number[];
};

export type ParsedDataEvent = {
  timestamp: number;
  values: Record<string, number>;
};

export type ProtocolParsedEvent = {
  plugin_id: string;
  device_id: string;
  original_data: number[];
  parsed_data: Record<string, unknown>;
  timestamp: number;
};

export const EventBusTopics = {
  SERIAL_DATA: 'serial:data',
  SERIAL_CONNECTED: 'serial:connected',
  SERIAL_DISCONNECTED: 'serial:disconnected',
  BLE_DATA: 'ble:data',
  BLE_CONNECTED: 'ble:connected',
  BLE_DISCONNECTED: 'ble:disconnected',
  GH3036_FRAME: 'gh3036:frame',
  PROTOCOL_PARSED: 'protocol:parsed',
} as const;

export const TauriEvents = {
  EVENT_BUS: 'event-bus',
} as const;

export function onEventBus(callback: (event: EventBusEvent) => void): Promise<UnlistenFn> {
  return listen<EventBusEvent>(TauriEvents.EVENT_BUS, (event) => {
    callback(event.payload);
  });
}

export function onTopic<T>(topic: string, callback: (payload: T) => void): Promise<UnlistenFn> {
  return listen<EventBusEvent>(TauriEvents.EVENT_BUS, (event) => {
    if (event.payload.topic === topic) {
      try {
        const parsed = JSON.parse(event.payload.payload) as T;
        callback(parsed);
      } catch (err) {
        console.error(`[onTopic] Failed to parse payload for topic "${topic}":`, err);
      }
    }
  });
}

export function onSerialData(callback: (event: SerialDataEvent) => void): Promise<UnlistenFn> {
  return onTopic<SerialDataEvent>(EventBusTopics.SERIAL_DATA, callback);
}

export function onBleData(callback: (event: BleDataEvent) => void): Promise<UnlistenFn> {
  return onTopic<BleDataEvent>(EventBusTopics.BLE_DATA, callback);
}

export function onParsedData(callback: (event: ParsedDataEvent) => void): Promise<UnlistenFn> {
  return onTopic<ParsedDataEvent>(EventBusTopics.PROTOCOL_PARSED, callback);
}

export const eventBus = {
  on: onEventBus,
  onTopic,
};
