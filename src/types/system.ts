export interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  language: 'zh-CN' | 'en-US';
  autoReconnect: boolean;
  autoReconnectInterval: number;
  maxLogLines: number;
  soundEnabled: boolean;
  soundOnConnect: boolean;
  soundOnDisconnect: boolean;
  soundOnData: boolean;
}

export interface ConnectionStatus {
  type: 'serial' | 'ble';
  status: 'disconnected' | 'connecting' | 'connected' | 'disconnecting';
  message?: string;
  lastError?: string;
}

export interface LogEntry {
  id: string;
  timestamp: number;
  level: 'info' | 'warn' | 'error' | 'debug';
  source: string;
  message: string;
  details?: unknown;
}

export interface WindowState {
  width: number;
  height: number;
  x?: number;
  y?: number;
  isMaximized: boolean;
  isFullscreen: boolean;
}

export interface RecentConnection {
  type: 'serial' | 'ble';
  identifier: string;
  name?: string;
  lastConnected: number;
  config?: Record<string, unknown>;
}

export const DEFAULT_APP_SETTINGS: AppSettings = {
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

export const MAX_RECENT_CONNECTIONS = 10;
