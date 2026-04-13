import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { AppSettings, RecentConnection } from '../types/system';
import { DEFAULT_APP_SETTINGS, MAX_RECENT_CONNECTIONS } from '../types/system';

interface SerialConfig {
  baudRate: number;
  dataBits: number;
  parity: string;
  stopBits: number;
  flowControl: string;
}

interface BleModeConfig {
  mode: 'native' | 'at';
  atPort: string;
  atBaudRate: number;
}

interface ConfigState {
  settings: AppSettings;
  serialConfig: SerialConfig;
  bleConfig: BleModeConfig;
  recentConnections: RecentConnection[];

  getConfig: () => AppSettings;
  updateConfig: (partial: Partial<AppSettings>) => void;
  resetConfig: () => void;
  getSerialConfig: () => SerialConfig;
  saveSerialConfig: (config: SerialConfig) => void;
  getBleConfig: () => BleModeConfig;
  saveBleConfig: (config: BleModeConfig) => void;
  getRecentConnections: () => RecentConnection[];
  addRecentConnection: (connection: RecentConnection) => void;
  removeRecentConnection: (identifier: string) => void;
  clearRecentConnections: () => void;
}

const DEFAULT_SERIAL_CONFIG: SerialConfig = {
  baudRate: 115200,
  dataBits: 8,
  parity: 'none',
  stopBits: 1,
  flowControl: 'none',
};

const DEFAULT_BLE_CONFIG: BleModeConfig = {
  mode: 'native',
  atPort: '',
  atBaudRate: 115200,
};

export const useConfigStore = create<ConfigState>()(
  persist(
    (set, get) => ({
      settings: { ...DEFAULT_APP_SETTINGS },
      serialConfig: { ...DEFAULT_SERIAL_CONFIG },
      bleConfig: { ...DEFAULT_BLE_CONFIG },
      recentConnections: [],

      getConfig: () => get().settings,

      updateConfig: (partial) => {
        set((state) => ({
          settings: { ...state.settings, ...partial },
        }));
      },

      resetConfig: () => {
        set({ settings: { ...DEFAULT_APP_SETTINGS } });
      },

      getSerialConfig: () => get().serialConfig,

      saveSerialConfig: (config) => {
        set({ serialConfig: config });
      },

      getBleConfig: () => get().bleConfig,

      saveBleConfig: (config) => {
        set({ bleConfig: config });
      },

      getRecentConnections: () => get().recentConnections,

      addRecentConnection: (connection) => {
        set((state) => {
          const filtered = state.recentConnections.filter(
            (c) => c.identifier !== connection.identifier
          );
          const updated = [connection, ...filtered].slice(0, MAX_RECENT_CONNECTIONS);
          return { recentConnections: updated };
        });
      },

      removeRecentConnection: (identifier) => {
        set((state) => ({
          recentConnections: state.recentConnections.filter(
            (c) => c.identifier !== identifier
          ),
        }));
      },

      clearRecentConnections: () => {
        set({ recentConnections: [] });
      },
    }),
    {
      name: 'combridge-config',
      partialize: (state) => ({
        settings: state.settings,
        serialConfig: state.serialConfig,
        bleConfig: state.bleConfig,
        recentConnections: state.recentConnections,
      }),
    }
  )
);

export type { AppSettings as AppConfig, SerialConfig, BleModeConfig };
