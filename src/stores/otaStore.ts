import { create } from 'zustand';
import { otaApi } from '../api/ota';
import type {
  OtaDeviceInfo, OtaPackageInfo, OtaProtocolDescriptor, OtaStage, OtaStartOptions, OtaStatus,
} from '../types/ota';

export interface OtaLogEntry {
  id: number;
  time: string;
  level: 'info' | 'error';
  message: string;
}

const IDLE_STATUS: OtaStatus = {
  state: 'idle', stage: 'idle', protocol: null, address: null, firmwarePath: null,
  current: 0, total: 0, percent: 0, message: '', error: null, updatedAt: 0,
};

const STAGE_LABEL: Record<OtaStage, string> = {
  idle: '待机', unpacking: '解包固件', validating: '校验固件', preparing: '准备通道',
  transferring: '传输固件', activating: '激活固件', rebooting: '重启设备',
  completed: '升级完成', failed: '升级失败',
};

let logId = 0;

function appendLog(logs: OtaLogEntry[], level: 'info' | 'error', message: string): OtaLogEntry[] {
  return [...logs, { id: ++logId, time: new Date().toLocaleTimeString(), level, message }].slice(-500);
}

interface OtaStoreState {
  status: OtaStatus;
  protocols: OtaProtocolDescriptor[];
  selectedProtocol: string;
  deviceInfo: OtaDeviceInfo | null;
  packageInfo: OtaPackageInfo | null;
  logs: OtaLogEntry[];
  preflighting: boolean;
  validating: boolean;
  error: string | null;
  loadProtocols: () => Promise<void>;
  setProtocol: (id: string) => Promise<void>;
  validatePackage: (path: string) => Promise<void>;
  clearPackage: () => void;
  preflight: (address: string) => Promise<void>;
  start: (address: string, options: OtaStartOptions) => Promise<void>;
  cancel: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  handleProgress: (status: OtaStatus) => void;
  clearLogs: () => void;
}

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const useOtaStore = create<OtaStoreState>((set, get) => ({
  status: IDLE_STATUS,
  protocols: [],
  selectedProtocol: 'smp',
  deviceInfo: null,
  packageInfo: null,
  logs: [],
  preflighting: false,
  validating: false,
  error: null,

  loadProtocols: async () => {
    try {
      const protocols = await otaApi.listProtocols();
      const current = get().selectedProtocol;
      const selectedProtocol = protocols.some(item => item.id === current)
        ? current
        : (protocols[0]?.id ?? 'smp');
      set({ protocols, selectedProtocol });
    } catch (error) {
      console.error('OTA 协议列表加载失败', error);
      set(state => ({
        error: toMessage(error),
        logs: appendLog(state.logs, 'error', `协议列表加载失败：${toMessage(error)}`),
      }));
    }
  },

  setProtocol: async (id) => {
    if (get().selectedProtocol === id) {
      return;
    }
    const previousPath = get().packageInfo?.path;
    set({ selectedProtocol: id, packageInfo: null, error: null });
    if (previousPath) {
      await get().validatePackage(previousPath);
    }
  },

  validatePackage: async (path) => {
    set({ validating: true, packageInfo: null, error: null });
    try {
      const packageInfo = await otaApi.validatePackage(path, get().selectedProtocol);
      set(state => ({
        packageInfo,
        logs: appendLog(
          state.logs,
          'info',
          `固件包校验通过：${packageInfo.imageName}（${packageInfo.imageSize} 字节，协议 ${packageInfo.protocol}）`,
        ),
      }));
    } catch (error) {
      console.error('OTA 固件包校验失败', error);
      set(state => ({
        error: toMessage(error),
        logs: appendLog(state.logs, 'error', `固件包校验失败：${toMessage(error)}`),
      }));
    } finally {
      set({ validating: false });
    }
  },

  clearPackage: () => set({ packageInfo: null, error: null }),

  preflight: async (address) => {
    set({ preflighting: true });
    try {
      const deviceInfo = await otaApi.preflight(address, get().selectedProtocol);
      set(state => ({
        deviceInfo,
        logs: appendLog(state.logs, 'info', `预检成功：${deviceInfo.version ?? deviceInfo.name ?? address}`),
      }));
    } catch (error) {
      console.error('OTA 预检失败', error);
      set(state => ({
        error: toMessage(error),
        logs: appendLog(state.logs, 'error', `预检失败：${toMessage(error)}`),
      }));
    } finally {
      set({ preflighting: false });
    }
  },

  start: async (address, options) => {
    const { packageInfo } = get();
    if (!packageInfo) {
      set({ error: '请先选择并校验 .zip 固件包' });
      return;
    }
    try {
      const status = await otaApi.start(address, packageInfo.path, get().selectedProtocol, options);
      set(state => ({
        status,
        error: null,
        logs: appendLog(
          state.logs,
          'info',
          `开始升级：${packageInfo.fileName}（协议 ${get().selectedProtocol}）`,
        ),
      }));
    } catch (error) {
      console.error('OTA 升级启动失败', error);
      set(state => ({
        error: toMessage(error),
        logs: appendLog(state.logs, 'error', `升级启动失败：${toMessage(error)}`),
      }));
    }
  },

  cancel: async () => {
    try {
      await otaApi.cancel();
    } catch (error) {
      console.error('OTA 取消失败', error);
    }
  },

  refreshStatus: async () => {
    try {
      const status = await otaApi.getStatus();
      set({ status });
    } catch (error) {
      console.error('OTA 状态刷新失败', error);
    }
  },

  handleProgress: (status) => {
    set(state => {
      const changed = state.status.stage !== status.stage || state.status.message !== status.message;
      if (!changed) {
        return { status };
      }
      const suffix = status.total > 0 ? ` ${Math.round(status.percent)}%` : '';
      return {
        status,
        logs: appendLog(
          state.logs,
          status.state === 'failed' ? 'error' : 'info',
          `${STAGE_LABEL[status.stage]}${suffix}`,
        ),
      };
    });
  },

  clearLogs: () => set({ logs: [] }),
}));
