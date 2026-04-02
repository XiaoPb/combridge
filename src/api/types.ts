import type { SerialConfig, SerialPortInfo } from '../types';
import type { BleDeviceInfo, BleConnection } from '../types';

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

export interface BleWriteParams {
  deviceId: string;
  characteristicUuid: string;
  data: number[];
  withoutResponse?: boolean;
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
