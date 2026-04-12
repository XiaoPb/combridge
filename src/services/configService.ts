import { type AppSettings as AppConfig } from '../types/system';

export type { AppConfig };

export interface SerialConfig {
  baudRate: number;
  dataBits: number;
  parity: string;
  stopBits: number;
  flowControl: string;
}

export interface BleModeConfig {
  mode: 'native' | 'at';
  atPort?: string;
  atBaudRate?: number;
}

const DEFAULT_CONFIG: AppConfig = {
  theme: 'system',
  language: 'zh-CN',
  autoReconnect: false,
  autoReconnectInterval: 3000,
  maxLogLines: 1000,
  soundEnabled: true,
  soundOnConnect: true,
  soundOnDisconnect: true,
  soundOnData: false,
};

const CONFIG_KEY = 'combridge-config';
const SERIAL_CONFIG_KEY = 'combridge-serial-config';
const BLE_CONFIG_KEY = 'combridge-ble-config';
const RECENT_CONNECTIONS_KEY = 'combridge-recent-connections';

class ConfigService {
  private config: AppConfig;
  private listeners: Map<string, Set<(config: AppConfig) => void>> = new Map();

  constructor() {
    this.config = this.loadConfig();
  }

  private loadConfig(): AppConfig {
    try {
      const stored = localStorage.getItem(CONFIG_KEY);
      if (stored) {
        return { ...DEFAULT_CONFIG, ...JSON.parse(stored) };
      }
    } catch (err) {
      console.error('Failed to load config:', err);
    }
    return { ...DEFAULT_CONFIG };
  }

  private saveConfig(): void {
    try {
      localStorage.setItem(CONFIG_KEY, JSON.stringify(this.config));
      this.notifyListeners();
    } catch (err) {
      console.error('Failed to save config:', err);
    }
  }

  private notifyListeners(): void {
    const allListeners = this.listeners.get('change') || new Set();
    allListeners.forEach((listener) => listener(this.config));
  }

  getConfig(): AppConfig {
    return { ...this.config };
  }

  updateConfig(updates: Partial<AppConfig>): void {
    this.config = { ...this.config, ...updates };
    this.saveConfig();
  }

  resetConfig(): void {
    this.config = { ...DEFAULT_CONFIG };
    this.saveConfig();
  }

  subscribe(listener: (config: AppConfig) => void): () => void {
    if (!this.listeners.has('change')) {
      this.listeners.set('change', new Set());
    }
    this.listeners.get('change')!.add(listener);

    return () => {
      this.listeners.get('change')?.delete(listener);
    };
  }

  getSerialConfig(): SerialConfig {
    try {
      const stored = localStorage.getItem(SERIAL_CONFIG_KEY);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch (err) {
      console.error('Failed to load serial config:', err);
    }
    return {
      baudRate: 115200,
      dataBits: 8,
      parity: 'none',
      stopBits: 1,
      flowControl: 'none',
    };
  }

  saveSerialConfig(config: SerialConfig): void {
    try {
      localStorage.setItem(SERIAL_CONFIG_KEY, JSON.stringify(config));
    } catch (err) {
      console.error('Failed to save serial config:', err);
    }
  }

  getBleConfig(): BleModeConfig {
    try {
      const stored = localStorage.getItem(BLE_CONFIG_KEY);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch (err) {
      console.error('Failed to load BLE config:', err);
    }
    return {
      mode: 'native',
    };
  }

  saveBleConfig(config: BleModeConfig): void {
    try {
      localStorage.setItem(BLE_CONFIG_KEY, JSON.stringify(config));
    } catch (err) {
      console.error('Failed to save BLE config:', err);
    }
  }

  getRecentConnections(): Array<{ type: string; identifier: string; name?: string; lastConnected: number }> {
    try {
      const stored = localStorage.getItem(RECENT_CONNECTIONS_KEY);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch (err) {
      console.error('Failed to load recent connections:', err);
    }
    return [];
  }

  addRecentConnection(connection: { type: string; identifier: string; name?: string }): void {
    try {
      const recent = this.getRecentConnections();
      const filtered = recent.filter((c) => c.identifier !== connection.identifier);
      const updated = [
        { ...connection, lastConnected: Date.now() },
        ...filtered,
      ].slice(0, 10);
      localStorage.setItem(RECENT_CONNECTIONS_KEY, JSON.stringify(updated));
    } catch (err) {
      console.error('Failed to save recent connection:', err);
    }
  }

  removeRecentConnection(identifier: string): void {
    try {
      const recent = this.getRecentConnections();
      const filtered = recent.filter((c) => c.identifier !== identifier);
      localStorage.setItem(RECENT_CONNECTIONS_KEY, JSON.stringify(filtered));
    } catch (err) {
      console.error('Failed to remove recent connection:', err);
    }
  }

  clearRecentConnections(): void {
    localStorage.removeItem(RECENT_CONNECTIONS_KEY);
  }
}

export const configService = new ConfigService();
export default configService;
