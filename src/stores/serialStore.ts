import { create } from 'zustand';
import type { SerialPortInfo, SerialConfig, SerialConnection } from '../types';
import { DEFAULT_SERIAL_CONFIG } from '../types';

export interface DataEntry {
  id: string;
  timestamp: number;
  data: number[];
  direction: 'send' | 'receive';
  format: 'hex' | 'text';
}

interface SerialState {
  ports: SerialPortInfo[];
  openPorts: SerialConnection[];
  currentPort: string | null;
  config: SerialConfig;
  receivedData: DataEntry[];
  sentData: DataEntry[];
  isScanning: boolean;
  error: string | null;

  setPorts: (ports: SerialPortInfo[]) => void;
  setOpenPorts: (ports: SerialConnection[]) => void;
  addOpenPort: (connection: SerialConnection) => void;
  removeOpenPort: (portName: string) => void;
  setCurrentPort: (portName: string | null) => void;
  setConfig: (config: SerialConfig) => void;
  updateConfig: (config: Partial<SerialConfig>) => void;
  addReceivedData: (entry: DataEntry) => void;
  addSentData: (entry: DataEntry) => void;
  clearReceivedData: () => void;
  clearSentData: () => void;
  clearAllData: () => void;
  setIsScanning: (isScanning: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState = {
  ports: [],
  openPorts: [],
  currentPort: null,
  config: DEFAULT_SERIAL_CONFIG,
  receivedData: [],
  sentData: [],
  isScanning: false,
  error: null,
};

export const useSerialStore = create<SerialState>((set) => ({
  ...initialState,

  setPorts: (ports) => set({ ports }),

  setOpenPorts: (openPorts) => set({ openPorts }),

  addOpenPort: (connection) =>
    set((state) => {
      const exists = state.openPorts.find((p) => p.portName === connection.portName);
      if (exists) {
        return {
          openPorts: state.openPorts.map((p) =>
            p.portName === connection.portName ? connection : p
          ),
        };
      }
      return { openPorts: [...state.openPorts, connection] };
    }),

  removeOpenPort: (portName) =>
    set((state) => ({
      openPorts: state.openPorts.filter((p) => p.portName !== portName),
      currentPort: state.currentPort === portName ? null : state.currentPort,
    })),

  setCurrentPort: (currentPort) => set({ currentPort }),

  setConfig: (config) => set({ config }),

  updateConfig: (config) =>
    set((state) => ({ config: { ...state.config, ...config } })),

  addReceivedData: (entry) =>
    set((state) => ({
      receivedData: [...state.receivedData, entry].slice(-1000),
    })),

  addSentData: (entry) =>
    set((state) => ({
      sentData: [...state.sentData, entry].slice(-1000),
    })),

  clearReceivedData: () => set({ receivedData: [] }),

  clearSentData: () => set({ sentData: [] }),

  clearAllData: () => set({ receivedData: [], sentData: [] }),

  setIsScanning: (isScanning) => set({ isScanning }),

  setError: (error) => set({ error }),

  reset: () => set(initialState),
}));

export const generateId = (): string => {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
};

export const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('zh-CN', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
  });
};

export const formatData = (data: number[], format: 'hex' | 'text'): string => {
  if (format === 'hex') {
    return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
  }
  return new TextDecoder().decode(new Uint8Array(data));
};

export const parseData = (input: string, format: 'hex' | 'text'): number[] => {
  if (format === 'hex') {
    const hex = input.replace(/\s+/g, '');
    const result: number[] = [];
    for (let i = 0; i < hex.length; i += 2) {
      result.push(parseInt(hex.substr(i, 2), 16));
    }
    return result;
  }
  return Array.from(new TextEncoder().encode(input));
};
