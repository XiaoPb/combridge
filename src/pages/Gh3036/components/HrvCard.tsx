import React from 'react';
import { Card, Typography, theme, Progress, Space } from 'antd';
import { HeartTwoTone } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../../stores/gh3036Store';

const { Text } = Typography;

const HrvCard: React.FC = () => {
  const { token } = theme.useToken();
  const { t } = useTranslation('gh3036');
  const { vitalSigns } = useGh3036Store();

  const getConfidenceColor = (conf: number) => {
    if (conf >= 75) return token.colorSuccess;
    if (conf >= 25) return token.colorWarning;
    return token.colorError;
  };

  const getConfidenceText = (conf: number | null): string => {
    if (conf === null) return '--';
    if (conf === 0) return t('monitor.hrvConfidence0') || '不可信';
    if (conf === 25) return t('monitor.hrvConfidence25') || '低置信度';
    if (conf === 75) return t('monitor.hrvConfidence75') || '高置信度';
    if (conf === 100) return t('monitor.hrvConfidence100') || '可信';
    return `${conf}%`;
  };

  return (
    <Card
      size="small"
      style={{
        height: '100%',
        borderLeft: `3px solid ${token.colorPrimary}`,
      }}
      styles={{
        body: {
          padding: '12px 16px',
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          height: 'calc(100% - 1px)',
        },
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <HeartTwoTone twoToneColor={token.colorPrimary} />
        <Text type="secondary" style={{ fontSize: 12 }}>
          {t('monitor.hrv')}
        </Text>
      </div>
      
      <div style={{ flex: 1 }}>
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Text type="secondary" style={{ fontSize: 11 }}>{t('monitor.hrvRri')}</Text>
            <Text style={{ fontSize: 11, color: token.colorTextSecondary }}>
              {t('monitor.hrvRriCount')}: {vitalSigns.hrvRriCount ?? '--'}
            </Text>
          </div>
          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
            {vitalSigns.hrvRri.map((rri, index) => (
              <div
                key={index}
                style={{
                  padding: '2px 6px',
                  borderRadius: 4,
                  backgroundColor: rri > 0 ? token.colorPrimaryBg : token.colorBgContainer,
                  border: `1px solid ${rri > 0 ? token.colorPrimaryBorder : token.colorBorder}`,
                }}
              >
                <Text style={{ fontSize: 12, color: rri > 0 ? token.colorText : token.colorTextDisabled }}>
                  {rri > 0 ? `${rri}ms` : '--'}
                </Text>
              </div>
            ))}
          </div>
        </Space>
      </div>

      {vitalSigns.hrvConfidence !== null && (
        <div style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 2 }}>
            <Text type="secondary" style={{ fontSize: 11 }}>{t('monitor.confidence')}</Text>
            <Text style={{ fontSize: 11, color: getConfidenceColor(vitalSigns.hrvConfidence) }}>
              {getConfidenceText(vitalSigns.hrvConfidence)}
            </Text>
          </div>
          <Progress
            percent={vitalSigns.hrvConfidence}
            size="small"
            showInfo={false}
            strokeColor={getConfidenceColor(vitalSigns.hrvConfidence)}
            trailColor={token.colorBorderSecondary}
          />
        </div>
      )}
    </Card>
  );
};

export default HrvCard;
