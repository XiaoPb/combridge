import React from 'react';
import { Typography, theme } from 'antd';

const { Text } = Typography;

interface TextWidgetProps {
  title: string;
  value: number;
  unit?: string;
  color?: string;
}

const TextWidget: React.FC<TextWidgetProps> = ({
  title,
  value,
  unit = '',
  color,
}) => {
  const { token } = theme.useToken();

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
      <Text type="secondary" style={{ fontSize: 12, marginBottom: 4 }}>
        {title}
      </Text>
      <Text
        style={{
          fontSize: 32,
          fontWeight: 'bold',
          color: color || token.colorText,
        }}
      >
        {value.toFixed(2)}
      </Text>
      {unit && (
        <Text type="secondary" style={{ fontSize: 14 }}>
          {unit}
        </Text>
      )}
    </div>
  );
};

export default TextWidget;
