export type DataSourceType = 'serial' | 'ble' | 'file' | 'manual';

export type ParserType = 'json' | 'csv' | 'delimiter' | 'regex' | 'lua';

export type WidgetType = 'lineChart' | 'barChart' | 'gauge' | 'text' | 'led' | 'compass' | 'accelerometer';

export type TabType = 'dashboard' | 'console' | 'settings' | 'jsonEditor';

export interface DataPoint {
  timestamp: number;
  values: Record<string, number>;
}

export interface RawDataPoint {
  timestamp: number;
  data: number[];
  direction: 'TX' | 'RX';
}

export interface WidgetConfig {
  id: string;
  type: WidgetType;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  dataKey: string;
  min?: number;
  max?: number;
  unit?: string;
  color?: string;
}

export interface DashboardConfig {
  id: string;
  name: string;
  dataSource: {
    type: DataSourceType;
    deviceId?: string;
    filePath?: string;
  };
  parser: {
    type: ParserType;
    scriptName?: string;
    config: Record<string, unknown>;
  };
  widgets: WidgetConfig[];
  refreshRate: number;
}

export interface ParserScriptInfo {
  name: string;
  description: string;
  author: string;
  version: string;
  isBuiltIn: boolean;
  filePath: string;
}

export interface JsonFieldInfo {
  path: string;
  name: string;
  field_type: 'number' | 'string' | 'boolean' | 'object' | 'array';
  sample_value?: unknown;
  depth: number;
}

export interface JsonStructureInfo {
  fields: JsonFieldInfo[];
  isArray: boolean;
  arrayItemType?: string;
  sampleCount: number;
}

export interface FieldDefinition {
  key: string;
  path: string;
  unit?: string;
}

export interface FieldComparison {
  status: 'existing' | 'new';
  field: JsonFieldInfo;
  selected: boolean;
}

export interface DatasetConfig {
  index: number;
  title: string;
  units: string;
  widget: string;
  graph: boolean;
  min: number;
  max: number;
  color?: string;
  led: boolean;
  ledHigh: number;
  log: boolean;
  alarm: number;
  fft: boolean;
  fftSamples: number;
  fftSamplingRate: number;
  value: string;
}

export interface WidgetGroup {
  title: string;
  widget: string;
  datasets: DatasetConfig[];
}

export interface DashboardJsonConfig {
  title: string;
  decoder: number;
  frameDetection: number;
  frameStart: string;
  frameEnd: string;
  frameParser: string;
  groups: WidgetGroup[];
  mapTilerApiKey?: string;
  thunderforestApiKey?: string;
}

export interface WidgetSupportConfig {
  graph: boolean;
  min: boolean;
  max: boolean;
  unit: boolean;
  color: boolean;
  led: boolean;
  ledHigh: boolean;
  fft: boolean;
  alarm: boolean;
  log: boolean;
}

export const WIDGET_SUPPORT_MATRIX: Record<string, WidgetSupportConfig> = {
  lineChart: {
    graph: true,
    min: true,
    max: true,
    unit: true,
    color: true,
    led: false,
    ledHigh: false,
    fft: false,
    alarm: true,
    log: true,
  },
  gauge: {
    graph: false,
    min: true,
    max: true,
    unit: true,
    color: true,
    led: false,
    ledHigh: false,
    fft: false,
    alarm: true,
    log: true,
  },
  text: {
    graph: false,
    min: false,
    max: false,
    unit: true,
    color: false,
    led: false,
    ledHigh: false,
    fft: false,
    alarm: false,
    log: true,
  },
  led: {
    graph: false,
    min: false,
    max: false,
    unit: false,
    color: true,
    led: true,
    ledHigh: true,
    fft: false,
    alarm: true,
    log: true,
  },
  compass: {
    graph: false,
    min: false,
    max: false,
    unit: false,
    color: false,
    led: false,
    ledHigh: false,
    fft: false,
    alarm: false,
    log: true,
  },
  accelerometer: {
    graph: true,
    min: true,
    max: true,
    unit: true,
    color: true,
    led: false,
    ledHigh: false,
    fft: false,
    alarm: true,
    log: true,
  },
};

export interface SerialConfig {
  port: string;
  baudRate: number;
  dataBits: 5 | 6 | 7 | 8;
  stopBits: 1 | 2;
  parity: 'none' | 'odd' | 'even';
  flowControl: 'none' | 'hardware' | 'software';
}

export interface BleConfig {
  deviceId: string;
  deviceName: string;
  serviceUuid: string;
  characteristicUuid: string;
  enableNotify: boolean;
}

export const DEFAULT_SERIAL_CONFIG: SerialConfig = {
  port: '',
  baudRate: 115200,
  dataBits: 8,
  stopBits: 1,
  parity: 'none',
  flowControl: 'none',
};

export const DEFAULT_DATASET_CONFIG: DatasetConfig = {
  index: 0,
  title: '',
  units: '',
  widget: 'text',
  graph: false,
  min: 0,
  max: 100,
  led: false,
  ledHigh: 1,
  log: false,
  alarm: 0,
  fft: false,
  fftSamples: 1024,
  fftSamplingRate: 100,
  value: '--.--',
};

export const DEFAULT_JSON_CONFIG: DashboardJsonConfig = {
  title: 'New Dashboard',
  decoder: 0,
  frameDetection: 1,
  frameStart: '$',
  frameEnd: ';',
  frameParser: `function parse(frame) {
    return frame.split(',');
}`,
  groups: [],
};
