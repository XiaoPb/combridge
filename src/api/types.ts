import type { SerialConfig, SerialPortInfo } from '../types';
import type { BleDeviceInfo, BleConnection, BleService, BleCharacteristic } from '../types';

export interface InvokeResult {
  success: boolean;
  error?: string;
}

export interface SerialListPortsResult {
  ports: SerialPortInfo[];
}

export interface SerialOpenParams {
  portName: string;
  config: SerialConfig;
}

export interface SerialWriteParams {
  portName: string;
  data: number[];
}

export interface BleScanResult {
  devices: BleDeviceInfo[];
}

export interface BleConnectResult {
  success: boolean;
  device?: BleConnection;
  error?: string;
}

export interface BleConfigureParams {
  mode: 'native' | 'at';
  serialPort?: string;
}

export interface BleConnectParams {
  address: string;
  timeout?: number;
}

export interface BleDiscoverServicesParams {
  deviceId: string;
}

export interface BleDiscoverServicesResult {
  services: BleService[];
}

export interface BleDiscoverCharacteristicsParams {
  deviceId: string;
  serviceUuid: string;
}

export interface BleDiscoverCharacteristicsResult {
  characteristics: BleCharacteristic[];
}

export interface BleReadParams {
  deviceId: string;
  characteristicUuid: string;
}

export interface BleWriteParams {
  deviceId: string;
  characteristicUuid: string;
  data: number[];
  withoutResponse?: boolean;
}

export interface BleSubscribeParams {
  deviceId: string;
  characteristicUuid: string;
}

export interface ApiError {
  code: string;
  message: string;
  details?: unknown;
}

export function isApiError(error: unknown): error is ApiError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message' in error
  );
}

export function createApiError(code: string, message: string, details?: unknown): ApiError {
  return { code, message, details };
}

export type PluginState = 'Unloaded' | 'Loaded' | 'Enabled' | 'Disabled' | 'Error';

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string | null;
  author: string | null;
  path: string;
  state: PluginState;
  hooks: string[];
  bound_devices: string[];
  error_message: string | null;
}

export interface ProtocolLoadParams {
  plugin_id: string;
  path: string;
}

export interface ProtocolBindParams {
  plugin_id: string;
  device_id: string;
}

export interface CacheEntry {
  timestamp: number;
  data: number[];
  direction: 'tx' | 'rx';
}

export interface CacheData {
  tx: CacheEntry[];
  rx: CacheEntry[];
}

export interface Gh3036ChannelConfig {
  channel_type: 'Serial' | 'Ble';
  device_id: string;
  characteristic_uuid: string | null;
}

export interface Gh3036CsvConfig {
  enabled: boolean;
  output_dir: string;
}

export interface Gh3036FrameData {
  function_id: number;
  function_name: string;
  frame_id: number;
  timestamp: number;
  gs_data: number[];
  rawdata: number[];
  flags: number[];
  algo_data: number[];
  agc_info: number[];
  phy_value: number[];
}

export interface Gh3036RpcParam {
  name: string;
  param_type: string;
  description: string;
  default_value: string | null;
}

export interface Gh3036RpcCommand {
  key: string;
  name: string;
  description: string;
  params: Gh3036RpcParam[];
}

export interface Gh3036VersionTypeConfig {
  type_value: number;
  name: string;
  description: string;
}

export interface Gh3036ConfigRegisterPreview {
  addr: string;
  value: string;
}

export interface Gh3036ConfigPreview {
  filePath: string;
  registerCount: number;
  registers: Gh3036ConfigRegisterPreview[];
}

export type FactoryTestStep = 
  | 'idle' 
  | 'prepare' 
  | 'chip_init' 
  | 'uuid' 
  | 'base_noise' 
  | 'ppg_noise' 
  | 'lpctr' 
  | 'environment_switch' 
  | 'lplctr' 
  | 'cleanup' 
  | 'completed';

export type FactoryTestStatus = 
  | 'idle' 
  | 'running' 
  | 'waiting_for_environment_switch' 
  | 'completed' 
  | 'failed' 
  | 'stopped';

export type FactoryComputeMode = 'mcu' | 'app';

export interface ChannelMeasurement {
  computed_value: number | null;
  device_value: number | null;
}

export interface ComputeConfig {
  sample_rate_hz?: number;
  min_number?: number;
  skip_number?: number;
  is_continuous?: boolean;
  timeout_ms?: number;
  gain_k?: number;
  led_current_ma?: number;
}

export interface FactoryTestStepResult {
  step: FactoryTestStep;
  success: boolean;
  message: string;
  data: (number | null)[];
  timestamp: number;
}

export interface FactoryTestResult {
  chip_init_status: number;
  uuid: number[];
  compute_mode: FactoryComputeMode;
  base_noise: ChannelMeasurement[];
  ppg_noise: ChannelMeasurement[];
  lpctr: ChannelMeasurement[];
  lplctr: ChannelMeasurement[];
  overall_result: string;
  timestamp: number;
}

export interface ConfigValidationResult {
  base_noise_config: string | null;
  ppg_noise_config: string | null;
  lpctr_config: string | null;
  lplctr_config: string | null;
  errors: string[];
  is_valid: boolean;
}

export interface FactoryTestProgressEvent {
  current_step: FactoryTestStep;
  status: FactoryTestStatus;
  step_result: FactoryTestStepResult | null;
  progress: number;
  message: string;
}

export type ThresholdOperator = 'lt' | 'le' | 'gt' | 'ge' | 'eq' | 'ne' | 'range';

export interface ThresholdConfig {
  operator: ThresholdOperator;
  value?: number;
  range?: [number, number];
  description?: string;
}

export interface ChannelRule {
  channels: number[];
  operator: ThresholdOperator;
  value?: number;
  range?: [number, number];
  description?: string;
}

export interface TestItemConfig {
  enabled: boolean;
  description?: string;
  unit?: string;
  mode?: number;
  channels?: number;
  compute?: ComputeConfig;
  global_threshold?: ThresholdConfig;
  channel_rules?: ChannelRule[];
}

export interface TestsConfig {
  chip_init?: TestItemConfig;
  chip_uid?: TestItemConfig;
  base_noise?: TestItemConfig;
  ppg_noise?: TestItemConfig;
  lpctr?: TestItemConfig;
  lplctr?: TestItemConfig;
}

export interface GlobalConfig {
  default_operator: ThresholdOperator;
  fail_action: 'stop' | 'continue';
}

export interface FactoryThresholdConfig {
  project: string;
  version: string;
  description?: string;
  chip?: string;
  global?: GlobalConfig;
  tests: TestsConfig;
}

export interface ChannelEvaluationResult {
  channel_index: number;
  value: number | null;
  pass: boolean;
  threshold_display: string;
  operator: string;
  threshold_value?: number;
  threshold_range?: [number, number];
  description?: string;
}

export interface TestEvaluationResult {
  test_name: string;
  enabled: boolean;
  pass: boolean;
  channel_results: ChannelEvaluationResult[];
  message: string;
  description?: string;
  unit?: string;
}

export interface FactoryEvaluationResult {
  overall_pass: boolean;
  project: string;
  test_results: TestEvaluationResult[];
  timestamp: number;
}

export interface TestStatus {
  enabled: boolean;
  has_global_threshold: boolean;
  channel_rules_count: number;
}

export interface TestsStatus {
  base_noise: TestStatus;
  ppg_noise: TestStatus;
  lpctr: TestStatus;
  lplctr: TestStatus;
}

export interface ThresholdConfigValidation {
  is_valid: boolean;
  file_path?: string;
  project?: string;
  version?: string;
  errors: string[];
  warnings: string[];
  tests_status: TestsStatus;
}

export interface ThresholdYamlFileLoadResult {
  file_path: string;
  config: FactoryThresholdConfig;
  validation: ThresholdConfigValidation;
}
