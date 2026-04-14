import React from 'react';
import { theme } from 'antd';
import type { WidgetConfig } from '../../../types/dashboard';
import { useDashboardStore } from '../../../stores/dashboardStore';

interface LedWidgetProps {
  config: WidgetConfig;
}

const LedWidget: React.FC<LedWidgetProps> = ({ config }) => {
  const { token } = theme.useToken();
  const { dataBuffer } = useDashboardStore();

  const value = dataBuffer.length > 0 
    ? dataBuffer[dataBuffer.length - 1].values[config.dataKey] ?? 0 
    : 0;
  const threshold = config.min ?? 0.5;
  const isOn = value >= threshold;

  return (
    <div
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 8,
      }}
    >
      <span style={{ fontSize: 12, color: token.colorTextSecondary }}>{config.title}</span>
      <div
        style={{
          width: 40,
          height: 40,
          borderRadius: '50%',
          background: isOn
            ? config.color || token.colorSuccess
            : token.colorFillSecondary,
          boxShadow: isOn
            ? `0 0 10px ${config.color || token.colorSuccess}`
            : 'none',
          transition: 'all 0.3s',
        }}
      />
      <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
        {isOn ? 'ON' : 'OFF'}
      </span>
    </div>
  );
};

export default LedWidget;
