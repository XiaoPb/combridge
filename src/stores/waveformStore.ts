import { create } from 'zustand';
import type { ParserConfig, WaveformBufferConfig, WaveformData, WaveformStatus } from '../api/waveform';
import { waveformApi } from '../api/waveform';
import { preferencesApi, type WaveformPreferences } from '../api/tauri';

interface WaveformPreferencesState {
  displayRows: number;
  refreshInterval: number;
  sidebarCollapsed: boolean;
}

interface WaveformState {
  buffers: string[];
  currentBuffer: string | null;
  status: WaveformStatus | null;
  data: WaveformData | null;
  isLoading: boolean;
  error: string | null;
  displayRows: number;
  refreshInterval: number;
  isRunning: boolean;
  preferences: WaveformPreferencesState;
}

interface WaveformActions {
  createBuffer: (bufferId: string, config: WaveformBufferConfig) => Promise<void>;
  removeBuffer: (bufferId: string) => Promise<void>;
  setCurrentBuffer: (bufferId: string) => void;
  configureParser: (bufferId: string, config: ParserConfig) => Promise<void>;
  parseAndStore: (bufferId: string, data: string) => Promise<void>;
  readData: (bufferId: string) => Promise<void>;
  getStatus: (bufferId: string) => Promise<void>;
  clearBuffer: (bufferId: string) => Promise<void>;
  refreshBuffers: () => Promise<void>;
  setDisplayRows: (rows: number) => void;
  setRefreshInterval: (ms: number) => void;
  startRefresh: () => void;
  stopRefresh: () => void;
  clearError: () => void;
  loadPreferences: () => Promise<void>;
  updatePreferences: (updates: Partial<WaveformPreferencesState>) => Promise<void>;
}

export type WaveformStore = WaveformState & WaveformActions;

const DEFAULT_BUFFER_CONFIG: WaveformBufferConfig = {
  capacity: 1000,
  column_names: ['CH0', 'CH1', 'CH2', 'CH3', 'CH4'],
};

const DEFAULT_PARSER_CONFIG: ParserConfig = {
  parser_type: 'delimiter',
  delimiter: ',',
  pattern: null,
  column_names: ['CH0', 'CH1', 'CH2', 'CH3', 'CH4'],
  trim_whitespace: true,
};

const DEFAULT_PREFERENCES: WaveformPreferencesState = {
  displayRows: 20,
  refreshInterval: 100,
  sidebarCollapsed: false,
};

export const useWaveformStore = create<WaveformStore>((set, get) => ({
  buffers: [],
  currentBuffer: null,
  status: null,
  data: null,
  isLoading: false,
  error: null,
  displayRows: 500,
  refreshInterval: 33,
  isRunning: false,
  preferences: DEFAULT_PREFERENCES,

  createBuffer: async (bufferId: string, config: WaveformBufferConfig) => {
    set({ isLoading: true, error: null });
    try {
      await waveformApi.createBuffer(bufferId, config);
      await get().refreshBuffers();
      await waveformApi.configureParser(bufferId, DEFAULT_PARSER_CONFIG);
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    } finally {
      set({ isLoading: false });
    }
  },

  removeBuffer: async (bufferId: string) => {
    set({ isLoading: true, error: null });
    try {
      await waveformApi.removeBuffer(bufferId);
      await get().refreshBuffers();
      if (get().currentBuffer === bufferId) {
        set({ currentBuffer: null, status: null, data: null });
      }
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    } finally {
      set({ isLoading: false });
    }
  },

  setCurrentBuffer: (bufferId: string) => {
    set({ currentBuffer: bufferId });
    get().getStatus(bufferId);
  },

  configureParser: async (bufferId: string, config: ParserConfig) => {
    set({ isLoading: true, error: null });
    try {
      await waveformApi.configureParser(bufferId, config);
      await get().getStatus(bufferId);
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    } finally {
      set({ isLoading: false });
    }
  },

  parseAndStore: async (bufferId: string, data: string) => {
    try {
      await waveformApi.parseAndStore(bufferId, data);
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  readData: async (bufferId: string) => {
    try {
      const rows = get().displayRows;
      const data = await waveformApi.readData(bufferId, rows);
      set({ data });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  getStatus: async (bufferId: string) => {
    try {
      const status = await waveformApi.getStatus(bufferId);
      set({ status });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  clearBuffer: async (bufferId: string) => {
    set({ isLoading: true, error: null });
    try {
      await waveformApi.clearBuffer(bufferId);
      set({ data: null });
      await get().getStatus(bufferId);
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    } finally {
      set({ isLoading: false });
    }
  },

  refreshBuffers: async () => {
    try {
      const buffers = await waveformApi.listBuffers();
      set({ buffers });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  setDisplayRows: (rows: number) => {
    set({ displayRows: rows });
  },

  setRefreshInterval: (ms: number) => {
    set({ refreshInterval: ms });
  },

  startRefresh: () => {
    set({ isRunning: true });
  },

  stopRefresh: () => {
    set({ isRunning: false });
  },

  clearError: () => {
    set({ error: null });
  },

  loadPreferences: async () => {
    try {
      const prefs = await preferencesApi.get();
      if (prefs.waveform) {
        const waveformPrefs: WaveformPreferencesState = {
          displayRows: prefs.waveform.display_rows,
          refreshInterval: prefs.waveform.refresh_interval,
          sidebarCollapsed: prefs.waveform.sidebar_collapsed,
        };
        set({
          preferences: waveformPrefs,
          displayRows: waveformPrefs.displayRows,
          refreshInterval: waveformPrefs.refreshInterval,
        });
      }
    } catch (err) {
      console.error('加载波形偏好设置失败:', err);
    }
  },

  updatePreferences: async (updates: Partial<WaveformPreferencesState>) => {
    const state = get();
    const newPreferences = { ...state.preferences, ...updates };
    set({
      preferences: newPreferences,
      displayRows: newPreferences.displayRows,
      refreshInterval: newPreferences.refreshInterval,
    });
    try {
      const apiPrefs: WaveformPreferences = {
        display_rows: newPreferences.displayRows,
        refresh_interval: newPreferences.refreshInterval,
        sidebar_collapsed: newPreferences.sidebarCollapsed,
      };
      await preferencesApi.updateWaveform(apiPrefs);
    } catch (err) {
      console.error('保存波形偏好设置失败:', err);
    }
  },
}));

export { DEFAULT_BUFFER_CONFIG, DEFAULT_PARSER_CONFIG };
