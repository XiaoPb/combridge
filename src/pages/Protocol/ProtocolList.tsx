import React from 'react';
import { Card, Table, Tag, Button, Space, Tooltip, Popconfirm, Empty, Typography } from 'antd';
import {
  PlayCircleOutlined,
  StopOutlined,
  DeleteOutlined,
  ApiOutlined,
  EditOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation('protocol');

  const columns: ColumnsType<PluginInfo> = [
    {
      title: t('label.name'),
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
      title: t('common:version'),
      dataIndex: 'version',
      key: 'version',
      width: 80,
    },
    {
      title: t('label.state'),
      dataIndex: 'state',
      key: 'state',
      width: 100,
      render: (state: PluginInfo['state']) => (
        <Tag color={getPluginStateColor(state)}>{getPluginStateText(state)}</Tag>
      ),
    },
    {
      title: t('label.hooks'),
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
      title: t('label.boundDevices'),
      dataIndex: 'bound_devices',
      key: 'bound_devices',
      width: 100,
      render: (devices: string[]) => (
        <Tooltip title={devices.length > 0 ? devices.join(', ') : t('message.noBinding')}>
          <Tag icon={<ApiOutlined />} color={devices.length > 0 ? 'blue' : 'default'}>
            {devices.length}
          </Tag>
        </Tooltip>
      ),
    },
    {
      title: t('label.description'),
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
      render: (text: string | null) => text || '-',
    },
    {
      title: t('common:operation'),
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
                  <Tooltip title={t('tooltip.disable')}>
                    <Button
                      type="text"
                      size="small"
                      icon={<StopOutlined />}
                      onClick={() => onDisable(record.id)}
                    />
                  </Tooltip>
                ) : (
                  <Tooltip title={t('tooltip.enable')}>
                    <Button
                      type="text"
                      size="small"
                      icon={<PlayCircleOutlined />}
                      onClick={() => onEnable(record.id)}
                    />
                  </Tooltip>
                )}
                <Tooltip title={t('tooltip.edit')}>
                  <Button
                    type="text"
                    size="small"
                    icon={<EditOutlined />}
                    onClick={() => onEdit(record.id)}
                  />
                </Tooltip>
                <Popconfirm
                  title={t('message.confirmUnload')}
                  onConfirm={() => onUnload(record.id)}
                  okText={t('common:confirm')}
                  cancelText={t('common:cancel')}
                >
                  <Tooltip title={t('tooltip.unload')}>
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
    <Card title={t('title.protocolList')} size="small">
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
              description={t('label.noProtocol')}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            >
              <Text type="secondary">{t('message.clickToAddProtocol')}</Text>
            </Empty>
          ),
        }}
        scroll={{ y: 400 }}
      />
    </Card>
  );
};

export default ProtocolList;
