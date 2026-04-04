import React, { useState } from 'react';
import { Card, Button, Input, Table, Space, Tag, Progress, Empty, InputNumber, Typography, Flex } from 'antd';
import { SearchOutlined, StopOutlined } from '@ant-design/icons';
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
      title: '设备名称',
      dataIndex: 'name',
      key: 'name',
      render: (name: string) => name || <Text type="secondary">未命名</Text>,
    },
    {
      title: 'MAC 地址',
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
      title: '信号强度',
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
      title: '状态',
      key: 'status',
      width: 80,
      render: (_: unknown, record: BleDeviceInfo) => {
        const isConnected = connections?.some(c => c.address === record.address);
        return (
          <Tag color={isConnected ? 'green' : 'default'}>
            {isConnected ? '已连接' : '未连接'}
          </Tag>
        );
      },
    },
    {
      title: '操作',
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
            {isConnected ? '断开' : '连接'}
          </Button>
        );
      },
    },
  ];

  return (
    <Card
      title="设备扫描"
      size="small"
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
      styles={{ body: { flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', padding: 8 }}}
      extra={
        <Space>
          <Flex align="center" gap={4}>
            <Text type="secondary">扫描时长</Text>
            <InputNumber
              min={1}
              max={60}
              value={scanTimeout}
              onChange={(v) => setScanTimeout(v || 10)}
              style={{ width: 80 }}
              disabled={isScanning}
            />
            <Text type="secondary">秒</Text>
          </Flex>
          {isScanning ? (
            <Button
              type="primary"
              danger
              icon={<StopOutlined />}
              onClick={onStopScan}
            >
              停止
            </Button>
          ) : (
            <Button
              type="primary"
              icon={<SearchOutlined />}
              onClick={handleScan}
            >
              扫描
            </Button>
          )}
        </Space>
      }
    >
      <Space vertical style={{ width: '100%', marginBottom: 16 }}>
        <Search
          placeholder="按名称过滤设备"
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
          <Text type="secondary">正在扫描中...</Text>
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
              isScanning ? '正在扫描设备...' : '暂无设备，点击扫描按钮开始'
            }
          />
        </div>
      )}
    </Card>
  );
};

export default BleScanner;
