// @vitest-environment happy-dom

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  otaState: {
    status: {
      state: 'idle',
      stage: 'idle',
      protocol: null,
      address: null,
      firmwarePath: null,
      current: 0,
      total: 0,
      percent: 0,
      message: '空闲',
      error: null,
      updatedAt: 0,
    },
    protocols: [],
    selectedProtocol: 'smp',
    deviceInfo: null,
    packageInfo: null,
    preflighting: false,
    validating: false,
    error: null,
    loadProtocols: vi.fn(async () => undefined),
    setProtocol: vi.fn(async () => undefined),
    validatePackage: vi.fn(async () => undefined),
    clearPackage: vi.fn(),
    preflight: vi.fn(async () => undefined),
    start: vi.fn(async () => undefined),
    cancel: vi.fn(async () => undefined),
    refreshStatus: vi.fn(async () => undefined),
  } as Record<string, unknown>,
  bleConnections: [] as unknown[],
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('../../stores/otaStore', () => ({
  useOtaStore: (selector?: (state: Record<string, unknown>) => unknown) =>
    selector ? selector(mocks.otaState) : mocks.otaState,
}));

vi.mock('../../stores/bleStore', () => ({
  useBleStore: (selector: (state: { connections: unknown[] }) => unknown) =>
    selector({ connections: mocks.bleConnections }),
}));

import { UpgradeTab } from './UpgradeTab';

describe('UpgradeTab', () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.bleConnections.length = 0;
    mocks.otaState.packageInfo = null;
  });

  it('shows a warning and disables start when no BLE device is connected', () => {
    render(<UpgradeTab />);

    expect(screen.getByText('device.none')).toBeTruthy();
    expect(screen.getByRole('button', { name: /actions\.start/ })).toHaveProperty('disabled', true);
  });

  it('loads protocols and refreshes status on mount', () => {
    render(<UpgradeTab />);

    expect(mocks.otaState.loadProtocols).toHaveBeenCalledTimes(1);
    expect(mocks.otaState.refreshStatus).toHaveBeenCalledTimes(1);
  });

  it('renders validated firmware package details', () => {
    mocks.bleConnections.push({
      address: 'AA:BB:CC:DD:EE:FF',
      name: 'Demo Sensor',
      isConnected: true,
      services: [],
    });
    mocks.otaState.packageInfo = {
      path: 'C:/firmware/demo.zip',
      fileName: 'demo.zip',
      packageSize: 1024,
      protocol: 'smp',
      imageName: 'demo.bin',
      imageSize: 2048,
      imageSha256: '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      fields: [{ key: 'version', value: '1.2.0' }],
    };

    render(<UpgradeTab />);

    expect(screen.getByText(/demo\.zip（1\.0 KB）/)).toBeTruthy();
    expect(screen.getByText(/demo\.bin（2\.0 KB）/)).toBeTruthy();
    expect(screen.getByText('1.2.0')).toBeTruthy();
    expect(screen.getByText('firmware.valid')).toBeTruthy();
  });
});
