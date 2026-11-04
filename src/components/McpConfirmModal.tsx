/**
 * MCP 危险操作确认弹窗（docs/mcp-server-plan.md §8.3）。
 *
 * 为什么不是客户端弹窗：MCP 的 elicitation 需要客户端在 `initialize` 里声明能力，
 * 而 WorkBuddy Connector 5.9.3 这类桌面宿主没有声明，服务端只能拿到
 * `CapabilityNotSupported`。ComBridge 自己就是本机 GUI 进程，于是确认框由这里弹。
 *
 * 三条设计约束：
 * 1. **必须看得见**：后端在发事件前会把主窗口提前台，所以这里只管渲染。
 * 2. **排队，不并发**：模型可能连着触发多次写操作，一次只问一件事。
 * 3. **超时即拒绝**：后端 120s 后按拒绝处理，前端到点自动收起，不能留一个死框。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Alert, Button, Checkbox, Modal, Space, Tag, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { mcpApi, type McpConfirmRequest } from '../api/tauri';

const { Text, Paragraph } = Typography;

const EVENT_REQUEST = 'mcp-confirm-request';
const EVENT_SETTLED = 'mcp-confirm-settled';

interface SettledPayload {
  id: string;
  decision: 'accepted' | 'declined' | 'timeout';
}

export default function McpConfirmModal() {
  const { t } = useTranslation('system');
  const [queue, setQueue] = useState<McpConfirmRequest[]>([]);
  const [remember, setRemember] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const seen = useRef<Set<string>>(new Set());

  const enqueue = useCallback((req: McpConfirmRequest) => {
    if (!req?.id || seen.current.has(req.id)) return;
    seen.current.add(req.id);
    setQueue((prev) => (prev.some((r) => r.id === req.id) ? prev : [...prev, req]));
  }, []);

  const drop = useCallback((id: string) => {
    setQueue((prev) => prev.filter((r) => r.id !== id));
  }, []);

  useEffect(() => {
    let unRequest: UnlistenFn | undefined;
    let unSettled: UnlistenFn | undefined;
    let disposed = false;

    (async () => {
      unRequest = await listen<McpConfirmRequest>(EVENT_REQUEST, (event) => {
        enqueue(event.payload);
      });
      unSettled = await listen<SettledPayload>(EVENT_SETTLED, (event) => {
        drop(event.payload.id);
      });
      // 事件是主通道，但"窗口刚起来、事件已经发过"的竞态要靠补拉兜底
      try {
        const pending = await mcpApi.listConfirms();
        if (!disposed) pending.forEach(enqueue);
      } catch (err) {
        console.warn('[McpConfirm] 读取待确认请求失败', err);
      }
    })();

    return () => {
      disposed = true;
      unRequest?.();
      unSettled?.();
    };
  }, [enqueue, drop]);

  const current = queue[0];
  const currentId = current?.id;

  // 倒计时只在有请求时跑，避免空转
  useEffect(() => {
    if (!currentId) return;
    setRemember(false);
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [currentId]);

  const remainingSecs = current
    ? Math.max(0, Math.ceil((current.deadlineAt - now) / 1000))
    : 0;

  // 到点自动收起：后端此刻已按超时拒绝，界面不能还留着可点的确认按钮
  useEffect(() => {
    if (current && remainingSecs <= 0) drop(current.id);
  }, [current, remainingSecs, drop]);

  const settle = useCallback(
    async (accepted: boolean) => {
      if (!current || submitting) return;
      setSubmitting(true);
      try {
        await mcpApi.resolveConfirm(current.id, accepted, accepted && remember);
      } catch (err) {
        console.warn('[McpConfirm] 应答失败（按已失效处理）', err);
      } finally {
        drop(current.id);
        setSubmitting(false);
      }
    },
    [current, remember, submitting, drop]
  );

  if (!current) return null;

  const minutes = Math.round((current.grantTtlSecs || 0) / 60);

  return (
    <Modal
      open
      title={t('mcp.confirm.title')}
      closable={false}
      mask={{ closable: false }}
      keyboard={false}
      width={560}
      footer={[
        <Button key="decline" onClick={() => settle(false)} disabled={submitting}>
          {t('mcp.confirm.decline')}
        </Button>,
        <Button
          key="accept"
          type="primary"
          danger
          loading={submitting}
          onClick={() => settle(true)}
        >
          {t('mcp.confirm.accept')}
        </Button>,
      ]}
    >
      <Alert
        type="warning"
        showIcon
        title={`${current.risk} · ${current.scopeLabel}`}
        description={t('mcp.confirm.source', { tool: current.tool })}
        style={{ marginBottom: 12 }}
      />

      <Paragraph
        style={{
          whiteSpace: 'pre-wrap',
          marginBottom: 12,
          fontFamily: 'var(--font-mono, monospace)',
          fontSize: 13,
        }}
      >
        {current.detail}
      </Paragraph>

      {current.grantable && (
        <Checkbox checked={remember} onChange={(e) => setRemember(e.target.checked)}>
          {t('mcp.confirm.remember', {
            target: current.scopeLabel,
            risk: current.risk,
          })}
          <Text type="secondary" style={{ display: 'block', fontSize: 12, marginLeft: 24 }}>
            {t('mcp.confirm.rememberHint', { minutes })}
          </Text>
        </Checkbox>
      )}

      <Space style={{ marginTop: 12 }}>
        <Tag color={remainingSecs <= 20 ? 'red' : 'default'}>
          {t('mcp.confirm.remaining', { seconds: remainingSecs })}
        </Tag>
        {queue.length > 1 && (
          <Tag>{t('mcp.confirm.queued', { count: queue.length - 1 })}</Tag>
        )}
      </Space>
    </Modal>
  );
}
