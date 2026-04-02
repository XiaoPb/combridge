import { create } from 'zustand';
import type { SerialPortInfo, SerialConfig } from '../types';
import { DEFAULT_SERIAL_CONFIG } from '../types';

export interface DataEntry {
  id: string;
  timestamp: number;
  data: number[];
  direction: 'send' | 'receive';
  format: 'hex' | 'text';
}

export interface SerialTab {
  key: string;
  portName: string;
  config: SerialConfig;
  isConnected: boolean;
  openedAt?: number;
  receivedData: DataEntry[];
  sentData: DataEntry[];
}

interface SerialState {
  ports: SerialPortInfo[];
  tabs: SerialTab[];
  activeTabKey: string | null;
  isScanning: boolean;
  error: string | null;

  setPorts: (ports: SerialPortInfo[]) => void;
  
  addTab: (portName: string, config?: SerialConfig) => string;
  removeTab: (key: string) => void;
  setActiveTab: (key: string | null) => void;
  updateTab: (key: string, updates: Partial<SerialTab>) => void;
  
  addReceivedData: (portName: string, entry: DataEntry) => void;
  addSentData: (portName: string, entry: DataEntry) => void;
  clearTabData: (key: string) => void;
  
  setIsScanning: (isScanning: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState = {
  ports: [],
  tabs: [],
  activeTabKey: null,
  isScanning: false,
  error: null,
};

export const useSerialStore = create<SerialState>((set) => ({
  ...initialState,

  setPorts: (ports) => set({ ports }),

  addTab: (portName, config = DEFAULT_SERIAL_CONFIG) => {
    const key = `${portName}-${Date.now()}`;
    const newTab: SerialTab = {
      key,
      portName,
      config,
      isConnected: false,
      receivedData: [],
      sentData: [],
    };
    set((state) => ({
      tabs: [...state.tabs, newTab],
      activeTabKey: key,
    }));
    return key;
  },

  removeTab: (key) =>
    set((state) => {
      const newTabs = state.tabs.filter((t) => t.key !== key);
      let newActiveKey = state.activeTabKey;
      if (state.activeTabKey === key) {
        newActiveKey = newTabs.length > 0 ? newTabs[newTabs.length - 1].key : null;
      }
      return { tabs: newTabs, activeTabKey: newActiveKey };
    }),

  setActiveTab: (activeTabKey) => set({ activeTabKey }),

  updateTab: (key, updates) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.key === key ? { ...t, ...updates } : t)),
    })),

  addReceivedData: (portName, entry) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.portName === portName
          ? { ...t, receivedData: [...t.receivedData, entry].slice(-1000) }
          : t
      ),
    })),

  addSentData: (portName, entry) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.portName === portName
          ? { ...t, sentData: [...t.sentData, entry].slice(-1000) }
          : t
      ),
    })),

  clearTabData: (key) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.key === key ? { ...t, receivedData: [], sentData: [] } : t
      ),
    })),

  setIsScanning: (isScanning) => set({ isScanning }),

  setError: (error) => set({ error }),

  reset: () => set(initialState),
}));

export const generateId = (): string => {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
};

export const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp);
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${hours}:${minutes}:${seconds}.${ms}`;
};

export const formatData = (data: number[], format: 'hex' | 'text'): string => {
  if (!data || data.length === 0) return '';
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

export const getConnectedPorts = (tabs: SerialTab[]): string[] => {
  return tabs.filter((t) => t.isConnected).map((t) => t.portName);
};
