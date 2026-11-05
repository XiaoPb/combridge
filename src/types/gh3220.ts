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

export const GH3220_FUNCTION_BITS: Record<string, number> = {
  adt: 0,
  hr: 1,
  hrv: 2,
  hsm: 3,
  fpbp: 4,
  bp: 4,
  pwa: 5,
  spo2: 6,
  ecg: 7,
  pwtt: 8,
  green: 9,
  soft_adt_green: 9,
  bt: 10,
  resp: 11,
  af: 12,
  test1: 13,
  test2: 14,
  ir: 15,
  soft_adt_ir: 15,
};

/** 与命令行工具保持一致：支持 `hr,spo2` 或 `0x6` */
export function parseFuncMask(input: string): number {
  const text = input.trim();
  if (!text) return 0;
  if (text.toLowerCase().startsWith('0x')) {
    const value = Number.parseInt(text.slice(2), 16);
    if (Number.isNaN(value)) throw new Error(`无法解析功能位: ${input}`);
    return value >>> 0;
  }

  let mask = 0;
  for (const part of text.split(',')) {
    const name = part.trim().toLowerCase();
    if (!name) continue;
    const bit = GH3220_FUNCTION_BITS[name];
    if (bit === undefined) throw new Error(`未知功能: ${part.trim()}`);
    mask |= 1 << bit;
  }
  return mask >>> 0;
}

export function funcMaskToHex(mask: number): string {
  return `0x${(mask >>> 0).toString(16)}`;
}
