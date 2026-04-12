export type DataSourceType = 'serial' | 'ble' | 'file' | 'manual';

export type ParserType = 'json' | 'csv' | 'delimiter' | 'regex' | 'lua';

export type WidgetType = 'lineChart' | 'barChart' | 'gauge' | 'text' | 'led' | 'compass' | 'accelerometer';

export interface DataPoint {
  timestamp: number;
  values: Record<string, number>;
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
