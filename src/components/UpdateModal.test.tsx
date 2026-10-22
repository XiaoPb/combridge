// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  openUrl: vi.fn(async () => undefined),
  download: vi.fn(async () => ({ path: 'C:/tmp/a.exe', name: 'a.exe' })),
  openInstaller: vi.fn(async () => undefined),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => undefined),
}));

vi.mock('../api/tauri', () => ({
  updateApi: {
    download: mocks.download,
    openInstaller: mocks.openInstaller,
  },
  systemApi: {
    openUrl: mocks.openUrl,
  },
}));

import UpdateModal from './UpdateModal';
import type { UpdateInfo } from '../api/tauri';

const update: UpdateInfo = {
  current_version: '0.6.0',
  latest_version: '0.7.0',
  release_name: 'ComBridge v0.7.0',
  published_at: '2026-10-22T00:00:00Z',
  body: [
    '## 版本信息',
    '',
    '| 项目 | 值 |',
    '| --- | --- |',
    '| 版本 | `v0.7.0` |',
    '',
    '### 新功能',
    '',
    '- 支持 OTA 升级',
    '',
    '[对比](https://github.com/XiaoPb/combridge/compare/v0.6.0...v0.7.0)',
  ].join('\n'),
  asset: {
    name: 'ComBridge_0.7.0_x64-setup.exe',
    original_url: 'https://example.com/ComBridge_0.7.0_x64-setup.exe',
    proxy_urls: [],
    size: 1024,
  },
};

describe('UpdateModal', () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders release notes as markdown including tables and lists', () => {
    render(<UpdateModal update={update} open onClose={() => undefined} />);

    expect(screen.getByRole('heading', { name: '版本信息', level: 2 })).toBeTruthy();
    expect(screen.getByRole('heading', { name: '新功能', level: 3 })).toBeTruthy();
    expect(screen.getByRole('table')).toBeTruthy();
    expect(screen.getByText('v0.7.0')).toBeTruthy();
    expect(screen.getByText('支持 OTA 升级')).toBeTruthy();
  });

  it('opens links with the system browser instead of navigating the webview', async () => {
    render(<UpdateModal update={update} open onClose={() => undefined} />);

    fireEvent.click(screen.getByRole('link', { name: '对比' }));

    await waitFor(() => {
      expect(mocks.openUrl).toHaveBeenCalledWith(
        'https://github.com/XiaoPb/combridge/compare/v0.6.0...v0.7.0',
      );
    });
  });
});
