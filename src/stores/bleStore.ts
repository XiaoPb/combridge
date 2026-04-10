import { create } from 'zustand';
import type { BleDeviceInfo, BleConnection, BleService, BleCharacteristic } from '../types';
import { preferencesApi } from '../api/tauri';

export type BleMode = 'native' | 'at';

export interface BleNotification {
  id: string;
  deviceId: string;
  characteristicUuid: string;
  data: number[];
  timestamp: number;
}

export interface AtConnectionTab {
  id: string;
  address: string;
  name: string | null;
  txUuid: string;
  rxUuid: string;
  connectedAt: number;
  receivedData: AtDataEntry[];
  sentData: AtDataEntry[];
}

export interface AtDataEntry {
  id: string;
  timestamp: number;
  data: number[];
  direction: 'send' | 'receive';
}

export interface AtConfig {
  portName: string;
  baudRate: number;
  timeoutMs: number;
  txUuid: string | null;
  rxUuid: string | null;
  srvUuid: string | null;
}

export interface BlePreferences {
  displayFormat: 'hex' | 'text';
  autoScroll: boolean;
  inputFormat: 'hex' | 'text';
  withoutResponse: boolean;
  configCollapsed: boolean;
  gattCollapsed: boolean;
  panelCollapsed: boolean;
}

const DEFAULT_PREFERENCES: BlePreferences = {
  displayFormat: 'text',
  autoScroll: true,
  inputFormat: 'text',
  withoutResponse: false,
  configCollapsed: false,
  gattCollapsed: false,
  panelCollapsed: false,
};

interface BleState {
  mode: BleMode;
  serialPort: string | null;
  devices: BleDeviceInfo[];
  connections: BleConnection[];
  currentDevice: string | null;
  services: BleService[];
  characteristics: BleCharacteristic[];
  notifications: BleNotification[];
  isScanning: boolean;
  isConnecting: boolean;
  isConfigured: boolean;
  error: string | null;
  preferences: BlePreferences;
  atConfig: AtConfig | null;
  atTabs: AtConnectionTab[];
  activeAtTabId: string | null;
  
  setMode: (mode: BleMode) => void;
  setSerialPort: (serialPort: string | null) => void;
  setDevices: (devices: BleDeviceInfo[]) => void;
  addDevice: (device: BleDeviceInfo) => void;
  updateDevice: (address: string, device: Partial<BleDeviceInfo>) => void;
  removeDevice: (address: string) => void;
  clearDevices: () => void;
  setConnections: (connections: BleConnection[]) => void;
  addConnection: (connection: BleConnection) => void;
  updateConnection: (address: string, connection: Partial<BleConnection>) => void;
  removeConnection: (address: string) => void;
  setCurrentDevice: (currentDevice: string | null) => void;
  setServices: (services: BleService[]) => void;
  addService: (service: BleService) => void;
  clearServices: () => void;
  setCharacteristics: (characteristics: BleCharacteristic[]) => void;
  updateCharacteristic: (uuid: string, characteristic: Partial<BleCharacteristic>) => void;
  clearCharacteristics: () => void;
  addNotification: (notification: BleNotification) => void;
  clearNotifications: () => void;
  setIsScanning: (isScanning: boolean) => void;
  setIsConnecting: (isConnecting: boolean) => void;
  setIsConfigured: (isConfigured: boolean) => void;
  setError: (error: string | null) => void;
  loadPreferences: () => Promise<void>;
  updatePreferences: (updates: Partial<BlePreferences>) => Promise<void>;
  reset: () => void;
  setAtConfig: (config: AtConfig) => void;
  updateAtConfig: (updates: Partial<AtConfig>) => void;
  addAtTab: (tab: AtConnectionTab) => void;
  updateAtTab: (tabId: string, updates: Partial<AtConnectionTab>) => void;
  removeAtTab: (tabId: string) => void;
  setActiveAtTabId: (tabId: string | null) => void;
  addAtReceivedData: (tabId: string, entry: AtDataEntry) => void;
  addAtSentData: (tabId: string, entry: AtDataEntry) => void;
  clearAtTabData: (tabId: string) => void;
}

export const useBleStore = create<BleState>((set, _get) => ({
  mode: 'native' as BleMode,
  serialPort: null,
  devices: [],
  connections: [],
  currentDevice: null,
  services: [],
  characteristics: [],
  notifications: [],
  isScanning: false,
  isConnecting: false,
  isConfigured: false,
  error: null,
  preferences: DEFAULT_PREFERENCES,
  atConfig: null,
  atTabs: [],
  activeAtTabId: null,

  setMode: (mode: BleMode) => set({ mode }),

  setSerialPort: (serialPort: string | null) => set({ serialPort }),

  setDevices: (devices: BleDeviceInfo[]) => set({ devices }),

  addDevice: (device: BleDeviceInfo) =>
    set((state) => {
      const exists = state.devices.find((d) => d.address === device.address);
      if (exists) {
        return {
          devices: state.devices.map((d) =>
            d.address === device.address ? { ...d, ...device } : d
          ),
        };
      }
      return { devices: [...state.devices, device] };
    }),

  updateDevice: (address: string, device: Partial<BleDeviceInfo>) =>
    set((state) => ({
      devices: state.devices.map((d) =>
        d.address === address ? { ...d, ...device } : d
      ),
    })),

  removeDevice: (address: string) =>
    set((state) => ({
      devices: state.devices.filter((d) => d.address !== address),
    })),

  clearDevices: () => set({ devices: [] }),

  setConnections: (connections: BleConnection[]) => set({ connections }),

  addConnection: (connection: BleConnection) =>
    set((state) => {
      const exists = state.connections.find((c) => c.address === connection.address);
      if (exists) {
        return {
          connections: state.connections.map((c) =>
            c.address === connection.address ? connection : c
          ),
        };
      }
      return { connections: [...state.connections, connection] };
    }),

  updateConnection: (address: string, connection: Partial<BleConnection>) =>
    set((state) => ({
      connections: state.connections.map((c) =>
        c.address === address ? { ...c, ...connection } : c
      ),
    })),

  removeConnection: (address: string) =>
    set((state) => ({
      connections: state.connections.filter((c) => c.address !== address),
      currentDevice: state.currentDevice === address ? null : state.currentDevice,
    })),

  setCurrentDevice: (currentDevice: string | null) => set({ currentDevice }),

  setServices: (services: BleService[]) => set({ services }),

  addService: (service: BleService) =>
    set((state) => {
      const exists = state.services.find((s) => s.uuid === service.uuid);
      if (exists) {
        return {
          services: state.services.map((s) =>
            s.uuid === service.uuid ? service : s
          ),
        };
      }
      return { services: [...state.services, service] };
    }),

  clearServices: () => set({ services: [] }),

  setCharacteristics: (characteristics: BleCharacteristic[]) => set({ characteristics }),

  updateCharacteristic: (uuid: string, characteristic: Partial<BleCharacteristic>) =>
    set((state) => ({
      characteristics: state.characteristics.map((c) =>
        c.uuid === uuid ? { ...c, ...characteristic } : c
      ),
    })),

  clearCharacteristics: () => set({ characteristics: [] }),

  addNotification: (notification: BleNotification) =>
    set((state) => ({
      notifications: [...state.notifications, notification].slice(-500),
    })),

  clearNotifications: () => set({ notifications: [] }),

  setIsScanning: (isScanning: boolean) => set({ isScanning }),

  setIsConnecting: (isConnecting: boolean) => set({ isConnecting }),

  setIsConfigured: (isConfigured: boolean) => set({ isConfigured }),

  setError: (error: string | null) => set({ error }),

  loadPreferences: async () => {
    try {
      const prefs = await preferencesApi.get();
      if (prefs && prefs.ble) {
        set({ preferences: prefs.ble });
      }
    } catch (err) {
      console.error('加载BLE偏好设置失败:', err);
    }
  },

  updatePreferences: async (updates: Partial<BlePreferences>) => {
    set((state) => ({
      preferences: { ...state.preferences, ...updates },
    }));
    try {
      await preferencesApi.updateBle(useBleStore.getState().preferences);
    } catch (err) {
      console.error('保存BLE偏好设置失败:', err);
    }
  },

  reset: () => set({
    mode: 'native' as BleMode,
    serialPort: null,
    devices: [],
    connections: [],
    currentDevice: null,
    services: [],
    characteristics: [],
    notifications: [],
    isScanning: false,
    isConnecting: false,
    isConfigured: false,
    error: null,
    preferences: DEFAULT_PREFERENCES,
    atConfig: null,
    atTabs: [],
    activeAtTabId: null,
  }),

  setAtConfig: (config: AtConfig) => set({ atConfig: config }),

  updateAtConfig: (updates: Partial<AtConfig>) =>
    set((state) => ({
      atConfig: state.atConfig ? { ...state.atConfig, ...updates } : updates as AtConfig,
    })),

  addAtTab: (tab: AtConnectionTab) =>
    set((state) => {
      const exists = state.atTabs.find((t) => t.id === tab.id);
      if (exists) return state;
      return { atTabs: [...state.atTabs, tab] };
    }),

  updateAtTab: (tabId: string, updates: Partial<AtConnectionTab>) =>
    set((state) => ({
      atTabs: state.atTabs.map((t) =>
        t.id === tabId ? { ...t, ...updates } : t
      ),
    })),

  removeAtTab: (tabId: string) =>
    set((state) => ({
      atTabs: state.atTabs.filter((t) => t.id !== tabId),
      activeAtTabId: state.activeAtTabId === tabId ? null : state.activeAtTabId,
    })),

  setActiveAtTabId: (tabId: string | null) => set({ activeAtTabId: tabId }),

  addAtReceivedData: (tabId: string, entry: AtDataEntry) =>
    set((state) => ({
      atTabs: state.atTabs.map((t) =>
        t.id === tabId
          ? { ...t, receivedData: [...t.receivedData, entry].slice(-1000) }
          : t
      ),
    })),

  addAtSentData: (tabId: string, entry: AtDataEntry) =>
    set((state) => ({
      atTabs: state.atTabs.map((t) =>
        t.id === tabId
          ? { ...t, sentData: [...t.sentData, entry].slice(-1000) }
          : t
      ),
    })),

  clearAtTabData: (tabId: string) =>
    set((state) => ({
      atTabs: state.atTabs.map((t) =>
        t.id === tabId ? { ...t, receivedData: [], sentData: [] } : t
      ),
    })),
}));

export const generateBleId = (): string => {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
};

export const formatBleTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp);
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${hours}:${minutes}:${seconds}.${ms}`;
};

export const formatBleData = (data: number[], format: 'hex' | 'text'): string => {
  if (format === 'hex') {
    return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
  }
  return new TextDecoder().decode(new Uint8Array(data));
};

export const parseBleData = (input: string, format: 'hex' | 'text'): number[] => {
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

export const formatUuid = (uuid: string): string => {
  if (uuid.length === 4) {
    return `0000${uuid}-0000-1000-8000-00805f9b34fb`;
  }
  if (uuid.length === 32) {
    return `${uuid.slice(0, 8)}-${uuid.slice(8, 12)}-${uuid.slice(12, 16)}-${uuid.slice(16, 20)}-${uuid.slice(20)}`;
  }
  return uuid.toLowerCase();
};

export const getShortUuid = (uuid: string): string => {
  const formatted = formatUuid(uuid);
  if (formatted.startsWith('0000') && formatted.endsWith('-0000-1000-8000-00805f9b34fb')) {
    return formatted.slice(4, 8).toUpperCase();
  }
  return formatted;
};

export const formatMacAddress = (address: string): string => {
  if (!address) return '-';
  const match = address.match(/([0-9a-fA-F]{2}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2})/);
  if (match) {
    return match[1].toUpperCase();
  }
  const parts = address.split('-');
  if (parts.length >= 2) {
    const macPart = parts[parts.length - 1];
    if (/^[0-9a-fA-F:]+$/.test(macPart)) {
      return macPart.toUpperCase();
  }
  }
  return address;
};
