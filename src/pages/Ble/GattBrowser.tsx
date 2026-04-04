import React, { useState } from 'react';
import { Card, Tree, Empty, Tag, Space, Typography, Spin } from 'antd';
import { FolderOutlined, FileTextOutlined, SettingOutlined } from '@ant-design/icons';
import type { BleService, BleCharacteristic } from '../../types';
import { getShortUuid } from '../../stores/bleStore';
import { getServiceName, getCharacteristicName } from '../../types/ble';

const { Text } = Typography;

interface GattBrowserProps {
  services: BleService[];
  loading?: boolean;
  onServiceSelect: (serviceUuid: string) => void;
  onCharacteristicSelect: (characteristic: BleCharacteristic) => void;
}

const GattBrowser: React.FC<GattBrowserProps> = ({
  services,
  loading,
  onServiceSelect,
  onCharacteristicSelect,
}) => {
  const [selectedKeys, setSelectedKeys] = useState<React.Key[]>([]);

  const getPropertyTags = (properties: BleCharacteristic['properties']) => {
    const tags: { color: string; label: string }[] = [];
    if (properties.read) tags.push({ color: 'green', label: 'R' });
    if (properties.write) tags.push({ color: 'blue', label: 'W' });
    if (properties.writeWithoutResponse) tags.push({ color: 'cyan', label: 'W' });
    if (properties.notify) tags.push({ color: 'orange', label: 'N' });
    if (properties.indicate) tags.push({ color: 'purple', label: 'I' });
    return tags;
  };

  const treeData = services.map((service) => ({
    key: service.uuid,
    title: (
      <Space>
        <FolderOutlined style={{ color: '#1890ff' }} />
        <Text strong>{getServiceName(service.uuid)} (0x{getShortUuid(service.uuid)})</Text>
        {service.isPrimary && <Tag color="blue">Primary</Tag>}
      </Space>
    ),
    children: service.characteristics.map((char) => ({
      key: `${service.uuid}-${char.uuid}`,
      title: (
        <Space>
          <FileTextOutlined style={{ color: '#52c41a' }} />
          <Text>{getCharacteristicName(char.uuid)} (0x{getShortUuid(char.uuid)})</Text>
          {getPropertyTags(char.properties).map((tag) => (
            <Tag key={tag.label} color={tag.color} style={{ fontSize: '10px', padding: '0 4px' }}>
              {tag.label}
            </Tag>
          ))}
        </Space>
      ),
      isLeaf: true,
    })),
  }));

  const handleSelect = (keys: React.Key[]) => {
    setSelectedKeys(keys);
    if (keys.length > 0) {
      const key = keys[0] as string;
      for (const service of services) {
        const char = service.characteristics.find((c) => `${service.uuid}-${c.uuid}` === key);
        if (char) {
          onCharacteristicSelect(char);
          return;
        }
      }
      const service = services.find((s) => s.uuid === key);
      if (service) {
        onServiceSelect(service.uuid);
      }
    }
  };

  if (loading) {
    return (
      <Card title="GATT 服务浏览器" size="small">
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Spin tip="正在发现服务..." />
        </div>
      </Card>
    );
  }

  if (services.length === 0) {
    return (
      <Card title="GATT 服务浏览器" size="small">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无服务，请先连接设备并发现服务"
        />
      </Card>
    );
  }

  return (
    <Card
      title={
        <Space>
          <SettingOutlined />
          <span>GATT 服务浏览器</span>
          <Tag color="blue">{services.length} 个服务</Tag>
        </Space>
      }
      size="small"
    >
      <Tree
        showLine
        selectedKeys={selectedKeys}
        treeData={treeData}
        onSelect={handleSelect}
        style={{ fontSize: '13px' }}
      />
    </Card>
  );
};

export default GattBrowser;
