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

export interface FactoryTestStepResult {
  step: FactoryTestStep;
  success: boolean;
  message: string;
  data: number[];
  timestamp: number;
}

export interface FactoryTestResult {
  chip_init_status: number;
  uuid: number[];
  base_noise: number[];
  ppg_noise: number[];
  lpctr: number[];
  lplctr: number[];
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
