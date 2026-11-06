import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/tauri', () => ({ invoke: vi.fn() }));

import {
  DEFAULT_COMMAND_CONFIG,
  normalizePersisted,
  useGh3220CommandConfig,
} from './gh3220CommandConfig';
import { GH3220_COMMAND_CATALOG } from '../constants/gh3220CommandSchema';
import type { Gh3220DeviceInfo, Gh3220Preferences, Gh3220Status } from '../types/gh3220';

const status: Gh3220Status = {
  state: 'capturing',
  link: 'ble',
  address: 'AA:BB',
  name: 'GH3220_Nordic',
  error: null,
  capturing: true,
  chunkSize: 244,
};

const deviceInfo = (openedFunc: number): Gh3220DeviceInfo => ({
  fwVersion: 'FW',
  protocolVersion: 'P',
  chipVersion: 'C',
  driverVersion: 'D',
  maxLen: 238,
  openedFunc,
  linkOk: true,
});

const prefs: Gh3220Preferences = {
  csv: { enabled: false, outputDir: 'C:/csv', idleCloseSecs: 60 },
  capture: { channelsOverride: 4, autoStop: true },
  debugTcp: { enabled: false, address: 'localhost:9000' },
};

describe('GH3220 命令配置集中状态（对齐 rpcConfig）', () => {
  beforeEach(() => {
    useGh3220CommandConfig.getState().reset();
  });

  it('真机 openedFunc=0x2 反填「心率 HR」，0x42 反填两项', () => {
    const store = useGh3220CommandConfig.getState();
    store.syncFromDevice(deviceInfo(0x2), null, status);
    expect(useGh3220CommandConfig.getState().selectedFunctions).toEqual(['hr']);
    useGh3220CommandConfig.getState().syncFromDevice(deviceInfo(0x42), null, status);
    expect(useGh3220CommandConfig.getState().selectedFunctions).toEqual(['hr', 'spo2']);
  });

  it('用户手动改过选择后，刷新真机不覆盖', () => {
    useGh3220CommandConfig.getState().setSelectedFunctions(['ecg']);
    useGh3220CommandConfig.getState().syncFromDevice(deviceInfo(0x2), null, status);
    expect(useGh3220CommandConfig.getState().selectedFunctions).toEqual(['ecg']);
  });

  it('功能选择同步到目录里全部 5 条 functionMask 行', () => {
    useGh3220CommandConfig.getState().setSelectedFunctions(['adt', 'hr']);
    const { rowValues } = useGh3220CommandConfig.getState();
    const maskRows = GH3220_COMMAND_CATALOG.filter((def) =>
      def.params.some((param) => param.kind === 'functionMask')
    );
    expect(maskRows).toHaveLength(5);
    for (const def of maskRows) {
      expect(rowValues[def.key].funcMask).toEqual(['adt', 'hr']);
    }
  });

  it('偏好与链路信息回填，isRunning 不跨 reset 保留', () => {
    useGh3220CommandConfig.getState().syncFromDevice(null, prefs, status);
    const state = useGh3220CommandConfig.getState();
    expect(state.csvEnabled).toBe('false');
    expect(state.idleCloseSecs).toBe('60');
    expect(state.channelsOverride).toBe('4');
    expect(state.autoStop).toBe('true');
    expect(state.tcpAddress).toBe('localhost:9000');
    expect(state.mtu).toBe('244');
    state.setConfig({ isRunning: true });
    expect(useGh3220CommandConfig.getState().isRunning).toBe(true);
    useGh3220CommandConfig.getState().reset();
    expect(useGh3220CommandConfig.getState().isRunning).toBe(false);
    expect(useGh3220CommandConfig.getState().selectedFunctions).toEqual(
      DEFAULT_COMMAND_CONFIG.selectedFunctions
    );
  });

  it('旧持久化 funcMaskText 归一化为 selectedFunctions', () => {
    expect(normalizePersisted({ funcMaskText: 'hr,spo2' }).selectedFunctions).toEqual([
      'hr',
      'spo2',
    ]);
    expect(normalizePersisted({ funcMaskText: '0x2' }).selectedFunctions).toEqual(['hr']);
    expect(normalizePersisted({ selectedFunctions: ['ecg'] }).selectedFunctions).toEqual(['ecg']);
  });
});
