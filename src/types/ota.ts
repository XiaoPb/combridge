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
  /** Goodix 双区写入地址（按加载地址 + 4K 对齐的镜像大小计算）；其他协议不存在。 */
  dualBankAddress?: string;
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

export interface OtaStartOptions {
  forceConfirm: boolean;
  upgradeOnly: boolean;
  /** Goodix 路由：auto / normal / fastService；SMP 忽略。 */
  route?: 'auto' | 'normal' | 'fastService';
  /** Goodix 传输模式：auto / normal / fast；SMP 忽略。 */
  transferMode?: 'auto' | 'normal' | 'fast';
  /** Goodix Flash 模式：dual（默认，后台拷贝）/ single（原地升级）；SMP 忽略。 */
  bankMode?: 'dual' | 'single';
  /** Goodix 显式写入地址（十六进制）；仅双区模式可配置，为空时自动计算。 */
  targetAddress?: string;
  /** Goodix 完成后是否立即复位运行。 */
  runImmediately?: boolean;
}
