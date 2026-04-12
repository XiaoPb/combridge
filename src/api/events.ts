import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type SerialDataEvent = {
  port_name: string;
  data: number[];
  timestamp?: number;
};

export type SerialErrorEvent = {
  portName: string;
  error: string;
};

export type BleDataEvent = {
  deviceId: string;
  characteristicUuid: string;
  data: number[];
  timestamp: number;
};

export type BleConnectionEvent = {
  deviceId: string;
  address: string;
  name?: string;
  connected: boolean;
};

export type BleErrorEvent = {
  deviceId?: string;
  error: string;
};

export type BleScanResultEvent = {
  device: {
    address: string;
    name?: string;
    rssi?: number;
    isConnectable: boolean;
    services?: string[];
    manufacturerData?: Record<string, number[]>;
  };
  timestamp: number;
};

export type BleModeChangedEvent = {
  mode: 'native' | 'at';
  serialPort?: string;
};

export type ParsedDataEvent = {
  timestamp: number;
  values: Record<string, number>;
};

export const TauriEvents = {
  SERIAL_DATA: 'serial-data',
  SERIAL_ERROR: 'serial-error',
  SERIAL_CONNECTED: 'serial-connected',
  SERIAL_DISCONNECTED: 'serial-disconnected',
  BLE_DATA: 'ble-notify',
  BLE_CONNECTED: 'ble-connected',
  BLE_DISCONNECTED: 'ble-disconnected',
  BLE_ERROR: 'ble-error',
  BLE_SCAN_RESULT: 'ble-scan-result',
  BLE_MODE_CHANGED: 'ble-mode-changed',
  PARSED_DATA: 'parsed-data',
} as const;

export function onSerialData(callback: (event: SerialDataEvent) => void): Promise<UnlistenFn> {
  return listen<SerialDataEvent>(TauriEvents.SERIAL_DATA, (event) => {
    callback(event.payload);
  });
}

export function onSerialError(callback: (event: SerialErrorEvent) => void): Promise<UnlistenFn> {
  return listen<SerialErrorEvent>(TauriEvents.SERIAL_ERROR, (event) => {
    callback(event.payload);
  });
}

export function onSerialConnected(callback: (portName: string) => void): Promise<UnlistenFn> {
  return listen<string>(TauriEvents.SERIAL_CONNECTED, (event) => {
    callback(event.payload);
  });
}

export function onSerialDisconnected(callback: (portName: string) => void): Promise<UnlistenFn> {
  return listen<string>(TauriEvents.SERIAL_DISCONNECTED, (event) => {
    callback(event.payload);
  });
}

export const serialEvents = {
  onData: onSerialData,
  onError: onSerialError,
  onConnected: onSerialConnected,
  onDisconnected: onSerialDisconnected,
};

export function onBleData(callback: (event: BleDataEvent) => void): Promise<UnlistenFn> {
  return listen<BleDataEvent>(TauriEvents.BLE_DATA, (event) => {
    console.debug('[BLE Event] 收到 ble-notify:', JSON.stringify(event.payload));
    callback(event.payload);
  });
}

export function onBleConnected(callback: (event: BleConnectionEvent) => void): Promise<UnlistenFn> {
  return listen<BleConnectionEvent>(TauriEvents.BLE_CONNECTED, (event) => {
    callback(event.payload);
  });
}

export function onBleDisconnected(callback: (event: BleConnectionEvent) => void): Promise<UnlistenFn> {
  return listen<BleConnectionEvent>(TauriEvents.BLE_DISCONNECTED, (event) => {
    callback(event.payload);
  });
}

export function onBleError(callback: (event: BleErrorEvent) => void): Promise<UnlistenFn> {
  return listen<BleErrorEvent>(TauriEvents.BLE_ERROR, (event) => {
    callback(event.payload);
  });
}

export function onBleScanResult(callback: (device: unknown) => void): Promise<UnlistenFn> {
  return listen<unknown>(TauriEvents.BLE_SCAN_RESULT, (event) => {
    callback(event.payload);
  });
}

export function onBleModeChanged(callback: (event: BleModeChangedEvent) => void): Promise<UnlistenFn> {
  return listen<BleModeChangedEvent>(TauriEvents.BLE_MODE_CHANGED, (event) => {
    callback(event.payload);
  });
}

export function onParsedData(callback: (event: ParsedDataEvent) => void): Promise<UnlistenFn> {
  return listen<ParsedDataEvent>(TauriEvents.PARSED_DATA, (event) => {
    callback(event.payload);
  });
}

export const bleEvents = {
  onData: onBleData,
  onConnected: onBleConnected,
  onDisconnected: onBleDisconnected,
  onError: onBleError,
  onScanResult: onBleScanResult,
  onModeChanged: onBleModeChanged,
};

export interface EventListeners {
  serialData?: UnlistenFn;
  serialError?: UnlistenFn;
  serialConnected?: UnlistenFn;
  serialDisconnected?: UnlistenFn;
  bleData?: UnlistenFn;
  bleConnected?: UnlistenFn;
  bleDisconnected?: UnlistenFn;
  bleError?: UnlistenFn;
  bleScanResult?: UnlistenFn;
  bleModeChanged?: UnlistenFn;
}

export async function cleanupListeners(listeners: EventListeners): Promise<void> {
  const unlistenFns = Object.values(listeners).filter((fn): fn is UnlistenFn => fn !== undefined);
  await Promise.all(unlistenFns.map((fn) => fn()));
}
