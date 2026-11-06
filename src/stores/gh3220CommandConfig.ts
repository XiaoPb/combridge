/**
 * GH3220 命令配置集中状态（V3 P1，对齐 GH3036 的 `rpcConfig` + `setRpcConfig`）
 *
 * - UI 状态只存语义值（selectedFunctions / workMode / action…），不存数值文本；
 * - 配置页命令表、调试页、采集页、监控页共享同一份 `selectedFunctions`；
 * - `syncFromDevice()` 用真机 `openedFunc` / 偏好 / 链路状态反向回填，
 *   但用户手动改过的字段不再覆盖（对齐 GH3036 的 txCharManuallyEdited 思路）。
 * - 只持久化选择类偏好；`isRunning` 与 rowValues 不跨会话保留。
 */

import { create } from 'zustand';
import {
  defaultRowValues,
  GH3220_COMMAND_CATALOG,
  type ParamRaw,
} from '../constants/gh3220CommandSchema';
import {
  decodeFunctionMask,
  encodeEventMask,
  GH3220_DEFAULT_UUIDS,
  parseFunctionMaskText,
} from '../constants/gh3220Functions';
import type { Gh3220DeviceInfo, Gh3220Preferences, Gh3220Status } from '../types/gh3220';

const STORAGE_KEY = 'gh3220:commandConfig';
const STORAGE_VERSION = 1;

/** 允许跨会话保留的语义字段（不含 isRunning / rowValues / 时间戳） */
export interface Gh3220CommandConfigValues {
  selectedFunctions: string[];
  workMode: string;
  eventMaskKeys: string[];
  debugKind: string;
  chipAction: string;
  versionType: string;
  mtu: string;
  tcpAddress: string;
  writeUuid: string;
  notifyUuid: string;
  channelsOverride: string;
  idleCloseSecs: string;
  csvEnabled: string;
  autoStart: string;
  autoStop: string;
  slotOn: string;
  slotEn: string;
  slot: string;
  extra: string;
}

export interface Gh3220CommandConfigState extends Gh3220CommandConfigValues {
  isRunning: boolean;
  rowValues: Record<string, Record<string, ParamRaw>>;
  /** 用户手动改过的语义字段，syncFromDevice 不再覆盖 */
  manualEdited: Record<string, boolean>;
  lastUpdatedFromDevice: number | null;
  setConfig: (partial: Partial<Gh3220CommandConfigState>) => void;
  setSelectedFunctions: (keys: string[]) => void;
  setEventMaskKeys: (keys: string[]) => void;
  setRowValue: (commandKey: string, fieldKey: string, value: ParamRaw) => void;
  syncFromDevice: (
    info: Gh3220DeviceInfo | null,
    prefs: Gh3220Preferences | null,
    status: Gh3220Status
  ) => void;
  reset: () => void;
}

export const DEFAULT_COMMAND_CONFIG: Gh3220CommandConfigValues = {
  selectedFunctions: ['hr'],
  workMode: '2',
  eventMaskKeys: [],
  debugKind: 'status',
  chipAction: 'wakeup',
  versionType: '0x01',
  mtu: 'auto',
  tcpAddress: '127.0.0.1:9000',
  writeUuid: GH3220_DEFAULT_UUIDS.write,
  notifyUuid: GH3220_DEFAULT_UUIDS.notify,
  channelsOverride: 'auto',
  idleCloseSecs: '10',
  csvEnabled: 'true',
  autoStart: 'true',
  autoStop: 'false',
  slotOn: 'true',
  slotEn: '0',
  slot: '0',
  extra: '0',
};

const PERSISTED_KEYS = Object.keys(DEFAULT_COMMAND_CONFIG) as Array<keyof Gh3220CommandConfigValues>;

function pickPersisted(state: Gh3220CommandConfigState): Record<string, unknown> {
  const data: Record<string, unknown> = { version: STORAGE_VERSION };
  for (const key of PERSISTED_KEYS) data[key] = state[key];
  data.manualEdited = state.manualEdited;
  return data;
}

/** 兼容旧值：老版本存的 `funcMaskText: 'hr,spo2'` 转成 selectedFunctions */
export function normalizePersisted(raw: Record<string, unknown>): Partial<Gh3220CommandConfigState> {
  const out: Record<string, unknown> = {};
  for (const key of PERSISTED_KEYS) {
    if (raw[key] !== undefined) out[key] = raw[key];
  }
  if (typeof raw.funcMaskText === 'string' && raw.selectedFunctions === undefined) {
    try {
      out.selectedFunctions = decodeFunctionMask(parseFunctionMaskText(raw.funcMaskText));
    } catch {
      // 旧文本无法解析时保留默认值
    }
  }
  if (Array.isArray(raw.selectedFunctions)) {
    out.selectedFunctions = raw.selectedFunctions.filter((item): item is string => typeof item === 'string');
  }
  if (Array.isArray(raw.eventMaskKeys)) {
    out.eventMaskKeys = raw.eventMaskKeys.filter((item): item is string => typeof item === 'string');
  }
  if (raw.manualEdited && typeof raw.manualEdited === 'object') {
    out.manualEdited = raw.manualEdited as Record<string, boolean>;
  }
  return out as Partial<Gh3220CommandConfigState>;
}

function loadPersisted(): Partial<Gh3220CommandConfigState> {
  if (typeof localStorage === 'undefined') return {};
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    return normalizePersisted(parsed);
  } catch {
    return {};
  }
}

function savePersisted(state: Gh3220CommandConfigState): void {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(pickPersisted(state)));
  } catch {
    // 配额或隐私模式下忽略，不影响交互
  }
}

/** 把功能选择同步到目录里所有 `functionMask` 行，保证四处选择一致 */
function withFunctionMask(
  rowValues: Record<string, Record<string, ParamRaw>>,
  keys: string[]
): Record<string, Record<string, ParamRaw>> {
  const next = { ...rowValues };
  for (const def of GH3220_COMMAND_CATALOG) {
    if (!def.params.some((param) => param.kind === 'functionMask')) continue;
    next[def.key] = { ...(next[def.key] ?? {}), funcMask: [...keys] };
  }
  return next;
}

function initialState(): Gh3220CommandConfigState {
  const persisted = loadPersisted();
  const selectedFunctions =
    (persisted.selectedFunctions as string[] | undefined) ?? DEFAULT_COMMAND_CONFIG.selectedFunctions;
  const rowValues = withFunctionMask(defaultRowValues(), selectedFunctions);
  return {
    ...DEFAULT_COMMAND_CONFIG,
    ...persisted,
    selectedFunctions,
    isRunning: false,
    rowValues,
    manualEdited: (persisted.manualEdited as Record<string, boolean> | undefined) ?? {},
    lastUpdatedFromDevice: null,
  } as Gh3220CommandConfigState;
}

export const useGh3220CommandConfig = create<Gh3220CommandConfigState>((set, get) => ({
  ...initialState(),

  setConfig: (partial) => {
    set(partial);
    savePersisted(get());
  },

  setSelectedFunctions: (keys) => {
    set((state) => ({
      selectedFunctions: keys,
      manualEdited: { ...state.manualEdited, selectedFunctions: true },
      rowValues: withFunctionMask(state.rowValues, keys),
    }));
    savePersisted(get());
  },

  setEventMaskKeys: (keys) => {
    set((state) => ({
      eventMaskKeys: keys,
      manualEdited: { ...state.manualEdited, eventMaskKeys: true },
      rowValues: {
        ...state.rowValues,
        setReportEvent: { ...(state.rowValues.setReportEvent ?? {}), mask: [...keys] },
      },
    }));
    savePersisted(get());
  },

  setRowValue: (commandKey, fieldKey, value) => {
    set((state) => ({
      rowValues: {
        ...state.rowValues,
        [commandKey]: { ...(state.rowValues[commandKey] ?? {}), [fieldKey]: value },
      },
    }));
    savePersisted(get());
  },

  syncFromDevice: (info, prefs, status) => {
    const state = get();
    const patch: Partial<Gh3220CommandConfigState> = { lastUpdatedFromDevice: Date.now() };
    const manual = state.manualEdited;

    if (info && !manual.selectedFunctions && typeof info.openedFunc === 'number' && info.openedFunc > 0) {
      const keys = decodeFunctionMask(info.openedFunc);
      patch.selectedFunctions = keys;
      patch.rowValues = withFunctionMask(state.rowValues, keys);
    }
    if (prefs) {
      if (!manual.csvEnabled) patch.csvEnabled = String(prefs.csv.enabled);
      if (!manual.idleCloseSecs) patch.idleCloseSecs = String(prefs.csv.idleCloseSecs);
      if (!manual.channelsOverride) {
        patch.channelsOverride =
          prefs.capture.channelsOverride === null || prefs.capture.channelsOverride === undefined
            ? 'auto'
            : String(prefs.capture.channelsOverride);
      }
      if (!manual.autoStop) patch.autoStop = String(prefs.capture.autoStop);
      if (!manual.tcpAddress && prefs.debugTcp.address) patch.tcpAddress = prefs.debugTcp.address;
    }
    if (!manual.mtu && status.chunkSize > 0) patch.mtu = String(status.chunkSize);

    set(patch);
    savePersisted(get());
    // 事件掩码仅做一次编码自检，避免持久化脏值让后续渲染抛错
    try {
      encodeEventMask(get().eventMaskKeys);
    } catch {
      set({ eventMaskKeys: [] });
    }
  },

  reset: () => {
    set({
      ...DEFAULT_COMMAND_CONFIG,
      isRunning: false,
      rowValues: withFunctionMask(defaultRowValues(), DEFAULT_COMMAND_CONFIG.selectedFunctions),
      manualEdited: {},
      lastUpdatedFromDevice: null,
    });
    savePersisted(get());
  },
}));

/** 供单测与非 React 上下文读取 */
export const getGh3220CommandConfig = (): Gh3220CommandConfigState =>
  useGh3220CommandConfig.getState();
