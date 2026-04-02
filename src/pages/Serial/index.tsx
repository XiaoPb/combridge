import React, { useEffect, useState, useRef, useCallback } from 'react';
import { Layout, Card, Button, Select, Spin, Typography, Space, Alert, Input, Segmented, Switch, Empty, Tag, Tabs as AntTabs } from 'antd';
import { ReloadOutlined, UsbOutlined, DisconnectOutlined, SendOutlined, ClearOutlined, DownloadOutlined, MenuFoldOutlined, MenuUnfoldOutlined } from '@ant-design/icons';
import { useSerial } from '../../hooks/useSerial';
import { formatTimestamp, formatData } from '../../stores/serialStore';
import { DEFAULT_BAUD_RATES, DEFAULT_SERIAL_CONFIG } from '../../types';
import type { SerialConfig } from '../../types';

const { Sider, Content } = Layout;
const { Text, Title } = Typography;
const { TextArea } = Input;

const SerialPage: React.FC = () => {
  const {
    ports,
    tabs,
    activeTab,
    activeTabKey,
    isScanning,
    error,
    scanPorts,
    openPort,
    closePort,
    sendData,
    clearTabData,
    updateTabConfig,
    setActiveTab,
    addTab,
    removeTab,
    setError,
  } = useSerial();

  const [siderCollapsed, setSiderCollapsed] = useState(false);
  const [displayFormat, setDisplayFormat] = useState<'hex' | 'text'>('hex');
  const [displayMode, setDisplayMode] = useState<'all' | 'receive' | 'send'>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const [inputData, setInputData] = useState('');
  const [sendFormat, setSendFormat] = useState<'hex' | 'text'>('text');
  const [appendNewline, setAppendNewline] = useState(true);
  const [newlineType, setNewlineType] = useState<'lf' | 'crlf'>('lf');
  const [selectedPort, setSelectedPort] = useState<string | null>(null);
  const [tempConfig, setTempConfig] = useState<SerialConfig>(DEFAULT_SERIAL_CONFIG);

  const containerRef = useRef<any>(null);
  const lastDataCountRef = useRef(0);

  useEffect(() => {
    scanPorts();
  }, []);

  const connectedPorts = tabs.filter((t) => t.isConnected).map((t) => t.portName);
  const availablePorts = (ports || []).filter(
    (p) => !connectedPorts.includes(p.name) || p.name === activeTab?.portName
  );

  const handleOpenPort = async () => {
    if (!selectedPort) {
      setError('请选择串口');
      return;
    }
    await openPort(selectedPort, tempConfig);
    setSelectedPort(null);
  };

  const handleClosePort = async () => {
    if (activeTabKey) {
      await closePort(activeTabKey);
    }
  };

  const handleSendData = async () => {
    if (!inputData.trim()) return;
    let dataToSend = inputData;
    if (sendFormat === 'text' && appendNewline) {
      dataToSend += newlineType === 'crlf' ? '\r\n' : '\n';
    }
    if (activeTabKey) {
      await sendData(activeTabKey, dataToSend, sendFormat);
    }
  };

  const handleAddTab = () => {
    const key = addTab('新串口');
    setActiveTab(key);
  };

  const handleRemoveTab = async (targetKey: string) => {
    const tab = tabs.find((t) => t.key === targetKey);
    if (tab?.isConnected) {
      try {
        await closePort(targetKey);
      } catch {
        // ignore close errors
      }
    }
    removeTab(targetKey);
  };

  const allData = activeTab
    ? [...(activeTab.receivedData || []), ...(activeTab.sentData || [])].sort((a, b) => a.timestamp - b.timestamp)
    : [];
  const filteredData = displayMode === 'all'
    ? allData
    : displayMode === 'receive'
    ? activeTab?.receivedData
    : activeTab?.sentData;

  useEffect(() => {
    if (autoScroll && containerRef.current && filteredData && (filteredData?.length || 0) !== lastDataCountRef.current) {
      lastDataCountRef.current = filteredData.length;
      requestAnimationFrame(() => {
        if (containerRef.current) {
          containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
      });
    }
  }, [filteredData, autoScroll]);

  const handleScroll = useCallback((e: React.UIEvent<HTMLTextAreaElement>) => {
    const target = e.currentTarget;
    const isAtBottom = target.scrollHeight - target.scrollTop - target.clientHeight < 10;
    setAutoScroll(isAtBottom);
  }, []);

  const handleExport = () => {
    if (!filteredData || filteredData.length === 0) return;
    const content = (filteredData || [])
      .map((entry) => {
        const timestamp = formatTimestamp(entry.timestamp);
        const direction = entry.direction === 'receive' ? 'RX' : 'TX';
        const data = formatData(entry.data, displayFormat);
        return `[${timestamp}] [${direction}] [${(entry.data || []).length} byte] ${data}`;
      })
      .join('\n');

    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `serial-data-${activeTab?.portName || 'unknown'}-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const renderTabContent = () => {
    if (!activeTab) {
      return (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
          <Empty description="请选择或添加串口标签页" />
        </div>
      );
    }

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
                    value={selectedPort || activeTab.portName}
                    onChange={setSelectedPort}
                    disabled={activeTab.isConnected}
                    options={availablePorts.map((port) => ({
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
                  value={activeTab.config?.baudRate || tempConfig.baudRate}
                  onChange={(value) => {
                    if (activeTab.isConnected) {
                      updateTabConfig(activeTabKey!, { baudRate: value });
                    } else {
                      setTempConfig({ ...tempConfig, baudRate: value });
                    }
                  }}
                  disabled={activeTab.isConnected}
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
                    value={activeTab.config?.dataBits || tempConfig.dataBits}
                    onChange={(value) => {
                      if (activeTab.isConnected) {
                        updateTabConfig(activeTabKey!, { dataBits: value });
                      } else {
                        setTempConfig({ ...tempConfig, dataBits: value });
                      }
                    }}
                    disabled={activeTab.isConnected}
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
                    value={activeTab.config?.stopBits || tempConfig.stopBits}
                    onChange={(value) => {
                      if (activeTab.isConnected) {
                        updateTabConfig(activeTabKey!, { stopBits: value });
                      } else {
                        setTempConfig({ ...tempConfig, stopBits: value });
                      }
                    }}
                    disabled={activeTab.isConnected}
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
                    value={activeTab.config?.parity || tempConfig.parity}
                    onChange={(value) => {
                      if (activeTab.isConnected) {
                        updateTabConfig(activeTabKey!, { parity: value });
                      } else {
                        setTempConfig({ ...tempConfig, parity: value });
                      }
                    }}
                    disabled={activeTab.isConnected}
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
                    value={activeTab.config?.flowControl || tempConfig.flowControl}
                    onChange={(value) => {
                      if (activeTab.isConnected) {
                        updateTabConfig(activeTabKey!, { flowControl: value });
                      } else {
                        setTempConfig({ ...tempConfig, flowControl: value });
                      }
                    }}
                    disabled={activeTab.isConnected}
                    options={[
                      { value: 'none', label: '无' },
                      { value: 'hardware', label: '硬件' },
                      { value: 'software', label: '软件' },
                    ]}
                  />
                </div>
              </div>
              <div style={{ marginTop: 16 }}>
                {activeTab.isConnected ? (
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
                    disabled={!selectedPort && !activeTab.portName}
                    block
                  >
                    打开串口
                  </Button>
                )}
              </div>

              <div style={{ marginTop: 16, padding: 12, background: 'var(--bg-primary)', borderRadius: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>当前配置:</Text>
                <Text code style={{ fontSize: 11, display: 'block', marginTop: 4 }}>
                  {activeTab.config?.baudRate || tempConfig.baudRate}, {activeTab.config?.dataBits || tempConfig.dataBits}{activeTab.config?.stopBits || tempConfig.stopBits}, {activeTab.config?.parity || tempConfig.parity}, {activeTab.config?.flowControl || tempConfig.flowControl}
                </Text>
              </div>
            </Space>
          </div>
        </Sider>

        <Layout style={{ background: 'transparent', flex: 1, minWidth: 0, overflow: 'hidden' }}>
          <Content style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
            {error && (
              <Alert
                message={error}
                type="error"
                closable
                onClose={() => setError(null)}
                style={{ marginBottom: 12, flexShrink: 0 }}
              />
            )}

            <Card
              size="small"
              style={{ flex: '1 1 0', display: 'flex', flexDirection: 'column', marginBottom: 12, minHeight: 0 }}
              bodyStyle={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 12, overflow: 'hidden', minHeight: 0 }}
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
                  <Button icon={<DownloadOutlined />} onClick={handleExport} disabled={(filteredData?.length || 0) === 0} size="small">
                    导出
                  </Button>
                  <Button icon={<ClearOutlined />} onClick={() => activeTabKey && clearTabData(activeTabKey)} disabled={(filteredData?.length || 0) === 0} size="small">
                    清空
                  </Button>
                </Space>
              }
            >
              <TextArea
                onScroll={handleScroll}
                value={(filteredData || [])
                  .map((entry) => {
                    const timestamp = formatTimestamp(entry.timestamp);
                    const direction = entry.direction === 'receive' ? 'RX' : 'TX';
                    const data = formatData(entry.data, displayFormat);
                    return `[${timestamp}][${direction}][${(entry.data || []).length} byte] ${data}`;
                  })
                  .join('\n')}
                style={{
                  flex: '1 1 0',
                  overflow: 'auto',
                  background: 'var(--bg-primary)',
                  padding: 8,
                  borderRadius: 4,
                  fontFamily: 'Consolas, Monaco, monospace',
                  fontSize: 13,
                  lineHeight: 1.4,
                  minHeight: 0,
                  resize: 'none',
                }}
                readOnly
                placeholder={(filteredData?.length || 0) === 0 ? '暂无数据' : ''}
              />
            </Card>

            <Card
              size="small"
              style={{ flex: '0 0 auto', flexShrink: 0 }}
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
                    <>
                      <Space size={4}>
                        <Text type="secondary" style={{ fontSize: 12 }}>追加换行</Text>
                        <Switch size="small" checked={appendNewline} onChange={setAppendNewline} />
                      </Space>
                      {appendNewline && (
                        <Segmented
                          value={newlineType}
                          onChange={(value) => setNewlineType(value as 'lf' | 'crlf')}
                          size="small"
                          options={[
                            { value: 'lf', label: 'LF (\\n)' },
                            { value: 'crlf', label: 'CRLF (\\r\\n)' },
                          ]}
                        />
                      )}
                    </>
                  )}
                </Space>
              }
            >
              <Space.Compact style={{ width: '100%' }}>
                <TextArea
                  value={inputData}
                  onChange={(e) => setInputData(e.target.value)}
                  placeholder={sendFormat === 'hex' ? '输入十六进制数据，如: 01 02 03 FF' : '输入要发送的文本'}
                  disabled={!activeTab?.isConnected}
                  autoSize={{ minRows: 2, maxRows: 4 }}
                  style={{ fontFamily: sendFormat === 'hex' ? 'Consolas, Monaco, monospace' : 'inherit' }}
                />
                <Button
                  type="primary"
                  icon={<SendOutlined />}
                  onClick={handleSendData}
                  disabled={!activeTab?.isConnected || !inputData.trim()}
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

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ marginBottom: 8, flexShrink: 0 }}>
        <AntTabs
          type="editable-card"
          size="small"
          activeKey={activeTabKey || undefined}
          onChange={(key) => setActiveTab(key)}
          onEdit={(targetKey, action) => {
            if (action === 'add') {
              handleAddTab();
            } else if (action === 'remove') {
              handleRemoveTab(targetKey as string);
            }
          }}
          items={tabs.map((tab) => ({
            key: tab.key,
            label: (
              <span style={{ fontSize: 12 }}>
                {tab.portName}
                {tab.isConnected && (
                  <Tag color="success" style={{ marginLeft: 4, fontSize: 10 }}>
                    ●
                  </Tag>
                )}
              </span>
            ),
            closable: tabs.length > 1,
          }))}
          style={{ marginBottom: 0 }}
        />
      </div>
      <div style={{ flex: 1, overflow: 'hidden' }}>
        {renderTabContent()}
      </div>
    </div>
  );
};

export default SerialPage;
