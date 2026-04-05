import React, { useState } from 'react';
import { Card, Button, Input, Table, Space, Tag, Progress, Empty, InputNumber, Typography, Flex } from 'antd';
import { SearchOutlined, StopOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { BleDeviceInfo, BleConnection } from '../../types';
import { formatMacAddress } from '../../stores/bleStore';

const { Text } = Typography;
const { Search } = Input;

interface BleScannerProps {
  devices: BleDeviceInfo[];
  connections: BleConnection[];
  isScanning: boolean;
  onScan: (timeout?: number) => void;
  onStopScan: () => void;
  onConnect: (address: string) => void;
}

const BleScanner: React.FC<BleScannerProps> = ({
  devices,
  connections,
  isScanning,
  onScan,
  onStopScan,
  onConnect,
}) => {
  const { t } = useTranslation('ble');
  const [filterName, setFilterName] = useState('');
  const [scanTimeout, setScanTimeout] = useState(10);

  const filteredDevices = (devices || []).filter((device) => {
    if (filterName && device.name) {
      return device.name.toLowerCase().includes(filterName.toLowerCase());
    }
    return true;
  });

  const handleScan = () => {
    onScan(scanTimeout * 1000);
  };

  const getRssiColor = (rssi?: number): string => {
    if (!rssi) return 'default';
    if (rssi >= -50) return 'green';
    if (rssi >= -70) return 'blue';
    if (rssi >= -90) return 'orange';
    return 'red';
  };

  const getRssiPercent = (rssi?: number): number => {
    if (!rssi) return 0;
    return Math.min(100, Math.max(0, (rssi + 100) * 2));
  };

  const columns = [
    {
      title: t('label.deviceName'),
      dataIndex: 'name',
      key: 'name',
      render: (name: string) => name || <Text type="secondary">{t('label.unnamed')}</Text>,
    },
    {
      title: t('label.macAddress'),
      dataIndex: 'address',
      key: 'address',
      width: 180,
      render: (address: string) => (
        <Text code style={{ fontSize: '12px' }}>
          {formatMacAddress(address)}
        </Text>
      ),
    },
    {
      title: t('label.rssi'),
      dataIndex: 'rssi',
      key: 'rssi',
      width: 150,
      render: (rssi?: number) =>
        rssi ? (
          <Flex vertical gap="small" style={{ width: '100%' }}>
            <Tag color={getRssiColor(rssi)}>{rssi} dBm</Tag>
            <Progress
              percent={getRssiPercent(rssi)}
              size="small"
              showInfo={false}
              strokeColor={getRssiColor(rssi) === 'green' ? '#52c41a' : getRssiColor(rssi) === 'blue' ? '#1890ff' : getRssiColor(rssi) === 'orange' ? '#fa8c16' : '#f5222d'}
            />
          </Flex>
        ) : (
          <Text type="secondary">-</Text>
        ),
    },
    {
      title: t('label.status'),
      key: 'status',
      width: 80,
      render: (_: unknown, record: BleDeviceInfo) => {
        const isConnected = connections?.some(c => c.address === record.address);
        return (
          <Tag color={isConnected ? 'green' : 'default'}>
            {isConnected ? t('status.connected') : t('status.disconnected')}
          </Tag>
        );
      },
    },
    {
      title: t('label.operation'),
      key: 'action',
      width: 100,
      render: (_: unknown, record: BleDeviceInfo) => {
        const isConnected = connections?.some(c => c.address === record.address);
        return (
          <Button
            type={isConnected ? 'default' : 'primary'}
            size="small"
            danger={isConnected}
            disabled={isConnected === false && record.isConnectable === false}
            onClick={() => onConnect(record.address)}
          >
            {isConnected ? t('button.disconnect') : t('button.connect')}
          </Button>
        );
      },
    },
  ];

  return (
    <Card
      title={t('title.deviceScan')}
      size="small"
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
      styles={{ body: { flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', padding: 8 }}}
      extra={
        <Space>
          <Flex align="center" gap={4}>
            <Text type="secondary">{t('label.scanDuration')}</Text>
            <InputNumber
              min={1}
              max={60}
              value={scanTimeout}
              onChange={(v) => setScanTimeout(v || 10)}
              style={{ width: 80 }}
              disabled={isScanning}
            />
            <Text type="secondary">{t('label.seconds')}</Text>
          </Flex>
          {isScanning ? (
            <Button
              type="primary"
              danger
              icon={<StopOutlined />}
              onClick={onStopScan}
            >
              {t('button.stop')}
            </Button>
          ) : (
            <Button
              type="primary"
              icon={<SearchOutlined />}
              onClick={handleScan}
            >
              {t('button.scan')}
            </Button>
          )}
        </Space>
      }
    >
      <Space vertical style={{ width: '100%', marginBottom: 16 }}>
        <Search
          placeholder={t('placeholder.filterByName')}
          value={filterName}
          onChange={(e) => setFilterName(e.target.value)}
          allowClear
          style={{ width: '100%' }}
        />
      </Space>

      {isScanning && (
        <div style={{ marginBottom: 16 }}>
          <Progress
            percent={100}
            status="active"
            showInfo={false}
            strokeColor="#1890ff"
          />
          <Text type="secondary">{t('status.scanningDevices')}</Text>
        </div>
      )}

      {(filteredDevices || []).length > 0 ? (
        <div style={{ flex: '1 1 0', minHeight: 0, overflow: 'auto' }}>
          <Table
            dataSource={filteredDevices}
            columns={columns}
            rowKey="address"
            size="small"
            pagination={false}
            style={{ height: '100%' }}
          />
        </div>
      ) : (
        <div style={{ flex: '1 1 0', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              isScanning ? t('status.scanningDevices') : t('placeholder.noDevicesClickScan')
            }
          />
        </div>
      )}
    </Card>
  );
};

export default BleScanner;
