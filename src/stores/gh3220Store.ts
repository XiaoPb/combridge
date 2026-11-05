import { create } from 'zustand';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { gh3220Api } from '../api/gh3220';
import {
  onGh3220Data,
  onGh3220Event,
  onGh3220Stats,
  onGh3220Status,
} from '../api/events';
import type {
  Gh3220CaptureConfig,
  Gh3220CaptureStatus,
  Gh3220DeviceInfo,
  Gh3220EventPayload,
  Gh3220LinkConfig,
  Gh3220Preferences,
  Gh3220Record,
  Gh3220State as Gh3220LinkState,
  Gh3220Stats,
  Gh3220Status,
} from '../types/gh3220';

const MAX_RECORDS = 300;
const MAX_EVENTS = 200;

/** 波形默认窗口：10s × 25Hz = 250 行 */
const DEFAULT_SAMPLE_RATE = 25;
const DEFAULT_DURATION_SECONDS = 10;

/** 单个功能的波形缓冲（行式，与 MultiLineChart 的 rows 对齐） */
export interface Gh3220FrameBuffer {
  functionId: number;
  /** 每行的通道数；通道宽度变化时缓冲整体重置 */
  channelCount: number;
  rawRows: number[][];
  agcRows: number[][];
  accRows: number[][];
  lastTsMs: number;
}

export type Gh3220DataMode = 'raw' | 'agc';

const EMPTY_STATUS: Gh3220Status = {
  state: 'disconnected',
  link: null,
  address: null,
  name: null,
  error: null,
  capturing: false,
  chunkSize: 0,
};

const EMPTY_STATS: Gh3220Stats = {
  frames: 0,
  crcErrors: 0,
  resyncs: 0,
  badLen: 0,
  packets: 0,
  records: 0,
  badPackets: 0,
  spliceFragments: 0,
  spliceAborts: 0,
  lostFrames: 0,
};

const EMPTY_CAPTURE: Gh3220CaptureStatus = {
  active: false,
  startedAtMs: 0,
  records: 0,
  csvEnabled: false,
  files: [],
  channelsOverride: null,
};

let unlisteners: UnlistenFn[] = [];

function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object') {
    const record = error as Record<string, unknown>;
    if (typeof record.message === 'string') return record.message;
    if (typeof record.error_code === 'string') return String(record.error_code);
  }
  return String(error);
}

interface Gh3220Store {
  status: Gh3220Status;
  stats: Gh3220Stats;
  capture: Gh3220CaptureStatus;
  deviceInfo: Gh3220DeviceInfo | null;
  records: Gh3220Record[];
  events: Gh3220EventPayload[];
  preferences: Gh3220Preferences | null;
  loading: boolean;
  error: string | null;

  // 波形监控（对齐 gh3036Store 的 framesData 模式）
  frames: Map<number, Gh3220FrameBuffer>;
  selectedFunctionId: number | null;
  dataMode: Gh3220DataMode;
  sampleRate: number;
  displayDurationSeconds: number;
  chartLegendSelected: Record<string, boolean>;

  connect: (config: Gh3220LinkConfig) => Promise<void>;
  disconnect: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  loadDeviceInfo: () => Promise<void>;
  loadPreferences: () => Promise<void>;
  savePreferences: (prefs: Gh3220Preferences) => Promise<void>;
  startCapture: (config: Gh3220CaptureConfig) => Promise<void>;
  stopCapture: () => Promise<void>;
  refreshCapture: () => Promise<void>;
  refreshStats: () => Promise<void>;
  clearStats: () => Promise<void>;
  clearRecords: () => void;
  clearEvents: () => void;
  subscribeEvents: () => Promise<void>;
  unsubscribeEvents: () => void;
  setError: (message: string | null) => void;

  setSelectedFunctionId: (id: number | null) => void;
  setDataMode: (mode: Gh3220DataMode) => void;
  setSampleRate: (rate: number) => void;
  setDisplayDurationSeconds: (seconds: number) => void;
  setChartLegendSelected: (selected: Record<string, boolean>) => void;
  clearWaveformData: () => void;
}

/** 把一批记录追加进环形缓冲（一次事件一批，避免逐条 set） */
function appendRecords(
  frames: Map<number, Gh3220FrameBuffer>,
  records: Gh3220Record[],
  maxRows: number
): Map<number, Gh3220FrameBuffer> {
  if (records.length === 0) return frames;
  const next = new Map(frames);

  for (const record of records) {
    const width = record.raw.length;
    if (width === 0) continue;
    let buffer = next.get(record.functionId);

    // 通道宽度变化（或首帧）：重建该功能缓冲
    if (!buffer || buffer.channelCount !== width) {
      buffer = {
        functionId: record.functionId,
        channelCount: width,
        rawRows: [],
        agcRows: [],
        accRows: [],
        lastTsMs: 0,
      };
      next.set(record.functionId, buffer);
    }

    buffer.rawRows.push([...record.raw]);
    if (record.agc && record.agc.length === width) {
      buffer.agcRows.push([...record.agc]);
    }
    if (record.gs) {
      buffer.accRows.push([record.gs[0], record.gs[1], record.gs[2]]);
    }
    buffer.lastTsMs = record.tsMs;

    if (buffer.rawRows.length > maxRows) {
      buffer.rawRows.splice(0, buffer.rawRows.length - maxRows);
    }
    if (buffer.agcRows.length > maxRows) {
      buffer.agcRows.splice(0, buffer.agcRows.length - maxRows);
    }
    if (buffer.accRows.length > maxRows) {
      buffer.accRows.splice(0, buffer.accRows.length - maxRows);
    }
  }

  return next;
}

export const useGh3220Store = create<Gh3220Store>((set) => ({
  status: EMPTY_STATUS,
  stats: EMPTY_STATS,
  capture: EMPTY_CAPTURE,
  deviceInfo: null,
  records: [],
  events: [],
  preferences: null,
  loading: false,
  error: null,

  frames: new Map(),
  selectedFunctionId: null,
  dataMode: 'raw' as Gh3220DataMode,
  sampleRate: DEFAULT_SAMPLE_RATE,
  displayDurationSeconds: DEFAULT_DURATION_SECONDS,
  chartLegendSelected: {},

  connect: async (config) => {
    set({ loading: true, error: null });
    try {
      const status = await gh3220Api.connect(config);
      set({ status, loading: false });
    } catch (error) {
      set({ loading: false, error: errorMessage(error) });
      throw error;
    }
  },

  disconnect: async () => {
    set({ loading: true, error: null });
    try {
      await gh3220Api.disconnect();
      set({
        status: EMPTY_STATUS,
        deviceInfo: null,
        capture: EMPTY_CAPTURE,
        loading: false,
      });
    } catch (error) {
      set({ loading: false, error: errorMessage(error) });
    }
  },

  refreshStatus: async () => {
    try {
      const status = await gh3220Api.getStatus();
      set({ status });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  loadDeviceInfo: async () => {
    set({ loading: true, error: null });
    try {
      const deviceInfo = await gh3220Api.getDeviceInfo();
      set({ deviceInfo, loading: false });
    } catch (error) {
      set({ loading: false, error: errorMessage(error) });
    }
  },

  loadPreferences: async () => {
    try {
      const preferences = await gh3220Api.getPreferences();
      set({ preferences });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  savePreferences: async (preferences) => {
    try {
      await gh3220Api.updatePreferences(preferences);
      set({ preferences });
    } catch (error) {
      set({ error: errorMessage(error) });
      throw error;
    }
  },

  startCapture: async (config) => {
    set({ loading: true, error: null });
    try {
      await gh3220Api.captureStart(config);
      const capture = await gh3220Api.getCaptureStatus();
      const status = await gh3220Api.getStatus();
      set({ capture, status, loading: false });
    } catch (error) {
      set({ loading: false, error: errorMessage(error) });
      throw error;
    }
  },

  stopCapture: async () => {
    set({ loading: true, error: null });
    try {
      await gh3220Api.captureStop();
      const capture = await gh3220Api.getCaptureStatus();
      const status = await gh3220Api.getStatus();
      set({ capture, status, loading: false });
    } catch (error) {
      set({ loading: false, error: errorMessage(error) });
    }
  },

  refreshCapture: async () => {
    try {
      const capture = await gh3220Api.getCaptureStatus();
      set({ capture });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  refreshStats: async () => {
    try {
      const stats = await gh3220Api.getStats();
      set({ stats });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  clearStats: async () => {
    try {
      await gh3220Api.clearStats();
      set({ stats: EMPTY_STATS });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  clearRecords: () => set({ records: [] }),
  clearEvents: () => set({ events: [] }),
  setError: (error) => set({ error }),

  setSelectedFunctionId: (id) => set({ selectedFunctionId: id }),
  setDataMode: (mode) => set({ dataMode: mode }),
  setSampleRate: (rate) => set({ sampleRate: rate }),
  setDisplayDurationSeconds: (seconds) => set({ displayDurationSeconds: seconds }),
  setChartLegendSelected: (selected) => set({ chartLegendSelected: selected }),
  clearWaveformData: () => set({ frames: new Map() }),

  subscribeEvents: async () => {
    if (unlisteners.length > 0) return;

    const handlers: Array<Promise<UnlistenFn>> = [
      onGh3220Status((payload) => {
        set({ status: payload.status });
      }),
      onGh3220Data((payload) => {
        set((state) => {
          const maxRows = Math.max(10, state.displayDurationSeconds * state.sampleRate);
          return {
            records: [...state.records, ...payload.records].slice(-MAX_RECORDS),
            frames: appendRecords(state.frames, payload.records, maxRows),
          };
        });
      }),
      onGh3220Event((payload) => {
        set((state) => ({
          events: [payload, ...state.events].slice(0, MAX_EVENTS),
        }));
      }),
      onGh3220Stats((payload) => {
        set({ stats: payload });
      }),
    ];

    unlisteners = await Promise.all(handlers);
  },

  unsubscribeEvents: () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
    unlisteners = [];
  },
}));

export type { Gh3220LinkState };
