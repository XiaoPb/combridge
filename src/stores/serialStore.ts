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

export type SerialTabType = 'launcher' | 'port';

export interface SerialTab {
  key: string;
  tabType: SerialTabType;
  portName: string;
  config: SerialConfig;
  isConnected: boolean;
  openedAt?: number;
  receivedData: DataEntry[];
  sentData: DataEntry[];
  settingsCollapsed: boolean;
}

interface SerialState {
  ports: SerialPortInfo[];
  tabs: SerialTab[];
  activeTabKey: string | null;
  isScanning: boolean;
  error: string | null;

  setPorts: (ports: SerialPortInfo[]) => void;
  
  addLauncherTab: () => string;
  addPortTab: (portName: string, config?: SerialConfig) => string;
  removeTab: (key: string) => void;
  setActiveTab: (key: string | null) => void;
  updateTab: (key: string, updates: Partial<SerialTab>) => void;
  
  addReceivedData: (portName: string, entry: DataEntry) => void;
  addSentData: (portName: string, entry: DataEntry) => void;
  clearTabData: (key: string) => void;
  
  setIsScanning: (isScanning: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
  
  getPortTab: (portName: string) => SerialTab | undefined;
  hasPortTab: (portName: string) => boolean;
}

const LAUNCHER_TAB_KEY = 'serial-launcher';

const initialState = {
  ports: [],
  tabs: [{
    key: LAUNCHER_TAB_KEY,
    tabType: 'launcher' as SerialTabType,
    portName: '',
    config: DEFAULT_SERIAL_CONFIG,
    isConnected: false,
    receivedData: [],
    sentData: [],
    settingsCollapsed: false,
  }],
  activeTabKey: LAUNCHER_TAB_KEY,
  isScanning: false,
  error: null,
};

export const useSerialStore = create<SerialState>((set, get) => ({
  ...initialState,

  setPorts: (ports) => set({ ports }),

  addLauncherTab: () => {
    const state = get();
    const existingLauncher = state.tabs.find(t => t.tabType === 'launcher');
    if (existingLauncher) {
      set({ activeTabKey: existingLauncher.key });
      return existingLauncher.key;
    }
    
    const key = LAUNCHER_TAB_KEY;
    const newTab: SerialTab = {
      key,
      tabType: 'launcher',
      portName: '',
      config: DEFAULT_SERIAL_CONFIG,
      isConnected: false,
      receivedData: [],
      sentData: [],
      settingsCollapsed: false,
    };
    set((state) => ({
      tabs: [...state.tabs, newTab],
      activeTabKey: key,
    }));
    return key;
  },

  addPortTab: (portName, config = DEFAULT_SERIAL_CONFIG) => {
    const state = get();
    const existingTab = state.tabs.find(t => t.portName === portName && t.tabType === 'port');
    if (existingTab) {
      set({ activeTabKey: existingTab.key });
      return existingTab.key;
    }
    
    const key = `port-${portName}-${Date.now()}`;
    const newTab: SerialTab = {
      key,
      tabType: 'port',
      portName,
      config,
      isConnected: false,
      receivedData: [],
      sentData: [],
      settingsCollapsed: true,
    };
    set((state) => ({
      tabs: [...state.tabs, newTab],
      activeTabKey: key,
    }));
    return key;
  },

  removeTab: (key) =>
    set((state) => {
      if (key === LAUNCHER_TAB_KEY) return state;
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
        t.portName === portName && t.tabType === 'port'
          ? { ...t, receivedData: [...t.receivedData, entry].slice(-1000) }
          : t
      ),
    })),

  addSentData: (portName, entry) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.portName === portName && t.tabType === 'port'
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

  getPortTab: (portName: string) => {
    const state = get();
    return state.tabs.find(t => t.portName === portName && t.tabType === 'port');
  },

  hasPortTab: (portName: string) => {
    const state = get();
    return state.tabs.some(t => t.portName === portName && t.tabType === 'port');
  },
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
  return tabs.filter((t) => t.isConnected && t.tabType === 'port').map((t) => t.portName);
};
