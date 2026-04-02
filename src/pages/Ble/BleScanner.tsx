import React, { useState } from 'react';
import { Card, Button, Input, Table, Space, Tag, Progress, Empty, InputNumber } from 'antd';
import { SearchOutlined, StopOutlined } from '@ant-design/icons';
import type { BleDeviceInfo } from '../../types';
import { formatBleTimestamp } from '../../stores/bleStore';

const { Search } = Input;

interface BleScannerProps {
  devices: BleDeviceInfo[];
  isScanning: boolean;
  onScan: (timeout?: number) => void;
  onStopScan: () => void;
  onConnect: (address: string) => void;
}

const BleScanner: React.FC<BleScannerProps> = ({
  devices,
  isScanning,
  onScan,
  onStopScan,
  onConnect,
}) => {
  const [filterName, setFilterName] = useState('');
  const [scanTimeout, setScanTimeout] = useState(10);

  const filteredDevices = devices.filter((device) => {
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
      render: (address: string) => (
        <Text code style={{ fontSize: '12px' }}>
          {address}
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
          <Space direction="vertical" size={0} style={{ width: '100%' }}>
            <Tag color={getRssiColor(rssi)}>{rssi} dBm</Tag>
            <Progress
              percent={getRssiPercent(rssi)}
              size="small"
              showInfo={false}
              strokeColor={getRssiColor(rssi) === 'green' ? '#52c41a' : getRssiColor(rssi) === 'blue' ? '#1890ff' : getRssiColor(rssi) === 'orange' ? '#fa8c16' : '#f5222d'}
            />
          </Space>
        ) : (
          <Text type="secondary">-</Text>
        ),
    },
    {
      title: '可连接',
      dataIndex: 'isConnectable',
      key: 'isConnectable',
      width: 80,
      render: (connectable: boolean) => (
        <Tag color={connectable ? 'green' : 'red'}>
          {connectable ? '是' : '否'}
        </Tag>
      ),
    },
    {
      title: '发现时间',
      dataIndex: 'discoveredAt',
      key: 'discoveredAt',
      width: 120,
      render: (time: number) => formatBleTimestamp(time),
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      render: (_: unknown, record: BleDeviceInfo) => (
        <Button
          type="primary"
          size="small"
          disabled={!record.isConnectable}
          onClick={() => onConnect(record.address)}
        >
          连接
        </Button>
      ),
    },
  ];

  return (
    <Card
      title="设备扫描"
      size="small"
      extra={
        <Space>
          <InputNumber
            min={1}
            max={60}
            value={scanTimeout}
            onChange={(v) => setScanTimeout(v || 10)}
            addonAfter="秒"
            style={{ width: 100 }}
            disabled={isScanning}
          />
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
      <Space direction="vertical" style={{ width: '100%', marginBottom: 16 }}>
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

      {filteredDevices.length > 0 ? (
        <Table
          dataSource={filteredDevices}
          columns={columns}
          rowKey="address"
          size="small"
          pagination={{ pageSize: 10, showSizeChanger: false }}
        />
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            isScanning ? '正在扫描设备...' : '暂无设备，点击扫描按钮开始'
          }
        />
      )}
    </Card>
  );
};

const Text = Typography.Text;
import { Typography } from 'antd';

export default BleScanner;
