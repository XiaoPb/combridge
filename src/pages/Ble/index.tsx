import React, { useState, useEffect } from 'react';
import { Layout, Card, Button, Alert, Typography, Space } from 'antd';
import { ApiOutlined, SettingOutlined, MenuFoldOutlined, MenuUnfoldOutlined } from '@ant-design/icons';
import { useBle } from '../../hooks/useBle';
import { useSerialStore } from '../../stores/serialStore';
import { serialApi } from '../../api/tauri';
import BleModeSelector from './BleModeSelector';
import BleScanner from './BleScanner';
import BleConnection from './BleConnection';
import GattBrowser from './GattBrowser';
import CharacteristicPanel from './CharacteristicPanel';
import AtConfigPanel from './AtConfigPanel';
import type { BleCharacteristic } from '../../types';

const { Sider, Content } = Layout;
const { Title } = Typography;

const BlePage: React.FC = () => {
  const {
    mode,
    serialPort,
    devices,
    connections,
    currentDevice,
    services,
    isScanning,
    isConnecting,
    error,
    configure,
    scanDevices,
    stopScan,
    connectDevice,
    disconnectDevice,
    discoverServices,
    discoverCharacteristics,
    readCharacteristic,
    writeCharacteristic,
    subscribeNotify,
    unsubscribeNotify,
    setCurrentDevice,
  } = useBle();

  const { ports, setPorts } = useSerialStore();
  const [selectedCharacteristic, setSelectedCharacteristic] = useState<BleCharacteristic | null>(null);
  const [discoveringServices, setDiscoveringServices] = useState(false);
  const [siderCollapsed, setSiderCollapsed] = useState(false);

  useEffect(() => {
    serialApi.listPorts().then(setPorts).catch(console.error);
  }, [setPorts]);

  useEffect(() => {
    if (currentDevice && connections.find((c) => c.deviceId === currentDevice)?.isConnected) {
      setDiscoveringServices(true);
      discoverServices(currentDevice).finally(() => {
        setDiscoveringServices(false);
      });
    }
  }, [currentDevice, connections, discoverServices]);

  const handleModeChange = async (newMode: 'native' | 'at') => {
    await configure(newMode, serialPort || undefined);
  };

  const handleSerialPortChange = async (port: string) => {
    await configure(mode, port);
  };

  const handleScan = (timeout?: number) => {
    scanDevices(timeout ? { timeout } : undefined);
  };

  const handleConnect = async (address: string) => {
    await connectDevice(address);
  };

  const handleDisconnect = async (deviceId: string) => {
    await disconnectDevice(deviceId);
  };

  const handleServiceSelect = async (serviceUuid: string) => {
    if (currentDevice) {
      await discoverCharacteristics(serviceUuid, currentDevice);
    }
  };

  const handleCharacteristicSelect = (characteristic: BleCharacteristic) => {
    setSelectedCharacteristic(characteristic);
  };

  const handleRead = async (uuid: string) => {
    await readCharacteristic(uuid);
  };

  const handleWrite = async (uuid: string, data: string, format: 'hex' | 'text', withoutResponse: boolean) => {
    await writeCharacteristic(uuid, data, format, withoutResponse);
  };

  const handleSubscribe = async (uuid: string) => {
    await subscribeNotify(uuid);
  };

  const handleUnsubscribe = async (uuid: string) => {
    await unsubscribeNotify(uuid);
  };

  const handleSendAtCommand = (command: string) => {
    console.debug('Send AT command:', command);
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {error && (
        <Alert
          message="错误"
          description={error}
          type="error"
          closable
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      {isConnecting && (
        <Alert
          message="正在连接..."
          type="info"
          showIcon
          style={{ marginBottom: 8, flexShrink: 0 }}
        />
      )}

      <Layout style={{ flex: '1 1 0', background: 'transparent', minHeight: 0 }}>
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
            marginRight: siderCollapsed ? 0 : 8,
            overflow: 'hidden',
            transition: 'all 0.2s',
          }}
        >
          <div style={{ padding: 8, height: '100%', overflow: 'auto' }}>
            <Title level={5} style={{ marginBottom: 8 }}>BLE 配置</Title>

            <Space direction="vertical" style={{ width: '100%' }} size="middle">
              <Card
                size="small"
                title={
                  <span>
                    <SettingOutlined style={{ marginRight: 8 }} />
                    模式配置
                  </span>
                }
                style={{ background: 'var(--bg-primary)' }}
                bodyStyle={{ padding: 8 }}
              >
                <BleModeSelector
                  mode={mode}
                  serialPort={serialPort}
                  ports={ports}
                  onModeChange={handleModeChange}
                  onSerialPortChange={handleSerialPortChange}
                />
              </Card>

              <Card
                size="small"
                title={
                  <span>
                    <ApiOutlined style={{ marginRight: 8 }} />
                    AT 配置
                  </span>
                }
                style={{ background: 'var(--bg-primary)' }}
                bodyStyle={{ padding: 8 }}
              >
                <AtConfigPanel
                  ports={ports}
                  selectedPort={serialPort}
                  onPortChange={handleSerialPortChange}
                  onSendCommand={handleSendAtCommand}
                />
              </Card>

              <Card
                size="small"
                title="连接列表"
                style={{ background: 'var(--bg-primary)' }}
                bodyStyle={{ padding: 8 }}
              >
                <BleConnection
                  connections={connections}
                  currentDevice={currentDevice}
                  onSelect={setCurrentDevice}
                  onDisconnect={handleDisconnect}
                />
              </Card>
            </Space>
          </div>
        </Sider>

        <Layout style={{ background: 'transparent', flex: 1, minWidth: 0, overflow: 'hidden' }}>
          <Content style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
            <Card
              size="small"
              style={{ flex: '1 1 0', display: 'flex', flexDirection: 'column', marginBottom: 8, minHeight: 0 }}
              bodyStyle={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 8, overflow: 'hidden', minHeight: 0 }}
              title={
                <Space>
                  <Button
                    type="text"
                    icon={siderCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                    onClick={() => setSiderCollapsed(!siderCollapsed)}
                  />
                  <span>设备扫描</span>
                </Space>
              }
            >
              <BleScanner
                devices={devices}
                isScanning={isScanning}
                onScan={handleScan}
                onStopScan={stopScan}
                onConnect={handleConnect}
              />
            </Card>

            <Card
              size="small"
              style={{ flex: '1 1 0', display: 'flex', flexDirection: 'column', minHeight: 0 }}
              bodyStyle={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 8, overflow: 'hidden', minHeight: 0 }}
              title="GATT 浏览器"
            >
              <div style={{ display: 'flex', gap: 8, flex: '1 1 0', minHeight: 0, overflow: 'hidden' }}>
                <div style={{ flex: '1 1 0', minWidth: 0, overflow: 'auto' }}>
                  <GattBrowser
                    services={services}
                    loading={discoveringServices}
                    onServiceSelect={handleServiceSelect}
                    onCharacteristicSelect={handleCharacteristicSelect}
                  />
                </div>
                <div style={{ flex: '1 1 0', minWidth: 0, overflow: 'auto' }}>
                  <CharacteristicPanel
                    characteristic={selectedCharacteristic}
                    onRead={handleRead}
                    onWrite={handleWrite}
                    onSubscribe={handleSubscribe}
                    onUnsubscribe={handleUnsubscribe}
                  />
                </div>
              </div>
            </Card>
          </Content>
        </Layout>
      </Layout>
    </div>
  );
};

export default BlePage;
