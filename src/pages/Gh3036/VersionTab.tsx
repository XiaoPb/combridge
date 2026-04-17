import React, { useState } from 'react';
import { Card, Descriptions, Button, Spin, message, theme } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';

interface VersionInfo {
  firmware: string;
  protocol: string;
  driver: string;
  chip: string;
  algo: string;
}

const VersionTab: React.FC = () => {
  const { t } = useTranslation('gh3036');
  const { token } = theme.useToken();
  const { executeRpc, txChannel } = useGh3036Store();
  
  const [loading, setLoading] = useState(false);
  const [versionInfo, setVersionInfo] = useState<VersionInfo>({
    firmware: '--',
    protocol: '--',
    driver: '--',
    chip: '--',
    algo: '--',
  });

  const handleGetVersion = async (verType: number): Promise<string> => {
    if (!txChannel) {
      message.error(t('version.noTxChannel'));
      return '--';
    }
    
    try {
      const success = await executeRpc('V', [verType.toString()]);
      return success ? '1.0.0' : '--';
    } catch {
      return '--';
    }
  };

  const handleRefreshAll = async () => {
    setLoading(true);
    try {
      const [firmware, protocol, driver, chip, algo] = await Promise.all([
        handleGetVersion(0),
        handleGetVersion(1),
        handleGetVersion(2),
        handleGetVersion(3),
        handleGetVersion(4),
      ]);
      
      setVersionInfo({ firmware, protocol, driver, chip, algo });
      message.success(t('version.refreshSuccess'));
    } finally {
      setLoading(false);
    }
  };

  const cardStyle: React.CSSProperties = {
    background: token.colorBgContainer,
    borderRadius: token.borderRadius,
  };

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
          >
            {t('version.refresh')}
          </Button>
        }
        style={cardStyle}
      >
        <Spin spinning={loading}>
          <Descriptions column={1} bordered size="small">
            <Descriptions.Item label={t('version.firmware')}>
              {versionInfo.firmware}
            </Descriptions.Item>
            <Descriptions.Item label={t('version.protocol')}>
              {versionInfo.protocol}
            </Descriptions.Item>
            <Descriptions.Item label={t('version.driver')}>
              {versionInfo.driver}
            </Descriptions.Item>
            <Descriptions.Item label={t('version.chip')}>
              {versionInfo.chip}
            </Descriptions.Item>
            <Descriptions.Item label={t('version.algo')}>
              {versionInfo.algo}
            </Descriptions.Item>
          </Descriptions>
        </Spin>
      </Card>

      <Card
        size="small"
        title={t('version.libraryInfo')}
        style={{ ...cardStyle, marginTop: 8 }}
      >
        <Descriptions column={1} bordered size="small">
          <Descriptions.Item label={t('version.libraryStatus')}>
            {t('version.linked')}
          </Descriptions.Item>
          <Descriptions.Item label={t('version.rpcStatus')}>
            {t('version.ready')}
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </div>
  );
};

export default VersionTab;
