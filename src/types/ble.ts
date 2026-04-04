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
  '2a06': 'Alert Level',
  '2a07': 'External Report Reference',
  '2a08': 'Date Time',
  '2a09': 'Day of Week',
  '2a0a': 'Day Date Time',
  '2a0b': 'Exact Time 100',
  '2a0c': 'Exact Time 256',
  '2a0d': 'DST Offset',
  '2a0e': 'Time Zone',
  '2a0f': 'Local Time Information',
  '2a11': 'Time with DST',
  '2a12': 'Time Accuracy',
  '2a13': 'Time Source',
  '2a14': 'Reference Time Information',
  '2a15': 'Time Broadcast',
  '2a16': 'Notification URI',
  '2a17': 'Unread Alert Status',
  '2a18': 'Alert Category ID Bit Mask',
  '2a19': 'Battery Level',
  '2a1a': 'Battery Power State',
  '2a1b': 'Battery Level State',
  '2a1c': 'Temperature Measurement',
  '2a1d': 'Temperature Type',
  '2a1e': 'Intermediate Temperature',
  '2a1f': 'Temperature Celsius',
  '2a20': 'Temperature Fahrenheit',
  '2a21': 'Measurement Interval',
  '2a22': 'Boot Keyboard Input Report',
  '2a23': 'System ID',
  '2a24': 'Model Number String',
  '2a25': 'Serial Number String',
  '2a26': 'Firmware Revision String',
  '2a27': 'Hardware Revision String',
  '2a28': 'Software Revision String',
  '2a29': 'Manufacturer Name String',
  '2a2a': 'IEEE 11073-20601 Regulatory Certification Data List',
  '2a2b': 'Current Time',
  '2a2c': 'Scan Refresh',
  '2a2d': 'Scan Interval Window',
  '2a2e': 'PnP ID',
  '2a2f': 'Glucose Feature',
  '2a30': 'Glucose Measurement',
  '2a31': 'Glucose Measurement Context',
  '2a32': 'Boot Keyboard Output Report',
  '2a33': 'Boot Mouse Input Report',
  '2a34': 'Glucose Rate',
  '2a35': 'Blood Pressure Measurement',
  '2a36': 'Intermediate Cuff Pressure',
  '2a37': 'Heart Rate Measurement',
  '2a38': 'Body Sensor Location',
  '2a39': 'Heart Rate Control Point',
  '2a3a': 'Removable',
  '2a3b': 'Service Required',
  '2a3c': 'Scientific Temperature Celsius',
  '2a3d': 'String',
  '2a3e': 'Network Availability',
  '2a3f': 'Alert Status',
  '2a40': 'Ringer Control Point',
  '2a41': 'Ringer Setting',
  '2a42': 'Alert Category ID Bit Mask',
  '2a43': 'Alert Category ID',
  '2a44': 'Alert Notification Control Point',
  '2a45': 'Unread Alert Status',
  '2a46': 'New Alert',
  '2a47': 'Supported New Alert Category',
  '2a48': 'Supported Unread Alert Category',
  '2a49': 'Blood Pressure Feature',
  '2a4a': 'HID Information',
  '2a4b': 'Report Map',
  '2a4c': 'HID Control Point',
  '2a4d': 'Report',
  '2a4e': 'Protocol Mode',
  '2a4f': 'Scan Interval Window',
  '2a50': 'PnP ID',
  '2a51': 'Glucose Feature',
  '2a52': 'Record Access Control Point',
  '2a53': 'RSC Measurement',
  '2a54': 'RSC Feature',
  '2a55': 'SC Control Point',
  '2a56': 'Digital',
  '2a57': 'Digital Output',
  '2a58': 'Analog',
  '2a59': 'Analog Output',
  '2a5a': 'Aggregate',
  '2a5b': 'Cycling Power Measurement',
  '2a5c': 'Cycling Power Vector',
  '2a5d': 'Cycling Power Feature',
  '2a5e': 'Cycling Power Control Point',
  '2a5f': 'Location and Speed',
  '2a60': 'Navigation',
  '2a61': 'Position Quality',
  '2a62': 'LN Feature',
  '2a63': 'LN Control Point',
  '2a64': 'Elevation',
  '2a65': 'Pressure',
  '2a66': 'Temperature',
  '2a67': 'Humidity',
  '2a68': 'True Wind Speed',
  '2a69': 'True Wind Direction',
  '2a6a': 'Apparent Wind Speed',
  '2a6b': 'Apparent Wind Direction',
  '2a6c': 'Gust Factor',
  '2a6d': 'Pollen Concentration',
  '2a6e': 'UV Index',
  '2a6f': 'Irradiance',
  '2a70': 'Rainfall',
  '2a71': 'Wind Chill',
  '2a72': 'Heat Index',
  '2a73': 'Dew Point',
  '2a74': 'Trend',
  '2a75': 'Descriptor Value Changed',
  '2a76': 'Aerobic Heart Rate Lower Limit',
  '2a77': 'Aerobic Threshold',
  '2a78': 'Age',
  '2a79': 'Anaerobic Heart Rate Lower Limit',
  '2a7a': 'Anaerobic Heart Rate Upper Limit',
  '2a7b': 'Anaerobic Threshold',
  '2a7c': 'Aerobic Heart Rate Upper Limit',
  '2a7d': 'Date of Birth',
  '2a7e': 'Date of Threshold Assessment',
  '2a7f': 'Email Address',
  '2a80': 'Fat Burn Heart Rate Lower Limit',
  '2a81': 'Fat Burn Heart Rate Upper Limit',
  '2a82': 'First Name',
  '2a83': 'Five Zone Heart Rate Limits',
  '2a84': 'Gender',
  '2a85': 'Heart Rate Max',
  '2a86': 'Height',
  '2a87': 'Hip Circumference',
  '2a88': 'Last Name',
  '2a89': 'Maximum Recommended Heart Rate',
  '2a8a': 'Resting Heart Rate',
  '2a8b': 'Sport Type for Aerobic and Anaerobic Thresholds',
  '2a8c': 'Three Zone Heart Rate Limits',
  '2a8d': 'Two Zone Heart Rate Limit',
  '2a8e': 'VO2 Max',
  '2a8f': 'Waist Circumference',
  '2a90': 'Weight',
  '2a91': 'Database Change Increment',
  '2a92': 'User Index',
  '2a93': 'Body Composition Feature',
  '2a94': 'Body Composition Measurement',
  '2a95': 'Weight Measurement',
  '2a96': 'Weight Scale Feature',
  '2a97': 'User Control Point',
  '2a98': 'Magnetic Flux Density - 2D',
  '2a99': 'Magnetic Flux Density - 3D',
  '2a9a': 'Language',
  '2a9b': 'Barometric Pressure Trend',
  '2a9c': 'Bond Management Control Point',
  '2a9d': 'Bond Management Feature',
  '2a9e': 'Central Address Resolution',
  '2a9f': 'CGM Measurement',
  '2aa0': 'CGM Feature',
  '2aa1': 'CGM Status',
  '2aa2': 'CGM Session Start Time',
  '2aa3': 'CGM Session Run Time',
  '2aa4': 'CGM Specific Ops Control Point',
  '2aa5': 'Indoor Positioning Configuration',
  '2aa6': 'Latitude',
  '2aa7': 'Longitude',
  '2aa8': 'Local North Coordinate',
  '2aa9': 'Local East Coordinate',
  '2aaa': 'Floor Number',
  '2aab': 'Altitude',
  '2aac': 'Uncertainty',
  '2aad': 'Location Name',
  '2aae': 'URI',
  '2aaf': 'HTTP Headers',
  '2ab0': 'HTTP Status Code',
  '2ab1': 'HTTP Entity Body',
  '2ab2': 'HTTP Control Point',
  '2ab3': 'HTTPS Security',
  '2ab4': 'TDS Control Point',
  '2ab5': 'OTS Feature',
  '2ab6': 'Object Name',
  '2ab7': 'Object Type',
  '2ab8': 'Object Size',
  '2ab9': 'Object First-Created',
  '2aba': 'Object Last-Modified',
  '2abb': 'Object ID',
  '2abc': 'Object Properties',
  '2abd': 'Object Action Control Point',
  '2abe': 'Object List Control Point',
  '2abf': 'Object List Filter',
  '2ac0': 'Object Changed',
  '2ac1': 'Resolvable Private Address Only',
  '2ac2': 'Audio Input State',
  '2ac3': 'Gain Settings Attribute',
  '2ac4': 'Audio Input Type',
  '2ac5': 'Audio Input Status',
  '2ac6': 'Audio Input Control Point',
  '2ac7': 'Audio Input Description',
  '2ac8': 'Volume State',
  '2ac9': 'Volume Control Point',
  '2aca': 'Volume Flags',
  '2acb': 'Offset State',
  '2acc': 'Audio Location',
  '2acd': 'Volume Offset Control Point',
  '2ace': 'Audio Output Description',
  '2acf': 'Set Identity Resolving Key',
  '2ad0': 'Size',
  '2ad1': 'Lock',
  '2ad2': 'Encrypted Data Key Material',
  '2ad3': 'Audio Stream Control Point',
  '2ad4': 'Broadcast Receive State',
  '2ad5': 'Scan Delegator Data',
  '2ad6': 'Broadcast Audio Scan Control Point',
  '2ad7': 'PACS',
  '2ad8': 'Audio Availability',
  '2ad9': 'Supported Audio Contexts',
  '2ada': 'Ammonia Concentration',
  '2adb': 'Carbon Monoxide Concentration',
  '2adc': 'Methane Concentration',
  '2add': 'Nitrogen Dioxide Concentration',
  '2ade': 'Non-Methane Volatile Organic Compounds Concentration',
  '2adf': 'Ozone Concentration',
  '2ae0': 'Particulate Matter - PM1 Concentration',
  '2ae1': 'Particulate Matter - PM2.5 Concentration',
  '2ae2': 'Particulate Matter - PM10 Concentration',
  '2ae3': 'Sulfur Dioxide Concentration',
  '2ae4': 'Sulfur Hexafluoride Concentration',
  '2ae5': 'Total Volatile Organic Compounds Concentration',
  '2ae6': 'CO2 Concentration',
  '2ae7': 'Cosine of the Angle',
  '2ae8': 'Angle',
  '2ae9': 'Sincos of the Angle',
  '2aea': 'Feature',
  '2aeb': 'Sink PAC',
  '2aec': 'Sink Audio Locations',
  '2aed': 'Source PAC',
  '2aee': 'Source Audio Locations',
  '2aef': 'Volume Offset State',
  '2af0': 'Audio Prefab ID',
  '2af1': 'Total Group Data',
  '2af2': 'Relative Runtime in a Current Range',
  '2af3': 'Relative Runtime in a Generic Level Range',
  '2af4': 'Sensing Configuration',
  '2af5': 'Sensing Configuration Status',
  '2af6': 'ES Trigger Setting',
  '2af7': 'ES Trigger Setting Description',
  '2af8': 'ES Trigger Setting Counter',
  '2af9': 'Observation Schedule',
  '2afa': 'Observation Schedule Status',
  '2afb': 'Valid Range',
  '2afc': 'ES Configuration',
  '2afd': 'ES Measurement',
  '2afe': 'ES Trigger Setting',
  '2aff': 'ES Trigger Setting Description',
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
      return cleanUuid.slice(4, 8);
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
