import React from 'react';
import { theme } from 'antd';

interface LedWidgetProps {
  title: string;
  value: number;
  threshold?: number;
  color?: string;
}

const LedWidget: React.FC<LedWidgetProps> = ({
  title,
  value,
  threshold = 0.5,
  color,
}) => {
  const { token } = theme.useToken();
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
      <span style={{ fontSize: 12, color: token.colorTextSecondary }}>{title}</span>
      <div
        style={{
          width: 40,
          height: 40,
          borderRadius: '50%',
          background: isOn
            ? color || token.colorSuccess
            : token.colorFillSecondary,
          boxShadow: isOn
            ? `0 0 10px ${color || token.colorSuccess}`
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
