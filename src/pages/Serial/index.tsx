import React, { useEffect, useState, useRef, useCallback } from 'react';
import { Layout, Card, Button, Select, Spin, Typography, Space, Alert, Input, Segmented, Switch, Empty, Tag } from 'antd';
import { ReloadOutlined, UsbOutlined, DisconnectOutlined, SendOutlined, ClearOutlined, DownloadOutlined, MenuFoldOutlined, MenuUnfoldOutlined, ArrowDownOutlined, ArrowUpOutlined } from '@ant-design/icons';
import { useSerial } from '../../hooks/useSerial';
import { formatTimestamp, formatData } from '../../stores/serialStore';
import type { DataEntry } from '../../stores/serialStore';
import { DEFAULT_BAUD_RATES } from '../../types';

const { Sider, Content } = Layout;
const { Text, Title } = Typography;
const { TextArea } = Input;

const SerialPage: React.FC = () => {
  const {
    ports,
    currentPort,
    config,
    receivedData,
    sentData,
    isScanning,
    error,
    scanPorts,
    openPort,
    closePort,
    sendData,
    clearAllData,
    updatePortConfig,
    setCurrentPort,
    isConnected,
  } = useSerial();

  const [siderCollapsed, setSiderCollapsed] = useState(false);
  const [displayFormat, setDisplayFormat] = useState<'hex' | 'text'>('hex');
  const [displayMode, setDisplayMode] = useState<'all' | 'receive' | 'send'>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const [inputData, setInputData] = useState('');
  const [sendFormat, setSendFormat] = useState<'hex' | 'text'>('text');
  const [appendNewline, setAppendNewline] = useState(true);

  const containerRef = useRef<HTMLDivElement>(null);
  const lastDataCountRef = useRef(0);

  useEffect(() => {
    scanPorts();
  }, []);

  const handleOpenPort = async () => {
    if (currentPort) {
      await openPort(currentPort, config);
    }
  };

  const handleClosePort = async () => {
    if (currentPort) {
      await closePort(currentPort);
    }
  };

  const handleSendData = async () => {
    if (!inputData.trim()) return;
    let dataToSend = inputData;
    if (sendFormat === 'text' && appendNewline) {
      dataToSend += '\n';
    }
    await sendData(dataToSend, sendFormat);
  };

  const connected = currentPort ? isConnected(currentPort) : false;

  const allData = [...(receivedData || []), ...(sentData || [])].sort((a, b) => a.timestamp - b.timestamp);
  const filteredData = displayMode === 'all'
    ? allData
    : displayMode === 'receive'
    ? receivedData
    : sentData;

  useEffect(() => {
    if (autoScroll && containerRef.current && filteredData.length !== lastDataCountRef.current) {
      lastDataCountRef.current = filteredData.length;
      requestAnimationFrame(() => {
        if (containerRef.current) {
          containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
      });
    }
  }, [filteredData, autoScroll]);

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const target = e.currentTarget;
    const isAtBottom = target.scrollHeight - target.scrollTop - target.clientHeight < 10;
    setAutoScroll(isAtBottom);
  }, []);

  const handleExport = () => {
    const content = (filteredData || [])
      .map((entry) => {
        const timestamp = formatTimestamp(entry.timestamp);
        const direction = entry.direction === 'receive' ? 'RX' : 'TX';
        const data = formatData(entry.data, displayFormat);
        return `[${timestamp}] ${direction}: ${data}`;
      })
      .join('\n');

    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `serial-data-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <Layout style={{ height: '100%', background: 'transparent' }}>
      <Sider
        collapsible
        collapsed={siderCollapsed}
        onCollapse={setSiderCollapsed}
        width={280}
        collapsedWidth={0}
        trigger={null}
        style={{
          background: 'var(--bg-secondary)',
          borderRadius: '8px',
          marginRight: siderCollapsed ? 0 : 16,
          overflow: 'hidden',
          transition: 'all 0.2s',
        }}
      >
        <div style={{ padding: 16, height: '100%', overflow: 'auto' }}>
          <Title level={5} style={{ marginBottom: 16 }}>串口设置</Title>
          
          <Space direction="vertical" style={{ width: '100%' }} size="middle">
            <div>
              <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>串口选择</Text>
              <Space.Compact style={{ width: '100%' }}>
                <Select
                  style={{ width: 'calc(100% - 80px)' }}
                  placeholder="选择串口"
                  value={currentPort}
                  onChange={setCurrentPort}
                  disabled={connected}
                  options={(ports || []).map((port) => ({
                    value: port.name,
                    label: (
                      <div>
                        <Text strong>{port.name}</Text>
                        {port.manufacturer && (
                          <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
                            {port.manufacturer}
                          </Text>
                        )}
                      </div>
                    ),
                  }))}
                />
                <Button
                  icon={isScanning ? <Spin size="small" /> : <ReloadOutlined />}
                  onClick={scanPorts}
                  disabled={isScanning}
                  style={{ width: 80 }}
                >
                  扫描
                </Button>
              </Space.Compact>
            </div>

            <div>
              <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>波特率</Text>
              <Select
                style={{ width: '100%' }}
                value={config.baudRate}
                onChange={(value) => updatePortConfig({ baudRate: value })}
                disabled={connected}
                options={DEFAULT_BAUD_RATES.map((rate) => ({
                  value: rate,
                  label: `${rate} bps`,
                }))}
              />
            </div>

            <div style={{ display: 'flex', gap: 8 }}>
              <div style={{ flex: 1 }}>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>数据位</Text>
                <Select
                  style={{ width: '100%' }}
                  value={config.dataBits}
                  onChange={(value) => updatePortConfig({ dataBits: value })}
                  disabled={connected}
                  options={[
                    { value: 5, label: '5' },
                    { value: 6, label: '6' },
                    { value: 7, label: '7' },
                    { value: 8, label: '8' },
                  ]}
                />
              </div>
              <div style={{ flex: 1 }}>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>停止位</Text>
                <Select
                  style={{ width: '100%' }}
                  value={config.stopBits}
                  onChange={(value) => updatePortConfig({ stopBits: value })}
                  disabled={connected}
                  options={[
                    { value: 1, label: '1' },
                    { value: 2, label: '2' },
                  ]}
                />
              </div>
            </div>

            <div style={{ display: 'flex', gap: 8 }}>
              <div style={{ flex: 1 }}>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>校验位</Text>
                <Select
                  style={{ width: '100%' }}
                  value={config.parity}
                  onChange={(value) => updatePortConfig({ parity: value })}
                  disabled={connected}
                  options={[
                    { value: 'none', label: '无' },
                    { value: 'odd', label: '奇' },
                    { value: 'even', label: '偶' },
                  ]}
                />
              </div>
              <div style={{ flex: 1 }}>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>流控制</Text>
                <Select
                  style={{ width: '100%' }}
                  value={config.flowControl}
                  onChange={(value) => updatePortConfig({ flowControl: value })}
                  disabled={connected}
                  options={[
                    { value: 'none', label: '无' },
                    { value: 'hardware', label: '硬件' },
                    { value: 'software', label: '软件' },
                  ]}
                />
              </div>
            </div>

            <div style={{ marginTop: 16 }}>
              {connected ? (
                <Button
                  type="primary"
                  danger
                  icon={<DisconnectOutlined />}
                  onClick={handleClosePort}
                  block
                >
                  关闭串口
                </Button>
              ) : (
                <Button
                  type="primary"
                  icon={<UsbOutlined />}
                  onClick={handleOpenPort}
                  disabled={!currentPort}
                  block
                >
                  打开串口
                </Button>
              )}
            </div>

            <div style={{ marginTop: 16, padding: 12, background: 'var(--bg-primary)', borderRadius: 4 }}>
              <Text type="secondary" style={{ fontSize: 12 }}>当前配置:</Text>
              <Text code style={{ fontSize: 11, display: 'block', marginTop: 4 }}>
                {config.baudRate}, {config.dataBits}{config.stopBits}, {config.parity}, {config.flowControl}
              </Text>
            </div>
          </Space>
        </div>
      </Sider>

      <Layout style={{ background: 'transparent', flex: 1, minWidth: 0 }}>
        <Content style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
          {error && (
            <Alert
              message={error}
              type="error"
              closable
              style={{ marginBottom: 12 }}
            />
          )}

          <Card
            size="small"
            style={{ flex: '1 1 80%', display: 'flex', flexDirection: 'column', marginBottom: 12 }}
            bodyStyle={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 12, overflow: 'hidden' }}
            title={
              <Space>
                <Button
                  type="text"
                  icon={siderCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                  onClick={() => setSiderCollapsed(!siderCollapsed)}
                />
                <span>数据视图</span>
              </Space>
            }
            extra={
              <Space>
                <Segmented
                  value={displayMode}
                  onChange={(value) => setDisplayMode(value as 'all' | 'receive' | 'send')}
                  options={[
                    { value: 'all', label: '全部' },
                    { value: 'receive', label: '接收' },
                    { value: 'send', label: '发送' },
                  ]}
                />
                <Segmented
                  value={displayFormat}
                  onChange={(value) => setDisplayFormat(value as 'hex' | 'text')}
                  options={[
                    { value: 'hex', label: 'HEX' },
                    { value: 'text', label: 'TEXT' },
                  ]}
                />
                <Button icon={<DownloadOutlined />} onClick={handleExport} disabled={(filteredData || []).length === 0} size="small">
                  导出
                </Button>
                <Button icon={<ClearOutlined />} onClick={clearAllData} disabled={(filteredData || []).length === 0} size="small">
                  清空
                </Button>
              </Space>
            }
          >
            <div
              ref={containerRef}
              onScroll={handleScroll}
              style={{
                flex: 1,
                overflow: 'auto',
                background: 'var(--bg-primary)',
                padding: 8,
                borderRadius: 4,
                fontFamily: 'Consolas, Monaco, monospace',
                fontSize: 13,
              }}
            >
              {(filteredData || []).length === 0 ? (
                <Empty description="暂无数据" style={{ marginTop: 100 }} />
              ) : (
                (filteredData || []).map((entry: DataEntry) => (
                  <div
                    key={entry.id}
                    style={{
                      padding: '4px 8px',
                      marginBottom: 4,
                      background: entry.direction === 'receive' ? 'rgba(82, 196, 26, 0.1)' : 'rgba(24, 144, 255, 0.1)',
                      borderRadius: 4,
                      borderLeft: `3px solid ${entry.direction === 'receive' ? '#52c41a' : '#1890ff'}`,
                    }}
                  >
                    <Space size={8}>
                      <Tag color={entry.direction === 'receive' ? 'success' : 'processing'}>
                        {entry.direction === 'receive' ? <ArrowDownOutlined /> : <ArrowUpOutlined />}
                        <span style={{ marginLeft: 4 }}>{entry.direction === 'receive' ? 'RX' : 'TX'}</span>
                      </Tag>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {formatTimestamp(entry.timestamp)}
                      </Text>
                      <Text>[{(entry.data || []).length} bytes]</Text>
                    </Space>
                    <div style={{ marginTop: 4, wordBreak: 'break-all' }}>
                      {formatData(entry.data, displayFormat)}
                    </div>
                  </div>
                ))
              )}
            </div>
          </Card>

          <Card
            size="small"
            style={{ flex: '0 0 auto' }}
            bodyStyle={{ padding: 12 }}
            title={
              <Space>
                <span>发送面板</span>
                <Segmented
                  value={sendFormat}
                  onChange={(value) => setSendFormat(value as 'hex' | 'text')}
                  size="small"
                  options={[
                    { value: 'text', label: '文本' },
                    { value: 'hex', label: 'HEX' },
                  ]}
                />
                {sendFormat === 'text' && (
                  <Space size={4}>
                    <Text type="secondary" style={{ fontSize: 12 }}>追加换行</Text>
                    <Switch size="small" checked={appendNewline} onChange={setAppendNewline} />
                  </Space>
                )}
              </Space>
            }
          >
            <Space.Compact style={{ width: '100%' }}>
              <TextArea
                value={inputData}
                onChange={(e) => setInputData(e.target.value)}
                placeholder={sendFormat === 'hex' ? '输入十六进制数据，如: 01 02 03 FF' : '输入要发送的文本'}
                disabled={!connected}
                autoSize={{ minRows: 2, maxRows: 4 }}
                style={{ fontFamily: sendFormat === 'hex' ? 'Consolas, Monaco, monospace' : 'inherit' }}
              />
              <Button
                type="primary"
                icon={<SendOutlined />}
                onClick={handleSendData}
                disabled={!connected || !inputData.trim()}
              >
                发送
              </Button>
            </Space.Compact>
          </Card>
        </Content>
      </Layout>
    </Layout>
  );
};

export default SerialPage;
