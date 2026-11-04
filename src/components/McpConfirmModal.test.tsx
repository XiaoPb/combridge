// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => {
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  return {
    handlers,
    listConfirms: vi.fn(async () => [] as unknown[]),
    resolveConfirm: vi.fn(async () => true),
    revokeGrants: vi.fn(async () => 0),
  };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) =>
      options ? `${key}:${JSON.stringify(options)}` : key,
  }),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, cb: (event: { payload: unknown }) => void) => {
    mocks.handlers.set(name, cb);
    return () => mocks.handlers.delete(name);
  }),
}));

vi.mock('../api/tauri', () => ({
  mcpApi: {
    listConfirms: mocks.listConfirms,
    resolveConfirm: mocks.resolveConfirm,
    revokeGrants: mocks.revokeGrants,
  },
}));

import McpConfirmModal from './McpConfirmModal';

const baseRequest = {
  id: 'cf_1',
  tool: 'combridge_serial_write',
  risk: '串口写入',
  scopeLabel: 'COM3',
  detail: '向 COM3 写入 2 字节：AA 55',
  grantable: true,
  grantTtlSecs: 1800,
  createdAt: Date.now(),
  deadlineAt: Date.now() + 120_000,
  remainingMs: 120_000,
};

/** 模拟后端 emit：把请求推给已注册的监听器 */
function emitRequest(payload: unknown) {
  const handler = mocks.handlers.get('mcp-confirm-request');
  expect(handler, '组件必须订阅 mcp-confirm-request').toBeTruthy();
  handler!({ payload });
}

describe('McpConfirmModal', () => {
  beforeEach(() => {
    mocks.handlers.clear();
    mocks.listConfirms.mockClear();
    mocks.listConfirms.mockResolvedValue([]);
    mocks.resolveConfirm.mockClear();
  });

  afterEach(() => {
    cleanup();
  });

  it('没有待确认请求时不渲染任何东西', async () => {
    render(<McpConfirmModal />);
    await waitFor(() => expect(mocks.listConfirms).toHaveBeenCalled());
    expect(screen.queryByText('mcp.confirm.title')).toBeNull();
  });

  it('收到事件后展示风险、目标与具体内容', async () => {
    render(<McpConfirmModal />);
    emitRequest(baseRequest);

    expect(await screen.findByText('mcp.confirm.title')).toBeTruthy();
    expect(screen.getByText('串口写入 · COM3')).toBeTruthy();
    expect(screen.getByText('向 COM3 写入 2 字节：AA 55')).toBeTruthy();
  });

  it('点确认时把 accepted 回传后端，未勾选则不申请放行', async () => {
    render(<McpConfirmModal />);
    emitRequest(baseRequest);

    fireEvent.click(await screen.findByText('mcp.confirm.accept'));

    await waitFor(() =>
      expect(mocks.resolveConfirm).toHaveBeenCalledWith('cf_1', true, false)
    );
  });

  it('勾选「本次会话内允许」后回传 remember=true', async () => {
    render(<McpConfirmModal />);
    emitRequest(baseRequest);

    const checkbox = await screen.findByRole('checkbox');
    fireEvent.click(checkbox);
    fireEvent.click(screen.getByText('mcp.confirm.accept'));

    await waitFor(() =>
      expect(mocks.resolveConfirm).toHaveBeenCalledWith('cf_1', true, true)
    );
  });

  it('取消时回传拒绝，且不申请放行', async () => {
    render(<McpConfirmModal />);
    emitRequest(baseRequest);

    fireEvent.click(await screen.findByRole('checkbox'));
    fireEvent.click(screen.getByText('mcp.confirm.decline'));

    await waitFor(() =>
      expect(mocks.resolveConfirm).toHaveBeenCalledWith('cf_1', false, false)
    );
  });

  it('不可放行的操作（如 OTA 烧录）不显示勾选框', async () => {
    render(<McpConfirmModal />);
    emitRequest({ ...baseRequest, grantable: false });

    await screen.findByText('mcp.confirm.title');
    expect(screen.queryByRole('checkbox')).toBeNull();
  });

  it('启动时补拉未应答的请求，防事件丢失', async () => {
    mocks.listConfirms.mockResolvedValue([{ ...baseRequest, id: 'cf_9' }]);
    render(<McpConfirmModal />);

    expect(await screen.findByText('mcp.confirm.title')).toBeTruthy();
    fireEvent.click(screen.getByText('mcp.confirm.accept'));
    await waitFor(() =>
      expect(mocks.resolveConfirm).toHaveBeenCalledWith('cf_9', true, false)
    );
  });

  it('同一请求重复到达只入队一次', async () => {
    render(<McpConfirmModal />);
    emitRequest(baseRequest);
    emitRequest(baseRequest);

    await screen.findByText('mcp.confirm.title');
    // 队列里只有一条时不会出现"还有 N 个待确认"
    expect(screen.queryByText(/mcp.confirm.queued/)).toBeNull();
  });
});
