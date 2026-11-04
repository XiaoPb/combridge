// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { McpStatus } from '../../api/tauri';

const mocks = vi.hoisted(() => ({
  getStatus: vi.fn(),
  applySettings: vi.fn(),
  rotateToken: vi.fn(),
  wireClient: vi.fn(),
  setAutoWire: vi.fn(),
  revokeGrants: vi.fn(async () => 0),
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
    wireClient: mocks.wireClient,
    setAutoWire: mocks.setAutoWire,
    revokeGrants: mocks.revokeGrants,
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
  autoWire: true,
  clientConfigPath: 'C:/Users/me/.workbuddy/mcp.json',
  clientWireState: 'wired',
  clientWireDetail: null,
  confirmGrants: [],
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
  mocks.wireClient.mockResolvedValue(baseStatus());
  mocks.setAutoWire.mockImplementation(async (enabled: boolean) => ({
    ...baseStatus(),
    autoWire: enabled,
  }));
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

    // 页面现在有两个开关（服务、自动接入），第一个是服务总开关
    fireEvent.click(screen.getAllByRole('switch')[0]);

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

  it('reports the auto-wire state and rewrites the client config on demand', async () => {
    render(<McpIntegration />);
    await flush();

    expect(screen.getByText('mcp.autoWire.wired')).toBeTruthy();
    expect(screen.getByText('C:/Users/me/.workbuddy/mcp.json')).toBeTruthy();

    fireEvent.click(screen.getByText('mcp.autoWire.rewrite'));
    await waitFor(() => expect(mocks.wireClient).toHaveBeenCalledTimes(1));
  });

  it('turning auto-wire off just stops maintaining the entry', async () => {
    render(<McpIntegration />);
    await flush();

    // 第二个开关是自动接入
    fireEvent.click(screen.getAllByRole('switch')[1]);
    await waitFor(() => expect(mocks.setAutoWire).toHaveBeenCalledWith(false));
  });

  it('explains how to configure by hand when no client is detected', async () => {
    mocks.getStatus.mockResolvedValue({
      ...baseStatus(),
      clientWireState: 'noClient',
      clientConfigPath: '',
    });

    render(<McpIntegration />);
    await flush();

    expect(screen.getByText('mcp.autoWire.noClient')).toBeTruthy();
    expect(screen.getByText('mcp.autoWire.noClientHint')).toBeTruthy();
  });

  it('shows why the written config is stale', async () => {
    mocks.getStatus.mockResolvedValue({
      ...baseStatus(),
      clientWireState: 'stale',
      clientWireDetail: 'token 与当前运行的不一致',
    });

    render(<McpIntegration />);
    await flush();

    expect(screen.getByText('mcp.autoWire.stale')).toBeTruthy();
    expect(screen.getByText('token 与当前运行的不一致')).toBeTruthy();
  });

  it('opens the audit folder rather than a not-yet-created file', async () => {
    render(<McpIntegration />);
    await flush();

    fireEvent.click(screen.getByText('mcp.button.openAudit'));
    await waitFor(() => expect(mocks.showInFolder).toHaveBeenCalledWith('C:/log'));
  });

  it('lists temporary allowances so the user can see what was waved through', async () => {
    mocks.getStatus.mockResolvedValue({
      ...baseStatus(),
      confirmGrants: [
        {
          tool: 'combridge_serial_write',
          scope: 'COM3',
          scopeLabel: 'COM3',
          risk: '串口写入',
          grantedAt: Date.now(),
          expiresAt: Date.now() + 1_500_000,
          remainingSecs: 1_500,
        },
      ],
    });

    render(<McpIntegration />);
    await flush();

    expect(screen.getByText(/串口写入 @ COM3/)).toBeTruthy();
    expect(screen.getByText('mcp.grant.revoke')).toBeTruthy();
  });

  it('shows an empty state instead of an unexplained blank when nothing is granted', async () => {
    render(<McpIntegration />);
    await flush();

    expect(screen.getByText('mcp.grant.empty')).toBeTruthy();
    expect(screen.queryByText('mcp.grant.revoke')).toBeNull();
  });

  it('revokes allowances and refreshes the panel', async () => {
    mocks.getStatus
      .mockResolvedValueOnce({
        ...baseStatus(),
        confirmGrants: [
          {
            tool: 'combridge_serial_write',
            scope: 'COM3',
            scopeLabel: 'COM3',
            risk: '串口写入',
            grantedAt: Date.now(),
            expiresAt: Date.now() + 1_500_000,
            remainingSecs: 1_500,
          },
        ],
      })
      // 撤销后重新拉状态时已经没有放行了
      .mockResolvedValue({ ...baseStatus(), confirmGrants: [] });
    mocks.revokeGrants.mockResolvedValueOnce(1);

    render(<McpIntegration />);
    await flush();

    fireEvent.click(screen.getByText('mcp.grant.revoke'));
    await waitFor(() => expect(mocks.revokeGrants).toHaveBeenCalledTimes(1));
    // 撤销后必须重新拉状态，否则界面还挂着已经失效的标签
    await waitFor(() => expect(screen.getByText('mcp.grant.empty')).toBeTruthy());
  });
});
