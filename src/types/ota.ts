export type OtaState = 'idle' | 'running' | 'success' | 'failed' | 'cancelled';

export type OtaStage =
  | 'idle' | 'unpacking' | 'validating' | 'preparing' | 'transferring'
  | 'activating' | 'rebooting' | 'completed' | 'failed';

export interface OtaKeyValue { key: string; value: string; }

export interface OtaProtocolDescriptor { id: string; name: string; description: string; }

export interface OtaStatus {
  state: OtaState;
  stage: OtaStage;
  protocol: string | null;
  address: string | null;
  firmwarePath: string | null;
  current: number;
  total: number;
  percent: number;
  message: string;
  error: string | null;
  updatedAt: number;
}

export interface OtaPackageInfo {
  path: string;
  fileName: string;
  packageSize: number;
  protocol: string;
  imageName: string;
  imageSize: number;
  imageSha256: string;
  fields: OtaKeyValue[];
}

export interface OtaDeviceInfo {
  address: string;
  protocol: string;
  name: string | null;
  mode: string | null;
  mtu: number | null;
  version: string | null;
  fields: OtaKeyValue[];
}

export interface OtaStartOptions { forceConfirm: boolean; upgradeOnly: boolean; }
