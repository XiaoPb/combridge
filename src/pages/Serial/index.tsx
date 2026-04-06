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
import { useTranslation } from 'react-i18next';

const { Sider, Content } = Layout;
const { Text, Title } = Typography;
const { TextArea } = Input;

const SerialPage: React.FC = () => {
  const { t } = useTranslation('serial');
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
      setError(t('message.selectPort'));
      return;
    }
    
    if (hasPortTab(selectedPort)) {
      message.warning(t('message.portAlreadyOpen', { port: selectedPort }));
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
      message.warning(t('message.selectPortFirst'));
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
      message.warning(t('message.noDataToExport'));
      return;
    }
    
    try {
      const result = await serialApi.exportData(
        activeTab.portName,
        allDataToExport,
        rxData
      );
      message.success(t('message.exportSuccess', { logPath: result.logPath, datPath: result.datPath }));
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : t('message.exportFailed', { error: '' }).split(':')[0];
      message.error(t('message.exportFailed', { error: errorMsg }));
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
              {t('title.launcher')}
            </Title>
            
            <Space orientation="vertical" style={{ width: '100%' }} size="middle">
              <div>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.portSelect')}</Text>
                <Space.Compact style={{ width: '100%' }}>
                  <Select
                    style={{ width: 'calc(100% - 80px)' }}
                    placeholder={t('placeholder.selectPort')}
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
                    {t('button.scan')}
                  </Button>
                </Space.Compact>
              </div>

              <div>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.baudRate')}</Text>
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
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.dataBits')}</Text>
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
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.stopBits')}</Text>
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
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.parity')}</Text>
                  <Select
                    style={{ width: '100%' }}
                    value={tempConfig.parity}
                    onChange={(value) => setTempConfig({ ...tempConfig, parity: value })}
                    options={[
                      { value: 'none', label: t('option.parityNone') },
                      { value: 'odd', label: t('option.parityOdd') },
                      { value: 'even', label: t('option.parityEven') },
                    ]}
                  />
                </div>
                <div style={{ flex: 1 }}>
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.flowControl')}</Text>
                  <Select
                    style={{ width: '100%' }}
                    value={tempConfig.flowControl}
                    onChange={(value) => setTempConfig({ ...tempConfig, flowControl: value })}
                    options={[
                      { value: 'none', label: t('option.flowNone') },
                      { value: 'hardware', label: t('option.flowHardware') },
                      { value: 'software', label: t('option.flowSoftware') },
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
                  {t('button.openPort')}
                </Button>
              </div>

              <div style={{ marginTop: 8, padding: 8, background: 'var(--bg-primary)', borderRadius: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>{t('label.currentConfig')}:</Text>
                <Text code style={{ fontSize: 11, display: 'block', marginTop: 4 }}>
                  {tempConfig.baudRate}, {tempConfig.dataBits}{tempConfig.stopBits}, {tempConfig.parity}, {tempConfig.flowControl}
                </Text>
              </div>
              
              {connectedPorts.length > 0 && (
                <div style={{ marginTop: 8 }}>
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.connectedPorts')}:</Text>
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
                <Space orientation="vertical" size="small">
                  <Text>{t('message.selectPortToStart')}</Text>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {t('message.eachPortTab')}
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
          <Empty description={t('message.selectPortTab')} />
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
            <Title level={5} style={{ marginBottom: 8 }}>{t('title.settings')}</Title>
            
            <Space orientation="vertical" style={{ width: '100%' }} size="middle">
              <div>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.port')}</Text>
                <Input value={activeTab.portName} disabled style={{ width: '100%' }} />
              </div>

              <div>
                <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.baudRate')}</Text>
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
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.dataBits')}</Text>
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
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.stopBits')}</Text>
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
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.parity')}</Text>
                  <Select
                    style={{ width: '100%' }}
                    value={activeTab.config?.parity || DEFAULT_SERIAL_CONFIG.parity}
                    onChange={(value) => updateTabConfig(activeTabKey!, { parity: value })}
                    disabled={activeTab.isConnected}
                    options={[
                      { value: 'none', label: t('option.parityNone') },
                      { value: 'odd', label: t('option.parityOdd') },
                      { value: 'even', label: t('option.parityEven') },
                    ]}
                  />
                </div>
                <div style={{ flex: 1 }}>
                  <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>{t('label.flowControl')}</Text>
                  <Select
                    style={{ width: '100%' }}
                    value={activeTab.config?.flowControl || DEFAULT_SERIAL_CONFIG.flowControl}
                    onChange={(value) => updateTabConfig(activeTabKey!, { flowControl: value })}
                    disabled={activeTab.isConnected}
                    options={[
                      { value: 'none', label: t('option.flowNone') },
                      { value: 'hardware', label: t('option.flowHardware') },
                      { value: 'software', label: t('option.flowSoftware') },
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
                    {t('button.closePort')}
                  </Button>
                ) : (
                  <Button
                    type="primary"
                    icon={<UsbOutlined />}
                    onClick={handleReconnectPort}
                    block
                  >
                    {t('button.reconnect')}
                  </Button>
                )}
              </div>

              <div style={{ marginTop: 8, padding: 8, background: 'var(--bg-primary)', borderRadius: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>{t('label.currentConfig')}:</Text>
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
                  <span>{t('title.dataView')}</span>
                </Space>
              }
              extra={
                <Space>
                  <Tooltip title={autoScroll ? t('tooltip.autoScrollOn') : t('tooltip.autoScrollOff')}>
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
                      { value: 'all', label: t('display.all') },
                      { value: 'receive', label: t('display.receive') },
                      { value: 'send', label: t('display.send') },
                    ]}
                  />
                  <Segmented
                    value={displayFormat}
                    onChange={(value) => setDisplayFormat(value as 'hex' | 'text')}
                    options={[
                      { value: 'hex', label: t('display.hex') },
                      { value: 'text', label: t('display.text') },
                    ]}
                  />
                  <Button icon={<DownloadOutlined />} onClick={handleExport} disabled={filteredData.length === 0} size="small">
                    {t('button.export')}
                  </Button>
                  <Button icon={<ClearOutlined />} onClick={() => activeTabKey && clearTabData(activeTabKey)} disabled={filteredData.length === 0} size="small">
                    {t('button.clear')}
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
                placeholder={(filteredData?.length || 0) === 0 ? t('placeholder.noData') : ''}
              />
            </Card>

            <Card
              size="small"
              style={{ flex: '0 0 auto', flexShrink: 0 }}
              styles={{ body: { padding: 8 } }}
              title={
                <Space>
                  <span>{t('title.sendPanel')}</span>
                  <Segmented
                    value={sendFormat}
                    onChange={(value) => setSendFormat(value as 'hex' | 'text')}
                    size="small"
                    options={[
                      { value: 'text', label: t('send.text') },
                      { value: 'hex', label: t('display.hex') },
                    ]}
                  />
                  {sendFormat === 'text' && (
                    <>
                      <Space size={4}>
                        <Text type="secondary" style={{ fontSize: 12 }}>{t('send.appendNewline')}</Text>
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
                  placeholder={sendFormat === 'hex' ? t('placeholder.inputHex') : t('placeholder.inputText')}
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
                  {t('button.send')}
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
