import React, { useState, useEffect } from 'react';
import { Card, Table, Button, Space, Select, Tag, Empty, Typography, Popconfirm, message } from 'antd';
import { LinkOutlined, DisconnectOutlined, PlusOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { PluginInfo } from '../../api/types';
import { useSerialStore } from '../../stores/serialStore';

const { Text } = Typography;

interface BindingItem {
  key: string;
  pluginId: string;
  pluginName: string;
  deviceId: string;
  deviceName: string;
  boundAt: number;
}

interface BindConfigProps {
  protocols: PluginInfo[];
  onBind: (pluginId: string, deviceId: string) => Promise<boolean>;
  onUnbind: (pluginId: string, deviceId: string) => Promise<boolean>;
}

const BindConfig: React.FC<BindConfigProps> = ({
  protocols,
  onBind,
  onUnbind,
}) => {
  const [bindings, setBindings] = useState<BindingItem[]>([]);
  const [selectedPlugin, setSelectedPlugin] = useState<string | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [isBinding, setIsBinding] = useState(false);

  const { ports, openPorts } = useSerialStore();

  useEffect(() => {
    const items: BindingItem[] = [];
    protocols.forEach((protocol) => {
      protocol.bound_devices.forEach((deviceId) => {
        items.push({
          key: `${protocol.id}-${deviceId}`,
          pluginId: protocol.id,
          pluginName: protocol.name,
          deviceId,
          deviceName: deviceId,
          boundAt: Date.now(),
        });
      });
    });
    setBindings(items);
  }, [protocols]);

  const availableDevices = openPorts.length > 0
    ? openPorts.map((p) => ({ value: p.portName, label: p.portName }))
    : ports.map((p) => ({ value: p.portName, label: p.portName }));

  const availablePlugins = protocols
    .filter((p) => p.state === 'Enabled')
    .map((p) => ({ value: p.id, label: `${p.name} (${p.id})` }));

  const handleBind = async () => {
    if (!selectedPlugin || !selectedDevice) {
      message.warning('请选择协议和设备');
      return;
    }

    const exists = bindings.find(
      (b) => b.pluginId === selectedPlugin && b.deviceId === selectedDevice
    );
    if (exists) {
      message.warning('该绑定已存在');
      return;
    }

    setIsBinding(true);
    const success = await onBind(selectedPlugin, selectedDevice);
    if (success) {
      setSelectedPlugin(null);
      setSelectedDevice(null);
    }
    setIsBinding(false);
  };

  const handleUnbind = async (pluginId: string, deviceId: string) => {
    await onUnbind(pluginId, deviceId);
  };

  const columns: ColumnsType<BindingItem> = [
    {
      title: '协议',
      dataIndex: 'pluginName',
      key: 'pluginName',
      render: (text: string, record: BindingItem) => (
        <Space>
          <Tag color="blue">{text}</Tag>
          <Text type="secondary" style={{ fontSize: 11 }}>
            {record.pluginId}
          </Text>
        </Space>
      ),
    },
    {
      title: '设备',
      dataIndex: 'deviceName',
      key: 'deviceName',
      render: (text: string) => <Tag color="green">{text}</Tag>,
    },
    {
      title: '操作',
      key: 'actions',
      width: 100,
      render: (_: unknown, record: BindingItem) => (
        <Popconfirm
          title="确定要解绑吗？"
          onConfirm={() => handleUnbind(record.pluginId, record.deviceId)}
          okText="确定"
          cancelText="取消"
        >
          <Button type="text" size="small" danger icon={<DisconnectOutlined />}>
            解绑
          </Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <Card
      title={
        <Space>
          <LinkOutlined />
          <span>协议绑定配置</span>
        </Space>
      }
      size="small"
    >
      <Space vertical style={{ width: '100%' }} size="middle">
        <Space wrap>
          <Select
            placeholder="选择协议"
            value={selectedPlugin}
            onChange={setSelectedPlugin}
            style={{ width: 200 }}
            options={availablePlugins}
            disabled={availablePlugins.length === 0}
          />
          <Select
            placeholder="选择设备"
            value={selectedDevice}
            onChange={setSelectedDevice}
            style={{ width: 200 }}
            options={availableDevices}
            disabled={availableDevices.length === 0}
          />
          <Button
            type="primary"
            icon={<PlusOutlined />}
            loading={isBinding}
            disabled={!selectedPlugin || !selectedDevice}
            onClick={handleBind}
          >
            绑定
          </Button>
        </Space>

        {availablePlugins.length === 0 && (
          <Text type="secondary" style={{ fontSize: 12 }}>
            暂无已启用的协议，请先启用协议后再进行绑定
          </Text>
        )}

        {availableDevices.length === 0 && (
          <Text type="secondary" style={{ fontSize: 12 }}>
            暂无可用设备，请先打开串口或连接设备
          </Text>
        )}

        <Table
          columns={columns}
          dataSource={bindings}
          size="small"
          pagination={false}
          locale={{
            emptyText: (
              <Empty
                description="暂无绑定"
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              >
                <Text type="secondary">选择协议和设备后点击"绑定"按钮</Text>
              </Empty>
            ),
          }}
          scroll={{ y: 200 }}
        />
      </Space>
    </Card>
  );
};

export default BindConfig;
