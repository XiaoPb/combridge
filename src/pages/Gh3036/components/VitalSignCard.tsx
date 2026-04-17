import React from 'react';
import { Card, Typography, theme } from 'antd';

const { Text } = Typography;

interface VitalSignCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  status?: 'normal' | 'warning' | 'error' | 'success';
  icon?: React.ReactNode;
}

const VitalSignCard: React.FC<VitalSignCardProps> = ({
  title,
  value,
  unit,
  status = 'normal',
  icon,
}) => {
  const { token } = theme.useToken();

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
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        {icon && <span style={{ color: getStatusColor() }}>{icon}</span>}
        <Text type="secondary" style={{ fontSize: 12 }}>
          {title}
        </Text>
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
      </div>
    </Card>
  );
};

export default VitalSignCard;
