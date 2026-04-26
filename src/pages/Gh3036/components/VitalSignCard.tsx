import React from 'react';
import { Card, Typography, theme, Progress, Button, Space } from 'antd';
import { SettingOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';

const { Text } = Typography;

interface VitalSignCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  status?: 'normal' | 'warning' | 'error' | 'success';
  icon?: React.ReactNode;
  confidence?: number | null;
  subValue?: number | null;
  subLabel?: string;
  onConfig?: () => void;
  configLabel?: string;
}

const VitalSignCard: React.FC<VitalSignCardProps> = ({
  title,
  value,
  unit,
  status = 'normal',
  icon,
  confidence,
  subValue,
  subLabel,
  onConfig,
  configLabel,
}) => {
  const { token } = theme.useToken();
  const { t } = useTranslation('gh3036');

  const getStatusColor = () => {
    switch (status) {
      case 'success':
        return token.colorSuccess;
      case 'warning':
        return token.colorWarning;
      case 'error':
        return token.colorError;
      default:
        return token.colorPrimary;
    }
  };

  const displayValue = value !== null ? value : '--';
  const displayUnit = unit || '';

  const getConfidenceColor = (conf: number) => {
    if (conf >= 80) return token.colorSuccess;
    if (conf >= 50) return token.colorWarning;
    return token.colorError;
  };

  return (
    <Card
      size="small"
      style={{
        height: '100%',
        borderLeft: `3px solid ${getStatusColor()}`,
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
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {icon && <span style={{ color: getStatusColor() }}>{icon}</span>}
          <Text type="secondary" style={{ fontSize: 12 }}>
            {title}
          </Text>
        </div>
        {onConfig && (
          <Button
            type="text"
            size="small"
            icon={<SettingOutlined />}
            onClick={onConfig}
            title={configLabel || t('monitor.configRef')}
            style={{ color: token.colorTextSecondary }}
          />
        )}
      </div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 4 }}>
        <Text
          strong
          style={{
            fontSize: 24,
            color: value !== null ? token.colorText : token.colorTextDisabled,
          }}
        >
          {displayValue}
        </Text>
        {displayUnit && (
          <Text type="secondary" style={{ fontSize: 12 }}>
            {displayUnit}
          </Text>
        )}
        {subValue !== null && subValue !== undefined && subLabel && (
          <Text type="secondary" style={{ fontSize: 12, marginLeft: 8 }}>
            {subLabel}: {(subValue / 10000).toFixed(4)}
          </Text>
        )}
      </div>
      {confidence !== null && confidence !== undefined && (
        <div style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 2 }}>
            <Text type="secondary" style={{ fontSize: 11 }}>{t('monitor.confidence')}</Text>
            <Text style={{ fontSize: 11, color: getConfidenceColor(confidence) }}>
              {confidence.toFixed(0)}%
            </Text>
          </div>
          <Progress
            percent={confidence}
            size="small"
            showInfo={false}
            strokeColor={getConfidenceColor(confidence)}
            trailColor={token.colorBorderSecondary}
          />
        </div>
      )}
    </Card>
  );
};

export default VitalSignCard;
