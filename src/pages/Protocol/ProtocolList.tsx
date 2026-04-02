import React from 'react';
import { Card, Table, Tag, Button, Space, Tooltip, Popconfirm, Empty, Typography } from 'antd';
import {
  PlayCircleOutlined,
  StopOutlined,
  DeleteOutlined,
  ApiOutlined,
  EditOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { PluginInfo } from '../../api/types';
import { getPluginStateColor, getPluginStateText } from '../../stores/protocolStore';

const { Text } = Typography;

interface ProtocolListProps {
  protocols: PluginInfo[];
  loading: boolean;
  currentProtocol: string | null;
  onSelect: (pluginId: string) => void;
  onEnable: (pluginId: string) => void;
  onDisable: (pluginId: string) => void;
  onUnload: (pluginId: string) => void;
  onEdit: (pluginId: string) => void;
}

const ProtocolList: React.FC<ProtocolListProps> = ({
  protocols,
  loading,
  currentProtocol,
  onSelect,
  onEnable,
  onDisable,
  onUnload,
  onEdit,
}) => {
  const columns: ColumnsType<PluginInfo> = [
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
      render: (text: string, record: PluginInfo) => (
        <Button
          type={currentProtocol === record.id ? 'primary' : 'link'}
          onClick={() => onSelect(record.id)}
        >
          {text}
        </Button>
      ),
    },
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 120,
      render: (text: string) => <Text code>{text}</Text>,
    },
    {
      title: '版本',
      dataIndex: 'version',
      key: 'version',
      width: 80,
    },
    {
      title: '状态',
      dataIndex: 'state',
      key: 'state',
      width: 100,
      render: (state: PluginInfo['state']) => (
        <Tag color={getPluginStateColor(state)}>{getPluginStateText(state)}</Tag>
      ),
    },
    {
      title: '钩子',
      dataIndex: 'hooks',
      key: 'hooks',
      width: 150,
      render: (hooks: string[]) => (
        <Space size={4} wrap>
          {hooks.slice(0, 3).map((hook) => (
            <Tag key={hook} style={{ fontSize: 11 }}>
              {hook}
            </Tag>
          ))}
          {hooks.length > 3 && <Tag>+{hooks.length - 3}</Tag>}
        </Space>
      ),
    },
    {
      title: '绑定设备',
      dataIndex: 'bound_devices',
      key: 'bound_devices',
      width: 100,
      render: (devices: string[]) => (
        <Tooltip title={devices.length > 0 ? devices.join(', ') : '无绑定'}>
          <Tag icon={<ApiOutlined />} color={devices.length > 0 ? 'blue' : 'default'}>
            {devices.length}
          </Tag>
        </Tooltip>
      ),
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
      render: (text: string | null) => text || '-',
    },
    {
      title: '操作',
      key: 'actions',
      width: 180,
      render: (_: unknown, record: PluginInfo) => {
        const isLoaded = record.state !== 'Unloaded';
        const isEnabled = record.state === 'Enabled';

        return (
          <Space size="small">
            {isLoaded && (
              <>
                {isEnabled ? (
                  <Tooltip title="禁用">
                    <Button
                      type="text"
                      size="small"
                      icon={<StopOutlined />}
                      onClick={() => onDisable(record.id)}
                    />
                  </Tooltip>
                ) : (
                  <Tooltip title="启用">
                    <Button
                      type="text"
                      size="small"
                      icon={<PlayCircleOutlined />}
                      onClick={() => onEnable(record.id)}
                    />
                  </Tooltip>
                )}
                <Tooltip title="编辑">
                  <Button
                    type="text"
                    size="small"
                    icon={<EditOutlined />}
                    onClick={() => onEdit(record.id)}
                  />
                </Tooltip>
                <Popconfirm
                  title="确定要卸载此协议吗？"
                  onConfirm={() => onUnload(record.id)}
                  okText="确定"
                  cancelText="取消"
                >
                  <Tooltip title="卸载">
                    <Button type="text" size="small" danger icon={<DeleteOutlined />} />
                  </Tooltip>
                </Popconfirm>
              </>
            )}
          </Space>
        );
      },
    },
  ];

  return (
    <Card title="协议列表" size="small">
      <Table
        columns={columns}
        dataSource={protocols}
        rowKey="id"
        size="small"
        loading={loading}
        pagination={false}
        locale={{
          emptyText: (
            <Empty
              description="暂无协议"
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            >
              <Text type="secondary">点击上方"加载协议"按钮添加协议脚本</Text>
            </Empty>
          ),
        }}
        scroll={{ y: 400 }}
      />
    </Card>
  );
};

export default ProtocolList;
