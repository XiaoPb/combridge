import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { decode as msgpackDecode } from '@msgpack/msgpack';
import type {
  Gh3220DataEvent,
  Gh3220EventPayload,
  Gh3220Record,
  Gh3220Stats,
  Gh3220Status,
  Gh3220StatusEvent,
} from '../types/gh3220';

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
  ref_data?: number[][];
}

export interface Gh3036RefDataPayload {
  hr_values: number[];
  hr_count: number;
  hr_valid: boolean;
  hrv_values: number[];
  hrv_count: number;
  hrv_valid: boolean;
  spo2_values: number[];
  spo2_count: number;
  spo2_valid: boolean;
  timestamp: number;
}

/** 帧流看门狗状态：宿主侧超过阈值未收到帧时由后端发布 */
export interface Gh3036StreamStatusPayload {
  /** 帧流是否已停止 */
  stalled: boolean;
  /** 距最后一帧的毫秒数（恢复时表示停顿时长） */
  elapsed_ms: number;
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
  GH3220_STATUS: 'gh3220:status',
  GH3220_DATA: 'gh3220:data',
  GH3220_EVENT: 'gh3220:event',
  GH3220_STATS: 'gh3220:stats',
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

// ---------------------------------------------------------------------------
// GH3220 事件线格式归一化
//
// 后端 msgpack（rmp-serde）把结构体编码为**数组**（按字段声明顺序），而
// invoke(JSON) 返回 camelCase 对象。这里把数组形式归一化为 TS 类型声明的
// 对象形状，同时保持对对象形式的兼容。
// 字段顺序必须与 src-tauri/src/gh3220/types.rs 中结构体声明顺序一致。
// ---------------------------------------------------------------------------

/* eslint-disable @typescript-eslint/no-explicit-any */
function normalizeGh3220Status(raw: any): Gh3220Status {
  if (Array.isArray(raw)) {
    return {
      state: raw[0] ?? 'disconnected',
      link: raw[1] ?? null,
      address: raw[2] ?? null,
      name: raw[3] ?? null,
      error: raw[4] ?? null,
      capturing: Boolean(raw[5]),
      chunkSize: raw[6] ?? 0,
    };
  }
  return raw;
}

function normalizeGh3220Record(raw: any): Gh3220Record {
  if (Array.isArray(raw)) {
    return {
      functionId: raw[0] ?? 0,
      frameId: raw[1] ?? 0,
      raw: Array.isArray(raw[2]) ? raw[2] : [],
      agc: Array.isArray(raw[3]) ? raw[3] : [],
      gs: raw[4] ?? null,
      gyro: raw[5] ?? null,
      cap: raw[6] ?? null,
      temp: raw[7] ?? null,
      flags: Array.isArray(raw[8]) ? raw[8] : [],
      algo: Array.isArray(raw[9]) ? raw[9] : [],
      tsMs: raw[10] ?? 0,
    };
  }
  return raw;
}

function normalizeGh3220DataEvent(raw: any): Gh3220DataEvent {
  const base = Array.isArray(raw)
    ? { seq: raw[0], records: raw[1], dropped: raw[2] }
    : raw;
  return {
    seq: base?.seq ?? 0,
    records: Array.isArray(base?.records) ? base.records.map(normalizeGh3220Record) : [],
    dropped: base?.dropped ?? 0,
  };
}

function normalizeGh3220StatusEvent(raw: any): Gh3220StatusEvent {
  const status = Array.isArray(raw) ? raw[0] : raw?.status;
  return { status: normalizeGh3220Status(status) };
}

function normalizeGh3220EventPayload(raw: any): Gh3220EventPayload {
  if (Array.isArray(raw)) {
    return {
      eventId: raw[0] ?? 0,
      reportId: raw[1] ?? 0,
      acked: Boolean(raw[2]),
      tsMs: raw[3] ?? 0,
    };
  }
  return raw;
}

function normalizeGh3220Stats(raw: any): Gh3220Stats {
  if (Array.isArray(raw)) {
    return {
      frames: raw[0] ?? 0,
      crcErrors: raw[1] ?? 0,
      resyncs: raw[2] ?? 0,
      badLen: raw[3] ?? 0,
      packets: raw[4] ?? 0,
      records: raw[5] ?? 0,
      badPackets: raw[6] ?? 0,
      spliceFragments: raw[7] ?? 0,
      spliceAborts: raw[8] ?? 0,
      lostFrames: raw[9] ?? 0,
    };
  }
  return raw;
}
/* eslint-enable @typescript-eslint/no-explicit-any */

export function onGh3220Status(callback: (event: Gh3220StatusEvent) => void): Promise<UnlistenFn> {
  return onTopic<Gh3220StatusEvent>(EventBusTopics.GH3220_STATUS, (payload) =>
    callback(normalizeGh3220StatusEvent(payload))
  );
}

export function onGh3220Data(callback: (event: Gh3220DataEvent) => void): Promise<UnlistenFn> {
  return onTopic<Gh3220DataEvent>(EventBusTopics.GH3220_DATA, (payload) =>
    callback(normalizeGh3220DataEvent(payload))
  );
}

export function onGh3220Event(callback: (event: Gh3220EventPayload) => void): Promise<UnlistenFn> {
  return onTopic<Gh3220EventPayload>(EventBusTopics.GH3220_EVENT, (payload) =>
    callback(normalizeGh3220EventPayload(payload))
  );
}

export function onGh3220Stats(callback: (event: Gh3220Stats) => void): Promise<UnlistenFn> {
  return onTopic<Gh3220Stats>(EventBusTopics.GH3220_STATS, (payload) =>
    callback(normalizeGh3220Stats(payload))
  );
}

export const eventBus = {
  on: onEventBus,
  onTopic,
};
