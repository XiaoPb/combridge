export type Gh3220LinkKind = 'ble' | 'tcp';

export type Gh3220State = 'disconnected' | 'connecting' | 'ready' | 'capturing';

export interface Gh3220LinkConfig {
  kind: Gh3220LinkKind;
  address: string;
  writeUuid?: string;
  notifyUuid?: string;
  mtu?: number;
}

export interface Gh3220Status {
  state: Gh3220State;
  link: Gh3220LinkKind | null;
  address: string | null;
  name: string | null;
  error: string | null;
  capturing: boolean;
  chunkSize: number;
}

export interface Gh3220DeviceInfo {
  fwVersion: string;
  protocolVersion: string;
  chipVersion: string;
  driverVersion: string;
  maxLen: number;
  openedFunc: number;
  linkOk: boolean;
}

export interface Gh3220Record {
  functionId: number;
  frameId: number;
  raw: number[];
  agc: number[];
  gs?: [number, number, number] | null;
  gyro?: [number, number, number] | null;
  cap?: [number, number, number, number] | null;
  temp?: [number, number, number, number] | null;
  flags: number[];
  algo: Array<[number, number]>;
  tsMs: number;
}

export interface Gh3220Stats {
  frames: number;
  crcErrors: number;
  resyncs: number;
  badLen: number;
  packets: number;
  records: number;
  badPackets: number;
  spliceFragments: number;
  spliceAborts: number;
  lostFrames: number;
}

export interface Gh3220DataEvent {
  seq: number;
  records: Gh3220Record[];
  dropped: number;
}

export interface Gh3220EventPayload {
  eventId: number;
  reportId: number;
  acked: boolean;
  tsMs: number;
}

export interface Gh3220StatusEvent {
  status: Gh3220Status;
}

export interface Gh3220DebugStatus {
  kind: number;
  bytes: number;
  data: number[];
}

export interface Gh3220CaptureConfig {
  autoStart?: boolean;
  autoStop?: boolean;
  mode?: number;
  slotEn?: number;
  funcMask?: number;
  channelsOverride?: number | null;
  csvEnabled?: boolean | null;
}

export interface Gh3220CaptureStatus {
  active: boolean;
  startedAtMs: number;
  records: number;
  csvEnabled: boolean;
  files: string[];
  channelsOverride: number | null;
}

export interface Gh3220CsvPreferences {
  enabled: boolean;
  outputDir: string;
  idleCloseSecs: number;
}

export interface Gh3220CapturePreferences {
  channelsOverride: number | null;
  autoStop: boolean;
}

export interface Gh3220DebugTcpPreferences {
  enabled: boolean;
  address: string;
}

export interface Gh3220Preferences {
  csv: Gh3220CsvPreferences;
  capture: Gh3220CapturePreferences;
  debugTcp: Gh3220DebugTcpPreferences;
}

import {
  GH3220_FUNCTION_BIT_ALIASES,
  parseFunctionMaskText,
} from '../constants/gh3220Functions';

/**
 * 语义文本 → bit 序号。全表与历史别名统一由语义层维护
 * （见 constants/gh3220Functions.ts），此处保留导出以兼容既有引用。
 */
export const GH3220_FUNCTION_BITS: Record<string, number> = {
  ...GH3220_FUNCTION_BIT_ALIASES,
};

/**
 * 与命令行工具保持一致：支持 `hr,spo2`、`hr hrv lead_det`、`0x6` 或十进制数值。
 * 未知文本按数值 mask 解析，无法解析时抛错由调用方提示。
 */
export function parseFuncMask(input: string): number {
  return parseFunctionMaskText(input);
}

export function funcMaskToHex(mask: number): string {
  return `0x${(mask >>> 0).toString(16)}`;
}

/** §6.6 方案 B：后端目录返回的协议命令号 / 版本类型条目 */
export interface Gh3220NamedValueDto {
  key: string;
  value: number;
  nameZh: string;
  nameEn: string;
}

export interface Gh3220CommandCodeDto {
  key: string;
  cmd: number;
}
