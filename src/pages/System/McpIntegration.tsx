import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Input,
  InputNumber,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message,
} from 'antd';
import {
  ApiOutlined,
  CopyOutlined,
  EyeInvisibleOutlined,
  EyeOutlined,
  FolderOpenOutlined,
  ReloadOutlined,
  SafetyCertificateOutlined,
  SyncOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { mcpApi, systemApi, type McpStatus } from '../../api/tauri';
import { formatErrorMessage } from '../../utils/errorMessage';
import { buildMcpSnippet, MCP_CLIENT_KINDS, type McpClientKind } from '../../utils/mcpSnippet';

const { Text, Paragraph } = Typography;

/** 把 token 打码，只留头尾各 4 位，方便对账又不足以被抄走 */
const maskToken = (token: string) => (token.length <= 8 ? '••••••••' : `${token.slice(0, 4)}••••••••${token.slice(-4)}`);

const displayTime = (ms: number) => (ms > 0 ? new Date(ms).toLocaleString() : '—');

const McpIntegration: React.FC = () => {
  const { t } = useTranslation('system');

  const [status, setStatus] = useState<McpStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);
  const [rotating, setRotating] = useState(false);
  const [showToken, setShowToken] = useState(false);
  const [client, setClient] = useState<McpClientKind>('claudeCode');

  // 草稿值：端口/档位改动后点「保存并应用」才生效，避免边输入边重启服务
  const [enabled, setEnabled] = useState(false);
  const [port, setPort] = useState(47821);
  const [tier, setTier] = useState('standard');

  const adopt = useCallback((next: McpStatus) => {
    setStatus(next);
    setEnabled(next.enabled);
    setPort(next.configPort);
    setTier(next.configTier);
  }, []);

  const load = useCallback(async () => {
    try {
      const next = await mcpApi.getStatus();
      adopt(next);
    } catch (err) {
      message.error(formatErrorMessage(err, t('mcp.message.loadFailed')));
    } finally {
      setLoading(false);
    }
  }, [adopt, t]);

  useEffect(() => {
    load();
  }, [load]);

  const apply = useCallback(
    async (next: { enabled: boolean; port: number; tier: string }) => {
      setApplying(true);
      try {
        const result = await mcpApi.applySettings(next.enabled, next.port, next.tier);
        adopt(result);
        if (result.enabled && !result.running) {
          message.error(result.lastError || t('mcp.message.applyFailed'));
        } else {
          message.success(t('mcp.message.applied'));
        }
      } catch (err) {
        message.error(formatErrorMessage(err, t('mcp.message.applyFailed')));
      } finally {
        setApplying(false);
      }
    },
    [adopt, t]
  );

  const handleToggle = (checked: boolean) => {
    setEnabled(checked);
    apply({ enabled: checked, port, tier });
  };

  const handleApply = () => apply({ enabled, port, tier });

  const handleRotate = async () => {
    setRotating(true);
    try {
      const result = await mcpApi.rotateToken();
      adopt(result);
      message.success(t('mcp.message.rotated'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('mcp.message.rotateFailed')));
    } finally {
      setRotating(false);
    }
  };

  /** 打开审计目录：优先审计目录，其次数据目录；两者都不存在时明确提示而不是静默失败 */
  const openAuditDir = async () => {
    const target = status?.auditDir || status?.dataDir;
    if (!target) {
      message.error(t('mcp.message.openFailed'));
      return;
    }
    try {
      await systemApi.showInFolder(target);
    } catch (err) {
      message.error(formatErrorMessage(err, t('mcp.message.openFailed')));
    }
  };

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      message.success(t('mcp.button.copied'));
    } catch {
      message.error(t('mcp.message.copyFailed'));
    }
  };

  const url = status?.url ?? `http://127.0.0.1:${port}/mcp`;
  const token = status?.token ?? '';

  const snippet = useMemo(() => {
    const built = buildMcpSnippet(client, url, token);
    return built || t('mcp.notice.noToken');
  }, [client, token, url, t]);

  const statusTag = () => {
    if (!status) return null;
    if (status.running) return <Tag color="green">{t('mcp.status.running')}</Tag>;
    if (status.enabled) return <Tag color="red">{t('mcp.status.failed')}</Tag>;
    return <Tag>{t('mcp.status.stopped')}</Tag>;
  };

  return (
    <div style={{ padding: '0 8px' }}>
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card
            size="small"
            title={
              <Space>
                <ApiOutlined />
                {t('mcp.title.service')}
                {statusTag()}
              </Space>
            }
            extra={
              <Button size="small" type="text" icon={<ReloadOutlined />} loading={loading} onClick={load} />
            }
          >
            <Space orientation="vertical" size={12} style={{ width: '100%' }}>
              <Space align="center" style={{ justifyContent: 'space-between', width: '100%' }}>
                <Space orientation="vertical" size={0}>
                  <Text>{t('mcp.label.enable')}</Text>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {t('mcp.label.enableHint')}
                  </Text>
                </Space>
                <Switch checked={enabled} loading={applying} onChange={handleToggle} />
              </Space>

              {status?.lastError && (
                <Alert type="error" showIcon title={status.lastError} />
              )}

              {status?.portFallback && status.running && (
                <Alert
                  type="warning"
                  showIcon
                  title={t('mcp.notice.portFallback', { port: status.port })}
                />
              )}

              {status?.running && port !== status.configPort && (
                <Alert
                  type="warning"
                  showIcon
                  title={t('mcp.notice.portChangeRestarts')}
                />
              )}

              {!status?.running && !enabled && (
                <Alert type="info" showIcon title={t('mcp.notice.disabled')} />
              )}

              <div>
                <Text>{t('mcp.label.port')}</Text>
                <InputNumber
                  min={1024}
                  max={65535}
                  style={{ width: '100%', marginTop: 4 }}
                  value={port}
                  onChange={(v) => setPort(v ?? 47821)}
                />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t('mcp.label.portHint')}
                </Text>
              </div>

              <div>
                <Text>{t('mcp.label.tier')}</Text>
                <Select
                  style={{ width: '100%', marginTop: 4 }}
                  value={tier}
                  onChange={setTier}
                  options={[
                    { value: 'dev', label: t('mcp.label.tierDev') },
                    { value: 'standard', label: t('mcp.label.tierStandard') },
                  ]}
                />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {tier === 'dev' ? t('mcp.label.tierDevDesc') : t('mcp.label.tierStandardDesc')}
                </Text>
              </div>

              <Button type="primary" block loading={applying} onClick={handleApply}>
                {t('mcp.button.apply')}
              </Button>

              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label={t('mcp.label.url')}>
                  <Text copyable={!!status?.running}>{url}</Text>
                </Descriptions.Item>
                <Descriptions.Item label={t('mcp.label.toolCount')}>
                  {status?.toolCount ?? '—'}
                </Descriptions.Item>
                <Descriptions.Item label={t('mcp.label.uptime')}>
                  {displayTime(status?.startedAt ?? 0)}
                </Descriptions.Item>
                <Descriptions.Item label={t('mcp.label.configPath')}>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {status?.configPath || '—'}
                  </Text>
                </Descriptions.Item>
              </Descriptions>
            </Space>
          </Card>
        </Col>

        <Col xs={24} lg={12}>
          <Card
            size="small"
            title={
              <Space>
                <CopyOutlined />
                {t('mcp.title.client')}
              </Space>
            }
          >
            <Space orientation="vertical" size={12} style={{ width: '100%' }}>
              <div>
                <Text>{t('mcp.label.token')}</Text>
                <Space.Compact style={{ width: '100%', marginTop: 4 }}>
                  <Input
                    readOnly
                    value={showToken ? token : maskToken(token)}
                    placeholder={t('mcp.notice.noToken')}
                  />
                  <Button
                    icon={showToken ? <EyeInvisibleOutlined /> : <EyeOutlined />}
                    onClick={() => setShowToken((v) => !v)}
                    disabled={!token}
                  />
                  <Button icon={<CopyOutlined />} onClick={() => copy(token)} disabled={!token} />
                </Space.Compact>
                <div style={{ marginTop: 8 }}>
                  <Space>
                    <Button
                      size="small"
                      icon={<SyncOutlined />}
                      loading={rotating}
                      onClick={handleRotate}
                      disabled={!status?.enabled}
                    >
                      {t('mcp.button.rotate')}
                    </Button>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t('mcp.label.tokenHint')}
                    </Text>
                  </Space>
                </div>
              </div>

              <div>
                <Text>{t('mcp.label.client')}</Text>
                <Select
                  style={{ width: '100%', marginTop: 4 }}
                  value={client}
                  onChange={setClient}
                  options={MCP_CLIENT_KINDS.map((k) => ({ value: k, label: t(`mcp.client.${k}`) }))}
                />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t(`mcp.clientPath.${client}`)}
                </Text>
              </div>

              <Input.TextArea
                readOnly
                value={snippet}
                autoSize={{ minRows: 6, maxRows: 12 }}
                style={{ fontFamily: 'Consolas, Monaco, monospace', fontSize: 12 }}
              />

              <Button icon={<CopyOutlined />} block onClick={() => copy(snippet)} disabled={!token}>
                {t('mcp.button.copySnippet')}
              </Button>

              <Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 0 }}>
                {t('mcp.notice.restartClient')}
              </Paragraph>
            </Space>
          </Card>
        </Col>

        <Col xs={24}>
          <Card
            size="small"
            title={
              <Space>
                <SafetyCertificateOutlined />
                {t('mcp.title.security')}
              </Space>
            }
          >
            <Space orientation="vertical" size={10} style={{ width: '100%' }}>
              <Text>{t('mcp.security.tierDev')}</Text>
              <Text>{t('mcp.security.tierStandard')}</Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t('mcp.security.auditHint')}
              </Text>
              <Space wrap>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {status?.auditFile || t('mcp.security.noAuditFile')}
                </Text>
                <Tag>{t('mcp.security.auditRecords', { count: status?.auditRecords ?? 0 })}</Tag>
                <Button
                  size="small"
                  icon={<FolderOpenOutlined />}
                  disabled={!status?.auditDir && !status?.dataDir}
                  onClick={openAuditDir}
                >
                  {t('mcp.button.openAudit')}
                </Button>
              </Space>
            </Space>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default McpIntegration;
