import React from 'react';
import { Card, Typography, theme, Divider } from 'antd';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../../stores/gh3036Store';

const { Text } = Typography;

const StatusCombinedCard: React.FC = () => {
  const { token } = theme.useToken();
  const { t } = useTranslation('gh3036');
  const { vitalSigns } = useGh3036Store();

  const getAdtStatus = (): 'normal' | 'success' | 'warning' => {
    if (vitalSigns.adt === null) return 'normal';
    return vitalSigns.adt === 1 ? 'success' : 'warning';
  };

  const getGnadtStatus = (): 'normal' | 'success' | 'error' => {
    if (vitalSigns.gnadt === null) return 'normal';
    const gnadtValue = vitalSigns.gnadt & 0x03;
    return gnadtValue === 1 ? 'success' : 'error';
  };

  const getAdtDisplayValue = (): string => {
    if (vitalSigns.adt === null) return '--';
    return vitalSigns.adt === 1 ? t('monitor.adtWear') : t('monitor.adtNotWear');
  };

  const getGnadtDisplayValue = (): string => {
    if (vitalSigns.gnadt === null) return '--';
    const gnadtValue = vitalSigns.gnadt & 0x03;
    return gnadtValue === 1 ? t('monitor.gnadtLive') : t('monitor.gnadtNotLive');
  };

  const getAdtColor = () => {
    switch (getAdtStatus()) {
      case 'success':
        return token.colorSuccess;
      case 'warning':
        return token.colorWarning;
      default:
        return token.colorTextDisabled;
    }
  };

  const getGnadtColor = () => {
    switch (getGnadtStatus()) {
      case 'success':
        return token.colorSuccess;
      case 'error':
        return token.colorError;
      default:
        return token.colorTextDisabled;
    }
  };

  return (
    <Card
      size="small"
      style={{
        height: '100%',
      }}
      styles={{
        body: {
          padding: 0,
          height: 'calc(100% - 1px)',
        },
      }}
    >
      <div style={{ display: 'flex', height: '100%' }}>
        <div
          style={{
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'center',
            alignItems: 'center',
            padding: '12px 8px',
            borderLeft: `3px solid ${getAdtColor()}`,
          }}
        >
          <Text type="secondary" style={{ fontSize: 11, marginBottom: 4 }}>
            {t('monitor.wearStatus')}
          </Text>
          <Text
            strong
            style={{
              fontSize: 14,
              color: vitalSigns.adt !== null ? getAdtColor() : token.colorTextDisabled,
            }}
          >
            {getAdtDisplayValue()}
          </Text>
        </div>
        <Divider type="vertical" style={{ height: 'auto', margin: '8px 0' }} />
        <div
          style={{
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'center',
            alignItems: 'center',
            padding: '12px 8px',
            borderRight: `3px solid ${getGnadtColor()}`,
          }}
        >
          <Text type="secondary" style={{ fontSize: 11, marginBottom: 4 }}>
            {t('monitor.livenessStatus')}
          </Text>
          <Text
            strong
            style={{
              fontSize: 14,
              color: vitalSigns.gnadt !== null ? getGnadtColor() : token.colorTextDisabled,
            }}
          >
            {getGnadtDisplayValue()}
          </Text>
        </div>
      </div>
    </Card>
  );
};

export default StatusCombinedCard;
