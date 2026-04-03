import { create } from 'zustand';
import type { BleDeviceInfo, BleConnection, BleService, BleCharacteristic } from '../types';

export type BleMode = 'native' | 'at';

export interface BleNotification {
  id: string;
  deviceId: string;
  characteristicUuid: string;
  data: number[];
  timestamp: number;
}

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

  setMode: (mode: BleMode) => void;
  setSerialPort: (port: string | null) => void;
  setDevices: (devices: BleDeviceInfo[]) => void;
  addDevice: (device: BleDeviceInfo) => void;
  updateDevice: (address: string, device: Partial<BleDeviceInfo>) => void;
  removeDevice: (address: string) => void;
  clearDevices: () => void;
  setConnections: (connections: BleConnection[]) => void;
  addConnection: (connection: BleConnection) => void;
  updateConnection: (address: string, connection: Partial<BleConnection>) => void;
  removeConnection: (address: string) => void;
  setCurrentDevice: (deviceId: string | null) => void;
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
  reset: () => void;
}

const initialState = {
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
};

export const useBleStore = create<BleState>((set) => ({
  ...initialState,

  setMode: (mode) => set({ mode }),

  setSerialPort: (serialPort) => set({ serialPort }),

  setDevices: (devices) => set({ devices }),

  addDevice: (device) =>
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

  updateDevice: (address, device) =>
    set((state) => ({
      devices: state.devices.map((d) =>
        d.address === address ? { ...d, ...device } : d
      ),
    })),

  removeDevice: (address) =>
    set((state) => ({
      devices: state.devices.filter((d) => d.address !== address),
    })),

  clearDevices: () => set({ devices: [] }),

  setConnections: (connections) => set({ connections }),

  addConnection: (connection) =>
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

  updateConnection: (address, connection) =>
    set((state) => ({
      connections: state.connections.map((c) =>
        c.address === address ? { ...c, ...connection } : c
      ),
    })),

  removeConnection: (address) =>
    set((state) => ({
      connections: state.connections.filter((c) => c.address !== address),
      currentDevice: state.currentDevice === address ? null : state.currentDevice,
    })),

  setCurrentDevice: (currentDevice) => set({ currentDevice }),

  setServices: (services) => set({ services }),

  addService: (service) =>
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

  setCharacteristics: (characteristics) => set({ characteristics }),

  updateCharacteristic: (uuid, characteristic) =>
    set((state) => ({
      characteristics: state.characteristics.map((c) =>
        c.uuid === uuid ? { ...c, ...characteristic } : c
      ),
    })),

  clearCharacteristics: () => set({ characteristics: [] }),

  addNotification: (notification) =>
    set((state) => ({
      notifications: [...state.notifications, notification].slice(-500),
    })),

  clearNotifications: () => set({ notifications: [] }),

  setIsScanning: (isScanning) => set({ isScanning }),

  setIsConnecting: (isConnecting) => set({ isConnecting }),

  setIsConfigured: (isConfigured) => set({ isConfigured }),

  setError: (error) => set({ error }),

  reset: () => set(initialState),
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
