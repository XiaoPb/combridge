import React, { useState, useEffect } from 'react';
import { Row, Col, Alert, Tabs } from 'antd';
import { ApiOutlined, SettingOutlined } from '@ant-design/icons';
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
    console.log('Send AT command:', command);
  };

  return (
    <div>
      {error && (
        <Alert
          message="错误"
          description={error}
          type="error"
          closable
          style={{ marginBottom: 16 }}
        />
      )}

      {isConnecting && (
        <Alert
          message="正在连接..."
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <Row gutter={16}>
        <Col xs={24} lg={8}>
          <Tabs
            defaultActiveKey="mode"
            items={[
              {
                key: 'mode',
                label: (
                  <span>
                    <SettingOutlined />
                    模式配置
                  </span>
                ),
                children: (
                  <BleModeSelector
                    mode={mode}
                    serialPort={serialPort}
                    ports={ports}
                    onModeChange={handleModeChange}
                    onSerialPortChange={handleSerialPortChange}
                  />
                ),
              },
              {
                key: 'at',
                label: (
                  <span>
                    <ApiOutlined />
                    AT 配置
                  </span>
                ),
                children: (
                  <AtConfigPanel
                    ports={ports}
                    selectedPort={serialPort}
                    onPortChange={handleSerialPortChange}
                    onSendCommand={handleSendAtCommand}
                  />
                ),
              },
            ]}
          />

          <div style={{ marginTop: 16 }}>
            <BleConnection
              connections={connections}
              currentDevice={currentDevice}
              onSelect={setCurrentDevice}
              onDisconnect={handleDisconnect}
            />
          </div>
        </Col>

        <Col xs={24} lg={16}>
          <BleScanner
            devices={devices}
            isScanning={isScanning}
            onScan={handleScan}
            onStopScan={stopScan}
            onConnect={handleConnect}
          />

          <Row gutter={16} style={{ marginTop: 16 }}>
            <Col xs={24} md={12}>
              <GattBrowser
                services={services}
                loading={discoveringServices}
                onServiceSelect={handleServiceSelect}
                onCharacteristicSelect={handleCharacteristicSelect}
              />
            </Col>
            <Col xs={24} md={12}>
              <CharacteristicPanel
                characteristic={selectedCharacteristic}
                onRead={handleRead}
                onWrite={handleWrite}
                onSubscribe={handleSubscribe}
                onUnsubscribe={handleUnsubscribe}
              />
            </Col>
          </Row>
        </Col>
      </Row>
    </div>
  );
};

export default BlePage;
