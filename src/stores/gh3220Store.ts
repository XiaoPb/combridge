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

  subscribeEvents: async () => {
    if (unlisteners.length > 0) return;

    const handlers: Array<Promise<UnlistenFn>> = [
      onGh3220Status((payload) => {
        set({ status: payload.status });
      }),
      onGh3220Data((payload) => {
        set((state) => ({
          records: [...state.records, ...payload.records].slice(-MAX_RECORDS),
        }));
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
