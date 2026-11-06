import React, { useEffect, useState } from 'react';
import { Alert, Button, Card, Select, Space, Table, Tag, Typography, message, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3220Store } from '../../stores/gh3220Store';
import { gh3220Api } from '../../api/gh3220';
import type { Gh3220DeviceInfo } from '../../types/gh3220';
import { formatErrorMessage } from '../../utils/errorMessage';
import { GH3220_SELECT_OPTIONS } from '../../constants/gh3220CommandSchema';
import { resolveGh3220Lang } from '../../constants/gh3220Functions';

interface VersionRow {
  key: string;
  typeName: string;
  typeValue: string;
  description: string;
  /** deviceInfo 中的取值字段 */
  field: keyof Omit<Gh3220DeviceInfo, 'linkOk' | 'maxLen' | 'openedFunc'>;
  version: string;
}

const ROW_TEMPLATE: Array<Omit<VersionRow, 'version'>> = [
  {
    key: 'fw',
    typeName: 'Firmware',
    typeValue: '0x01',
    description: 'version.fwDesc',
    field: 'fwVersion',
  },
  {
    key: 'protocol',
    typeName: 'Protocol',
    typeValue: '0x0E',
    description: 'version.protocolDesc',
    field: 'protocolVersion',
  },
  {
    key: 'chip',
    typeName: 'Chip',
    typeValue: '0x11',
    description: 'version.chipDesc',
    field: 'chipVersion',
  },
  {
    key: 'driver',
    typeName: 'Driver',
    typeValue: '0x10',
    description: 'version.driverDesc',
    field: 'driverVersion',
  },
];

const VersionTab: React.FC = () => {
  const { t, i18n } = useTranslation('gh3220');
  const lang = resolveGh3220Lang(i18n.language);
  const { token } = theme.useToken();
  const { status, deviceInfo, loadDeviceInfo } = useGh3220Store();

  const [rows, setRows] = useState<VersionRow[]>(
    ROW_TEMPLATE.map((row) => ({ ...row, version: '--' }))
  );
  const [refreshingKey, setRefreshingKey] = useState<string | null>(null);
  const [loadingAll, setLoadingAll] = useState(false);
  // V3：版本类型下拉只做定位/高亮，不改后端读取逻辑（GET_VER 仍是完整读取 4 项）
  const [selectedType, setSelectedType] = useState('0x01');

  const connected = status.state !== 'disconnected' && status.state !== 'connecting';
  const matchedRow = rows.find(
    (row) => row.typeValue.toLowerCase() === selectedType.toLowerCase()
  );

  // 设备信息变化时同步版本行（连接后 index 骨架已触发 loadDeviceInfo）
  useEffect(() => {
    if (!deviceInfo) return;
    setRows((current) =>
      current.map((row) => ({ ...row, version: (deviceInfo[row.field] as string) || '--' }))
    );
  }, [deviceInfo]);

  const handleRefreshOne = async (row: VersionRow) => {
    if (!connected) {
      message.warning(t('version.noDevice'));
      return;
    }
    setRefreshingKey(row.key);
    try {
      // 单项刷新借用一次完整设备信息读取（UProtocol GET_VER 无独立命令封装）
      const info = await gh3220Api.getDeviceInfo();
      useGh3220Store.setState({ deviceInfo: info });
      setRows((current) =>
        current.map((item) =>
          item.key === row.key ? { ...item, version: (info[row.field] as string) || '--' } : item
        )
      );
    } catch (err) {
      const errorMsg = formatErrorMessage(err, t('version.readFailed'));
      message.error(errorMsg);
      setRows((current) =>
        current.map((item) => (item.key === row.key ? { ...item, version: t('version.readFailed') } : item))
      );
    } finally {
      setRefreshingKey(null);
    }
  };

  const handleRefreshAll = async () => {
    if (!connected) {
      message.warning(t('version.noDevice'));
      return;
    }
    setLoadingAll(true);
    try {
      await loadDeviceInfo();
      message.success(t('version.refreshSuccess'));
    } catch (err) {
      message.error(formatErrorMessage(err, t('version.readFailed')));
    } finally {
      setLoadingAll(false);
    }
  };

  const columns: ColumnsType<VersionRow> = [
    {
      title: t('version.typeName'),
      dataIndex: 'typeName',
      key: 'typeName',
      width: 120,
      render: (name: string) => <Tag color="blue">{name}</Tag>,
    },
    {
      title: t('version.typeValue'),
      dataIndex: 'typeValue',
      key: 'typeValue',
      width: 90,
      render: (value: string) => <code>{value}</code>,
    },
    {
      title: t('version.description'),
      dataIndex: 'description',
      key: 'description',
      width: 180,
      render: (desc: string) => t(desc),
    },
    {
      title: t('version.version'),
      dataIndex: 'version',
      key: 'version',
      render: (version: string) => {
        const isSuccess = version !== '--' && version !== t('version.readFailed');
        return (
          <Space>
            {isSuccess ? (
              <CheckCircleOutlined style={{ color: token.colorSuccess }} />
            ) : (
              <CloseCircleOutlined style={{ color: token.colorTextDisabled }} />
            )}
            <Typography.Text code>{version}</Typography.Text>
          </Space>
        );
      },
    },
    {
      title: t('version.action'),
      key: 'action',
      width: 100,
      render: (_: unknown, record: VersionRow) => (
        <Button
          size="small"
          icon={<ReloadOutlined />}
          loading={refreshingKey === record.key}
          disabled={!connected}
          onClick={() => handleRefreshOne(record)}
        >
          {t('version.refresh')}
        </Button>
      ),
    },
  ];

  return (
    <div style={{ height: '100%', overflow: 'auto', padding: '8px 0' }}>
      <Card
        size="small"
        title={t('version.title')}
        extra={
          <Space size={8}>
            <Select
              size="small"
              style={{ width: 230 }}
              value={selectedType}
              options={GH3220_SELECT_OPTIONS.versionType(lang)}
              onChange={setSelectedType}
            />
            <Button
              type="primary"
              icon={<ReloadOutlined />}
              onClick={handleRefreshAll}
              loading={loadingAll}
              size="small"
              disabled={!connected}
            >
              {t('version.refreshAll')}
            </Button>
          </Space>
        }
        style={{ background: token.colorBgContainer, borderRadius: token.borderRadius }}
      >
        <Space direction="vertical" size={8} style={{ width: '100%', marginBottom: 8 }}>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {t('version.locateHint')}
          </Typography.Text>
          {!matchedRow ? (
            <Alert
              type="info"
              showIcon
              message={`${selectedType} · ${t('version.typeSelect')}`}
            />
          ) : null}
        </Space>
        <Table<VersionRow>
          columns={columns}
          dataSource={rows}
          rowKey="key"
          size="small"
          pagination={false}
          onRow={(record) => ({
            style:
              record.typeValue.toLowerCase() === selectedType.toLowerCase()
                ? { background: token.colorPrimaryBg, fontWeight: 600 }
                : {},
          })}
        />
      </Card>

      <Card
        size="small"
        title={t('info.title')}
        style={{
          marginTop: 8,
          background: token.colorBgContainer,
          borderRadius: token.borderRadius,
        }}
      >
        {deviceInfo ? (
          <Space size={8} wrap>
            <Tag color="blue">{`${t('info.maxLen')}: ${deviceInfo.maxLen}`}</Tag>
            <Tag color="blue">{`0x${(deviceInfo.openedFunc >>> 0).toString(16)}`}</Tag>
            <Tag color={deviceInfo.linkOk ? 'success' : 'error'}>
              {deviceInfo.linkOk ? t('info.linkOk') : t('info.linkDown')}
            </Tag>
          </Space>
        ) : (
          <Typography.Text type="secondary">{t('info.empty')}</Typography.Text>
        )}
      </Card>
    </div>
  );
};

export default VersionTab;
