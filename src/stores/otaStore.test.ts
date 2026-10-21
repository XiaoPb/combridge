import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useOtaStore } from './otaStore';
import type { OtaStatus } from '../types/ota';

vi.mock('../api/ota', () => ({
  otaApi: {
    listProtocols: vi.fn(),
    validatePackage: vi.fn(),
    preflight: vi.fn(),
    start: vi.fn(),
    getStatus: vi.fn(),
    cancel: vi.fn(),
  },
}));

const running: OtaStatus = {
  state: 'running',
  stage: 'transferring',
  protocol: 'smp',
  address: 'AA:BB',
  firmwarePath: 'C:/fw/firmware.zip',
  current: 100,
  total: 400,
  percent: 25,
  message: '正在上传固件',
  error: null,
  updatedAt: 1,
};

describe('otaStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useOtaStore.setState({
      status: {
        state: 'idle', stage: 'idle', protocol: null, address: null, firmwarePath: null,
        current: 0, total: 0, percent: 0, message: '', error: null, updatedAt: 0,
      },
      protocols: [],
      selectedProtocol: 'smp',
      deviceInfo: null,
      packageInfo: null,
      logs: [],
      error: null,
    });
  });

  it('更新进度并追加日志', () => {
    useOtaStore.getState().handleProgress(running);
    const state = useOtaStore.getState();
    expect(state.status.percent).toBe(25);
    expect(state.logs[state.logs.length - 1]?.message).toBe('传输固件 25%');
  });

  it('终态停止运行标记', () => {
    useOtaStore.getState().handleProgress({ ...running, state: 'success', stage: 'completed', percent: 100 });
    expect(useOtaStore.getState().status.state).toBe('success');
  });

  it('加载协议列表并保留默认协议', async () => {
    const { otaApi } = await import('../api/ota');
    vi.mocked(otaApi.listProtocols).mockResolvedValue([
      { id: 'smp', name: 'Nordic SMP', description: 'Zephyr MCUmgr over BLE + MCUboot' },
    ]);
    await useOtaStore.getState().loadProtocols();
    expect(useOtaStore.getState().protocols[0].id).toBe('smp');
    expect(useOtaStore.getState().selectedProtocol).toBe('smp');
  });

  it('校验 zip 后记录包信息并带上协议', async () => {
    const { otaApi } = await import('../api/ota');
    vi.mocked(otaApi.validatePackage).mockResolvedValue({
      path: 'C:/fw/dfu_application.zip',
      fileName: 'dfu_application.zip',
      packageSize: 374669,
      protocol: 'smp',
      imageName: 'app_chelsea_a.signed.bin',
      imageSize: 374140,
      imageSha256: 'a'.repeat(64),
      fields: [
        { key: 'name', value: 'app_chelsea_a' },
        { key: 'board', value: 'l_evk_t2_nrf54l15' },
        { key: 'soc', value: 'nrf54l15' },
        { key: 'version', value: '1.0.0+0' },
      ],
    });
    await useOtaStore.getState().validatePackage('C:/fw/dfu_application.zip');
    expect(useOtaStore.getState().packageInfo?.imageName).toBe('app_chelsea_a.signed.bin');
    expect(otaApi.validatePackage).toHaveBeenCalledWith('C:/fw/dfu_application.zip', 'smp');
  });

  it('忽略过期进度事件', () => {
    useOtaStore.getState().handleProgress({ ...running, updatedAt: 10 });
    useOtaStore.getState().handleProgress({ ...running, updatedAt: 5, percent: 5 });
    const state = useOtaStore.getState();
    expect(state.status.percent).toBe(25);
    expect(state.logs[state.logs.length - 1]?.message).toBe('传输固件 25%');
  });

  it('切换协议后重新校验原包', async () => {
    const { otaApi } = await import('../api/ota');
    vi.mocked(otaApi.validatePackage).mockResolvedValue({
      path: 'C:/fw/a.zip',
      fileName: 'a.zip',
      packageSize: 1,
      protocol: 'smp2',
      imageName: 'a.bin',
      imageSize: 1,
      imageSha256: 'a'.repeat(64),
      fields: [],
    });
    useOtaStore.setState({
      packageInfo: {
        path: 'C:/fw/a.zip',
        fileName: 'a.zip',
        packageSize: 1,
        protocol: 'smp',
        imageName: 'a.bin',
        imageSize: 1,
        imageSha256: 'a'.repeat(64),
        fields: [],
      },
      selectedProtocol: 'smp',
    });
    await useOtaStore.getState().setProtocol('smp2');
    expect(otaApi.validatePackage).toHaveBeenCalledWith('C:/fw/a.zip', 'smp2');
    expect(useOtaStore.getState().selectedProtocol).toBe('smp2');
  });
});
