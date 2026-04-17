import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { decode as msgpackDecode } from '@msgpack/msgpack';

export interface EventBusEvent {
  topic: string;
  payload: string;
  timestamp: number;
  encoding: 'json' | 'msgpack+base64';
}

export interface SerialDataPayload {
  device_id: string;
  data: number[];
  timestamp: number;
}

export interface BleDataPayload {
  device_id: string;
  address: string;
  characteristic_uuid: string;
  data: number[];
  timestamp: number;
}

export interface Gh3036FramePayload {
  function_id: number;
  function_name: string;
  frame_id: number;
  timestamp: number;
  channel_count: number;
  channels: number[];
}

export interface Gh3036FramesPayload {
  function_id: number;
  function_name: string;
  frame_count: number;
  channel_count: number;
  
  frame_cnts: number[];
  timestamps: number[];
  frame_ids: number[];
  
  ipd_pa: number[][];
  rawdata: number[][];
  flags: number[][];
  agc_info: number[][];
  
  acc_x: number[];
  acc_y: number[];
  acc_z: number[];
  gyro_x: number[];
  gyro_y: number[];
  gyro_z: number[];
  
  algo_results: number[][];
  led_drv_fs: [number, number][];
}

export interface ProtocolParsedPayload {
  plugin_id: string;
  device_id: string;
  original_data: number[];
  parsed_data: unknown;
  timestamp: number;
}

export interface SerialConnectedPayload {
  port_name: string;
  timestamp: number;
}

export interface SerialDisconnectedPayload {
  port_name: string;
  timestamp: number;
}

export interface BleConnectedPayload {
  address: string;
  name?: string;
  timestamp: number;
}

export interface BleDisconnectedPayload {
  address: string;
  name?: string;
  timestamp: number;
}

export type SerialDataEvent = SerialDataPayload;

export type SerialConnectedEvent = SerialConnectedPayload;

export type SerialDisconnectedEvent = SerialDisconnectedPayload;

export type BleDataEvent = BleDataPayload;

export type BleConnectedEvent = BleConnectedPayload;

export type BleDisconnectedEvent = BleDisconnectedPayload;

export type Gh3036FrameEvent = Gh3036FramePayload;

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
        let parsed: T;
        if (event.payload.encoding === 'json') {
          parsed = JSON.parse(event.payload.payload) as T;
        } else if (event.payload.encoding === 'msgpack+base64') {
          const binaryString = atob(event.payload.payload);
          const bytes = new Uint8Array(binaryString.length);
          for (let i = 0; i < binaryString.length; i++) {
            bytes[i] = binaryString.charCodeAt(i);
          }
          parsed = msgpackDecode(bytes) as T;
        } else {
          console.error(`[onTopic] Unknown encoding: ${event.payload.encoding}`);
          return;
        }
        callback(parsed);
      } catch (err) {
        console.error(`[onTopic] Failed to decode payload for topic "${topic}":`, err);
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
