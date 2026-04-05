import React, { useEffect, useState, useRef, useCallback } from 'react';
import { Layout, Card, Button, Select, Spin, Typography, Space, Alert, Input, Segmented, Switch, Empty, Tag, Tooltip } from 'antd';
import type { TextAreaRef } from 'antd/es/input/TextArea';
import { ReloadOutlined, UsbOutlined, DisconnectOutlined, SendOutlined, ClearOutlined, DownloadOutlined, MenuFoldOutlined, MenuUnfoldOutlined, PlusOutlined, VerticalAlignBottomOutlined } from '@ant-design/icons';
import { useSerial } from '../../hooks/useSerial';
import { formatTimestamp, formatData } from '../../stores/serialStore';
import { serialApi } from '../../api/tauri';
import { message } from 'antd';
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
    toggleTabSettings,
    setError,
    hasPortTab,
    preferences,
    updatePreferences,
  } = useSerial();

  const [inputData, setInputData] = useState('');
  const [selectedPort, setSelectedPort] = useState<string | null>(null);
  const [tempConfig, setTempConfig] = useState<SerialConfig>(DEFAULT_SERIAL_CONFIG);

  const containerRef = useRef<TextAreaRef>(null);
  const lastDataCountRef = useRef(0);

  const {
    displayFormat,
    displayMode,
    sendFormat,
    appendNewline,
    newlineType,
    autoScroll,
  } = preferences;

  const setDisplayFormat = (value: 'hex' | 'text') => updatePreferences({ displayFormat: value });
  const setDisplayMode = (value: 'all' | 'receive' | 'send') => updatePreferences({ displayMode: value });
  const setSendFormat = (value: 'hex' | 'text') => updatePreferences({ sendFormat: value });
  const setAppendNewline = (value: boolean) => updatePreferences({ appendNewline: value });
  const setNewlineType = (value: 'lf' | 'crlf') => updatePreferences({ newlineType: value });
  const setAutoScroll = (value: boolean) => updatePreferences({ autoScroll: value });

  useEffect(() => {
    scanPorts();
  }, []);

  const connectedPorts = tabs.filter((t) => t.isConnected && t.tabType === 'port').map((t) => t.portName);
  const availablePorts = (ports || []).filter(
    (p) => !connectedPorts.includes(p.name) || p.name === activeTab?.portName
  );

  const isLauncherTab = activeTab?.tabType === 'launcher';
  const isPortTab = activeTab?.tabType === 'port';

  const handleOpenPort = async () => {
    if (!selectedPort) {
      setError('请选择串口');
      return;
    }
    
    if (hasPortTab(selectedPort)) {
      message.warning(`串口 ${selectedPort} 已有打开的标签页`);
      return;
    }
    
    await openPort(selectedPort, tempConfig);
    setSelectedPort(null);
  };

  const handleClosePort = async () => {
    if (activeTabKey && activeTab?.isConnected) {
      await closePort(activeTabKey);
    }
  };

  const handleReconnectPort = async () => {
    if (!activeTab?.portName) return;
    await openPort(activeTab.portName, activeTab.config);
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

  const allData = activeTab && isPortTab
    ? [...(activeTab.receivedData || []), ...(activeTab.sentData || [])].sort((a, b) => a.timestamp - b.timestamp)
    : [];
  const filteredData = displayMode === 'all'
    ? allData
    : displayMode === 'receive'
    ? (activeTab?.receivedData || [])
    : (activeTab?.sentData || []);

  useEffect(() => {
    if (autoScroll && filteredData && (filteredData?.length || 0) !== lastDataCountRef.current) {
      lastDataCountRef.current = filteredData.length;
      requestAnimationFrame(() => {
        const textArea = containerRef.current?.resizableTextArea?.textArea;
        if (textArea) {
          textArea.scrollTop = textArea.scrollHeight;
        }
      });
    }
  }, [filteredData, autoScroll]);

  const handleScroll = useCallback((e: React.UIEvent<HTMLTextAreaElement>) => {
    const target = e.currentTarget;
    const isAtBottom = target.scrollHeight - target.scrollTop - target.clientHeight < 10;
    if (autoScroll !== isAtBottom) {
      setAutoScroll(isAtBottom);
    }
  }, [autoScroll]);

  const handleExport = async () => {
    if (!activeTab || !activeTab.portName) {
      message.warning('请先选择串口');
      return;
    }
    
    const allDataToExport = allData.map((entry) => ({
      timestamp: entry.timestamp,
      data: entry.data,
      direction: entry.direction,
    }));
    
    const rxData = (activeTab.receivedData || [])
      .flatMap((entry) => entry.data);
    
    if (allDataToExport.length === 0) {
      message.warning('没有数据可导出');
      return;
    }
    
    try {
      const result = await serialApi.exportData(
        activeTab.portName,
        allDataToExport,
        rxData
      );
      message.success(`数据已导出:\n日志: ${result.logPath}\n数据: ${result.datPath}`);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : '导出失败';
      message.error(`导出失败: ${errorMsg}`);
    }
  };

  const renderLauncherContent = () => {
    return (
      <Layout style={{ height: '100%', background: 'transparent' }}>
        <Sider
          collapsible
          collapsed={false}
          width={320}
          trigger={null}
          style={{
            background: 'var(--bg-secondary)',
            borderRadius: '8px',
            marginRight: 8,
            overflow: 'hidden',
          }}
        >
          <div style={{ padding: 16, height: '100%', overflow: 'auto' }}>
            <Title level={5} style={{ marginBottom: 16 }}>
              <UsbOutlined style={{ marginRight: 8 }} />
              串口启动台
            </Title>
            
            <Space direction="vertical" style={{ width: '100%' }} size="middle">
              <div>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>串口选择</Text>
                <Space.Compact style={{ width: '100%' }}>
                  <Select
                    style={{ width: 'calc(100% - 80px)' }}
                    placeholder="选择串口"
                    value={selectedPort}
                    onChange={setSelectedPort}
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
                  value={tempConfig.baudRate}
                  onChange={(value) => setTempConfig({ ...tempConfig, baudRate: value })}
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
                    value={tempConfig.dataBits}
                    onChange={(value) => setTempConfig({ ...tempConfig, dataBits: value })}
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
                    value={tempConfig.stopBits}
                    onChange={(value) => setTempConfig({ ...tempConfig, stopBits: value })}
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
                    value={tempConfig.parity}
                    onChange={(value) => setTempConfig({ ...tempConfig, parity: value })}
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
                    value={tempConfig.flowControl}
                    onChange={(value) => setTempConfig({ ...tempConfig, flowControl: value })}
                    options={[
                      { value: 'none', label: '无' },
                      { value: 'hardware', label: '硬件' },
                      { value: 'software', label: '软件' },
                    ]}
                  />
                </div>
              </div>
              
              <div style={{ marginTop: 8 }}>
                <Button
                  type="primary"
                  icon={<PlusOutlined />}
                  onClick={handleOpenPort}
                  disabled={!selectedPort}
                  block
                  size="large"
                >
                  打开串口
                </Button>
              </div>

              <div style={{ marginTop: 8, padding: 8, background: 'var(--bg-primary)', borderRadius: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>当前配置:</Text>
                <Text code style={{ fontSize: 11, display: 'block', marginTop: 4 }}>
                  {tempConfig.baudRate}, {tempConfig.dataBits}{tempConfig.stopBits}, {tempConfig.parity}, {tempConfig.flowControl}
                </Text>
              </div>
              
              {connectedPorts.length > 0 && (
                <div style={{ marginTop: 8 }}>
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>已连接的串口:</Text>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
                    {connectedPorts.map((port) => (
                      <Tag key={port} color="success">{port}</Tag>
                    ))}
                  </div>
                </div>
              )}
            </Space>
          </div>
        </Sider>

        <Layout style={{ background: 'transparent', flex: 1, minWidth: 0, overflow: 'hidden' }}>
          <Content style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden', alignItems: 'center', justifyContent: 'center' }}>
            <Empty
              description={
                <Space direction="vertical" size="small">
                  <Text>选择串口并点击"打开串口"开始通信</Text>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    每个串口将创建独立的标签页
                  </Text>
                </Space>
              }
            />
          </Content>
        </Layout>
      </Layout>
    );
  };

  const renderPortContent = () => {
    if (!activeTab) {
      return (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
          <Empty description="请选择串口标签页" />
        </div>
      );
    }

    return (
      <Layout style={{ height: '100%', background: 'transparent' }}>
        <Sider
          collapsible
          collapsed={activeTab.settingsCollapsed}
          onCollapse={() => toggleTabSettings(activeTabKey!)}
          width={280}
          collapsedWidth={0}
          trigger={null}
          style={{
            background: 'var(--bg-secondary)',
            borderRadius: '8px',
            marginRight: activeTab.settingsCollapsed ? 0 : 8,
            overflow: 'hidden',
            transition: 'all 0.2s',
          }}
        >
          <div style={{ padding: 8, height: '100%', overflow: 'auto' }}>
            <Title level={5} style={{ marginBottom: 8 }}>串口设置</Title>
            
            <Space direction="vertical" style={{ width: '100%' }} size="middle">
              <div>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>串口</Text>
                <Input value={activeTab.portName} disabled style={{ width: '100%' }} />
              </div>

              <div>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>波特率</Text>
                <Select
                  style={{ width: '100%' }}
                  value={activeTab.config?.baudRate || DEFAULT_SERIAL_CONFIG.baudRate}
                  onChange={(value) => updateTabConfig(activeTabKey!, { baudRate: value })}
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
                    value={activeTab.config?.dataBits || DEFAULT_SERIAL_CONFIG.dataBits}
                    onChange={(value) => updateTabConfig(activeTabKey!, { dataBits: value })}
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
                    value={activeTab.config?.stopBits || DEFAULT_SERIAL_CONFIG.stopBits}
                    onChange={(value) => updateTabConfig(activeTabKey!, { stopBits: value })}
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
                    value={activeTab.config?.parity || DEFAULT_SERIAL_CONFIG.parity}
                    onChange={(value) => updateTabConfig(activeTabKey!, { parity: value })}
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
                    value={activeTab.config?.flowControl || DEFAULT_SERIAL_CONFIG.flowControl}
                    onChange={(value) => updateTabConfig(activeTabKey!, { flowControl: value })}
                    disabled={activeTab.isConnected}
                    options={[
                      { value: 'none', label: '无' },
                      { value: 'hardware', label: '硬件' },
                      { value: 'software', label: '软件' },
                    ]}
                  />
                </div>
              </div>
              
              <div style={{ marginTop: 8 }}>
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
                    onClick={handleReconnectPort}
                    block
                  >
                    重新连接
                  </Button>
                )}
              </div>

              <div style={{ marginTop: 8, padding: 8, background: 'var(--bg-primary)', borderRadius: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>当前配置:</Text>
                <Text code style={{ fontSize: 11, display: 'block', marginTop: 4 }}>
                  {activeTab.config?.baudRate || DEFAULT_SERIAL_CONFIG.baudRate}, {activeTab.config?.dataBits || DEFAULT_SERIAL_CONFIG.dataBits}{activeTab.config?.stopBits || DEFAULT_SERIAL_CONFIG.stopBits}, {activeTab.config?.parity || DEFAULT_SERIAL_CONFIG.parity}, {activeTab.config?.flowControl || DEFAULT_SERIAL_CONFIG.flowControl}
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
                style={{ marginBottom: 8, flexShrink: 0 }}
              />
            )}

            <Card
              size="small"
              style={{ flex: '1 1 0', display: 'flex', flexDirection: 'column', marginBottom: 8, minHeight: 0 }}
              styles={{ body: { flex: 1, display: 'flex', flexDirection: 'column', padding: 8, overflow: 'hidden', minHeight: 0 } }}
              title={
                <Space>
                  <Button
                    type="text"
                    icon={activeTab.settingsCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                    onClick={() => toggleTabSettings(activeTabKey!)}
                  />
                  <span>数据视图</span>
                </Space>
              }
              extra={
                <Space>
                  <Tooltip title={autoScroll ? '自动滚动: 开启' : '自动滚动: 关闭'}>
                    <Button
                      type={autoScroll ? 'primary' : 'default'}
                      icon={<VerticalAlignBottomOutlined />}
                      onClick={() => setAutoScroll(!autoScroll)}
                      size="small"
                    />
                  </Tooltip>
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
                  <Button icon={<DownloadOutlined />} onClick={handleExport} disabled={filteredData.length === 0} size="small">
                    导出
                  </Button>
                  <Button icon={<ClearOutlined />} onClick={() => activeTabKey && clearTabData(activeTabKey)} disabled={filteredData.length === 0} size="small">
                    清空
                  </Button>
                </Space>
              }
            >
              <TextArea
                ref={containerRef}
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
              styles={{ body: { padding: 8 } }}
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
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden', padding: 8 }}>
      {isLauncherTab ? renderLauncherContent() : renderPortContent()}
    </div>
  );
};

export default SerialPage;
