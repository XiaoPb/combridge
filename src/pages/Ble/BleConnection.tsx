import React from 'react';
import { Card, List, Button, Tag, Space, Typography, Empty, Descriptions } from 'antd';
import { LinkOutlined, DisconnectOutlined } from '@ant-design/icons';
import type { BleConnection } from '../../types';
import { formatBleTimestamp } from '../../stores/bleStore';

const { Text } = Typography;

interface BleConnectionProps {
  connections: BleConnection[];
  currentDevice: string | null;
  onSelect: (deviceId: string) => void;
  onDisconnect: (deviceId: string) => void;
}

const BleConnection: React.FC<BleConnectionProps> = ({
  connections,
  currentDevice,
  onSelect,
  onDisconnect,
}) => {
  const formatDuration = (connectedAt?: number): string => {
    if (!connectedAt) return '-';
    const seconds = Math.floor((Date.now() - connectedAt) / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
      return `${hours}小时${minutes % 60}分钟`;
    }
    if (minutes > 0) {
      return `${minutes}分钟${seconds % 60}秒`;
    }
    return `${seconds}秒`;
  };

  if (connections.length === 0) {
    return (
      <Card title="连接管理" size="small">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无连接"
        />
      </Card>
    );
  }

  return (
    <Card title="连接管理" size="small">
      <List
        dataSource={connections}
        renderItem={(connection) => {
          const deviceId = connection.deviceId || connection.address;
          return (
            <List.Item
              key={deviceId}
              actions={[
                <Button
                  key="select"
                  type="link"
                  size="small"
                  icon={<LinkOutlined />}
                  onClick={() => onSelect(deviceId)}
                  disabled={currentDevice === deviceId}
                >
                  选择
                </Button>,
                <Button
                  key="disconnect"
                  type="link"
                  size="small"
                  danger
                  icon={<DisconnectOutlined />}
                  onClick={() => onDisconnect(deviceId)}
                >
                  断开
                </Button>,
              ]}
              style={{
                backgroundColor: currentDevice === deviceId ? '#f0f5ff' : undefined,
                padding: '8px 12px',
                borderRadius: 4,
              }}
            >
              <List.Item.Meta
                title={
                  <Space>
                    <Text strong>{connection.name || '未命名设备'}</Text>
                    <Tag color={connection.isConnected ? 'green' : 'red'}>
                      {connection.isConnected ? '已连接' : '已断开'}
                    </Tag>
                    {currentDevice === deviceId && (
                      <Tag color="blue">当前</Tag>
                    )}
                  </Space>
                }
                description={
                  <Descriptions size="small" column={2}>
                    <Descriptions.Item label="地址">
                      <Text code style={{ fontSize: '11px' }}>
                        {connection.address}
                      </Text>
                    </Descriptions.Item>
                    <Descriptions.Item label="MTU">
                      {connection.mtu || '默认'}
                    </Descriptions.Item>
                    <Descriptions.Item label="连接时间">
                      {formatBleTimestamp(connection.connectedAt || 0)}
                    </Descriptions.Item>
                    <Descriptions.Item label="持续时间">
                      {formatDuration(connection.connectedAt)}
                    </Descriptions.Item>
                  </Descriptions>
                }
              />
            </List.Item>
          );
        }}
      />
    </Card>
  );
};

export default BleConnection;
