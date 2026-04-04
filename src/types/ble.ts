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
  IMMEDIATE_ALERT: '00001802-0000-1000-8000-00805f9b34fb',
  LINK_LOSS: '00001803-0000-1000-8000-00805f9b34fb',
  TX_POWER: '00001804-0000-1000-8000-00805f9b34fb',
  CURRENT_TIME: '00001805-0000-1000-8000-00805f9b34fb',
  REFERENCE_TIME_UPDATE: '00001806-0000-1000-8000-00805f9b34fb',
  NEXT_DST_CHANGE: '00001807-0000-1000-8000-00805f9b34fb',
  GLUCOSE: '00001808-0000-1000-8000-00805f9b34fb',
  HEALTH_THERMOMETER: '00001809-0000-1000-8000-00805f9b34fb',
  DEVICE_INFORMATION: '0000180a-0000-1000-8000-00805f9b34fb',
  BLOOD_PRESSURE: '00001810-0000-1000-8000-00805f9b34fb',
  ALERT_NOTIFICATION: '00001811-0000-1000-8000-00805f9b34fb',
  HID: '00001812-0000-1000-8000-00805f9b34fb',
  SCAN_PARAMETERS: '00001813-0000-1000-8000-00805f9b34fb',
  RUNNING_SPEED_CADENCE: '00001814-0000-1000-8000-00805f9b34fb',
  CYCLING_SPEED_CADENCE: '00001815-0000-1000-8000-00805f9b34fb',
  CYCLING_POWER: '00001816-0000-1000-8000-00805f9b34fb',
  LOCATION_NAVIGATION: '00001817-0000-1000-8000-00805f9b34fb',
  ENVIRONMENTAL_SENSING: '0000181a-0000-1000-8000-00805f9b34fb',
  BODY_COMPOSITION: '00001819-0000-1000-8000-00805f9b34fb',
  WEIGHT_SCALE: '0000181b-0000-1000-8000-00805f9b34fb',
  BOND_MANAGEMENT: '0000181c-0000-1000-8000-00805f9b34fb',
  CONTINUOUS_GLUCOSE_MONITORING: '0000181d-0000-1000-8000-00805f9b34fb',
  INTERNET_PROTOCOL_SUPPORT: '0000181e-0000-1000-8000-00805f9b34fb',
  INDOOR_POSITIONING: '0000181f-0000-1000-8000-00805f9b34fb',
  PULSE_OXIMETER: '00001820-0000-1000-8000-00805f9b34fb',
  HTTP_PROXY: '00001821-0000-1000-8000-00805f9b34fb',
  TRANSPORT_DISCOVERY: '00001822-0000-1000-8000-00805f9b34fb',
  OBJECT_TRANSFER: '00001823-0000-1000-8000-00805f9b34fb',
  HEART_RATE: '0000180d-0000-1000-8000-00805f9b34fb',
  BATTERY: '0000180f-0000-1000-8000-00805f9b34fb',
  NORDIC_UART: '6e400001-b5a3-f393-e0a9-e50e24dcca9e',
  ANKI_DRIVE: '0000fef5-0000-1000-8000-00805f9b34fb',
} as const;

export const BLE_CHARACTERISTIC_UUID = {
  DEVICE_NAME: '00002a00-0000-1000-8000-00805f9b34fb',
  APPEARANCE: '00002a01-0000-1000-8000-00805f9b34fb',
  PERIPHERAL_PRIVACY_FLAG: '00002a02-0000-1000-8000-00805f9b34fb',
  RECONNECTION_ADDRESS: '00002a03-0000-1000-8000-00805f9b34fb',
  PERIPHERAL_PREFERRED_CONNECTION_PARAMETERS: '00002a04-0000-1000-8000-00805f9b34fb',
  SERVICE_CHANGED: '00002a05-0000-1000-8000-00805f9b34fb',
  SYSTEM_ID: '00002a23-0000-1000-8000-00805f9b34fb',
  MODEL_NUMBER_STRING: '00002a24-0000-1000-8000-00805f9b34fb',
  SERIAL_NUMBER_STRING: '00002a25-0000-1000-8000-00805f9b34fb',
  FIRMWARE_REVISION_STRING: '00002a26-0000-1000-8000-00805f9b34fb',
  HARDWARE_REVISION_STRING: '00002a27-0000-1000-8000-00805f9b34fb',
  SOFTWARE_REVISION_STRING: '00002a28-0000-1000-8000-00805f9b34fb',
  MANUFACTURER_NAME_STRING: '00002a29-0000-1000-8000-00805f9b34fb',
  IEEE_REGULATORY_CERTIFICATION: '00002a2a-0000-1000-8000-00805f9b34fb',
  PNP_ID: '00002a50-0000-1000-8000-00805f9b34fb',
  BATTERY_LEVEL: '00002a19-0000-1000-8000-00805f9b34fb',
  HEART_RATE_MEASUREMENT: '00002a37-0000-1000-8000-00805f9b34fb',
  BODY_SENSOR_LOCATION: '00002a38-0000-1000-8000-00805f9b34fb',
  HEART_RATE_CONTROL_POINT: '00002a39-0000-1000-8000-00805f9b34fb',
  BLOOD_PRESSURE_MEASUREMENT: '00002a35-0000-1000-8000-00805f9b34fb',
  INTERMEDIATE_CUFF_PRESSURE: '00002a36-0000-1000-8000-00805f9b34fb',
  DATE_TIME: '00002a08-0000-1000-8000-00805f9b34fb',
  DAY_DATE_TIME: '00002a0a-0000-1000-8000-00805f9b34fb',
  DAY_OF_WEEK: '00002a0b-0000-1000-8000-00805f9b34fb',
  EXACT_TIME_256: '00002a0c-0000-1000-8000-00805f9b34fb',
  DST_OFFSET: '00002a0d-0000-1000-8000-00805f9b34fb',
  TIME_ZONE: '00002a0e-0000-1000-8000-00805f9b34fb',
  LOCAL_TIME_INFORMATION: '00002a0f-0000-1000-8000-00805f9b34fb',
  REFERENCE_TIME_INFORMATION: '00002a14-0000-1000-8000-00805f9b34fb',
  TIME_UPDATE_CONTROL_POINT: '00002a43-0000-1000-8000-00805f9b34fb',
  TIME_UPDATE_STATE: '00002a44-0000-1000-8000-00805f9b34fb',
  TEMPERATURE_MEASUREMENT: '00002a1c-0000-1000-8000-00805f9b34fb',
  TEMPERATURE_TYPE: '00002a1d-0000-1000-8000-00805f9b34fb',
  INTERMEDIATE_TEMPERATURE: '00002a1e-0000-1000-8000-00805f9b34fb',
  TEMPERATURE_ENVIRONMENTAL: '00002a6e-0000-1000-8000-00805f9b34fb',
  HUMIDITY: '00002a6f-0000-1000-8000-00805f9b34fb',
  PRESSURE: '00002a70-0000-1000-8000-00805f9b34fb',
  NORDIC_UART_TX: '6e400002-b5a3-f393-e0a9-e50e24dcca9e',
  NORDIC_UART_RX: '6e400003-b5a3-f393-e0a9-e50e24dcca9e',
} as const;

const BLE_SERVICE_NAMES: Record<string, string> = {
  '1800': 'Generic Access',
  '1801': 'Generic Attribute',
  '1802': 'Immediate Alert',
  '1803': 'Link Loss',
  '1804': 'Tx Power',
  '1805': 'Current Time Service',
  '1806': 'Reference Time Update Service',
  '1807': 'Next DST Change Service',
  '1808': 'Glucose',
  '1809': 'Health Thermometer',
  '180a': 'Device Information',
  '180d': 'Heart Rate',
  '180f': 'Battery Service',
  '1810': 'Blood Pressure',
  '1811': 'Alert Notification Service',
  '1812': 'Human Interface Device',
  '1813': 'Scan Parameters',
  '1814': 'Running Speed and Cadence',
  '1815': 'Cycling Speed and Cadence',
  '1816': 'Cycling Power',
  '1817': 'Location and Navigation',
  '1819': 'Body Composition',
  '181a': 'Environmental Sensing',
  '181b': 'Weight Scale',
  '181c': 'Bond Management',
  '181d': 'Continuous Glucose Monitoring',
  '181e': 'Internet Protocol Support',
  '181f': 'Indoor Positioning',
  '1820': 'Pulse Oximeter',
  '1821': 'HTTP Proxy',
  '1822': 'Transport Discovery',
  '1823': 'Object Transfer',
  'fef5': 'Anki Drive',
  '6e400001-b5a3-f393-e0a9-e50e24dcca9e': 'Nordic UART Service',
};

const BLE_CHARACTERISTIC_NAMES: Record<string, string> = {
  '2a00': 'Device Name',
  '2a01': 'Appearance',
  '2a02': 'Peripheral Privacy Flag',
  '2a03': 'Reconnection Address',
  '2a04': 'Peripheral Preferred Connection Parameters',
  '2a05': 'Service Changed',
  '2a08': 'Date Time',
  '2a0a': 'Day Date Time',
  '2a0b': 'Day of Week',
  '2a0c': 'Exact Time 256',
  '2a0d': 'DST Offset',
  '2a0e': 'Time Zone',
  '2a0f': 'Local Time Information',
  '2a14': 'Reference Time Information',
  '2a19': 'Battery Level',
  '2a1c': 'Temperature Measurement',
  '2a1d': 'Temperature Type',
  '2a1e': 'Intermediate Temperature',
  '2a23': 'System ID',
  '2a24': 'Model Number String',
  '2a25': 'Serial Number String',
  '2a26': 'Firmware Revision String',
  '2a27': 'Hardware Revision String',
  '2a28': 'Software Revision String',
  '2a29': 'Manufacturer Name String',
  '2a2a': 'IEEE 11073-20601 Regulatory Certification Data List',
  '2a35': 'Blood Pressure Measurement',
  '2a36': 'Intermediate Cuff Pressure',
  '2a37': 'Heart Rate Measurement',
  '2a38': 'Body Sensor Location',
  '2a39': 'Heart Rate Control Point',
  '2a43': 'Time Update Control Point',
  '2a44': 'Time Update State',
  '2a50': 'PnP ID',
  '2a6e': 'Temperature (Environmental Sensing)',
  '2a6f': 'Humidity',
  '2a70': 'Pressure',
  '6e400002-b5a3-f393-e0a9-e50e24dcca9e': 'Nordic UART TX',
  '6e400003-b5a3-f393-e0a9-e50e24dcca9e': 'Nordic UART RX',
};

function normalizeUuid(uuid: string): string {
  const cleanUuid = uuid.toLowerCase().replace(/[^a-f0-9]/g, '');
  if (cleanUuid.length === 4) {
    return cleanUuid;
  }
  if (cleanUuid.length === 32) {
    return `${cleanUuid.slice(0, 8)}-${cleanUuid.slice(8, 12)}-${cleanUuid.slice(12, 16)}-${cleanUuid.slice(16, 20)}-${cleanUuid.slice(20)}`;
  }
  return uuid.toLowerCase();
}

function extractShortUuid(uuid: string): string | null {
  const cleanUuid = uuid.toLowerCase().replace(/[^a-f0-9]/g, '');
  if (cleanUuid.length === 4) {
    return cleanUuid;
  }
  if (cleanUuid.length === 32) {
    const baseUuid = '00001000800000805f9b34fb';
    if (cleanUuid.endsWith(baseUuid)) {
      return cleanUuid.slice(0, 4);
    }
  }
  return null;
}

export function getServiceName(uuid: string): string {
  const normalizedUuid = normalizeUuid(uuid);
  const shortUuid = extractShortUuid(uuid);
  if (shortUuid && BLE_SERVICE_NAMES[shortUuid]) {
    return BLE_SERVICE_NAMES[shortUuid];
  }
  if (BLE_SERVICE_NAMES[normalizedUuid]) {
    return BLE_SERVICE_NAMES[normalizedUuid];
  }
  return 'Unknown Service';
}

export function getCharacteristicName(uuid: string): string {
  const normalizedUuid = normalizeUuid(uuid);
  const shortUuid = extractShortUuid(uuid);
  if (shortUuid && BLE_CHARACTERISTIC_NAMES[shortUuid]) {
    return BLE_CHARACTERISTIC_NAMES[shortUuid];
  }
  if (BLE_CHARACTERISTIC_NAMES[normalizedUuid]) {
    return BLE_CHARACTERISTIC_NAMES[normalizedUuid];
  }
  return 'Unknown Characteristic';
}
