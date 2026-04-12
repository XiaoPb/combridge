import { create } from 'zustand';
import type { LogEntry } from '../types/system';

export type { LogEntry };
export type LogLevel = 'info' | 'warn' | 'error' | 'debug';

interface LogState {
  logs: LogEntry[];
  maxLogs: number;
  addLog: (level: LogLevel, source: string, message: string) => void;
  clearLogs: () => void;
  setMaxLogs: (max: number) => void;
}

const DEFAULT_MAX_LOGS = 1000;

const generateLogId = (): string => {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
};

export const useLogStore = create<LogState>((set, get) => ({
  logs: [],
  maxLogs: DEFAULT_MAX_LOGS,

  addLog: (level, source, message) => {
    const { logs, maxLogs } = get();
    const newLog: LogEntry = {
      id: generateLogId(),
      timestamp: Date.now(),
      level,
      source,
      message,
    };
    set({
      logs: [...logs, newLog].slice(-maxLogs),
    });
  },

  clearLogs: () => set({ logs: [] }),

  setMaxLogs: (max) => {
    const { logs } = get();
    set({
      maxLogs: max,
      logs: logs.slice(-max),
    });
  },
}));

export const formatLogTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp);
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${hours}:${minutes}:${seconds}.${ms}`;
};

export const levelColors: Record<LogLevel, string> = {
  debug: 'default',
  info: 'blue',
  warn: 'orange',
  error: 'red',
};

export const levelTexts: Record<LogLevel, string> = {
  debug: '调试',
  info: '信息',
  warn: '警告',
  error: '错误',
};
