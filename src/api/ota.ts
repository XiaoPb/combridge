import { invoke } from '@tauri-apps/api/core';
import type {
  OtaDeviceInfo, OtaPackageInfo, OtaProtocolDescriptor, OtaStartOptions, OtaStatus,
} from '../types/ota';

export const otaApi = {
  listProtocols: () => invoke<OtaProtocolDescriptor[]>('ota_list_protocols'),
  validatePackage: (firmwarePath: string, protocol?: string) =>
    invoke<OtaPackageInfo>('ota_validate_package', { firmwarePath, protocol }),
  preflight: (address: string, protocol?: string) =>
    invoke<OtaDeviceInfo>('ota_preflight', { address, protocol }),
  start: (address: string, firmwarePath: string, protocol: string | undefined, options: OtaStartOptions) =>
    invoke<OtaStatus>('ota_start', { address, firmwarePath, protocol, options }),
  getStatus: () => invoke<OtaStatus>('ota_get_status'),
  cancel: () => invoke<void>('ota_cancel'),
};
