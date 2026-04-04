export interface BleDeviceInfo {
  address: string;
  name?: string;
  rssi?: number;
  isConnectable: boolean;
  services?: string[];
  manufacturerData?: Record<string, number[]>;
  discoveredAt: number;
}

export interface BleCharacteristic {
  uuid: string;
  serviceUuid: string;
  properties: BleCharacteristicProperties;
  value?: number[];
}

export interface BleService {
  uuid: string;
  isPrimary: boolean;
  characteristics: BleCharacteristic[];
}

export interface BleCharacteristicProperties {
  read: boolean;
  write: boolean;
  writeWithoutResponse: boolean;
  notify: boolean;
  indicate: boolean;
}

export interface BleConnection {
  deviceId?: string;
  address: string;
  name?: string;
  isConnected: boolean;
  services: BleService[];
  connectedAt?: number;
  mtu?: number;
}

export interface BleScanOptions {
  filterName?: string;
  filterAddress?: string;
  filterRssi?: number;
  serviceUuids?: string[];
  timeout?: number;
}

export const BLE_SERVICE_UUID = {
  GENERIC_ACCESS: '00001800-0000-1000-8000-00805f9b34fb',
  GENERIC_ATTRIBUTE: '00001801-0000-1000-8000-00805f9b34fb',
  DEVICE_INFORMATION: '0000180a-0000-1000-8000-00805f9b34fb',
  BATTERY: '0000180f-0000-1000-8000-00805f9b34fb',
  HEART_RATE: '0000180d-0000-1000-8000-00805f9b34fb',
  NORDIC_UART: '6e400001-b5a3-f393-e0a9-e50e24dcca9e',
} as const;
