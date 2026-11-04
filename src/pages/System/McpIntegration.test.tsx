// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { McpStatus } from '../../api/tauri';

const mocks = vi.hoisted(() => ({
  getStatus: vi.fn(),
  applySettings: vi.fn(),
  rotateToken: vi.fn(),
  showInFolder: vi.fn(async () => undefined),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('../../api/tauri', () => ({
  mcpApi: {
    getStatus: mocks.getStatus,
    applySettings: mocks.applySettings,
    rotateToken: mocks.rotateToken,
  },
  systemApi: { showInFolder: mocks.showInFolder },
}));

import McpIntegration from './McpIntegration';

const TOKEN = 'aaaa1111bbbb2222cccc';

/** 每次返回全新对象：测试之间串改字段是这里最容易踩的坑 */
const baseStatus = (): McpStatus => ({
  enabled: true,
  running: true,
  tier: 'dev',
  configTier: 'dev',
  configPort: 47821,
  port: 47821,
  url: 'http://127.0.0.1:47821/mcp',
  token: TOKEN,
  startedAt: 1_700_000_000_000,
  portFallback: false,
  toolCount: 16,
  configPath: 'C:/data/mcp.json',
  dataDir: 'C:/data',
  auditDir: 'C:/log',
  auditFile: 'C:/log/mcp-audit-2026-11-04.jsonl',
  auditRecords: 42,
  lastError: null,
});

/** 等首屏状态落地（getStatus 已被调用；具体断言各自 waitFor） */
const flush = () => waitFor(() => expect(mocks.getStatus).toHaveBeenCalled());

const findSnippet = () =>
  Array.from(document.querySelectorAll('textarea')).find((el) =>
    el.value.includes('claude mcp add')
  );

beforeEach(() => {
  mocks.getStatus.mockResolvedValue(baseStatus());
  mocks.applySettings.mockImplementation(async (enabled: boolean, port: number, tier: string) => ({
    ...baseStatus(),
    enabled,
    running: enabled,
    port: enabled ? port : 0,
    tier,
    configTier: tier,
  }));
  mocks.rotateToken.mockResolvedValue({ ...baseStatus(), token: 'newtoken' });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('MCP integration panel', () => {
  it('shows running state and masks the token by default', async () => {
    render(<McpIntegration />);
    await flush();

    expect(screen.getByText('mcp.status.running')).toBeTruthy();
    // 打码后只留头尾各 4 位；完整 token 不该出现在页面上
    expect(screen.getByDisplayValue('aaaa••••••••cccc')).toBeTruthy();
    expect(screen.queryByDisplayValue(TOKEN)).toBeNull();
  });

  it('applies the off switch immediately without a restart', async () => {
    render(<McpIntegration />);
    await flush();

    fireEvent.click(screen.getByRole('switch'));

    await waitFor(() => expect(mocks.applySettings).toHaveBeenCalledTimes(1));
    expect(mocks.applySettings).toHaveBeenCalledWith(false, 47821, 'dev');
  });

  it('surfaces a start failure instead of pretending it is running', async () => {
    mocks.getStatus.mockResolvedValue({
      ...baseStatus(),
      running: false,
      port: 0,
      token: '',
      lastError: '端口 47821 绑定失败',
    });

    render(<McpIntegration />);
    await flush();

    expect(screen.getByText('mcp.status.failed')).toBeTruthy();
    expect(screen.getByText('端口 47821 绑定失败')).toBeTruthy();
  });

  it('offers a copy-pasteable CLI snippet and rotates the token on demand', async () => {
    render(<McpIntegration />);
    await flush();

    // antd 的 autoSize TextArea 会额外渲染一个测量用的隐藏 textarea，
    // 所以按内容找，别按 display value 找
    await waitFor(() => expect(findSnippet()).toBeTruthy());
    expect(findSnippet()!.value).toContain(`Authorization: Bearer ${TOKEN}`);

    fireEvent.click(screen.getByText('mcp.button.rotate'));
    await waitFor(() => expect(mocks.rotateToken).toHaveBeenCalledTimes(1));
  });

  it('opens the audit folder rather than a not-yet-created file', async () => {
    render(<McpIntegration />);
    await flush();

    fireEvent.click(screen.getByText('mcp.button.openAudit'));
    await waitFor(() => expect(mocks.showInFolder).toHaveBeenCalledWith('C:/log'));
  });
});
