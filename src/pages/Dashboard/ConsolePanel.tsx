import React, { useEffect, useRef, useState } from 'react';
import { Card, Button, Switch, Select, Space, Empty, Tag, theme } from 'antd';
import { ClearOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStore } from '../../stores/dashboardStore';
import type { RawDataPoint } from '../../types/dashboard';

const ConsolePanel: React.FC = () => {
  const { t } = useTranslation('dashboard');
  const { rawDataBuffer, clearRawDataBuffer } = useDashboardStore();
  const { token } = theme.useToken();
  const [displayMode, setDisplayMode] = useState<'hex' | 'ascii'>('hex');
  const [autoScroll, setAutoScroll] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [rawDataBuffer, autoScroll]);

  const formatData = (data: number[], mode: 'hex' | 'ascii'): string => {
    if (mode === 'hex') {
      return data.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
    }
    return data.map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : '.')).join('');
  };

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const seconds = date.getSeconds().toString().padStart(2, '0');
    const ms = date.getMilliseconds().toString().padStart(3, '0');
    return `${hours}:${minutes}:${seconds}.${ms}`;
  };

  const handleClear = () => {
    clearRawDataBuffer();
  };

  const terminalBg = '#1e1e1e';
  const terminalText = '#d4d4d4';
  const terminalBorder = '#333';
  const terminalTimestamp = '#888';

  return (
    <Card
      title={t('console.title') || '控制台'}
      size="small"
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
      styles={{ body: { flex: 1, overflow: 'hidden', padding: 0 } }}
      extra={
        <Space>
          <Select
            value={displayMode}
            onChange={setDisplayMode}
            size="small"
            options={[
              { value: 'hex', label: t('console.hex') || 'HEX' },
              { value: 'ascii', label: t('console.ascii') || 'ASCII' },
            ]}
            style={{ width: 80 }}
          />
          <span style={{ fontSize: 12, color: token.colorTextTertiary }}>
            {t('console.autoScroll') || '自动滚动'}
          </span>
          <Switch
            size="small"
            checked={autoScroll}
            onChange={setAutoScroll}
          />
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={handleClear}
          >
            {t('console.clear') || '清空'}
          </Button>
        </Space>
      }
    >
      <div
        ref={containerRef}
        style={{
          height: '100%',
          overflow: 'auto',
          padding: 8,
          background: terminalBg,
          fontFamily: 'Consolas, Monaco, monospace',
          fontSize: 12,
        }}
      >
        {rawDataBuffer.length === 0 ? (
          <Empty
            description={t('console.noData') || '暂无数据'}
            style={{ marginTop: 100, color: token.colorTextTertiary }}
          />
        ) : (
          rawDataBuffer.map((point: RawDataPoint, index: number) => (
            <div
              key={index}
              style={{
                padding: '2px 0',
                borderBottom: `1px solid ${terminalBorder}`,
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              <Tag
                color={point.direction === 'TX' ? 'blue' : 'green'}
                style={{ margin: 0, minWidth: 32, textAlign: 'center' }}
              >
                {point.direction}
              </Tag>
              <span style={{ color: terminalTimestamp, minWidth: 80 }}>
                {formatTimestamp(point.timestamp)}
              </span>
              <span style={{ color: terminalText, wordBreak: 'break-all' }}>
                {formatData(point.data, displayMode)}
              </span>
            </div>
          ))
        )}
      </div>
    </Card>
  );
};

export default ConsolePanel;
