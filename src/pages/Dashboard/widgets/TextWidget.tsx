import React from 'react';
import { Typography, theme } from 'antd';
import type { WidgetConfig } from '../../../types/dashboard';
import { useDashboardStore } from '../../../stores/dashboardStore';

const { Text } = Typography;

interface TextWidgetProps {
  config: WidgetConfig;
}

const TextWidget: React.FC<TextWidgetProps> = ({ config }) => {
  const { token } = theme.useToken();
  const { dataBuffer } = useDashboardStore();

  const lastData = dataBuffer[dataBuffer.length - 1];
  const value = lastData?.values[config.dataKey];

  return (
    <div
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <Text
        style={{
          fontSize: 32,
          fontWeight: 'bold',
          color: config.color || token.colorText,
        }}
      >
        {value !== undefined ? value.toFixed(2) : '--'}
      </Text>
      {config.unit && (
        <Text type="secondary" style={{ fontSize: 14 }}>
          {config.unit}
        </Text>
      )}
    </div>
  );
};

export default TextWidget;
