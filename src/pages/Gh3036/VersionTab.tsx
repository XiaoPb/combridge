import React, { useState, useEffect } from 'react';
import { Card, Table, Button, Space, message, theme, Tag } from 'antd';
import { ReloadOutlined, CheckCircleOutlined, CloseCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { gh3036Api } from '../../api/gh3036';
import type { Gh3036VersionTypeConfig } from '../../api/types';
import { formatErrorMessage } from '../../utils/errorMessage';

interface VersionInfo {
  [key: string]: string;
}

const VersionTab: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const { token } = theme.useToken();
  const { txChannel } = useGh3036Store();
  
  const [loading, setLoading] = useState(false);
  const [versionTypes, setVersionTypes] = useState<Gh3036VersionTypeConfig[]>([]);
  const [versionInfo, setVersionInfo] = useState<VersionInfo>({});
  const [refreshingKey, setRefreshingKey] = useState<string | null>(null);

  useEffect(() => {
    loadVersionTypes();
  }, []);

  const loadVersionTypes = async () => {
    try {
      const types = await gh3036Api.getVersionTypes();
      setVersionTypes(types);
      
      const initialInfo: VersionInfo = {};
      types.forEach(type => {
        initialInfo[type.name] = '--';
      });
      setVersionInfo(initialInfo);
    } catch (err) {
      console.error('加载版本类型配置失败:', err);
    }
  };

  const handleGetVersion = async (typeConfig: Gh3036VersionTypeConfig): Promise<string> => {
    if (!txChannel) {
      throw new Error(t('version.noTxChannel'));
    }
    
    const result = await gh3036Api.executeRpc('V', [typeConfig.type_value.toString()]);
    const versionStr = String.fromCharCode(...result.filter(c => c >= 32 && c < 127));
    return versionStr || '--';
  };

  const handleRefreshOne = async (typeConfig: Gh3036VersionTypeConfig) => {
    setRefreshingKey(typeConfig.name);
    try {
      const version = await handleGetVersion(typeConfig);
      setVersionInfo(prev => ({
        ...prev,
        [typeConfig.name]: version,
      }));
      message.success(`${typeConfig.description}: ${version}`);
    } catch (err) {
      const errorMsg = formatErrorMessage(err, t('errors.getVersion'));
      message.error(errorMsg);
      setVersionInfo(prev => ({
        ...prev,
        [typeConfig.name]: t('errors.getVersion'),
      }));
    } finally {
      setRefreshingKey(null);
    }
  };

  const handleRefreshAll = async () => {
    if (!txChannel) {
      message.error(t('version.noTxChannel'));
      return;
    }

    setLoading(true);
    const newVersionInfo: VersionInfo = { ...versionInfo };
    
    for (const typeConfig of versionTypes) {
      try {
        const version = await handleGetVersion(typeConfig);
        newVersionInfo[typeConfig.name] = version;
      } catch {
        newVersionInfo[typeConfig.name] = t('errors.getVersion');
      }
    }
    
    setVersionInfo(newVersionInfo);
    setLoading(false);
    message.success(t('version.refreshSuccess'));
  };

  const cardStyle: React.CSSProperties = {
    background: token.colorBgContainer,
    borderRadius: token.borderRadius,
  };

  const columns = [
    {
      title: t('version.typeName'),
      dataIndex: 'name',
      key: 'name',
      width: 120,
      render: (name: string) => <Tag color="blue">{name}</Tag>,
    },
    {
      title: t('version.typeValue'),
      dataIndex: 'type_value',
      key: 'type_value',
      width: 80,
      render: (value: number) => `0x${value.toString(16).toUpperCase().padStart(2, '0')}`,
    },
    {
      title: t('version.description'),
      dataIndex: 'description',
      key: 'description',
      width: 150,
    },
    {
      title: t('version.version'),
      key: 'version',
      render: (_: unknown, record: Gh3036VersionTypeConfig) => {
        const version = versionInfo[record.name] || '--';
        const isSuccess = version !== '--' && version !== t('errors.getVersion');
        return (
          <Space>
            {isSuccess ? (
              <CheckCircleOutlined style={{ color: token.colorSuccess }} />
            ) : (
              <CloseCircleOutlined style={{ color: token.colorTextDisabled }} />
            )}
            <span style={{ fontFamily: 'monospace' }}>{version}</span>
          </Space>
        );
      },
    },
    {
      title: t('version.action'),
      key: 'action',
      width: 100,
      render: (_: unknown, record: Gh3036VersionTypeConfig) => (
        <Button
          size="small"
          icon={<ReloadOutlined />}
          onClick={() => handleRefreshOne(record)}
          loading={refreshingKey === record.name}
          disabled={!txChannel}
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
          <Button
            type="primary"
            icon={<ReloadOutlined />}
            onClick={handleRefreshAll}
            loading={loading}
            size="small"
            disabled={!txChannel}
          >
            {t('version.refreshAll')}
          </Button>
        }
        style={cardStyle}
      >
        <Table
          columns={columns}
          dataSource={versionTypes}
          rowKey="name"
          size="small"
          pagination={false}
          loading={loading}
        />
      </Card>

      <Card
        size="small"
        title={t('version.libraryInfo')}
        style={{ ...cardStyle, marginTop: 8 }}
      >
        <Space orientation="vertical" style={{ width: '100%' }}>
          <div>
            <Tag color="green">{t('version.linked')}</Tag>
            <Tag color="blue">{t('version.ready')}</Tag>
          </div>
          {!txChannel && (
            <Tag color="orange">{t('version.noTxChannel')}</Tag>
          )}
        </Space>
      </Card>
    </div>
  );
};

export default VersionTab;
